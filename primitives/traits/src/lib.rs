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
    type StorageDataUnit: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Default
        + MaybeDisplay
        + AtLeast32BitUnsigned
        + Copy
        + MaxEncodedLen
        + HasCompact
        + Into<u64>;

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
    fn get_msp_of_bucket(bucket_id: &Self::BucketId) -> Option<Self::ProviderId>;

    /// Check if an account is the owner of a bucket.
    fn is_bucket_owner(
        who: &Self::AccountId,
        bucket_id: &Self::BucketId,
    ) -> Result<bool, DispatchError>;

    /// Check if a bucket is private.
    fn is_bucket_private(bucket_id: &Self::BucketId) -> Result<bool, DispatchError>;

    /// Derive the Bucket Id of a bucket, from its MSP, owner and name.
    fn derive_bucket_id(
        msp_id: &Self::ProviderId,
        owner: &Self::AccountId,
        bucket_name: BoundedVec<u8, Self::BucketNameLimit>,
    ) -> Self::BucketId;

    /// Get the root of a bucket.
    fn get_root_bucket(bucket_id: &Self::BucketId) -> Option<Self::MerkleHash>;

    /// Get bucket size.
    fn get_bucket_size(bucket_id: &Self::BucketId) -> Result<Self::StorageDataUnit, DispatchError>;

    /// Get the MSP of a bucket.
    fn get_msp_bucket(bucket_id: &Self::BucketId) -> Result<Self::ProviderId, DispatchError>;
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
    type StorageDataUnit: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Default
        + MaybeDisplay
        + AtLeast32BitUnsigned
        + Copy
        + MaxEncodedLen
        + HasCompact
        + Into<u64>;

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
    ) -> DispatchResult;

    /// Change MSP of a bucket.
    fn change_msp_bucket(bucket_id: &Self::BucketId, new_msp: &Self::ProviderId) -> DispatchResult;

    /// Change the root of a bucket.
    fn change_root_bucket(bucket_id: Self::BucketId, new_root: Self::MerkleHash) -> DispatchResult;

    /// Remove a root from a bucket of a MSP, removing the whole bucket from storage.
    fn remove_root_bucket(bucket_id: Self::BucketId) -> DispatchResult;

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
    type StorageDataUnit: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Default
        + MaybeDisplay
        + AtLeast32BitUnsigned
        + Copy
        + MaxEncodedLen
        + HasCompact
        + Into<u64>;

    /// Type of the counter of the total number of registered Storage Providers.
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
    type StorageDataUnit: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Default
        + MaybeDisplay
        + AtLeast32BitUnsigned
        + Copy
        + MaxEncodedLen
        + HasCompact
        + Into<u64>;

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
    fn get_provider_id(who: Self::AccountId) -> Option<Self::ProviderId>;

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
    fn get_provider_id(who: Self::AccountId) -> Option<Self::ProviderId>;

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
    type ProvidedUnit: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Default
        + MaybeDisplay
        + AtLeast32BitUnsigned
        + Copy
        + MaxEncodedLen
        + HasCompact
        + Into<u64>;

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
        + FullCodec;
    /// The hashing system (algorithm) being used for the Merkle Patricia Forests (e.g. Blake2).
    type MerkleHashing: Hash<Output = Self::MerkleHash>;
    /// The type that represents the randomness output.
    type RandomnessOutput: Parameter + Member + Debug;
    /// The numerical type used to represent ticks.
    /// The Proofs Dealer pallet uses ticks to keep track of time, for things like sending out
    /// challenges and making sure that Providers respond to them in time
    type TickNumber: Parameter
        + Member
        + AtLeast32BitUnsigned
        + Debug
        + Default
        + Copy
        + MaxEncodedLen
        + FullCodec
        + MaybeSerializeDeserialize
        + Zero
        + One
        + CheckedAdd
        + CheckedSub
        + CheckedDiv
        + CheckedMul
        + Saturating;

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

    /// Apply delta (mutations) to the partial trie based on the proof and the commitment.
    ///
    /// WARNING: This function should be used with caution, as it does not verify the root against a specific Provider.
    /// This means this function should only be used when the root is previously known to be correct, and in NO case should
    /// it be used to verify proofs associated with a challengeable Provider. That is what `apply_delta` is for.
    ///
    /// The new root is returned.
    fn generic_apply_delta(
        root: &Self::MerkleHash,
        mutations: &[(Self::MerkleHash, TrieMutation)],
        proof: &Self::ForestProof,
    ) -> Result<Self::MerkleHash, DispatchError>;

    /// Initialise a Provider's challenge cycle.
    ///
    /// Sets the last tick the Provider submitted a proof for to the current tick and sets the
    /// deadline for submitting a proof to the current tick + the Provider's period (based on its
    /// stake) + the challenges tick tolerance.
    fn initialise_challenge_cycle(who: &Self::ProviderId) -> DispatchResult;

    /// Get the current tick.
    ///
    /// The Proofs Dealer pallet uses ticks to keep track of time, for things like sending out
    /// challenges and making sure that Providers respond to them in time.
    fn get_current_tick() -> Self::TickNumber;
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
    fn has_active_payment_stream(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> bool;
}

/// The interface of the Payment Streams pallet that allows for the reading of user's solvency.
pub trait ReadUserSolvencyInterface {
    /// The type which represents a User account identifier.
    type AccountId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;

    /// Get if a user has been flagged as insolvent (without funds)
    fn is_user_insolvent(user_account: &Self::AccountId) -> bool;
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

    fn get_accrued_failed_proof_submissions(provider_id: &Self::ProviderId) -> Option<u32>;

    fn clear_accrued_failed_proof_submissions(provider_id: &Self::ProviderId);
}
