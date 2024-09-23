#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// TODO #[cfg(feature = "runtime-benchmarks")]
// TODO mod benchmarking;
pub mod types;
pub mod utils;

#[frame_support::pallet]
pub mod pallet {
    use codec::FullCodec;
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
        ReadChallengeableProvidersInterface, TrieProofDeltaApplier, TrieRemoveMutation,
    };
    use sp_runtime::{
        traits::{CheckedSub, Convert, Saturating},
        Perbill,
    };
    use sp_std::vec::Vec;
    use types::{KeyFor, ProviderIdFor};

    use crate::types::*;
    use crate::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

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
        /// The type of the challenge is a `[u8; 8]`` that actually represents a u64 number, which is
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
        type BlockFullnessPeriod: Get<BlockNumberFor<Self>>;

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
        /// if this is set to `Perbill::from_percent(50)`, then if more than half of the last `BlockFullnessPeriod`
        /// blocks are not full, then one of those blocks surely was produced by an honest collator, meaning
        /// that there was at least one truly _not_ full block in the last `BlockFullnessPeriod` blocks.
        #[pallet::constant]
        type MinNotFullBlocksRatio: Get<Perbill>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// A mapping from challenges tick to a random seed used for generating the challenges in that tick.
    ///
    /// This is used to keep track of the challenges' seed in the past.
    /// This mapping goes back only [`ChallengeHistoryLengthFor`] blocks. Previous challenges are removed.
    #[pallet::storage]
    #[pallet::getter(fn tick_to_challenges)]
    pub type TickToChallengesSeed<T: Config> =
        StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, RandomnessOutputFor<T>>;

    /// A mapping from challenges tick to a vector of custom challenged keys for that tick.
    ///
    /// This is used to keep track of the challenges that have been made in the past, specifically
    /// in the checkpoint challenge rounds.
    /// The vector is bounded by [`MaxCustomChallengesPerBlockFor`].
    /// This mapping goes back only [`ChallengeHistoryLengthFor`] ticks. Previous challenges are removed.
    #[pallet::storage]
    #[pallet::getter(fn tick_to_checkpoint_challenges)]
    pub type TickToCheckpointChallenges<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<(KeyFor<T>, Option<TrieRemoveMutation>), MaxCustomChallengesPerBlockFor<T>>,
    >;

    /// The challenge tick of the last checkpoint challenge round.
    ///
    /// This is used to determine when to include the challenges from the [`ChallengesQueue`] and
    /// [`PriorityChallengesQueue`] in the [`TickToCheckpointChallenges`] StorageMap. These checkpoint
    /// challenge rounds have to be answered by ALL Providers, and this is enforced by the
    /// `submit_proof` extrinsic.
    #[pallet::storage]
    #[pallet::getter(fn last_checkpoint_tick)]
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
    #[pallet::getter(fn tick_to_challenged_providers)]
    pub type TickToProvidersDeadlines<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        Blake2_128Concat,
        ProviderIdFor<T>,
        (),
    >;

    /// A mapping from a Provider to the last tick for which they SHOULD have submitted a proof.
    /// If for a Provider `p`, `LastTickProviderSubmittedAProofFor[p]` is `n`, then the
    /// Provider should submit a proof for tick `n + stake_to_challenge_period(p)`.
    ///
    /// This gets updated when a Provider submits a proof successfully and is used to determine the
    /// next tick for which the Provider should submit a proof, and it's deadline.
    ///
    /// If the Provider fails to submit a proof in time and is slashed, this will still get updated
    /// to the tick it should have submitted a proof for.
    #[pallet::storage]
    #[pallet::getter(fn last_tick_provider_submitted_proof_for)]
    pub type LastTickProviderSubmittedAProofFor<T: Config> =
        StorageMap<_, Blake2_128Concat, ProviderIdFor<T>, BlockNumberFor<T>>;

    /// A queue of keys that have been challenged manually.
    ///
    /// The elements in this queue will be challenged in the coming blocks,
    /// always ensuring that the maximum number of challenges per block is not exceeded.
    /// A `BoundedVec` is used because the `parity_scale_codec::MaxEncodedLen` trait
    /// is required, but using a `VecDeque` would be more efficient as this is a FIFO queue.
    #[pallet::storage]
    #[pallet::getter(fn challenges_queue)]
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
    #[pallet::getter(fn priority_challenges_queue)]
    pub type PriorityChallengesQueue<T: Config> = StorageValue<
        _,
        BoundedVec<(KeyFor<T>, Option<TrieRemoveMutation>), ChallengesQueueLengthFor<T>>,
        ValueQuery,
    >;

    /// A counter of blocks in which challenges were distributed.
    ///
    /// This counter is not necessarily the same as the block number, as challenges are
    /// distributed in the `on_poll` hook, which happens at the beginning of every block,
    /// so long as the block is not part of a [Multi-Block-Migration](https://github.com/paritytech/polkadot-sdk/pull/1781) (MBM).
    /// During MBMsm, the block number increases, but [`ChallengesTicker`] does not.
    #[pallet::storage]
    #[pallet::getter(fn challenges_ticker)]
    pub type ChallengesTicker<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn slashable_providers)]
    pub type SlashableProviders<T: Config> = StorageMap<_, Blake2_128Concat, ProviderIdFor<T>, u32>;

    /// A mapping from tick to Providers, which is set if the Provider submitted a valid proof in that tick.
    ///
    /// This is used to keep track of the Providers that have submitted proofs in the last few
    /// ticks, where availability only up to the last [`Config::TargetTicksStorageOfSubmitters`] ticks is guaranteed.
    /// This storage is then made available for other pallets to use through the `ProofSubmittersInterface`.
    #[pallet::storage]
    #[pallet::getter(fn valid_proof_submitters_last_ticks)]
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
    #[pallet::getter(fn last_deleted_tick)]
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
    #[pallet::getter(fn challenges_ticker_paused)]
    pub type ChallengesTickerPaused<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// A mapping from block number to the weight used in that block.
    ///
    /// This is used to check if the network is presumably under a spam attack.
    /// It is cleared for blocks older than `current_block` - ([`Config::BlockFullnessPeriod`] + 1).
    #[pallet::storage]
    #[pallet::getter(fn past_blocks_fullness)]
    pub type PastBlocksWeight<T: Config> =
        StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, Weight>;

    /// The number of blocks that have been considered _not_ full in the last [`Config::BlockFullnessPeriod`].
    ///
    /// This is used to check if the network is presumably under a spam attack.
    #[pallet::storage]
    #[pallet::getter(fn not_full_blocks_count)]
    pub type NotFullBlocksCount<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    // Pallets use events to inform users when important changes are made.
    // https://docs.substrate.io/v3/runtime/events-and-errors
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A manual challenge was submitted.
        NewChallenge {
            who: AccountIdFor<T>,
            key_challenged: KeyFor<T>,
        },

        /// A proof was accepted.
        ProofAccepted {
            provider: ProviderIdFor<T>,
            proof: Proof<T>,
        },

        /// A new challenge seed was generated.
        NewChallengeSeed {
            challenges_ticker: BlockNumberFor<T>,
            seed: RandomnessOutputFor<T>,
        },

        /// A new checkpoint challenge was generated.
        NewCheckpointChallenge {
            challenges_ticker: BlockNumberFor<T>,
            challenges: BoundedVec<
                (KeyFor<T>, Option<TrieRemoveMutation>),
                MaxCustomChallengesPerBlockFor<T>,
            >,
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

        /// A set of mutations has been applied to the Forest.
        MutationsApplied {
            provider: ProviderIdFor<T>,
            mutations: Vec<(KeyFor<T>, TrieRemoveMutation)>,
            new_root: KeyFor<T>,
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

        /// There is at least one key proven in the forest proof, that does not have a corresponding
        /// key proof.
        KeyProofNotFound,

        /// A key proof submitted by the Provider is invalid.
        /// This could be because the proof is not valid for the root of that key, or because the proof
        /// is not sufficient for the challenges made.
        KeyProofVerificationFailed,

        /// Failed to apply delta to the forest proof partial trie.
        FailedToApplyDelta,

        /// Failed to update the provider after a key removal mutation.
        FailedToUpdateProviderAfterKeyRemoval,

        /// The limit of Providers that can submit a proof in a single tick has been reached.
        TooManyValidProofSubmitters,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Introduce a new challenge.
        ///
        /// This function allows anyone to add a new challenge to the `ChallengesQueue`.
        /// The challenge will be dispatched in the coming blocks.
        /// Regular users are charged a small fee for submitting a challenge, which
        /// goes to the Treasury. Unless the one calling is a registered Provider.
        ///
        /// TODO: Consider checking also if there was a request to change MSP.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn challenge(origin: OriginFor<T>, key: KeyFor<T>) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

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
        /// by checking the `TickToChallengesSeed` StorageMap. The challenge tick that the
        /// Provider should have submitted a proof is calculated based on the last tick they
        /// submitted a proof for ([`LastTickProviderSubmittedAProofFor`]), and the proving period for
        /// that Provider, which is a function of their stake.
        /// This extrinsic also checks that there hasn't been a checkpoint challenge round
        /// in between the last time the Provider submitted a proof for and the tick
        /// for which the proof is being submitted. If there has been, the Provider is
        /// subject to slashing.
        ///
        /// If valid:
        /// - Pushes forward the Provider in the [`TickToProvidersDeadlines`] StorageMap a number
        /// of ticks corresponding to the stake of the Provider.
        /// - Registers this tick as the last tick in which the Provider submitted a proof.
        ///
        /// Execution of this extrinsic should be refunded if the proof is valid.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
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
                    let sp = T::ProvidersPallet::get_provider_id(who.clone())
                        .ok_or(Error::<T>::NotProvider)?;
                    sp
                }
            };

            Self::do_submit_proof(&provider, &proof)?;

            // Emit event.
            Self::deposit_event(Event::ProofAccepted { provider, proof });

            // Return a successful DispatchResultWithPostInfo.
            // If the proof is valid, the execution of this extrinsic should be refunded.
            Ok(Pays::No.into())
        }

        /// Initialise a Provider's challenge cycle.
        ///
        /// Only callable by sudo.
        ///
        /// Sets the last tick the Provider submitted a proof for to the current tick, and sets the
        /// deadline for submitting a proof to the current tick + the Provider's period + the tolerance.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn force_initialise_challenge_cycle(
            origin: OriginFor<T>,
            provider: ProviderIdFor<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was executed by the root origin.
            ensure_root(origin)?;

            // Execute checks and logic, update storage.
            <Self as ProofsDealerInterface>::initialise_challenge_cycle(&provider)?;

            // Return a successful DispatchResultWithPostInfo.
            Ok(Pays::No.into())
        }

        /// Set the [`ChallengesTickerPaused`] to `true` or `false`.
        ///
        /// Only callable by sudo.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn set_paused(origin: OriginFor<T>, paused: bool) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was executed by the root origin.
            ensure_root(origin)?;

            ChallengesTickerPaused::<T>::set(paused);

            // Emit the corresponding event.
            Self::deposit_event(Event::<T>::ChallengesTickerSet { paused });

            // Return a successful DispatchResultWithPostInfo.
            Ok(Pays::No.into())
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
        fn on_poll(_n: BlockNumberFor<T>, weight: &mut frame_support::weights::WeightMeter) {
            // TODO: Benchmark computational weight cost of this hook.

            // Only execute the `do_new_challenges_round` if the `ChallengesTicker` is not paused.
            if !ChallengesTickerPaused::<T>::get() {
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

        /// This hook is called on block initialization and returns the Weight of the `on_finalize` hook to
        /// let block builders know how much weight to reserve for it
        /// TODO: Benchmark on_finalize to get its weight and replace the placeholder weight for that
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(0, 2)
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
                block_number.checked_sub(&block_fullness_period.saturating_add(1u32.into()))
            {
                // If it is older than `BlockFullnessPeriod` + 1, we clear the storage.
                PastBlocksWeight::<T>::remove(oldest_block_fullness_number);
            }
        }

        // TODO: Document why we need to do this.
        // TODO: This is related to the limitation of `CheckpointChallengePeriod` having to be greater or equal
        // TODO: to the largest period of a Provider. The provider with largest period would be the one with the
        // TODO: smallest stake.
        fn integrity_test() {
            // TODO: Check that the `CheckpointChallengePeriod` is greater or equal to the largest period of a Provider. plus `ChallengeTicksTolerance`.
            // TODO: Check that `BlockFullnessPeriod` is smaller or equal than `CheckpointChallengePeriod`.
        }

        /// This hook is used to trim down the `ValidProofSubmittersLastTicks` StorageMap up to the `TargetTicksOfProofsStorage`.
        ///
        /// It runs when the block is being finalized (but before the `on_finalize` hook) and can consume all remaining weight.
        /// It returns the used weight, so it can be used to calculate the remaining weight for the block for any other
        /// pallets that have `on_idle` hooks.
        fn on_idle(n: BlockNumberFor<T>, weight: Weight) -> Weight {
            // TODO: Benchmark computational and proof size weight cost of this hook.
            Self::do_trim_valid_proof_submitters_last_ticks(n, weight)
        }
    }
}
