#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, FullCodec, HasCompact};
use frame_support::{
    dispatch::DispatchResult,
    pallet_prelude::{MaxEncodedLen, MaybeSerializeDeserialize, Member},
    sp_runtime::traits::{CheckEqual, MaybeDisplay, SimpleBitOps},
    traits::{fungible, Incrementable},
    BoundedBTreeSet, Parameter,
};
use scale_info::{prelude::fmt::Debug, TypeInfo};
use sp_core::Get;
use sp_runtime::{
    traits::{
        AtLeast32BitUnsigned, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Hash, One,
        Saturating, Zero,
    },
    BoundedVec, DispatchError,
};
use sp_std::{collections::btree_set::BTreeSet, vec::Vec};

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

/// Storage Hub global tick which should be relied upon for time-sensitive operations.
pub trait StorageHubTickGetter {
    type TickNumber: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Debug
        + Ord
        + MaxEncodedLen
        + One;

    fn get_current_tick() -> Self::TickNumber;
}

pub trait NumericalParam:
    Parameter
    + Member
    + MaybeSerializeDeserialize
    + Debug
    + Ord
    + MaxEncodedLen
    + Copy
    + Default
    + Zero
    + One
    + Saturating
    + CheckedAdd
    + CheckedMul
    + CheckedDiv
    + CheckedSub
    + AtLeast32BitUnsigned
    + HasCompact
    + FullCodec
{
}

// Automatically implement `NumericalParam` for any type that implements all the required traits
impl<T> NumericalParam for T where
    T: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Debug
        + Ord
        + MaxEncodedLen
        + Copy
        + Default
        + Zero
        + One
        + Saturating
        + CheckedAdd
        + CheckedMul
        + CheckedDiv
        + CheckedSub
        + AtLeast32BitUnsigned
        + HasCompact
        + FullCodec
{
}

/// A trait to read information about buckets registered in the system, such as their owner and
/// the MSP ID of the MSP that's storing it, etc.
pub trait ReadBucketsInterface {
    /// Type that can be used to identify accounts.
    type AccountId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;

    /// Type of the registered Providers' IDs.
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

    /// Type of the buckets' IDs.
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

    /// Type that represents the unit of storage data in which the capacity is measured.
    type StorageDataUnit: NumericalParam + MaybeDisplay;

    /// Type of the root of the buckets.
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

    /// Type of the ID that identifies a value proposition.
    type ValuePropId: Parameter
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

    /// Type of a bucket's read-access group's ID (which is the read-access NFT collection's ID).
    type ReadAccessGroupId: Member + Parameter + MaxEncodedLen + Copy + Incrementable;

    /// Byte limit of a bucket's name.
    type BucketNameLimit: Get<u32>;

    /// Check if a bucket exists.
    fn bucket_exists(bucket_id: &Self::BucketId) -> bool;

    /// Return if a bucket (represented by its Bucket ID) is stored by a specific MSP.
    fn is_bucket_stored_by_msp(msp_id: &Self::ProviderId, bucket_id: &Self::BucketId) -> bool;

    /// Get the read-access group's ID of a bucket, if there is one.
    fn get_read_access_group_id_of_bucket(
        bucket_id: &Self::BucketId,
    ) -> Result<Option<Self::ReadAccessGroupId>, DispatchError>;

    /// Get the MSP ID of the MSP that's storing a bucket.
    fn get_msp_of_bucket(
        bucket_id: &Self::BucketId,
    ) -> Result<Option<Self::ProviderId>, DispatchError>;

    /// Get the MSP ID of the MSP that's storing a bucket.
    fn get_bucket_msp(
        bucket_id: &Self::BucketId,
    ) -> Result<Option<Self::ProviderId>, DispatchError>;

    /// Check if an account is the owner of a bucket.
    fn is_bucket_owner(
        who: &Self::AccountId,
        bucket_id: &Self::BucketId,
    ) -> Result<bool, DispatchError>;

    /// Check if a bucket is private.
    fn is_bucket_private(bucket_id: &Self::BucketId) -> Result<bool, DispatchError>;

    /// Derive the Bucket Id of a bucket, from its owner and name.
    fn derive_bucket_id(
        owner: &Self::AccountId,
        bucket_name: BoundedVec<u8, Self::BucketNameLimit>,
    ) -> Self::BucketId;

    /// Get the root of a bucket.
    fn get_root_bucket(bucket_id: &Self::BucketId) -> Option<Self::MerkleHash>;

    /// Get the bucket owner.
    fn get_bucket_owner(bucket_id: &Self::BucketId) -> Result<Self::AccountId, DispatchError>;

    /// Get bucket size.
    fn get_bucket_size(bucket_id: &Self::BucketId) -> Result<Self::StorageDataUnit, DispatchError>;

    /// Get the bucket's value proposition ID.
    fn get_bucket_value_prop_id(
        bucket_id: &Self::BucketId,
    ) -> Result<Self::ValuePropId, DispatchError>;
}

/// A trait to change the state of buckets registered in the system, such as updating their privacy
/// settings, changing their root, etc.
pub trait MutateBucketsInterface {
    /// Type that can be used to identify accounts.
    type AccountId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;

    /// Type of the registered Providers' IDs.
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

    /// Type of the buckets' IDs.
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

    /// Type that represents the unit of storage data in which the capacity is measured.
    type StorageDataUnit: NumericalParam + MaybeDisplay;

    /// Type of a bucket's read-access group's ID (which is the read-access NFT collection's ID).
    type ReadAccessGroupId: Member + Parameter + MaxEncodedLen + Copy + Incrementable;

    /// Type of the root and keys in the Merkle Patricia Forest of a
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

    /// Type of the ID that identifies a value proposition.
    type ValuePropId: Parameter
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

    /// Update a bucket's privacy setting.
    fn update_bucket_privacy(bucket_id: Self::BucketId, privacy: bool) -> DispatchResult;

    /// Update a bucket's read-access group's ID. If None is passed, no one other than the owner
    /// will be able to access the bucket.
    fn update_bucket_read_access_group_id(
        bucket_id: Self::BucketId,
        maybe_read_access_group_id: Option<Self::ReadAccessGroupId>,
    ) -> DispatchResult;

    /// Add a new bucket under the MSP corresponding to `provider_id`, that will be owned by the account `user_id`.
    /// If `privacy` is true, the bucket will be private and optionally the `read_access_group_id` will be used to
    /// determine the collection of NFTs that can access the bucket.
    fn add_bucket(
        provider_id: Self::ProviderId,
        user_id: Self::AccountId,
        bucket_id: Self::BucketId,
        privacy: bool,
        maybe_read_access_group_id: Option<Self::ReadAccessGroupId>,
        value_prop_id: Self::ValuePropId,
    ) -> DispatchResult;

    /// Change the MSP that's currently storing the bucket.
    ///
    /// The new value proposition selected must belong to the new MSP.
    /// Keep in mind this function does not check if the new MSP has enough capacity to store the bucket,
    /// nor it updates the MSPs' used capacity. It only updates the payment streams and the lists of buckets
    /// stored each MSP has.
    fn assign_msp_to_bucket(
        bucket_id: &Self::BucketId,
        new_msp_id: &Self::ProviderId,
        new_value_prop_id: &Self::ValuePropId,
    ) -> DispatchResult;

    /// Set a bucket's `msp_id` to `None` and also removing the element from the list in `MainStorageProviderIdsToBuckets`
    fn unassign_msp_from_bucket(bucket_id: &Self::BucketId) -> DispatchResult;

    /// Change the root of a bucket.
    fn change_root_bucket(bucket_id: Self::BucketId, new_root: Self::MerkleHash) -> DispatchResult;

    /// Remove a root from a bucket of a MSP, removing the whole bucket from storage.
    fn delete_bucket(bucket_id: Self::BucketId) -> DispatchResult;

    // Delete a bucket without checking whether it's empty or its root is the default one.
    // Useful for cases when the runtime has to delete a bucket no matter its current status,
    // for example for an insolvent user.
    fn force_delete_bucket(msp_id: &Self::ProviderId, bucket_id: &Self::BucketId)
        -> DispatchResult;

    /// Increase the size of a bucket.
    fn increase_bucket_size(
        bucket_id: &Self::BucketId,
        delta: Self::StorageDataUnit,
    ) -> DispatchResult;

    /// Decrease the size of a bucket.
    fn decrease_bucket_size(
        bucket_id: &Self::BucketId,
        delta: Self::StorageDataUnit,
    ) -> DispatchResult;

    /// Decrease the size of a bucket that does not have an MSP.
    fn decrease_bucket_size_without_msp(
        bucket_id: &Self::BucketId,
        delta: Self::StorageDataUnit,
    ) -> DispatchResult;

    /// Change the root of a bucket that does not have an MSP.
    fn change_root_bucket_without_msp(
        bucket_id: Self::BucketId,
        new_root: Self::MerkleHash,
    ) -> DispatchResult;
}

/// A trait to read information about Storage Providers present in the
/// `storage-providers` pallet, such as if they are a BSP or MSP, their multiaddresses, etc.
pub trait ReadStorageProvidersInterface {
    /// Type of the registered Providers' IDs.
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

    /// Type that represents the unit of storage data in which the capacity is measured.
    type StorageDataUnit: NumericalParam + MaybeDisplay;

    /// Type of the counter of the total number of registered Storage Providers.
    type SpCount: NumericalParam + scale_info::TypeInfo;

    /// Type that represents a MultiAddress of a Storage Provider.
    type MultiAddress: Parameter
        + MaybeSerializeDeserialize
        + Debug
        + Ord
        + Default
        + AsRef<[u8]>
        + AsMut<[u8]>
        + MaxEncodedLen
        + FullCodec;

    /// Maximum number of MultiAddresses a provider can have.
    type MaxNumberOfMultiAddresses: Get<u32>;
    /// Type that represents the reputation weight of a Storage Provider.
    type ReputationWeight: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Default
        + Ord
        + FullCodec
        + Copy
        + Debug
        + scale_info::TypeInfo
        + MaxEncodedLen
        + CheckedAdd
        + One
        + Saturating
        + PartialOrd
        + sp_runtime::traits::Zero;

    /// Type of the ID that identifies a value proposition.
    type ValuePropId: Parameter
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

    /// Check if provider is a BSP.
    fn is_bsp(who: &Self::ProviderId) -> bool;

    /// Check if provider is a MSP.
    fn is_msp(who: &Self::ProviderId) -> bool;

    /// Get the total global reputation weight of all BSPs.
    fn get_global_bsps_reputation_weight() -> Self::ReputationWeight;

    /// Get the reputation weight of a registered Provider.
    fn get_bsp_reputation_weight(
        who: &Self::ProviderId,
    ) -> Result<Self::ReputationWeight, DispatchError>;

    /// Get number of registered BSPs.
    fn get_number_of_bsps() -> Self::SpCount;

    /// Get the capacity of a Provider (MSP or BSP).
    fn get_capacity(who: &Self::ProviderId) -> Self::StorageDataUnit;

    /// Get the capacity currently in use of a Provider (MSP or BSP).
    fn get_used_capacity(who: &Self::ProviderId) -> Self::StorageDataUnit;

    /// Get available capacity of a Provider (MSP or BSP).
    fn available_capacity(who: &Self::ProviderId) -> Self::StorageDataUnit;

    /// Get multiaddresses of a BSP.
    fn get_bsp_multiaddresses(
        who: &Self::ProviderId,
    ) -> Result<BoundedVec<Self::MultiAddress, Self::MaxNumberOfMultiAddresses>, DispatchError>;

    /// Check if a value proposition belongs to a MSP. Keep in mind this does not error out
    /// if the MSP does not exist, but returns false.
    fn is_value_prop_of_msp(who: &Self::ProviderId, value_prop_id: &Self::ValuePropId) -> bool;

    /// Check whether a value proposition of a MSP is currently available. Keep in mind this does not
    /// error out if the MSP or the value proposition does not exist, but returns false.
    fn is_value_prop_available(who: &Self::ProviderId, value_prop_id: &Self::ValuePropId) -> bool;
}

/// A trait to mutate the state of Storage Providers present in the `storage-providers` pallet.
/// This includes increasing and decreasing the data used by a Storage Provider.
pub trait MutateStorageProvidersInterface {
    /// Type of the registered Providers' IDs.
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

    /// Type that represents the unit of storage data in which the capacity is measured.
    type StorageDataUnit: NumericalParam;

    /// Increase the used capacity of a Storage Provider (MSP or BSP). To be called when confirming
    /// that it's storing a new file.
    fn increase_capacity_used(
        provider_id: &Self::ProviderId,
        delta: Self::StorageDataUnit,
    ) -> DispatchResult;

    /// Decrease the used capacity of a Storage Provider (MSP or BSP). To be called when confirming
    /// that it has deleted a previously stored file.
    fn decrease_capacity_used(
        provider_id: &Self::ProviderId,
        delta: Self::StorageDataUnit,
    ) -> DispatchResult;
}

/// A trait to read information about generic challengeable Providers, such as their ID, owner, root,
/// stake, etc.
pub trait ReadChallengeableProvidersInterface {
    /// Type that can be used to identify accounts.
    type AccountId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;

    /// Type of the registered challengeable Providers' IDs.
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

    /// Type of the root and keys in the Merkle Patricia Forest of a
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

    /// The Balance type of the runtime, which should correspond to the type of
    /// the staking balance of a registered Provider.
    type Balance: fungible::Inspect<Self::AccountId> + fungible::hold::Inspect<Self::AccountId>;

    /// Check if an account is a registered challengeable Provider.
    fn is_provider(who: Self::ProviderId) -> bool;

    /// Get the Provider Id from Account Id, if it is a registered challengeable Provider.
    fn get_provider_id(who: &Self::AccountId) -> Option<Self::ProviderId>;

    /// Get the Account Id of the owner of a registered challengeable Provider.
    fn get_owner_account(who: Self::ProviderId) -> Option<Self::AccountId>;

    /// Get the root for a registered challengeable Provider.
    fn get_root(who: Self::ProviderId) -> Option<Self::MerkleHash>;

    /// Get the default value for the root of a Merkle Patricia Forest.
    fn get_default_root() -> Self::MerkleHash;

    /// Get the stake for a registered challengeable Provider.
    fn get_stake(
        who: Self::ProviderId,
    ) -> Option<<Self::Balance as fungible::Inspect<Self::AccountId>>::Balance>;

    /// Get the minimum stake for a registered challengeable Provider.
    fn get_min_stake() -> <Self::Balance as fungible::Inspect<Self::AccountId>>::Balance;
}

/// A trait to mutate the state of challengeable Providers, such as updating their root.
pub trait MutateChallengeableProvidersInterface {
    /// Type of the registered challengeable Providers' IDs.
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

    /// Type of the root and keys in the Merkle Patricia Forest of a
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

    /// Update the root for a registered challengeable Provider.
    fn update_root(who: Self::ProviderId, new_root: Self::MerkleHash) -> DispatchResult;

    /// Update the information of a registered challengeable Provider after a successful trie element removal.
    fn update_provider_after_key_removal(
        who: &Self::ProviderId,
        removed_trie_value: &Vec<u8>,
    ) -> DispatchResult;
}

/// A trait to read information about generic Providers, such as their ID, owner, root, stake, etc.
pub trait ReadProvidersInterface {
    /// Type that can be used to identify accounts.
    type AccountId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;

    /// Type of the registered Providers' IDs.
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

    /// Type of the root and keys in the Merkle Patricia Forest of a
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

    /// The Balance type of the runtime, which should correspond to the type of
    /// the staking balance of a registered Provider.
    type Balance: fungible::Inspect<Self::AccountId> + fungible::hold::Inspect<Self::AccountId>;

    /// Check if an account is a registered Provider.
    fn is_provider(who: Self::ProviderId) -> bool;

    /// Get the Provider Id from Account Id, if it is a registered Provider.
    fn get_provider_id(who: &Self::AccountId) -> Option<Self::ProviderId>;

    /// Get the Account Id of the owner of a registered Provider.
    fn get_owner_account(who: Self::ProviderId) -> Option<Self::AccountId>;

    /// Get the Account Id of the payment account of a registered Provider.
    fn get_payment_account(who: Self::ProviderId) -> Option<Self::AccountId>;

    /// Get the root for a registered Provider.
    fn get_root(who: Self::ProviderId) -> Option<Self::MerkleHash>;

    /// Get the default value for the root of a Merkle Patricia Forest.
    fn get_default_root() -> Self::MerkleHash;

    /// Get the stake for a registered Provider.
    fn get_stake(
        who: Self::ProviderId,
    ) -> Option<<Self::Balance as fungible::Inspect<Self::AccountId>>::Balance>;

    /// Check if the provider is insolvent.
    fn is_provider_insolvent(who: Self::ProviderId) -> bool;
}

/// A trait to mutate the state of a generic Provider, such as updating their root.
pub trait MutateProvidersInterface {
    /// Type of the registered Providers' IDs.
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

    /// Type of the root and keys in the Merkle Patricia Forest of a
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

    /// Update the root for a registered Provider.
    fn update_root(who: Self::ProviderId, new_root: Self::MerkleHash) -> DispatchResult;
}

/// A trait to get system-wide metrics, such as the total available capacity of the network and
/// its total used capacity.
pub trait SystemMetricsInterface {
    /// Type of the unit provided by Providers
    type ProvidedUnit: NumericalParam;

    /// Get the total available capacity of units of the network.
    fn get_total_capacity() -> Self::ProvidedUnit;

    /// Get the total used capacity of units of the network.
    fn get_total_used_capacity() -> Self::ProvidedUnit;
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
        + PartialEq
        + Eq
        + Clone
        + FullCodec;
    /// The hashing system (algorithm) being used for the Merkle Patricia Forests (e.g. Blake2).
    type MerkleHashing: Hash<Output = Self::MerkleHash>;
    /// The type that represents the randomness output.
    type RandomnessOutput: Parameter + Member + Debug;
    /// The numerical type used to represent ticks.
    /// The Proofs Dealer pallet uses ticks to keep track of time, for things like sending out
    /// challenges and making sure that Providers respond to them in time
    type TickNumber: NumericalParam;

    /// Verify a proof just for the Merkle Patricia Forest, for a given Provider.
    ///
    /// This only verifies that something is included in the forest of the Provider. It is not a full
    /// proof of the Provider's data.
    fn verify_forest_proof(
        provider_id: &Self::ProviderId,
        challenges: &[Self::MerkleHash],
        proof: &Self::ForestProof,
    ) -> Result<BTreeSet<Self::MerkleHash>, DispatchError>;

    /// Verify a proof for a Merkle Patricia Forest, without requiring it to be associated with a Provider.
    ///
    /// WARNING: This function should be used with caution, as it does not verify the root against a specific Provider.
    /// This means this function should only be used when the root is previously known to be correct, and in NO case should
    /// it be used to verify proofs associated with a challengeable Provider. That is what `verify_forest_proof` is for.
    ///
    /// This only verifies that something is included in the forest that has the given root. It is not a full
    /// proof of its data.
    fn verify_generic_forest_proof(
        root: &Self::MerkleHash,
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
        should_remove_key: bool,
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

    /// Apply delta (mutations) to the partial trie based on the proof and the commitment.
    ///
    /// WARNING: This function should be used with caution, as it does not verify the root against a specific Provider.
    /// This means this function should only be used when the root is previously known to be correct, and in NO case should
    /// it be used to verify proofs associated with a challengeable Provider. That is what `apply_delta` is for.
    ///
    /// The new root is returned.
    ///
    /// Additional information for context on where the mutations were applied can be provided
    /// by using the `event_info` field.
    fn generic_apply_delta(
        root: &Self::MerkleHash,
        mutations: &[(Self::MerkleHash, TrieMutation)],
        proof: &Self::ForestProof,
        event_info: Option<Vec<u8>>,
    ) -> Result<Self::MerkleHash, DispatchError>;

    /// Stop a Provider's challenge cycle.
    ///
    /// If the provider doesn't have any files left its random challenge cycle is stopped since it shouldn't
    /// submit any proofs.
    fn stop_challenge_cycle(provider_id: &Self::ProviderId) -> DispatchResult;

    /// Initialise a Provider's challenge cycle.
    ///
    /// Sets the last tick the Provider submitted a proof for to the current tick and sets the
    /// deadline for submitting a proof:
    /// ```ignore
    /// last_tick_provider_submitted_proof_for = current_tick
    ///
    /// deadline = current_tick + provider_challenge_period + challenges_tolerance.
    /// ```
    ///
    /// The Provider's challenge period is calculated based on its stake.
    fn initialise_challenge_cycle(provider_id: &Self::ProviderId) -> DispatchResult;

    /// Get the current tick.
    ///
    /// The Proofs Dealer pallet uses ticks to keep track of time, for things like sending out
    /// challenges and making sure that Providers respond to them in time.
    fn get_current_tick() -> Self::TickNumber;

    /// Get the checkpoint challenge period.
    ///
    /// The Proofs Dealer pallet uses the checkpoint challenge period to determine the time period
    /// between checkpoint challenges.
    fn get_checkpoint_challenge_period() -> Self::TickNumber;
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
#[derive(Encode, Decode, TypeInfo, Clone, PartialEq, Debug)]
pub enum TrieMutation {
    Add(TrieAddMutation),
    Remove(TrieRemoveMutation),
}

#[derive(Encode, Decode, TypeInfo, Clone, PartialEq, Debug, Default)]
pub struct TrieAddMutation {
    pub value: Vec<u8>,
}

impl Into<TrieMutation> for TrieAddMutation {
    fn into(self) -> TrieMutation {
        TrieMutation::Add(self)
    }
}

impl TrieAddMutation {
    pub fn new(value: Vec<u8>) -> Self {
        Self { value }
    }
}

#[derive(Encode, Decode, TypeInfo, Clone, PartialEq, Debug, Default)]
pub struct TrieRemoveMutation {
    pub maybe_value: Option<Vec<u8>>,
}

impl TrieRemoveMutation {
    pub fn new() -> Self {
        Self { maybe_value: None }
    }

    pub fn with_value(value: Vec<u8>) -> Self {
        Self {
            maybe_value: Some(value),
        }
    }
}

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
    /// Returns the MemoryDB of the trie generated by the proof, the new root computed after applying the mutations
    /// and a vector of the key-value pairs that were changed by the mutations.
    fn apply_delta(
        root: &Self::Key,
        mutations: &[(Self::Key, TrieMutation)],
        proof: &Self::Proof,
    ) -> Result<
        (
            sp_trie::MemoryDB<H>,
            Self::Key,
            Vec<(Self::Key, Option<Vec<u8>>)>,
        ),
        DispatchError,
    >;
}

/// Interface used by the file system pallet in order to read storage from NFTs pallet (avoiding tight coupling).
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
    /// The type which represents ticks.
    ///
    /// Used to keep track of the system time.
    type TickNumber: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
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
    type Units: NumericalParam
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

    /// Get inner rate value of a fixed-rate payment stream between a User and a Provider
    fn get_inner_fixed_rate_payment_stream_value(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> Option<<Self::Balance as fungible::Inspect<Self::AccountId>>::Balance>;

    /// Check if a fixed-rate payment stream exists between a User and a Provider.
    fn fixed_rate_payment_stream_exists(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> bool;

    /// Create a new dynamic-rate payment stream from a User to a Provider.
    fn create_dynamic_rate_payment_stream(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
        amount_provided: &Self::Units,
    ) -> DispatchResult;

    /// Update the amount provided of an existing dynamic-rate payment stream.
    fn update_dynamic_rate_payment_stream(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
        new_amount_provided: &Self::Units,
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

    /// Get the amount provided of a dynamic-rate payment stream between a User and a Provider
    fn get_dynamic_rate_payment_stream_amount_provided(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> Option<Self::Units>;

    /// Check if a user has an active payment stream with a provider.
    fn has_active_payment_stream_with_user(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> bool;

    /// Check if a provider has any active payment streams.
    fn has_active_payment_stream(provider_id: &Self::ProviderId) -> bool;

    /// Add a privileged provider to the PrivilegedProviders storage, allowing it to charge every tick.
    fn add_privileged_provider(provider_id: &Self::ProviderId);

    /// Remove a privileged provider to the PrivilegedProviders storage.
    fn remove_privileged_provider(provider_id: &Self::ProviderId);

    /// Get current tick.
    fn current_tick() -> Self::TickNumber;
}

/// The interface of the Payment Streams pallet that allows for the reading of user's solvency.
pub trait ReadUserSolvencyInterface {
    /// The type which represents a User account identifier.
    type AccountId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;

    /// Get if a user has been flagged as insolvent (without funds)
    fn is_user_insolvent(user_account: &Self::AccountId) -> bool;
}

/// A trait to get and set the price per giga-unit per tick of the network.
///
/// This is used by the Payment Streams pallet to expose the function to get and update the price
/// per giga-unit per tick, which governs the amount to charge for dynamic-rate payment streams.
///
/// The use of giga-units instead of units is to avoid issues with decimal places, since the Balance type
/// might not granular enough to represent the price per unit.
pub trait PricePerGigaUnitPerTickInterface {
    /// The type which represents a price per unit per tick.
    type PricePerGigaUnitPerTick: NumericalParam;

    /// Get the price per unit per tick.
    fn get_price_per_giga_unit_per_tick() -> Self::PricePerGigaUnitPerTick;

    /// Update the price per unit per tick..
    fn set_price_per_giga_unit_per_tick(price_index: Self::PricePerGigaUnitPerTick);
}

/// The interface of the ProofsDealer pallet that allows other pallets to query and modify proof
/// submitters in the last ticks.
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

    fn get_current_tick() -> Self::TickNumber;

    fn get_accrued_failed_proof_submissions(provider_id: &Self::ProviderId) -> Option<u32>;

    fn clear_accrued_failed_proof_submissions(provider_id: &Self::ProviderId);
}

/// A trait to encode, decode and read information from file metadata.
pub trait FileMetadataInterface {
    /// The type which represents a file's metadata
    type Metadata: Parameter + Member + MaybeSerializeDeserialize + Debug + Encode + Decode;
    /// The type which represents the unit that we use to measure file size (e.g. bytes)
    type StorageDataUnit: NumericalParam + Into<u64>;

    fn decode(data: &[u8]) -> Result<Self::Metadata, codec::Error>;

    fn encode(metadata: &Self::Metadata) -> Vec<u8>;

    fn get_file_size(metadata: &Self::Metadata) -> Self::StorageDataUnit;

    fn owner(metadata: &Self::Metadata) -> &Vec<u8>;
}

/// A trait for implementing the formula to update the price of a unit of stored data.
///
/// This is used by the File System pallet, which requires some type to implement this trait,
/// and uses such implementation to update the price of a unit of stored data on every
/// `on_poll` hook execution.
pub trait UpdateStoragePrice {
    /// The numerical type which represents the price of a storage request.
    type Price: NumericalParam;
    /// The numerical type which represents units of storage data.
    type StorageDataUnit: NumericalParam;

    /// Update the price of a storage request.
    ///
    /// Takes into consideration, the total capacity of the network and the used capacity.
    /// Returns the new price of the storage request, according to the chosen formula.
    fn update_storage_price(
        current_price: Self::Price,
        used_capacity: Self::StorageDataUnit,
        total_capacity: Self::StorageDataUnit,
    ) -> Self::Price;
}

/// A trait to calculate the cut that should go to the Treasury, from what's charged by a Provider.
///
/// This is used by the Payment Streams pallet, which requires some type to implement this trait,
/// and uses such implementation to calculate the cut that should go to the Treasury, from what's charged by a Provider.
pub trait TreasuryCutCalculator {
    /// The numerical type which represents the balance type of the runtime.
    type Balance: NumericalParam;
    /// Type of the unit provided by Providers
    type ProvidedUnit: NumericalParam;

    /// Calculate the percentage of charged funds by a Provider that should go to the treasury.
    ///
    /// Returns the percentage of charged funds by a Provider that should go to the treasury.
    fn calculate_treasury_cut(
        provided_amount: Self::ProvidedUnit,
        used_amount: Self::ProvidedUnit,
        amount_to_charge: Self::Balance,
    ) -> Self::Balance;
}

/// The interface for the Commit-Reveal Randomness pallet.
pub trait CommitRevealRandomnessInterface {
    /// The type which represents a Provider's ID.
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

    /// Initialise a Provider's randomness commit-reveal cycle.
    ///
    /// Sets the Provider as a ProviderWithoutCommitment (that is, a Provider that has
    /// not submitted a seed commitment previously) and the sets its deadline to submit
    /// the initial seed commitment to the current tick + the Provider's period
    /// (based on its stake) + the randomness tick tolerance.
    fn initialise_randomness_cycle(who: &Self::ProviderId) -> DispatchResult;

    /// Stop a Provider's randomness commit-reveal cycle.
    ///
    /// This cleans up the Provider's used storage and allows the Provider
    /// to not be penalized for not submitting more randomness seeds.
    /// This makes it so this function should only be called when a Provider
    /// is being signed off from the network.
    fn stop_randomness_cycle(who: &Self::ProviderId) -> DispatchResult;
}
