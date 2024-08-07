#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, FullCodec, HasCompact};
use frame_support::dispatch::DispatchResult;
use frame_support::pallet_prelude::{MaxEncodedLen, MaybeSerializeDeserialize, Member};
use frame_support::sp_runtime::traits::{CheckEqual, MaybeDisplay, SimpleBitOps};
use frame_support::traits::{fungible, Incrementable};
use frame_support::{BoundedBTreeSet, Parameter};
use scale_info::prelude::fmt::Debug;
use scale_info::TypeInfo;
use sp_core::Get;
use sp_runtime::traits::{AtLeast32BitUnsigned, Hash, Saturating};
use sp_runtime::{BoundedVec, DispatchError};
use sp_std::collections::btree_set::BTreeSet;
use sp_std::vec::Vec;

#[cfg(feature = "std")]
pub trait MaybeDebug: Debug {}
#[cfg(feature = "std")]
impl<T: Debug> MaybeDebug for T {}
#[cfg(not(feature = "std"))]
pub trait MaybeDebug {}
#[cfg(not(feature = "std"))]
impl<T> MaybeDebug for T {}

#[derive(Encode)]
pub struct AsCompact<T: HasCompact>(#[codec(compact)] pub T);

/// A trait to lookup registered Providers.
///
/// It is abstracted over the `AccountId` type, `Provider` type, `MerkleHash` type and `Balance` type.
pub trait ProvidersInterface {
    /// The type corresponding to the staking balance of a registered Provider.
    type Balance: fungible::Inspect<Self::AccountId> + fungible::hold::Inspect<Self::AccountId>;
    /// The type which can be used to identify accounts.
    type AccountId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
    /// The type which represents a registered Provider's ID.
    type ProviderId: Parameter
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
    /// The type corresponding to the root of a registered Provider.
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

    /// Check if an account is a registered Provider.
    fn is_provider(who: Self::ProviderId) -> bool;

    /// Get the ProviderId from AccountId, if it is a registered Provider.
    fn get_provider_id(who: Self::AccountId) -> Option<Self::ProviderId>;

    /// Get the AccountId of the owner of a registered Provider.
    fn get_owner_account(who: Self::ProviderId) -> Option<Self::AccountId>;

    /// Get the root for a registered Provider.
    fn get_root(who: Self::ProviderId) -> Option<Self::MerkleHash>;

    /// Update the root for a registered Provider.
    fn update_root(who: Self::ProviderId, new_root: Self::MerkleHash) -> DispatchResult;

    /// Get the default value for the root of a Merkle Patricia Forest.
    fn get_default_root() -> Self::MerkleHash;

    /// Get the stake for a registered  Provider.
    fn get_stake(
        who: Self::ProviderId,
    ) -> Option<<Self::Balance as fungible::Inspect<Self::AccountId>>::Balance>;
}

/// A trait to get system-wide metrics, such as the total available capacity of the network and
/// its total used capacity.
pub trait SystemMetricsInterface {
    /// Type of the unit provided by Providers
    type ProvidedUnit: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Default
        + MaybeDisplay
        + AtLeast32BitUnsigned
        + Copy
        + MaxEncodedLen
        + HasCompact
        + Into<u32>;

    /// Get the total available capacity of units of the network.
    fn get_total_capacity() -> Self::ProvidedUnit;

    /// Get the total used capacity of units of the network.
    fn get_total_used_capacity() -> Self::ProvidedUnit;
}

pub trait ProvidersConfig {
    /// The type of ID that uniquely identifies a Merkle Trie Holder (BSPs/Buckets) from an AccountId
    type BucketId: Parameter
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
    /// The type of the Bucket NFT Collection ID.
    type ReadAccessGroupId: Member + Parameter + MaxEncodedLen + Copy + Incrementable;
}

/// A trait to lookup registered Providers, their Merkle Patricia Trie roots and their stake.
pub trait ReadProvidersInterface: ProvidersConfig + ProvidersInterface {
    /// Type that represents the total number of registered Storage Providers.
    type SpCount: Parameter
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
    /// Type that represents the multiaddress of a Storage Provider.
    type MultiAddress: Parameter
        + MaybeSerializeDeserialize
        + Debug
        + Ord
        + Default
        + AsRef<[u8]>
        + AsMut<[u8]>
        + MaxEncodedLen
        + FullCodec;
    /// Type that represents the byte limit of a bucket name.
    type BucketNameLimit: Get<u32>;
    /// Maximum number of multiaddresses a provider can have.
    type MaxNumberOfMultiAddresses: Get<u32>;

    /// Check if provider is a BSP.
    fn is_bsp(who: &Self::ProviderId) -> bool;

    /// Check if provider is a MSP.
    fn is_msp(who: &Self::ProviderId) -> bool;

    /// Get the payment account of a registered Provider.
    fn get_provider_payment_account(who: Self::ProviderId) -> Option<Self::AccountId>;

    /// Get number of registered BSPs.
    fn get_number_of_bsps() -> Self::SpCount;

    /// Get multiaddresses of a BSP.
    fn get_bsp_multiaddresses(
        who: &Self::ProviderId,
    ) -> Result<BoundedVec<Self::MultiAddress, Self::MaxNumberOfMultiAddresses>, DispatchError>;

    /// Check if account is the owner of a bucket.
    fn is_bucket_owner(
        who: &Self::AccountId,
        bucket_id: &Self::BucketId,
    ) -> Result<bool, DispatchError>;

    /// Is bucket stored by MSP.
    fn is_bucket_stored_by_msp(msp_id: &Self::ProviderId, bucket_id: &Self::BucketId) -> bool;

    /// Get `collection_id` of a bucket if there is one.
    fn get_read_access_group_id_of_bucket(
        bucket_id: &Self::BucketId,
    ) -> Result<Option<Self::ReadAccessGroupId>, DispatchError>;

    /// Get MSP storing a bucket.
    fn get_msp_of_bucket(bucket_id: &Self::BucketId) -> Option<Self::ProviderId>;

    /// Check if a bucket is private.
    fn is_bucket_private(bucket_id: &Self::BucketId) -> Result<bool, DispatchError>;

    /// Derive bucket Id from the owner and bucket name.
    fn derive_bucket_id(
        owner: &Self::AccountId,
        bucket_name: BoundedVec<u8, Self::BucketNameLimit>,
    ) -> Self::BucketId;
}

/// Interface to allow the File System pallet to modify the data used by the Storage Providers pallet.
pub trait MutateProvidersInterface: ProvidersConfig + ProvidersInterface {
    /// Data type for the measurement of storage size
    type StorageData: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Default
        + MaybeDisplay
        + AtLeast32BitUnsigned
        + Copy
        + MaxEncodedLen
        + HasCompact
        + Into<u32>;
    /// The type of the Merkle Patricia Root of the storage trie for BSPs and MSPs' buckets (a hash).
    type MerklePatriciaRoot: Parameter
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

    /// Increase the used data of a Storage Provider (generic, MSP or BSP).
    fn increase_data_used(who: &Self::ProviderId, delta: Self::StorageData) -> DispatchResult;

    /// Decrease the used data of a Storage Provider (generic, MSP or BSP).
    fn decrease_data_used(who: &Self::ProviderId, delta: Self::StorageData) -> DispatchResult;

    /// Add a new Bucket as a Provider
    fn add_bucket(
        provider_id: Self::ProviderId,
        user_id: Self::AccountId,
        bucket_id: Self::BucketId,
        privacy: bool,
        collection_id: Option<Self::ReadAccessGroupId>,
    ) -> DispatchResult;

    /// Update bucket privacy settings
    fn update_bucket_privacy(bucket_id: Self::BucketId, privacy: bool) -> DispatchResult;

    /// Update bucket collection ID
    fn update_bucket_read_access_group_id(
        bucket_id: Self::BucketId,
        maybe_collection_id: Option<Self::ReadAccessGroupId>,
    ) -> DispatchResult;

    /// Change the root of a bucket
    fn change_root_bucket(
        bucket_id: Self::BucketId,
        new_root: Self::MerklePatriciaRoot,
    ) -> DispatchResult;

    /// Remove a root from a bucket of a MSP, removing the whole bucket from storage
    fn remove_root_bucket(bucket_id: Self::BucketId) -> DispatchResult;
}

/// The interface to subscribe to updates on the Storage Providers pallet.
pub trait SubscribeProvidersInterface {
    /// The type which represents a registered Provider.
    type ProviderId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;

    /// Subscribe to the sign off of a BSP.
    fn subscribe_bsp_sign_off(who: &Self::ProviderId) -> DispatchResult;

    /// Subscribe to the sign up of a BSP.
    fn subscribe_bsp_sign_up(who: &Self::ProviderId) -> DispatchResult;
}

/// The interface for the ProofsDealer pallet.
///
/// It is abstracted over the `Provider` type, `Proof` type, `ForestProof` type and `MerkleHash` type.
/// It provides the functions to verify a proof, submit a new proof challenge and
/// submit a new challenge with priority.
pub trait ProofsDealerInterface {
    /// The type which represents a registered Provider.
    type ProviderId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
    /// The type that represents a proof just for the Merkle Patricia Forest.
    type ForestProof: Parameter + Member + Debug;
    /// The type that represents a proof for an inner key (leaf) of the Merkle Patricia Forest.
    type KeyProof: Parameter + Member + Debug;
    /// The type corresponding to the root and keys in the Merkle Patricia Forest of a
    /// registered Provider.
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
    type MerkleHashing: Hash<Output = Self::MerkleHash>;
    /// The type that represents the randomness output.
    type RandomnessOutput: Parameter + Member + Debug;

    /// Verify a proof just for the Merkle Patricia Forest, for a given Provider.
    ///
    /// This only verifies that something is included in the forest of the Provider. It is not a full
    /// proof of the Provider's data.
    fn verify_forest_proof(
        provider_id: &Self::ProviderId,
        challenges: &[Self::MerkleHash],
        proof: &Self::ForestProof,
    ) -> Result<BTreeSet<Self::MerkleHash>, DispatchError>;

    /// Verify a proof for a key within the Merkle Patricia Forest of a Provider.
    ///
    /// This only verifies a proof of the data at a specific key within the Provider's forest. It does
    /// not verify if that key is included in the Merkle Patricia Forest of the Provider.
    fn verify_key_proof(
        key: &Self::MerkleHash,
        challenges: &[Self::MerkleHash],
        proof: &Self::KeyProof,
    ) -> Result<BTreeSet<Self::MerkleHash>, DispatchError>;

    /// Submit a new proof challenge.
    fn challenge(key_challenged: &Self::MerkleHash) -> DispatchResult;

    /// Submit a new challenge with priority.
    fn challenge_with_priority(
        key_challenged: &Self::MerkleHash,
        mutation: Option<TrieRemoveMutation>,
    ) -> DispatchResult;

    /// Given a randomness seed, a provider id and a count, generate a list of challenges.
    fn generate_challenges_from_seed(
        seed: Self::RandomnessOutput,
        provider_id: &Self::ProviderId,
        count: u32,
    ) -> Vec<Self::MerkleHash>;

    /// Apply delta (mutations) to the partial trie based on the proof and the commitment.
    ///
    /// The new root is returned.
    fn apply_delta(
        provider_id: &Self::ProviderId,
        mutations: &[(Self::MerkleHash, TrieMutation)],
        proof: &Self::ForestProof,
    ) -> Result<Self::MerkleHash, DispatchError>;

    /// Initialise a Provider's challenge cycle.
    ///
    /// Sets the last tick the Provider submitted a proof for to the current tick and sets the
    /// deadline for submitting a proof to the current tick + the Provider's period (based on its
    /// stake) + the challenges tick tolerance.
    fn initialise_challenge_cycle(who: &Self::ProviderId) -> DispatchResult;
}

/// A trait to verify proofs based on commitments and challenges.
///
/// It is abstracted over the `Proof`, `Commitment` and `Challenge` types.
pub trait CommitmentVerifier {
    /// The type that represents the proof.
    type Proof: Parameter + Member + Debug;
    /// The type that represents the commitment (e.g. a Merkle root)
    type Commitment: MaybeDebug + Ord + Default + Copy + AsRef<[u8]> + AsMut<[u8]>;
    /// The type that represents the challenges which a proof is being verified against.
    type Challenge: MaybeDebug + Ord + Default + Copy + AsRef<[u8]> + AsMut<[u8]>;

    /// Verify a proof based on a commitment and a set of challenges.
    ///
    /// The function returns a vector of keys that are verified by the proof, or an error if the proof
    /// is invalid.
    fn verify_proof(
        commitment: &Self::Commitment,
        challenges: &[Self::Challenge],
        proof: &Self::Proof,
    ) -> Result<BTreeSet<Self::Challenge>, DispatchError>;
}

/// Enum representing the type of mutation (addition or removal of a key).
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq, Debug)]
pub enum TrieMutation {
    Add(TrieAddMutation),
    Remove(TrieRemoveMutation),
}

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq, Debug, Default)]
pub struct TrieAddMutation;

impl Into<TrieMutation> for TrieAddMutation {
    fn into(self) -> TrieMutation {
        TrieMutation::Add(self)
    }
}

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq, Debug, Default)]
pub struct TrieRemoveMutation;

impl Into<TrieMutation> for TrieRemoveMutation {
    fn into(self) -> TrieMutation {
        TrieMutation::Remove(self)
    }
}

/// A trait to apply mutations (delta) to a partial trie based on a proof and a commitment.
pub trait TrieProofDeltaApplier<H: sp_core::Hasher> {
    /// The type that represents the proof.
    type Proof: Parameter + Member + Debug;
    /// The type that represents the keys (e.g. a Merkle root, node keys, etc.)
    type Key: MaybeDebug + Ord + Default + Copy + AsRef<[u8]> + AsMut<[u8]>;

    /// Apply mutations (delta) to a partial trie based on a proof and a commitment.
    ///
    /// Returns the new root computed after applying the mutations.
    fn apply_delta(
        root: &Self::Key,
        mutations: &[(Self::Key, TrieMutation)],
        proof: &Self::Proof,
    ) -> Result<(sp_trie::MemoryDB<H>, Self::Key), DispatchError>;
}

/// Interface used by the file system pallet in order to read storage from NFTs pallet (avoiding tigth coupling).
pub trait InspectCollections {
    type CollectionId;

    /// Check if a collection exists.
    fn collection_exists(collection_id: &Self::CollectionId) -> bool;
}

/// The interface of the Payment Streams pallet.
///
/// It is to be used by other pallets to interact with the Payment Streams pallet to create, update and delete payment streams.
pub trait PaymentStreamsInterface {
    /// The type which represents the balance of the runtime.
    type Balance: fungible::Inspect<Self::AccountId>
        + fungible::Mutate<Self::AccountId>
        + fungible::hold::Inspect<Self::AccountId>
        + fungible::hold::Mutate<Self::AccountId>;
    /// The type which represents a User account identifier.
    type AccountId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
    /// The type which represents a Provider identifier.
    type ProviderId: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Debug
        + Ord
        + MaxEncodedLen
        + Copy;
    /// The type which represents a block number.
    type BlockNumber: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
    /// The type which represents a fixed-rate payment stream.
    type FixedRatePaymentStream: Encode
        + Decode
        + Parameter
        + Member
        + Debug
        + MaxEncodedLen
        + PartialEq
        + Clone;
    /// The type which represents a dynamic-rate payment stream.
    type DynamicRatePaymentStream: Encode
        + Decode
        + Parameter
        + Member
        + Debug
        + MaxEncodedLen
        + PartialEq
        + Clone;
    /// The type of the units that the Provider provides to the User (for example, for storage could be terabytes)
    type Units: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Default
        + MaybeDisplay
        + AtLeast32BitUnsigned
        + Saturating
        + Copy
        + MaxEncodedLen
        + HasCompact
        + Into<<Self::Balance as fungible::Inspect<Self::AccountId>>::Balance>;

    /// Create a new fixed-rate payment stream from a User to a Provider.
    fn create_fixed_rate_payment_stream(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
        rate: <Self::Balance as fungible::Inspect<Self::AccountId>>::Balance,
    ) -> DispatchResult;

    /// Update the rate of an existing fixed-rate payment stream.
    fn update_fixed_rate_payment_stream(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
        new_rate: <Self::Balance as fungible::Inspect<Self::AccountId>>::Balance,
    ) -> DispatchResult;

    /// Delete a fixed-rate payment stream.
    fn delete_fixed_rate_payment_stream(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> DispatchResult;

    /// Get the fixed-rate payment stream information between a User and a Provider
    fn get_fixed_rate_payment_stream_info(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> Option<Self::FixedRatePaymentStream>;

    /// Create a new dynamic-rate payment stream from a User to a Provider.
    fn create_dynamic_rate_payment_stream(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
        amount_provided: &Self::Units,
        current_price: <Self::Balance as fungible::Inspect<Self::AccountId>>::Balance,
        current_accumulated_price_index: <Self::Balance as fungible::Inspect<Self::AccountId>>::Balance,
    ) -> DispatchResult;

    /// Update the amount provided of an existing dynamic-rate payment stream.
    fn update_dynamic_rate_payment_stream(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
        new_amount_provided: &Self::Units,
        current_price: <Self::Balance as fungible::Inspect<Self::AccountId>>::Balance,
    ) -> DispatchResult;

    /// Delete a dynamic-rate payment stream.
    fn delete_dynamic_rate_payment_stream(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> DispatchResult;

    /// Get the dynamic-rate payment stream information between a User and a Provider
    fn get_dynamic_rate_payment_stream_info(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> Option<Self::DynamicRatePaymentStream>;
}

pub trait ProofSubmittersInterface {
    /// The type which represents a provider identifier.
    type ProviderId: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Debug
        + Ord
        + MaxEncodedLen
        + Copy;
    /// The type which represents a tick number.
    type TickNumber: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
    /// The type which represents the maximum limit of the number of proof submitters for a tick.
    type MaxProofSubmitters: Get<u32>;

    fn get_proof_submitters_for_tick(
        tick_number: &Self::TickNumber,
    ) -> Option<BoundedBTreeSet<Self::ProviderId, Self::MaxProofSubmitters>>;

    fn get_accrued_failed_proof_submissions(provider_id: &Self::ProviderId) -> Option<u32>;

    fn clear_accrued_failed_proof_submissions(provider_id: &Self::ProviderId);
}
