#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmark_proofs;
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod types;
pub mod utils;
pub mod weights;

#[frame_support::pallet]
pub mod pallet {
    use codec::FullCodec;
    use frame_support::traits::{EnsureOrigin, OriginTrait};
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::{ValueQuery, *},
        sp_runtime::traits::{CheckEqual, Hash, MaybeDisplay, SimpleBitOps},
        traits::{fungible, Randomness},
    };
    use frame_system::pallet_prelude::*;
    use scale_info::prelude::fmt::Debug;
    use shp_traits::{
        CommitmentVerifier, MutateChallengeableProvidersInterface, ProofsDealerInterface,
        ReadChallengeableProvidersInterface, TrieMutation, TrieProofDeltaApplier,
    };
    use sp_runtime::{
        traits::{CheckedSub, Convert, Zero},
        Perbill, SaturatedConversion,
    };
    use sp_std::vec::Vec;
    use types::{KeyFor, ProviderIdFor};

    use crate::*;
    use crate::{types::*, weights::*};

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: crate::weights::WeightInfo;

        /// The Providers pallet.
        /// To check if whoever submits a proof is a registered Provider.
        type ProvidersPallet: ReadChallengeableProvidersInterface<
                AccountId = Self::AccountId,
                MerkleHash = Self::MerkleTrieHash,
                Balance = Self::NativeBalance,
            > + MutateChallengeableProvidersInterface<ProviderId = <Self::ProvidersPallet as ReadChallengeableProvidersInterface>::ProviderId, MerkleHash = Self::MerkleTrieHash>;

        /// The type used to verify Merkle Patricia Forest proofs.
        /// This verifies proofs of keys belonging to the Merkle Patricia Forest.
        /// Something that implements the [`CommitmentVerifier`] trait.
        /// The type of the challenge is a hash, and it is expected that a proof will provide the
        /// exact hash if it exists in the forest, or the previous and next hashes if it does not.
        type ForestVerifier: CommitmentVerifier<Commitment = KeyFor<Self>, Challenge = KeyFor<Self>>
            + TrieProofDeltaApplier<
                Self::MerkleTrieHashing,
                Key = KeyFor<Self>,
                Proof = ForestVerifierProofFor<Self>,
            >;

        /// The type used to verify the proof of a specific key within the Merkle Patricia Forest.
        /// While [`Config::ForestVerifier`] verifies that some keys are in the Merkle Patricia Forest, this
        /// verifies specifically a proof for that key. For example, if the keys in the forest
        /// represent files, this would verify the proof for a specific file, and [`Config::ForestVerifier`]
        /// would verify that the file is in the forest.
        /// The type of the challenge is a `[u8; 8]` that actually represents a u64 number, which is
        /// the index of the chunk being challenged.
        type KeyVerifier: CommitmentVerifier<Commitment = KeyFor<Self>, Challenge = KeyFor<Self>>;

        /// Type to access the Balances Pallet.
        type NativeBalance: fungible::Inspect<Self::AccountId>
            + fungible::Mutate<Self::AccountId>
            + fungible::hold::Inspect<Self::AccountId>
            + fungible::hold::Mutate<Self::AccountId>;

        /// Type to access source of randomness.
        type RandomnessProvider: Randomness<Self::Hash, BlockNumberFor<Self>>;

        /// The type for the hashes of Merkle Patricia Forest nodes.
        /// Applies to keys (leaf nodes) and root hashes (root nodes).
        /// Generally a hash (the output of a Hasher).
        type MerkleTrieHash: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Debug
            + MaybeDisplay
            + SimpleBitOps
            + Ord
            + Default
            + Copy
            + CheckEqual
            + AsRef<[u8]>
            + AsMut<[u8]>
            + MaxEncodedLen
            + FullCodec;

        /// The hashing system (algorithm) being used for the Merkle Patricia Forests (e.g. Blake2).
        type MerkleTrieHashing: Hash<Output = Self::MerkleTrieHash> + TypeInfo;

        /// The type to convert a balance to a block number.
        type StakeToBlockNumber: Convert<BalanceFor<Self>, BlockNumberFor<Self>>;

        /// The number of random challenges that are generated per block, using the random seed
        /// generated for that block.
        #[pallet::constant]
        type RandomChallengesPerBlock: Get<u32>;

        /// The maximum number of custom challenges that can be made in a single checkpoint block.
        #[pallet::constant]
        type MaxCustomChallengesPerBlock: Get<u32>;

        /// The number of ticks that challenges history is kept for.
        /// After this many ticks, challenges are removed from [`TickToChallengesSeed`] StorageMap.
        /// A "tick" is usually one block, but some blocks may be skipped due to migrations.
        #[pallet::constant]
        type ChallengeHistoryLength: Get<BlockNumberFor<Self>>;

        /// The length of the `ChallengesQueue` StorageValue.
        /// This is to limit the size of the queue, and therefore the number of
        /// manual challenges that can be made.
        #[pallet::constant]
        type ChallengesQueueLength: Get<u32>;

        /// The number of blocks in between a checkpoint challenges round (i.e. with custom challenges).
        /// This is used to determine when to include the challenges from the `ChallengesQueue` and
        /// `PriorityChallengesQueue` in the `BlockToChallenges` StorageMap. These checkpoint challenge
        /// rounds have to be answered by ALL Providers, and this is enforced by the `submit_proof`
        /// extrinsic.
        ///
        /// WARNING: This period needs to be equal or larger than the challenge period of the smallest
        /// Provider in the network. If the smallest Provider has a challenge period of 10 ticks (blocks),
        /// then the checkpoint challenge period needs to be at least 10 ticks.
        #[pallet::constant]
        type CheckpointChallengePeriod: Get<BlockNumberFor<Self>>;

        /// The ratio to convert staked balance to block period.
        /// This is used to determine the period in which a Provider should submit a proof, based on
        /// their stake. The period is calculated as `StakeToChallengePeriod / stake`, saturating at [`Config::MinChallengePeriod`].
        #[pallet::constant]
        type StakeToChallengePeriod: Get<BalanceFor<Self>>;

        /// The minimum period in which a Provider can be challenged, regardless of their stake.
        #[pallet::constant]
        type MinChallengePeriod: Get<BlockNumberFor<Self>>;

        /// The tolerance in number of ticks (almost equivalent to blocks, but skipping MBM) that
        /// a Provider has to submit a proof, counting from the tick the challenge is emitted for
        /// that Provider.
        ///
        /// For example, if a Provider is supposed to submit a proof for tick `n`, and the tolerance
        /// is set to `t`, then the Provider has to submit a proof for challenges in tick `n`, before
        /// `n + t`.
        #[pallet::constant]
        type ChallengeTicksTolerance: Get<BlockNumberFor<Self>>;

        /// The fee charged for submitting a challenge.
        /// This fee goes to the Treasury, and is used to prevent spam. Registered Providers are
        /// exempt from this fee.
        #[pallet::constant]
        type ChallengesFee: Get<BalanceFor<Self>>;

        /// The fee charged for submitting a priority challenge.
        /// This fee goes to the Treasury, and is used to prevent spam.
        #[pallet::constant]
        type PriorityChallengesFee: Get<BalanceFor<Self>>;

        /// The target number of ticks for which to store the submitters that submitted valid proofs in them,
        /// stored in the `ValidProofSubmittersLastTicks` StorageMap. That storage will be trimmed down to this number
        /// of ticks in the `on_idle` hook of this pallet, to avoid bloating the state.
        #[pallet::constant]
        type TargetTicksStorageOfSubmitters: Get<u32>;

        /// The maximum amount of Providers that can submit a proof in a single block.
        /// Although this can be seen as an arbitrary limit, if set to the already existing
        /// implicit limit that is "how many `submit_proof` extrinsics fit in the weight of
        /// a block, this wouldn't add any additional artificial limit.
        #[pallet::constant]
        type MaxSubmittersPerTick: Get<u32>;

        /// The Treasury AccountId.
        /// The account to which:
        /// - The fees for submitting a challenge are transferred.
        /// - The slashed funds are transferred.
        #[pallet::constant]
        type Treasury: Get<Self::AccountId>;

        /// The period of blocks for which the block fullness is checked.
        ///
        /// This is the amount of blocks from the past, for which the block fullness has been checked
        /// and is stored. Blocks older than `current_block` - [`Config::BlockFullnessPeriod`] are
        /// cleared from storage.
        ///
        /// This constant should be equal or smaller than the [`Config::ChallengeTicksTolerance`] constant,
        /// if the goal is to prevent spamming attacks that would prevent honest Providers from submitting
        /// their proofs in time.
        #[pallet::constant]
        type BlockFullnessPeriod: Get<u32>;

        /// The minimum unused weight that a block must have to be considered _not_ full.
        ///
        /// This is used as part of the criteria for checking if the network is presumably under a spam attack.
        /// For example, this can be set to the benchmarked weight of a `submit_proof` extrinsic, which would
        /// mean that a block is not considered full if a `submit_proof` extrinsic could have still fit in it.
        #[pallet::constant]
        type BlockFullnessHeadroom: Get<Weight>;

        /// The minimum ratio (or percentage if you will) of blocks that must be considered _not_ full,
        /// from the total number of [`Config::BlockFullnessPeriod`] blocks taken into account.
        ///
        /// If less than this percentage of blocks are not full, the networks is considered to be presumably
        /// under a spam attack.
        /// This can also be thought of as the maximum ratio of misbehaving collators tolerated. For example,
        /// if this is set to `Perbill::from_percent(50)`, then if more than half of the last [`Config::BlockFullnessPeriod`]
        /// blocks are not full, then one of those blocks surely was produced by an honest collator, meaning
        /// that there was at least one truly _not_ full block in the last [`Config::BlockFullnessPeriod`] blocks.
        #[pallet::constant]
        type MinNotFullBlocksRatio: Get<Perbill>;

        /// The maximum number of Providers that can be slashed per tick.
        ///
        /// Providers are marked as slashable if they are found in the [`TickToProvidersDeadlines`] StorageMap
        /// for the current challenges tick. It is expected that most of the times, there will be little to
        /// no Providers in the [`TickToProvidersDeadlines`] StorageMap for the current challenges tick. That
        /// is because Providers are expected to submit proofs in time. However, in the extreme scenario where
        /// a large number of Providers are missing the proof submissions, this configuration is used to keep
        /// the execution of the `on_poll` hook bounded.
        #[pallet::constant]
        type MaxSlashableProvidersPerTick: Get<u32>;

        /// Custom origin that can dispatch new priority challenges.
        type PriorityChallengeOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Custom origin that can dispatch regular challenges.
        type ChallengeOrigin: EnsureOrigin<Self::RuntimeOrigin>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// A mapping from challenges tick to a random seed used for generating the challenges in that tick.
    ///
    /// This is used to keep track of the challenges' seed in the past.
    /// This mapping goes back only [`ChallengeHistoryLengthFor`] blocks. Previous challenges are removed.
    #[pallet::storage]
    pub type TickToChallengesSeed<T: Config> =
        StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, RandomnessOutputFor<T>>;

    /// A mapping from challenges tick to a vector of custom challenged keys for that tick.
    ///
    /// This is used to keep track of the challenges that have been made in the past, specifically
    /// in the checkpoint challenge rounds.
    /// The vector is bounded by [`MaxCustomChallengesPerBlockFor`].
    /// This mapping goes back only [`ChallengeHistoryLengthFor`] ticks. Previous challenges are removed.
    #[pallet::storage]
    pub type TickToCheckpointChallenges<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<CustomChallenge<T>, MaxCustomChallengesPerBlockFor<T>>,
    >;

    /// The challenge tick of the last checkpoint challenge round.
    ///
    /// This is used to determine when to include the challenges from the [`ChallengesQueue`] and
    /// [`PriorityChallengesQueue`] in the [`TickToCheckpointChallenges`] StorageMap. These checkpoint
    /// challenge rounds have to be answered by ALL Providers, and this is enforced by the
    /// `submit_proof` extrinsic.
    #[pallet::storage]
    pub type LastCheckpointTick<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    /// A mapping from challenge tick to a vector of challenged Providers for that tick.
    ///
    /// This is used to keep track of the Providers that have been challenged, and should
    /// submit a proof by the time of the [`ChallengesTicker`] reaches the number used as
    /// key in the mapping. Providers who do submit a proof are removed from their respective
    /// entry and pushed forward to the next tick in which they should submit a proof.
    /// Those who are still in the entry by the time the tick is reached are considered to
    /// have failed to submit a proof and subject to slashing.
    #[pallet::storage]
    pub type TickToProvidersDeadlines<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        Blake2_128Concat,
        ProviderIdFor<T>,
        (),
    >;

    /// A mapping from a Provider to its [`ProofSubmissionRecord`], which stores the last tick
    /// the Provider submitted a proof for, and the next tick the Provider should submit a proof for.
    ///
    /// Normally the difference between these two ticks is equal to the Provider's challenge period,
    /// but if the Provider's period is changed, this change only affects the next cycle. In other words,
    /// for one cycle, `next_tick_to_submit_proof_for - last_tick_proven ≠ provider_challenge_period`.
    ///
    /// If a Provider submits a proof successfully, both fields are updated.
    ///
    /// If the Provider fails to submit a proof in time and is slashed, only `next_tick_to_submit_proof_for`
    /// is updated.
    #[pallet::storage]
    pub type ProviderToProofSubmissionRecord<T: Config> =
        StorageMap<_, Blake2_128Concat, ProviderIdFor<T>, ProofSubmissionRecord<T>>;

    /// A queue of keys that have been challenged manually.
    ///
    /// The elements in this queue will be challenged in the coming blocks,
    /// always ensuring that the maximum number of challenges per block is not exceeded.
    /// A `BoundedVec` is used because the `parity_scale_codec::MaxEncodedLen` trait
    /// is required, but using a `VecDeque` would be more efficient as this is a FIFO queue.
    #[pallet::storage]
    pub type ChallengesQueue<T: Config> =
        StorageValue<_, BoundedVec<KeyFor<T>, ChallengesQueueLengthFor<T>>, ValueQuery>;

    /// A priority queue of keys that have been challenged manually.
    ///
    /// The difference between this and `ChallengesQueue` is that the challenges
    /// in this queue are given priority over the others. So this queue should be
    /// emptied before any of the challenges in the `ChallengesQueue` are dispatched.
    /// This queue should not be accessible to the public.
    /// The elements in this queue will be challenged in the coming blocks,
    /// always ensuring that the maximum number of challenges per block is not exceeded.
    /// A `BoundedVec` is used because the `parity_scale_codec::MaxEncodedLen` trait
    /// is required, but using a `VecDeque` would be more efficient as this is a FIFO queue.
    #[pallet::storage]
    pub type PriorityChallengesQueue<T: Config> =
        StorageValue<_, BoundedVec<CustomChallenge<T>, ChallengesQueueLengthFor<T>>, ValueQuery>;

    /// A counter of blocks in which challenges were distributed.
    ///
    /// This counter is not necessarily the same as the block number, as challenges are
    /// distributed in the `on_poll` hook, which happens at the beginning of every block,
    /// so long as the block is not part of a [Multi-Block-Migration](https://github.com/paritytech/polkadot-sdk/pull/1781) (MBM).
    /// During MBMsm, the block number increases, but [`ChallengesTicker`] does not.
    #[pallet::storage]
    pub type ChallengesTicker<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    #[pallet::storage]
    pub type SlashableProviders<T: Config> = StorageMap<_, Blake2_128Concat, ProviderIdFor<T>, u32>;

    /// A mapping from tick to Providers, which is set if the Provider submitted a valid proof in that tick.
    ///
    /// This is used to keep track of the Providers that have submitted proofs in the last few
    /// ticks, where availability only up to the last [`Config::TargetTicksStorageOfSubmitters`] ticks is guaranteed.
    /// This storage is then made available for other pallets to use through the `ProofSubmittersInterface`.
    #[pallet::storage]
    pub type ValidProofSubmittersLastTicks<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedBTreeSet<ProviderIdFor<T>, T::MaxSubmittersPerTick>,
    >;

    /// A value that represents the last tick that was deleted from the [`ValidProofSubmittersLastTicks`] StorageMap.
    ///
    /// This is used to know which tick to delete from the [`ValidProofSubmittersLastTicks`] StorageMap when the
    /// `on_idle` hook is called.
    #[pallet::storage]
    pub type LastDeletedTick<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    /// A boolean that represents whether the [`ChallengesTicker`] is paused.
    ///
    /// By default, this is `false`, meaning that the [`ChallengesTicker`] is incremented every time `on_poll` is called.
    /// This can be set to `true` which would pause the [`ChallengesTicker`], preventing `do_new_challenges_round` from
    /// being executed. Therefore:
    /// - No new random challenges would be emitted and added to [`TickToChallengesSeed`].
    /// - No new checkpoint challenges would be emitted and added to [`TickToCheckpointChallenges`].
    /// - Deadlines for proof submissions are indefinitely postponed.
    #[pallet::storage]
    pub type ChallengesTickerPaused<T: Config> = StorageValue<_, ()>;

    /// A mapping from block number to the weight used in that block.
    ///
    /// This is used to check if the network is presumably under a spam attack.
    /// It is cleared for blocks older than `current_block` - ([`Config::BlockFullnessPeriod`] + 1).
    #[pallet::storage]
    pub type PastBlocksWeight<T: Config> =
        StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, Weight>;

    /// The vector holding whether the last [`Config::BlockFullnessPeriod`] blocks were full or not.
    ///
    /// Each element in the vector represents a block, and is `true` if the block was full, and `false` otherwise.
    /// Note: Ideally we would use a `BitVec` to reduce storage, but since there's no bounded `BitVec` implementation
    /// we use a BoundedVec<bool> instead. This uses 7 more bits of storage per element.
    #[pallet::storage]
    pub type PastBlocksStatus<T: Config> =
        StorageValue<_, BoundedVec<bool, BlockFullnessPeriodFor<T>>, ValueQuery>;

    /// The tick to check and see if Providers failed to submit proofs before their deadline.
    ///
    /// In a normal situation, this should always be equal to [`ChallengesTicker`].
    /// However, in the unlikely scenario where a large number of Providers fail to submit proofs (larger
    /// than [`Config::MaxSlashableProvidersPerTick`]), and all of them had the same deadline, not all of
    /// them will be marked as slashable. Only the first [`Config::MaxSlashableProvidersPerTick`] will be.
    /// In that case, this stored tick will lag behind [`ChallengesTicker`].
    ///
    /// It is expected that this tick should catch up to [`ChallengesTicker`], as blocks with less
    /// slashable Providers follow.
    #[pallet::storage]
    pub type TickToCheckForSlashableProviders<T: Config> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        #[serde(skip)]
        pub _phantom: sp_std::marker::PhantomData<T>,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            // Start with an empty vector of checkpoint challenges.
            Self {
                _phantom: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            TickToCheckpointChallenges::<T>::insert(
                &BlockNumberFor::<T>::zero(),
                BoundedVec::<CustomChallenge<T>, MaxCustomChallengesPerBlockFor<T>>::default(),
            );
        }
    }

    // Pallets use events to inform users when important changes are made.
    // https://docs.substrate.io/v3/runtime/events-and-errors
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A manual challenge was submitted.
        NewChallenge {
            who: Option<AccountIdFor<T>>,
            key_challenged: KeyFor<T>,
        },

        /// A priority challenge was submitted.
        NewPriorityChallenge {
            who: Option<AccountIdFor<T>>,
            key_challenged: KeyFor<T>,
            should_remove_key: bool,
        },

        /// A proof was accepted.
        ProofAccepted {
            provider_id: ProviderIdFor<T>,
            proof: Proof<T>,
            last_tick_proven: BlockNumberFor<T>,
        },

        /// A new challenge seed was generated.
        NewChallengeSeed {
            challenges_ticker: BlockNumberFor<T>,
            seed: RandomnessOutputFor<T>,
        },

        /// A new checkpoint challenge was generated.
        NewCheckpointChallenge {
            challenges_ticker: BlockNumberFor<T>,
            challenges: BoundedVec<CustomChallenge<T>, MaxCustomChallengesPerBlockFor<T>>,
        },

        /// A provider was marked as slashable and their challenge deadline was forcefully pushed.
        SlashableProvider {
            provider: ProviderIdFor<T>,
            next_challenge_deadline: BlockNumberFor<T>,
        },

        /// No record of the last tick the Provider submitted a proof for.
        NoRecordOfLastSubmittedProof { provider: ProviderIdFor<T> },

        /// A provider's challenge cycle was initialised.
        NewChallengeCycleInitialised {
            current_tick: BlockNumberFor<T>,
            next_challenge_deadline: BlockNumberFor<T>,
            provider: ProviderIdFor<T>,
            maybe_provider_account: Option<T::AccountId>,
        },

        /// A set of mutations has been applied to the Forest of a given Provider.
        MutationsAppliedForProvider {
            provider_id: ProviderIdFor<T>,
            mutations: Vec<(KeyFor<T>, TrieMutation)>,
            old_root: KeyFor<T>,
            new_root: KeyFor<T>,
        },

        /// A set of mutations has been applied to a given Forest.
        /// This is the generic version of [`MutationsAppliedForProvider`](Event::MutationsAppliedForProvider)
        /// when [`generic_apply_delta`](ProofsDealerInterface::generic_apply_delta) is used
        /// and the root is not necessarily linked to a specific Provider.
        ///
        /// Additional information for context on where the mutations were applied can be provided
        /// by using the `event_info` field.
        MutationsApplied {
            mutations: Vec<(KeyFor<T>, TrieMutation)>,
            old_root: KeyFor<T>,
            new_root: KeyFor<T>,
            event_info: Option<Vec<u8>>,
        },

        /// The [`ChallengesTicker`] has been paused or unpaused.
        ChallengesTickerSet { paused: bool },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// General errors

        /// The proof submitter is not a registered Provider.
        NotProvider,

        /// `challenge` extrinsic errors

        /// The ChallengesQueue is full. No more manual challenges can be made
        /// until some of the challenges in the queue are dispatched.
        ChallengesQueueOverflow,

        /// The PriorityChallengesQueue is full. No more priority challenges can be made
        /// until some of the challenges in the queue are dispatched.
        PriorityChallengesQueueOverflow,

        /// The fee for submitting a challenge could not be charged.
        FeeChargeFailed,

        /// `submit_proof` extrinsic errors

        /// There are no key proofs submitted.
        EmptyKeyProofs,

        /// The root for the Provider could not be found.
        ProviderRootNotFound,

        /// Provider is submitting a proof when they have a zero root.
        /// Providers with zero roots are not providing any service, so they should not be
        /// submitting proofs.
        ZeroRoot,

        /// Provider is submitting a proof but there is no record of the last tick they
        /// submitted a proof for.
        /// Providers who are required to submit proofs should always have a record of the
        /// last tick they submitted a proof for, otherwise it means they haven't started
        /// providing service for any user yet.
        NoRecordOfLastSubmittedProof,

        /// The provider stake could not be found.
        ProviderStakeNotFound,

        /// Provider is submitting a proof but their stake is zero.
        ZeroStake,

        /// The staked balance of the Provider could not be converted to `u128`.
        /// This should not be possible, as the `Balance` type should be an unsigned integer type.
        StakeCouldNotBeConverted,

        /// Provider is submitting a proof for a tick in the future.
        ChallengesTickNotReached,

        /// Provider is submitting a proof for a tick before the last tick this pallet registers
        /// challenges for.
        ChallengesTickTooOld,

        /// Provider is submitting a proof for a tick too late, i.e. that the challenges tick
        /// is greater or equal than `challenges_tick` + `T::ChallengeTicksTolerance::get()`.
        ChallengesTickTooLate,

        /// The seed for the tick could not be found.
        /// This should not be possible for a tick within the `ChallengeHistoryLength` range, as
        /// seeds are generated for all ticks, and stored within this range.
        SeedNotFound,

        /// Checkpoint challenges not found in block.
        /// This should only be possible if `TickToCheckpointChallenges` is dereferenced for a tick
        /// that is not a checkpoint tick.
        CheckpointChallengesNotFound,

        /// The forest proof submitted by the Provider is invalid.
        /// This could be because the proof is not valid for the root, or because the proof is
        /// not sufficient for the challenges made.
        ForestProofVerificationFailed,

        /// The number of key proofs submitted does not match the number of keys proven in the forest proof.
        IncorrectNumberOfKeyProofs,

        /// There is at least one key proven in the forest proof, that does not have a corresponding
        /// key proof.
        KeyProofNotFound,

        /// A key proof submitted by the Provider is invalid.
        /// This could be because the proof is not valid for the root of that key, or because the proof
        /// is not sufficient for the challenges made.
        KeyProofVerificationFailed,

        /// Failed to apply delta to the forest proof partial trie.
        FailedToApplyDelta,

        /// After successfully applying delta for a set of mutations, the number of mutated keys is
        /// not the same as the number of mutations expected to have been applied.
        UnexpectedNumberOfRemoveMutations,

        /// Failed to update the provider after a key removal mutation.
        FailedToUpdateProviderAfterKeyRemoval,

        /// The limit of Providers that can submit a proof in a single tick has been reached.
        TooManyValidProofSubmitters,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Introduce a new challenge.
        ///
        /// This function allows authorized origins to add a new challenge to the `ChallengesQueue`.
        /// The challenge will be dispatched in the coming blocks.
        /// Users are charged a small fee for submitting a challenge, which
        /// goes to the Treasury.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::challenge())]
        pub fn challenge(origin: OriginFor<T>, key: KeyFor<T>) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was executed by the custom origin.
            T::ChallengeOrigin::ensure_origin(origin.clone())?;

            let who = origin.into_signer();

            Self::do_challenge(&who, &key)?;

            // Emit event.
            Self::deposit_event(Event::NewChallenge {
                who,
                key_challenged: key,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// For a Provider to submit a proof.
        ///
        /// Checks that `provider` is a registered Provider. If none
        /// is provided, the proof submitter is considered to be the Provider.
        /// Relies on a Providers pallet to get the root for the Provider.
        /// Validates that the proof corresponds to a challenge that was made in the past,
        /// by checking the [`TickToChallengesSeed`] StorageMap. The challenge tick that the
        /// Provider should be submitting a proof for is retrieved from [`ProviderToProofSubmissionRecord`],
        /// and it was calculated based on the last tick they submitted a proof for, and the challenge
        /// period for that Provider, at the time of the previous proof submission or when it was
        /// marked as slashable.
        ///
        /// This extrinsic also checks that there hasn't been a checkpoint challenge round
        /// in between the last time the Provider submitted a proof for and the tick
        /// for which the proof is being submitted. If there has been, the Provider is
        /// expected to include responses to the checkpoint challenges in the proof.
        ///
        /// If valid:
        /// - Pushes forward the Provider in the [`TickToProvidersDeadlines`] StorageMap a number
        /// of ticks corresponding to the stake of the Provider.
        /// - Registers the last tick for which the Provider submitted a proof for in
        /// [`ProviderToProofSubmissionRecord`], as well as the next tick for which the Provider
        /// should submit a proof for.
        ///
        /// Execution of this extrinsic should be refunded if the proof is valid.
        #[pallet::call_index(1)]
        #[pallet::weight({
            let max_random_key_proofs = T::RandomChallengesPerBlock::get().saturating_mul(2u32.into());
            let max_custom_key_proofs = T::MaxCustomChallengesPerBlock::get().saturating_mul(2u32.into());

            let max_key_proofs = max_random_key_proofs.saturating_add(max_custom_key_proofs);

            let key_proofs_len = SaturatedConversion::saturated_into::<u32>(
                proof.key_proofs.len()
            );
            match key_proofs_len {
                n if n <= max_random_key_proofs => {
                    T::WeightInfo::submit_proof_no_checkpoint_challenges_key_proofs(n)
                }
                n if n <= max_key_proofs => {
                    T::WeightInfo::submit_proof_with_checkpoint_challenges_key_proofs(n)
                }
                // More key proofs than `max_key_proofs` would inevitably fail the transaction.
                n => T::WeightInfo::submit_proof_with_checkpoint_challenges_key_proofs(n),
            }
        })]
        pub fn submit_proof(
            origin: OriginFor<T>,
            proof: Proof<T>,
            provider: Option<ProviderIdFor<T>>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Getting provider from the origin if none is provided.
            let provider = match provider {
                Some(provider) => provider,
                None => {
                    let sp =
                        T::ProvidersPallet::get_provider_id(&who).ok_or(Error::<T>::NotProvider)?;
                    sp
                }
            };

            let last_tick_proven = Self::do_submit_proof(&provider, &proof)?;

            // Emit event.
            Self::deposit_event(Event::ProofAccepted {
                provider_id: provider,
                proof,
                last_tick_proven,
            });

            // Return a successful DispatchResultWithPostInfo.
            // If the proof is valid, the execution of this extrinsic should be refunded. This is
            // to incentivise being a Provider in the network, since it diminishes the costs to be one substantially.
            Ok(Pays::No.into())
        }

        /// Initialise a Provider's challenge cycle.
        ///
        /// Only callable by sudo.
        ///
        /// Sets the last tick the Provider submitted a proof for to the current tick, and sets the
        /// deadline for submitting a proof to the current tick + the Provider's period + the tolerance.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::force_initialise_challenge_cycle())]
        pub fn force_initialise_challenge_cycle(
            origin: OriginFor<T>,
            provider: ProviderIdFor<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was executed by the root origin.
            ensure_root(origin)?;

            // Execute checks and logic, update storage.
            <Self as ProofsDealerInterface>::initialise_challenge_cycle(&provider)?;

            // Return a successful DispatchResultWithPostInfo.
            // This TX is free since is a sudo-only transaction used to fix potential issues
            // and for testing.
            Ok(Pays::No.into())
        }

        /// Set the [`ChallengesTickerPaused`] to `true` or `false`.
        ///
        /// Only callable by sudo.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::set_paused())]
        pub fn set_paused(origin: OriginFor<T>, paused: bool) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was executed by the root origin.
            ensure_root(origin)?;

            if paused {
                ChallengesTickerPaused::<T>::set(Some(()));
            } else {
                ChallengesTickerPaused::<T>::set(None);
            }

            // Emit the corresponding event.
            Self::deposit_event(Event::ChallengesTickerSet { paused });

            // Return a successful DispatchResultWithPostInfo.
            // This TX is free since is a sudo-only transaction used for testing.
            Ok(Pays::No.into())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(Weight::zero())]
        pub fn priority_challenge(
            origin: OriginFor<T>,
            key: KeyFor<T>,
            should_remove_key: bool,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was executed by the custom origin.
            T::PriorityChallengeOrigin::ensure_origin(origin.clone())?;

            let who = origin.into_signer();

            // Execute priority challenge.
            Self::do_priority_challenge(&who, &key, should_remove_key)?;

            // Emit event.
            Self::deposit_event(Event::NewPriorityChallenge {
                who,
                key_challenged: key,
                should_remove_key,
            });

            Ok(().into())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// This hook is used to generate new challenges.
        ///
        /// It will be called at the beginning of every block, if the block is not being part of a
        /// [Multi-Block-Migration](https://github.com/paritytech/polkadot-sdk/pull/1781) (MBM).
        /// For more information on the lifecycle of the block and its hooks, see the [Substrate
        /// documentation](https://paritytech.github.io/polkadot-sdk/master/frame_support/traits/trait.Hooks.html#method.on_poll).
        fn on_poll(_n: BlockNumberFor<T>, weight: &mut sp_weights::WeightMeter) {
            // Only execute the `do_new_challenges_round` if the `ChallengesTicker` is not paused.
            if ChallengesTickerPaused::<T>::get().is_none() {
                Self::do_new_challenges_round(weight);
            }

            // Check if the network is presumably under a spam attack.
            // If so, `ChallengesTicker` will be paused.
            // This check is done "a posteriori", meaning that we first increment the `ChallengesTicker`, send out challenges
            // and slash Providers if in the last block we didn't consider the network to be under spam.
            // Then if at this block we consider the network to be under spam, we pause the `ChallengesTicker`, which will not
            // be incremented in the next block.
            Self::do_check_spamming_condition(weight);
        }

        /// This hook is used to trim down the `ValidProofSubmittersLastTicks` StorageMap up to the `TargetTicksOfProofsStorage`.
        ///
        /// It runs when the block is being finalized (but before the `on_finalize` hook) and can consume all remaining weight.
        /// It returns the used weight, so it can be used to calculate the remaining weight for the block for any other
        /// pallets that have `on_idle` hooks.
        fn on_idle(n: BlockNumberFor<T>, weight: Weight) -> Weight {
            Self::do_trim_valid_proof_submitters_last_ticks(n, weight)
        }

        /// This hook is called on block initialization and returns the Weight of the `on_finalize` hook to
        /// let block builders know how much weight to reserve for it
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            T::WeightInfo::on_finalize()
        }

        fn on_finalize(block_number: BlockNumberFor<T>) {
            // Get weight usage in this block so far, for the dispatch class of `submit_proof` extrinsics.
            let weight_used = frame_system::Pallet::<T>::block_weight();
            let weight_used_for_class = weight_used.get(DispatchClass::Normal);

            // Store the weight usage in this block.
            PastBlocksWeight::<T>::insert(block_number, weight_used_for_class);

            // Get the oldest block weight registered.
            let block_fullness_period = T::BlockFullnessPeriod::get();

            // Clear the storage for block at `current_block` - (`BlockFullnessPeriod` + 1).
            if let Some(oldest_block_fullness_number) =
                block_number.checked_sub(&block_fullness_period.saturating_add(1u32).into())
            {
                // If it is older than `BlockFullnessPeriod` + 1, we clear the storage.
                PastBlocksWeight::<T>::remove(oldest_block_fullness_number);
            }
        }

        /// This integrity test checks that:
        /// 1. `CheckpointChallengePeriod` is greater or equal to the longest period a Provider can have.
        /// 2. `BlockFullnessPeriod` is smaller or equal than `ChallengeTicksTolerance`.
        /// 3. If `ChallengesFee` is greater than 0, then `ChallengeOrigin` cannot be root or none (since root and none cannot be charged fees).
        /// 4. If `PriorityChallengesFee` is greater than 0, then `PriorityChallengeOrigin` cannot be root or none (since root and none cannot be charged fees).
        ///
        /// Any code located in this hook is placed in an auto-generated test, and generated as a part
        /// of crate::construct_runtime's expansion.
        /// Look for a test case with a name along the lines of: __construct_runtime_integrity_test.
        fn integrity_test() {
            // Calculate longest period a Provider can have.
            // That would be the period of the Provider with the minimum stake.
            let min_stake = T::ProvidersPallet::get_min_stake();
            let max_period = Self::stake_to_challenge_period(min_stake);

            // Check that `CheckpointChallengePeriod` is greater or equal to the longest period a Provider can have plus the tolerance.
            assert!(
                T::CheckpointChallengePeriod::get() > max_period + T::ChallengeTicksTolerance::get(),
                "CheckpointChallengePeriod ({:?}) const in ProofsDealer pallet should be greater or equal than the longest period a Provider can have ({:?}).",
                T::CheckpointChallengePeriod::get(),
                max_period
            );

            // Check that `BlockFullnessPeriod` is smaller or equal than `ChallengeTicksTolerance`.
            assert!(
                T::ChallengeTicksTolerance::get() >= T::BlockFullnessPeriod::get().into(),
                "BlockFullnessPeriod const ({:?}) in ProofsDealer pallet should be smaller or equal than ChallengeTicksTolerance ({:?}).",
                T::BlockFullnessPeriod::get(),
                T::ChallengeTicksTolerance::get()
            );

            // Check that if `ChallengeOrigin` allows root or none, then `ChallengesFee` must be zero.
            // This prevents the misconfiguration where a fee is charged but the origin is root or none (which cannot be charged).

            // Test if ChallengeOrigin accepts root origin
            let root_origin = frame_system::RawOrigin::Root.into();
            if T::ChallengeOrigin::try_origin(root_origin).is_ok() {
                assert!(
                    T::ChallengesFee::get().is_zero(),
                    "ChallengesFee must be zero when ChallengeOrigin accepts root, as root cannot be charged fees. Current fee: {:?}",
                    T::ChallengesFee::get()
                );
            }

            // Test if ChallengeOrigin accepts none origin
            let none_origin = frame_system::RawOrigin::None.into();
            if T::ChallengeOrigin::try_origin(none_origin).is_ok() {
                assert!(
                    T::ChallengesFee::get().is_zero(),
                    "ChallengesFee must be zero when ChallengeOrigin accepts none, as none cannot be charged fees. Current fee: {:?}",
                    T::ChallengesFee::get()
                );
            }

            // Check that if `PriorityChallengeOrigin` allows root or none, then `PriorityChallengesFee` must be zero.
            // This prevents the misconfiguration where a fee is charged but the origin is root or none (which cannot be charged).

            // Test if PriorityChallengeOrigin accepts root origin
            let root_origin = frame_system::RawOrigin::Root.into();
            if T::PriorityChallengeOrigin::try_origin(root_origin).is_ok() {
                assert!(
                    T::PriorityChallengesFee::get().is_zero(),
                    "PriorityChallengesFee must be zero when PriorityChallengeOrigin accepts root, as root cannot be charged fees. Current fee: {:?}",
                    T::PriorityChallengesFee::get()
                );
            }

            // Test if PriorityChallengeOrigin accepts none origin
            let none_origin = frame_system::RawOrigin::None.into();
            if T::PriorityChallengeOrigin::try_origin(none_origin).is_ok() {
                assert!(
                    T::PriorityChallengesFee::get().is_zero(),
                    "PriorityChallengesFee must be zero when PriorityChallengeOrigin accepts none, as none cannot be charged fees. Current fee: {:?}",
                    T::PriorityChallengesFee::get()
                );
            }
        }
    }
}
