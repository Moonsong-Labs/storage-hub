use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use frame_support::traits::fungible::Inspect;
use frame_support::BoundedVec;
use scale_info::TypeInfo;

use crate::Config;

// We have to create:
// 1) A mapping from AccountId to BackupStorageProviderId (AccountId + salt, hashed)
// 2) A mapping from AccountId to MainStorageProviderId (AccountId + salt, hashed).
// (Where the StorageProviderId is saved depends on how the SP registered itself)
// 3) A mapping from BackupStorageProviderId to BackupStorageProvider (metadata such as root, total data available, total data used, multiaddresses, etc)
// 4) A mapping from MainStorageProviderId to MainStorageProvider (metadata such as vector of bucket ids, total data available, total data used, multiaddresses, etc)
// 5) A mapping from BucketId to Bucket (metadata such as root, User ID, MainStorageProviderId, etc)
// (It probably makes sense to have a basic MerkleTrieHolder structure that has the root and any general metadata, and then have the BackupStorageProvider and Bucket structs that inherit from MerkleTrieHolder and have any extra metadata)

// Then, we can cover the needed functionality for the Proofs pallet:
// - The is_provider function can be implemented by checking if the ProviderId given is either a BackupStorageProvider (by using map 3) or a Bucket (by using map 5)
// - The get_provider function can be implemented by returning the BackupStorageProviderId for a BackupStorageProvider (using map 1 + checking if BSP exists in map 3) or None otherwise (MSPs have to pass the BucketId to the proofs pallet)
// - The get_root function can be implemented (considering the MerkleTrieHolder struct has the root) using map 3 for BackupStorageProviders and map 5 for Buckets
// - The get_stake function can be implemented by using map 3 (for BackupStorageProviders) or map 5 into map 4 with MainStorageProviderId (for Buckets)

/// Structure that has the possible value propositions that a Main Storage Provider can offer (and the runtime is aware of)
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebugNoBound, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct ValueProposition<T: Config> {
    pub identifier: ValuePropId<T>,
    pub data_limit: StorageData<T>,
    pub protocols: BoundedVec<Protocols<T>, MaxProtocols<T>>,
    // todo!("add relevant fields here")
}

/// Structure that represents a Main Storage Provider. It holds the buckets that the MSP has, the total data that the MSP is able to store,
/// the amount of data that it is storing, and its libp2p multiaddresses.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebugNoBound, Clone)]
#[scale_info(skip_type_params(T))]
pub struct MainStorageProvider<T: Config> {
    pub buckets: Buckets<T>,
    pub total_data: StorageData<T>,
    pub data_used: StorageData<T>,
    pub multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>>,
    pub value_prop: ValueProposition<T>,
}

/// Structure that represents a Backup Storage Provider. It holds the total data that the BSP is able to store, the amount of data that it is storing,
/// its libp2p multiaddresses, and the root of the Merkle Patricia Trie that it stores.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebugNoBound, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct BackupStorageProvider<T: Config> {
    pub total_data: StorageData<T>,
    pub data_used: StorageData<T>,
    pub multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>>,
    pub root: MerklePatriciaRoot<T>,
}

/// Structure that represents a Bucket. It holds the root of the Merkle Patricia Trie, the User ID that owns the bucket,
/// and the MainStorageProviderId that the bucket belongs to.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebugNoBound, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct Bucket<T: Config> {
    pub root: MerklePatriciaRoot<T>,
    pub user_id: T::AccountId,
    pub msp_id: MainStorageProviderId<T>,
}

// Type aliases:

/// BalanceOf is the balance type of the runtime.
pub type BalanceOf<T> =
    <<T as Config>::NativeBalance as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

/// BackupStorageProviderId is the type that represents an ID of a Backup Storage Provider, uniquely linked with an AccountId
pub type BackupStorageProviderId<T> = <T as crate::Config>::HashId;
/// HashId is the type that represents an ID of a Main Storage Provider, uniquely linked with an AccountId
pub type MainStorageProviderId<T> = <T as crate::Config>::HashId;

/// MaxMultiAddressSize is the maximum size of the libp2p multiaddress of a Storage Provider in bytes.
pub type MaxMultiAddressSize<T> = <T as crate::Config>::MaxMultiAddressSize;
/// MaxMultiAddressAmount is the maximum amount of MultiAddresses that a Storage Provider can have.
pub type MaxMultiAddressAmount<T> = <T as crate::Config>::MaxMultiAddressAmount;
/// MultiAddress is a byte array that represents the libp2p multiaddress of a Storage Provider.
/// Its maximum size is defined in the runtime configuration, as MaxMultiAddressSize.
pub type MultiAddress<T> = BoundedVec<u8, MaxMultiAddressSize<T>>;

/// MerklePatriciaRoot is the type of the root of a Merkle Patricia Trie, either the root of a BSP or a bucket from an MSP.
pub type MerklePatriciaRoot<T> = <T as crate::Config>::MerklePatriciaRoot;
/// MerkleTrieHolderId is the type that identifies the different Merkle Patricia Trie holders (BSPs and buckets).
pub type HashId<T> = <T as crate::Config>::HashId;

/// StorageData is the type of the unit in which we measure data size. We define its required traits in the
/// pallet configuration so the runtime can use any type that implements them.
pub type StorageData<T> = <T as crate::Config>::StorageData;

/// Protocols is a vector of the protocols that (the runtime is aware of and) the Main Storage Provider supports.
/// Its maximum size is defined in the runtime configuration, as MaxProtocols.
pub type MaxProtocols<T> = <T as crate::Config>::MaxProtocols;
pub type Protocols<T> = BoundedVec<u8, MaxProtocols<T>>; // todo!("Define a type for protocols")

/// ValuePropId is the type that identifies the different Main Storage Provider value propositions, to allow tiered solutions
pub type ValuePropId<T> = <T as crate::Config>::ValuePropId;

/// BucketId is the type that identifies the different buckets that a Main Storage Provider can have.
pub type BucketId<T> = <T as crate::Config>::HashId;
/// MaxBuckets is the maximum amount of buckets that a Main Storage Provider can have.
pub type MaxBuckets<T> = <T as crate::Config>::MaxBuckets;
/// Buckets is a vector of the buckets that a Main Storage Provider has.
pub type Buckets<T> = BoundedVec<Bucket<T>, MaxBuckets<T>>;
