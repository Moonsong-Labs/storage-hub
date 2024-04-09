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

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;
pub mod types;
pub mod utils;

use scale_info::prelude::fmt::Debug;
pub use sp_trie::CompactProof;

#[frame_support::pallet]
pub mod pallet {
    use codec::FullCodec;
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::{ValueQuery, *},
        sp_runtime::traits::{CheckEqual, MaybeDisplay, SimpleBitOps},
        traits::fungible,
    };
    use frame_system::pallet_prelude::*;
    use sp_trie::CompactProof;
    use storage_hub_traits::{CommitmentVerifier, ProvidersInterface};
    use types::ProviderFor;

    use crate::types::*;
    use crate::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The Providers pallet.
        /// To check if whoever submits a proof is a registered Provider.
        type ProvidersPallet: ProvidersInterface<AccountId = Self::AccountId>;

        /// Type to access the Balances Pallet.
        type NativeBalance: fungible::Inspect<Self::AccountId>
            + fungible::Mutate<Self::AccountId>
            + fungible::hold::Inspect<Self::AccountId>
            + fungible::hold::Mutate<Self::AccountId>;

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

        /// The type used to verify Merkle Patricia Trie proofs.
        /// Something that implements the `CommitmentVerifier` trait.
        type KeyVerifier: CommitmentVerifier;

        /// The maximum number of challenges that can be made in a single block.
        #[pallet::constant]
        type MaxChallengesPerBlock: Get<u32>;

        /// The maximum number of Providers that can be challenged in block.
        #[pallet::constant]
        type MaxProvidersChallengedPerBlock: Get<u32>;

        /// The number of blocks that challenges history is kept for.
        /// After this many blocks, challenges are removed from `Challenges` StorageMap.
        #[pallet::constant]
        type ChallengeHistoryLength: Get<u32>;

        /// The length of the `ChallengesQueue` StorageValue.
        /// This is to limit the size of the queue, and therefore the number of
        /// manual challenges that can be made.
        #[pallet::constant]
        type ChallengesQueueLength: Get<u32>;

        /// The number of blocks in between a checkpoint challenges round (i.e. with custom challenges).
        /// This is used to determine when to include the challenges from the `ChallengesQueue` and
        /// `PriorityChallengesQueue` in the `BlockToChallenges` StorageMap. These checkpoint challenge
        /// rounds have to be answered by ALL Providers, and this is enforced by the
        /// `submit_proof` extrinsic.
        #[pallet::constant]
        type CheckpointChallengePeriod: Get<u32>;

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
    /// This is used to keep track of the challenges that have been made in the past.
    /// The vector is bounded by `MaxChallengesPerBlock`.
    /// This mapping goes back only `ChallengeHistoryLength` blocks. Previous challenges are removed.
    #[pallet::storage]
    #[pallet::getter(fn block_to_challenges)]
    pub type BlockToChallenges<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<KeyFor<T>, MaxChallengesPerBlockFor<T>>,
    >;

    /// A mapping from block number to a vector of challenged Providers for that block.
    ///
    /// This is used to keep track of the Providers that have been challenged, and should
    /// submit a proof by the time of the block used as the key. Providers who do submit
    /// a proof are removed from their respective entry and pushed forward to the next block in
    /// which they should submit a proof. Those who are still in the entry by the time the block
    /// is reached are considered to have failed to submit a proof and subject to slashing.
    #[pallet::storage]
    #[pallet::getter(fn block_to_challenged_sps)]
    pub type BlockToChallengedSps<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<ProviderFor<T>, MaxSpsChallengedPerBlockFor<T>>,
    >;

    /// A mapping from a Provider to the last block number they submitted a proof for.
    /// If for a Provider `sp`, `LastBlockSpSubmittedProofFor[sp]` is `n`, then the
    /// Provider should submit a proof for block `n + stake_to_challenge_period(sp)`.
    #[pallet::storage]
    #[pallet::getter(fn last_block_sp_submitted_proof_for)]
    pub type LastBlockSpSubmittedProofFor<T: Config> =
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

    /// The block number of the last checkpoint challenge round.
    ///
    /// This is used to determine when to include the challenges from the `ChallengesQueue` and
    /// `PriorityChallengesQueue` in the `BlockToChallenges` StorageMap. These checkpoint challenge
    /// rounds have to be answered by ALL Providers, and this is enforced by the
    /// `submit_proof` extrinsic.
    #[pallet::storage]
    #[pallet::getter(fn last_checkpoint_block)]
    pub type LastCheckpointBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

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

        /// A proof was rejected.
        ProofRejected {
            provider: ProviderFor<T>,
            proof: CompactProof,
            reason: ProofRejectionReason,
        },

        /// A proof was accepted.
        ProofAccepted {
            provider: ProviderFor<T>,
            proof: CompactProof,
        },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// The ChallengesQueue is full. No more manual challenges can be made
        /// until some of the challenges in the queue are dispatched.
        ChallengesQueueOverflow,

        /// The PriorityChallengesQueue is full. No more priority challenges can be made
        /// until some of the challenges in the queue are dispatched.
        PriorityChallengesQueueOverflow,

        /// The proof submitter is not a registered Provider.
        NotProvider,

        /// The fee for submitting a challenge could not be charged.
        FeeChargeFailed,
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
        /// Relies on a File System pallet to check if the root is valid for the Provider.
        /// Validates that the proof corresponds to a challenge that was made in the past,
        /// by checking the `BlockToChallenges` StorageMap. The block number that the
        /// Provider should have submitted a proof is calculated based on the last block they
        /// submitted a proof for (`LastBlockSpSubmittedProofFor`), and the proving period for
        /// that Provider, which is a function of their stake.
        /// This extrinsic also checks that there hasn't been a checkpoint challenge round
        /// in between the last time the Provider submitted a proof for and the block
        /// for which the proof is being submitted. If there has been, the Provider is
        /// subject to slashing.
        ///
        /// If valid:
        /// - Pushes forward the Provider in the `BlockToChallengedSps` StorageMap a number
        /// of blocks corresponding to the stake of the Provider.
        /// - Registers this block as the last block in which the Provider submitted a proof.
        ///
        /// Execution of this extrinsic should be refunded if the proof is valid.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn submit_proof(
            origin: OriginFor<T>,
            proof: CompactProof,
            root: ForestRootFor<T>,
            challenge_block: BlockNumberFor<T>,
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

            // TODO: Handle result of verification.
            Self::do_submit_proof(&provider, &proof)?;

            // TODO: Emit correct event.
            Self::deposit_event(Event::ProofAccepted { provider, proof });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Inherent extrinsic to register a new round of challenges.
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
        /// by checking the `BlockToChallengedSps` StorageMap. If a Provider is found
        /// to have failed to submit a proof, it is subject to slashing.
        ///
        /// Finally, it cleans up:
        /// - The `BlockToChallenges` StorageMap, removing entries older than `ChallengeHistoryLength`.
        /// - The `BlockToChallengedSps` StorageMap, removing entries for the current block number.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn new_challenges_round(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            // Inherents are not signed transactions.
            ensure_none(origin)?;

            // TODO: Handle result of verification.
            Self::do_new_challenges_round()?;

            // TODO: Emit events.

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }
    }
}
