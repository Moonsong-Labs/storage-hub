use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use frame_support::traits::fungible::Inspect;
use frame_support::BoundedVec;
use scale_info::TypeInfo;

use crate::Config;

/// Structure that has the possible value propositions that a main storage provider can offer (and the runtime is aware of)
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebugNoBound, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct ValueProposition<T: Config> {
    pub data_limit: StorageData<T>,
    pub protocols: BoundedVec<Protocols<T>, MaxProtocols<T>>,
    // todo!("add relevant fields here")
}

/// Structure that represents a main storage provider. It holds the amount of data that the MSP is able to store,
/// the amount of data that it IS storing, its libp2p multiaddress, and its value proposition.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebugNoBound, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct MainStorageProvider<T: Config> {
    pub total_data: StorageData<T>,
    pub data_used: StorageData<T>,
    pub multiaddress: MultiAddress<T>,
    pub value_prop: ValueProposition<T>,
}

/// Structure that represents a backup storage provider. It holds the amount of data that the BSP is able to store,
/// the amount of data that it is storing, and its libp2p multiaddress.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebugNoBound, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct BackupStorageProvider<T: Config> {
    pub total_data: StorageData<T>,
    pub data_used: StorageData<T>,
    pub multiaddress: MultiAddress<T>,
    pub root: MerklePatriciaRoot<T>,
}

// Type aliases:

/// BalanceOf is the balance type of the runtime.
pub type BalanceOf<T> =
    <<T as Config>::NativeBalance as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

/// MaxMultiAddressSize is the maximum size of the libp2p multiaddress of a Storage Provider in bytes.
pub type MaxMultiAddressSize<T> = <T as crate::Config>::MaxMultiAddressSize;
/// MultiAddress is a byte array that represents the libp2p multiaddress of a Storage Provider.
/// Its maximum size is defined in the runtime configuration, as MaxMultiAddressSize.
pub type MultiAddress<T> = BoundedVec<u8, MaxMultiAddressSize<T>>;

/// MerklePatriciaRoot is the type of the root of a Merkle Patricia Trie, either the root of a BSP or a bucket from an MSP.
pub type MerklePatriciaRoot<T> = <T as crate::Config>::MerklePatriciaRoot;

/// StorageData is the type of the unit in which we measure data size. We define its required traits in the
/// pallet configuration so the runtime can use any type that implements them.
pub type StorageData<T> = <T as crate::Config>::StorageData;

/// Protocols is a vector of the protocols that (the runtime is aware of and) the Main Storage Provider supports.
/// Its maximum size is defined in the runtime configuration, as MaxProtocols.
pub type MaxProtocols<T> = <T as crate::Config>::MaxProtocols;
pub type Protocols<T> = BoundedVec<u8, MaxProtocols<T>>; // todo!("Define a type for protocols")

/// MaxBsps is the maximum amount of Backup Storage Providers that can exist. It is defined in the runtime configuration.
pub type MaxBsps<T> = <T as crate::Config>::MaxBsps;

// todo!("ask if we should also have a limit of MSPs")
