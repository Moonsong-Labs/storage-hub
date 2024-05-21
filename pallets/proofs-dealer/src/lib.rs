// TODO: Remove this attribute.
#![allow(unused_variables)]
#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
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
    use sp_runtime::traits::Convert;
    use storage_hub_traits::{CommitmentVerifier, ProvidersInterface};
    use types::{KeyFor, ProviderFor};

    use crate::types::*;
    use crate::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The Providers pallet.
        /// To check if whoever submits a proof is a registered Provider.
        type ProvidersPallet: ProvidersInterface<
            AccountId = Self::AccountId,
            MerkleHash = Self::MerkleHash,
            Balance = Self::NativeBalance,
        >;

        /// The type used to verify Merkle Patricia Forest proofs.
        /// This verifies proofs of keys belonging to the Merkle Patricia Forest.
        /// Something that implements the `CommitmentVerifier` trait.
        /// The type of the challenge is a hash, and it is expected that a proof will provide the
        /// exact hash if it exists in the forest, or the previous and next hashes if it does not.
        type ForestVerifier: CommitmentVerifier<Commitment = KeyFor<Self>, Challenge = KeyFor<Self>>;

        /// The type used to verify the proof of a specific key within the Merkle Patricia Forest.
        /// While `ForestVerifier` verifies that some keys are in the Merkle Patricia Forest, this
        /// verifies specifically a proof for that key. For example, if the keys in the forest
        /// represent files, this would verify the proof for a specific file, and `ForestVerifier`
        /// would verify that the file is in the forest.
        /// The type of the challenge is a [u8; 8] that actually represents a u64 number, which is
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
        /// Applies to file keys (leaf nodes) and root hashes (root nodes).
        /// Generally a hash (the output of a Hasher).
        type MerkleHash: Parameter
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
        type MerkleHashing: Hash<Output = Self::MerkleHash> + TypeInfo;

        /// The type to convert a balance to a block number.
        type StakeToBlockNumber: Convert<BalanceFor<Self>, BlockNumberFor<Self>>;

        /// The number of random challenges that are generated per block, using the random seed
        /// generated for that block.
        #[pallet::constant]
        type RandomChallengesPerBlock: Get<u32>;

        /// The maximum number of custom challenges that can be made in a single checkpoint block.
        #[pallet::constant]
        type MaxCustomChallengesPerBlock: Get<u32>;

        /// The maximum number of Providers that can be challenged in block.
        #[pallet::constant]
        type MaxProvidersChallengedPerBlock: Get<u32>;

        /// The number of blocks that challenges history is kept for.
        /// After this many blocks, challenges are removed from `BlockToChallengesSeed` StorageMap.
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
        #[pallet::constant]
        type CheckpointChallengePeriod: Get<u32>;

        /// The ratio to convert staked balance to block period.
        /// This is used to determine the period in which a Provider should submit a proof, based on
        /// their stake. The period is calculated as `stake / StakeToBlockPeriod`, saturating at 1.
        #[pallet::constant]
        type StakeToChallengePeriod: Get<BalanceFor<Self>>;

        /// The fee charged for submitting a challenge.
        /// This fee goes to the Treasury, and is used to prevent spam. Registered Providers are
        /// exempt from this fee.
        #[pallet::constant]
        type ChallengesFee: Get<BalanceFor<Self>>;

        /// The Treasury AccountId.
        /// The account to which:
        /// - The fees for submitting a challenge are transferred.
        /// - The slashed funds are transferred.
        #[pallet::constant]
        type Treasury: Get<Self::AccountId>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// A mapping from block number to a vector of challenged file keys for that block.
    ///
    /// This is used to keep track of the challenges' seed in the past.
    /// This mapping goes back only `ChallengeHistoryLength` blocks. Previous challenges are removed.
    #[pallet::storage]
    #[pallet::getter(fn block_to_challenges)]
    pub type BlockToChallengesSeed<T: Config> =
        StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, MerkleHashFor<T>>;

    /// A mapping from block number to a vector of custom challenged file keys for that block.
    ///
    /// This is used to keep track of the challenges that have been made in the past, specifically
    /// in the checkpoint challenge rounds.
    /// The vector is bounded by `MaxCustomChallengesPerBlockFor`.
    /// This mapping goes back only `ChallengeHistoryLength` blocks. Previous challenges are removed.
    #[pallet::storage]
    #[pallet::getter(fn block_to_checkpoint_challenges)]
    pub type BlockToCheckpointChallenges<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<KeyFor<T>, MaxCustomChallengesPerBlockFor<T>>,
    >;

    /// The block number of the last checkpoint challenge round.
    ///
    /// This is used to determine when to include the challenges from the `ChallengesQueue` and
    /// `PriorityChallengesQueue` in the `BlockToChallenges` StorageMap. These checkpoint challenge
    /// rounds have to be answered by ALL Providers, and this is enforced by the
    /// `submit_proof` extrinsic.
    #[pallet::storage]
    #[pallet::getter(fn last_checkpoint_block)]
    pub type LastCheckpointBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    /// A mapping from block number to a vector of challenged Providers for that block.
    ///
    /// This is used to keep track of the Providers that have been challenged, and should
    /// submit a proof by the time of the block used as the key. Providers who do submit
    /// a proof are removed from their respective entry and pushed forward to the next block in
    /// which they should submit a proof. Those who are still in the entry by the time the block
    /// is reached are considered to have failed to submit a proof and subject to slashing.
    #[pallet::storage]
    #[pallet::getter(fn block_to_challenged_providers)]
    pub type BlockToChallengedProviders<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<ProviderFor<T>, MaxProvidersChallengedPerBlockFor<T>>,
    >;

    /// A mapping from a Provider to the last block number they submitted a proof for.
    /// If for a Provider `p`, `LastBlockProviderSubmittedProofFor[p]` is `n`, then the
    /// Provider should submit a proof for block `n + stake_to_challenge_period(p)`.
    #[pallet::storage]
    #[pallet::getter(fn last_block_provider_submitted_proof_for)]
    pub type LastBlockProviderSubmittedProofFor<T: Config> =
        StorageMap<_, Blake2_128Concat, ProviderFor<T>, BlockNumberFor<T>>;

    /// A queue of file keys that have been challenged manually.
    ///
    /// The elements in this queue will be challenged in the coming blocks,
    /// always ensuring that the maximum number of challenges per block is not exceeded.
    /// A `BoundedVec` is used because the `parity_scale_codec::MaxEncodedLen` trait
    /// is required, but using a `VecDeque` would be more efficient as this is a FIFO queue.
    #[pallet::storage]
    #[pallet::getter(fn challenges_queue)]
    pub type ChallengesQueue<T: Config> =
        StorageValue<_, BoundedVec<KeyFor<T>, ChallengesQueueLengthFor<T>>, ValueQuery>;

    /// A priority queue of file keys that have been challenged manually.
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
    pub type PriorityChallengesQueue<T: Config> =
        StorageValue<_, BoundedVec<KeyFor<T>, ChallengesQueueLengthFor<T>>, ValueQuery>;

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
            provider: ProviderFor<T>,
            proof: Proof<T>,
        },
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

        /// Provider is submitting a proof but there is no record of the last block they
        /// submitted a proof for.
        /// Providers who are required to submit proofs should always have a record of the
        /// last block they submitted a proof for, otherwise it means they haven't started
        /// providing service for any user yet.
        NoRecordOfLastSubmittedProof,

        /// The provider stake could not be found.
        ProviderStakeNotFound,

        /// Provider is submitting a proof but their stake is zero.
        ZeroStake,

        /// The staked balance of the Provider could not be converted to `u128`.
        /// This should not be possible, as the `Balance` type should be an unsigned integer type.
        StakeCouldNotBeConverted,

        /// Provider is submitting a proof for a block in the future.
        ChallengesBlockNotReached,

        /// Provider is submitting a proof for a block before the last block this pallet registers
        /// challenges for.
        ChallengesBlockTooOld,

        /// The seed for the block could not be found.
        /// This should not be possible for a block within the `ChallengeHistoryLength` range, as
        /// seeds are generated for all blocks, and stored within this range.
        SeedNotFound,

        /// Checkpoint challenges not found in block.
        /// This should only be possible if `BlockToCheckpointChallenges` is dereferenced for a block
        /// that is not a checkpoint block.
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
        /// by checking the `BlockToChallengesSeed` StorageMap. The block number that the
        /// Provider should have submitted a proof is calculated based on the last block they
        /// submitted a proof for (`LastBlockProviderSubmittedProofFor`), and the proving period for
        /// that Provider, which is a function of their stake.
        /// This extrinsic also checks that there hasn't been a checkpoint challenge round
        /// in between the last time the Provider submitted a proof for and the block
        /// for which the proof is being submitted. If there has been, the Provider is
        /// subject to slashing.
        ///
        /// If valid:
        /// - Pushes forward the Provider in the `BlockToChallengedProviders` StorageMap a number
        /// of blocks corresponding to the stake of the Provider.
        /// - Registers this block as the last block in which the Provider submitted a proof.
        ///
        /// Execution of this extrinsic should be refunded if the proof is valid.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn submit_proof(
            origin: OriginFor<T>,
            proof: Proof<T>,
            provider: Option<ProviderFor<T>>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Getting provider from the origin if none is provided.
            let provider = match provider {
                Some(provider) => provider,
                None => {
                    let sp = T::ProvidersPallet::get_provider(who.clone())
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

        /// Extrinsic to register a new round of challenges.
        ///
        /// This function is called by the block producer to register a new round of challenges.
        /// Random challenges are automatically generated based on some external source of
        /// randomness, and are added to `BlockToChallenges`, for this block's number.
        ///
        /// It also takes care of including the challenges from the `ChallengesQueue` and
        /// `PriorityChallengesQueue`. This custom challenges are only included in "checkpoint"
        /// blocks
        ///
        /// Additionally, it takes care of checking if there are Providers that have
        /// failed to submit a proof, and should have submitted one by this block. It does so
        /// by checking the `BlockToChallengedProviders` StorageMap. If a Provider is found
        /// to have failed to submit a proof, it is subject to slashing.
        ///
        /// Finally, it cleans up:
        /// - The `BlockToChallenges` StorageMap, removing entries older than `ChallengeHistoryLength`.
        /// - The `BlockToChallengedProviders` StorageMap, removing entries for the current block number.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn new_challenges_round(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            // TODO: Handle result of verification.
            Self::do_new_challenges_round()?;

            // TODO: Emit events.

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }
    }
}
