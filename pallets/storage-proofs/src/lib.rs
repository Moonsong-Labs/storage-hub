#![allow(unused_variables)]
#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;
pub mod types;
pub mod utils;

use codec::FullCodec;
use frame_support::{
    inherent::IsFatalError,
    pallet_prelude::*,
    sp_runtime::{
        traits::{AtLeast32BitUnsigned, CheckEqual, MaybeDisplay, SimpleBitOps},
        RuntimeString,
    },
    traits::fungible,
};
use scale_info::prelude::fmt::Debug;
use sp_trie::CompactProof;

// TODO: Define this.
const INHERENT_IDENTIFIER: InherentIdentifier = *b"todo____";

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
    use scale_info::prelude::fmt::Debug;
    use sp_trie::CompactProof;
    use types::SpFor;

    use crate::types::*;
    use crate::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The Storage Providers pallet.
        /// To check if whoever submits a proof is a registered Storage Provider.
        type StorageProviders: crate::StorageProvidersInterface<AccountId = Self::AccountId>;

        /// Type to access the Balances Pallet.
        type NativeBalance: fungible::Inspect<Self::AccountId>
            + fungible::hold::Inspect<Self::AccountId>
            + fungible::freeze::Inspect<Self::AccountId>;

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
        /// Something that implements the `TrieVerifier` trait.
        type TrieVerifier: crate::TrieVerifier;

        /// The maximum number of challenges that can be made in a single block.
        #[pallet::constant]
        type MaxChallengesPerBlock: Get<u32> + FullCodec;

        /// The maximum number of Storage Providers that can be challenged in block.
        #[pallet::constant]
        type MaxSpsChallengedPerBlock: Get<u32> + FullCodec;

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
        /// rounds have to be answered by ALL Storage Providers, and this is enforced by the
        /// `submit_proof` extrinsic.
        #[pallet::constant]
        type CheckpointChallengePeriod: Get<u32>;
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
        BoundedVec<FileKeyFor<T>, MaxChallengesPerBlockFor<T>>,
    >;

    /// A mapping from block number to a vector of challenged Storage Providers for that block.
    ///
    /// This is used to keep track of the Storage Providers that have been challenged, and should
    /// submit a proof by the time of the block used as the key. Storage Providers who do submit
    /// a proof are removed from their respective entry and pushed forward to the next block in
    /// which they should submit a proof. Those who are still in the entry by the time the block
    /// is reached are considered to have failed to submit a proof and subject to slashing.
    #[pallet::storage]
    #[pallet::getter(fn block_to_challenged_sps)]
    pub type BlockToChallengedSps<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<SpFor<T>, MaxSpsChallengedPerBlockFor<T>>,
    >;

    /// A mapping from a Storage Provider to the last block number they submitted a proof for.
    /// If for a Storage Provider `sp`, `LastBlockSpSubmittedProofFor[sp]` is `n`, then the
    /// Storage Provider should submit a proof for block `n + stake_to_challenge_period(sp)`.
    #[pallet::storage]
    #[pallet::getter(fn last_block_sp_submitted_proof_for)]
    pub type LastBlockSpSubmittedProofFor<T: Config> =
        StorageMap<_, Blake2_128Concat, SpFor<T>, BlockNumberFor<T>>;

    /// A queue of file keys that have been challenged manually.
    ///
    /// The elements in this queue will be challenged in the coming blocks,
    /// always ensuring that the maximum number of challenges per block is not exceeded.
    /// A `BoundedVec` is used because the `parity_scale_codec::MaxEncodedLen` trait
    /// is required, but using a `VecDeque` would be more efficient as this is a FIFO queue.
    #[pallet::storage]
    #[pallet::getter(fn challenges_queue)]
    pub type ChallengesQueue<T: Config> =
        StorageValue<_, BoundedVec<FileKeyFor<T>, ChallengesQueueLengthFor<T>>, ValueQuery>;

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
        StorageValue<_, BoundedVec<FileKeyFor<T>, ChallengesQueueLengthFor<T>>, ValueQuery>;

    /// The block number of the last checkpoint challenge round.
    ///
    /// This is used to determine when to include the challenges from the `ChallengesQueue` and
    /// `PriorityChallengesQueue` in the `BlockToChallenges` StorageMap. These checkpoint challenge
    /// rounds have to be answered by ALL Storage Providers, and this is enforced by the
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
        /// [who, file_key_challenged]
        NewChallenge(AccountIdFor<T>, FileKeyFor<T>),

        /// A storage proof was rejected.
        /// [storage_provider, proof, reason]
        ProofRejected(AccountIdFor<T>, CompactProof, ProofRejectionReason),

        /// A storage proof was accepted.
        /// [storage_provider, proof]
        ProofAccepted(AccountIdFor<T>, CompactProof),
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// The ChallengesQueue is full. No more manual challenges can be made
        /// until some of the challenges in the queue are dispatched.
        ChallengesQueueOverflow,

        /// The proof submitter is not a registered Storage Provider.
        NotStorageProvider,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Introduce a new challenge.
        ///
        /// This function allows anyone to add a new challenge to the `ChallengesQueue`.
        /// The challenge will be dispatched in the coming blocks.
        /// Regular users are charged a small fee for submitting a challenge, which
        /// goes to the Treasury. Unless the one calling is a registered Storage Provider.
        ///
        /// TODO: Consider checking also if there was a request to change MSP.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn challenge(
            origin: OriginFor<T>,
            file_key: FileKeyFor<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            Self::do_challenge(&who, &file_key)?;

            // Emit event.
            Self::deposit_event(Event::NewChallenge(who, file_key));

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// For a Storage Provider to submit a storage proof.
        ///
        /// Checks that `storage_provider` is a registered Storage Provider. If none
        /// is provided, the proof submitter is considered to be the Storage Provider.
        /// Relies on a File System pallet to check if the root is valid for the Storage Provider.
        /// Validates that the proof corresponds to a challenge that was made in the past,
        /// by checking the `BlockToChallenges` StorageMap. The block number that the Storage
        /// Provider should have submitted a proof is calculated based on the last block they
        /// submitted a proof for (`LastBlockSpSubmittedProofFor`), and the proving period for
        /// that Storage Provider, which is a function of their stake.
        /// This extrinsic also checks that there hasn't been a checkpoint challenge round
        /// in between the last time the Storage Provider submitted a proof for and the block
        /// for which the proof is being submitted. If there has been, the Storage Provider is
        /// subject to slashing.
        ///
        /// If valid:
        /// - Pushes forward the Storage Provider in the `BlockToChallengedSps` StorageMap a number
        /// of blocks corresponding to the stake of the Storage Provider.
        /// - Registers this block as the last block in which the Storage Provider submitted a proof.
        ///
        /// Execution of this extrinsic should be refunded if the proof is valid.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn submit_proof(
            origin: OriginFor<T>,
            proof: CompactProof,
            root: ForestRootFor<T>,
            challenge_block: BlockNumberFor<T>,
            storage_provider: Option<AccountIdFor<T>>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // TODO: Handle result of verification.
            Self::do_submit_proof(&who, &proof)?;

            // TODO: Emit correct event.
            Self::deposit_event(Event::ProofAccepted(who, proof));

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
        /// Additionally, it takes care of checking if there are Storage Providers that have
        /// failed to submit a proof, and should have submitted one by this block. It does so
        /// by checking the `BlockToChallengedSps` StorageMap. If a Storage Provider is found
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

    // This pallet provides an inherent, as such it implements ProvideInherent trait
    // https://paritytech.github.io/substrate/master/frame_support/inherent/trait.ProvideInherent.html
    #[pallet::inherent]
    impl<T: Config> ProvideInherent for Pallet<T> {
        type Call = Call<T>;
        // TODO: Specify this type.
        type Error = InherentError;
        // TODO: Specify this type.
        const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

        // This method is used to decide whether this inherent is requiered for the block to be accepted
        fn is_inherent_required(data: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
            // TODO: Implement this method
            unimplemented!()
        }

        fn create_inherent(data: &InherentData) -> Option<Self::Call> {
            // create and return the extrinsic call if the data could be read and decoded
            // TODO: Implement this method
            unimplemented!()
        }
        // Determine if a call is an inherent extrinsic
        fn is_inherent(call: &Self::Call) -> bool {
            // TODO: Implement this method
            unimplemented!()
        }
    }
}

#[derive(Encode)]
#[cfg_attr(feature = "std", derive(Debug, Decode))]
pub enum InherentError {
    Other(RuntimeString),
}

impl IsFatalError for InherentError {
    fn is_fatal_error(&self) -> bool {
        match *self {
            InherentError::Other(_) => true,
        }
    }
}

impl InherentError {
    /// Try to create an instance ouf of the given identifier and data.
    #[cfg(feature = "std")]
    pub fn try_from(id: &InherentIdentifier, data: &[u8]) -> Option<Self> {
        if id == &INHERENT_IDENTIFIER {
            <InherentError as codec::Decode>::decode(&mut &data[..]).ok()
        } else {
            None
        }
    }
}

// TODO: Move this to Storage Providers pallet.
/// A trait to lookup registered Storage Providers.
///
/// It is abstracted over the `AccountId` type, `StorageProvider` type, total number of users
/// and Balance type.
pub trait StorageProvidersInterface {
    /// The type which can be used to identify accounts.
    type AccountId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
    /// The type corresponding to the staking balance of a registered Storage Provider.
    type Balance: fungible::Inspect<Self::AccountId>
        + fungible::hold::Inspect<Self::AccountId>
        + fungible::freeze::Inspect<Self::AccountId>;
    /// The type which represents a registered Storage Provider.
    type StorageProvider: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Debug
        + Ord
        + MaxEncodedLen;
    /// The type which represents the total number of registered Storage Provider.
    type UserCount: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Ord
        + AtLeast32BitUnsigned
        + FullCodec
        + Copy
        + Default
        + Debug
        + scale_info::TypeInfo
        + MaxEncodedLen;

    /// Lookup a registered StorageProvider by their AccountId.
    fn get_sp(who: Self::AccountId) -> Option<Self::StorageProvider>;

    /// Check if an account is a registered Storage Provider.
    fn is_sp(who: Self::AccountId) -> bool;

    /// Lookup the total number of registered Storage Providers.
    fn total_sps() -> Self::UserCount;

    /// Get the stake for a registered Storage Provider.
    fn get_stake(who: Self::StorageProvider) -> Self::Balance;
}

// TODO: Move this to a primitives crate.
// TODO: Abstract better the types of arguments
/// A trait to verify Merkle Patricia Trie proofs.
pub trait TrieVerifier {
    fn verify_proof(root: &[u8; 32], challenges: &[u8; 32], proof: &CompactProof) -> bool;
}
