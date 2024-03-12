use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::RuntimeDebugNoBound;
use frame_support::traits::fungible::Inspect;
use frame_support::BoundedVec;
use scale_info::TypeInfo;

use crate::Config;

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebugNoBound, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct ValueProposition<T: Config> {
	pub data_limit: StorageData<T>,
	pub protocols: BoundedVec<Protocols<T>, MaxProtocols<T>>,
	// TODO: Add relevant fields here
}

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct MainStorageProvider<T: Config> {
	pub data_stored: StorageData<T>,
	pub multiaddress: MultiAddress<T>,
	pub value_prop: ValueProposition<T>,
}

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct BackupStorageProvider<T: Config> {
	pub data_stored: StorageData<T>,
	pub multiaddress: MultiAddress<T>,
	// pub root: MerklePatriciaRoot<T>, // The root of the Merkle Patricia Forest of this BSP. HANDLED IN FILE SYSTEM PALLET
}

// Type aliases:

// BalanceOf is the balance type of the runtime.
pub type BalanceOf<T> =
	<<T as Config>::NativeBalance as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

// MultiAddress is a byte array that represents the libp2p multiaddress of a storage provider.
// Its maximum size is defined in the runtime configuration, as MaxMultiAddressSize.
pub type MaxMultiAddressSize<T> = <T as crate::Config>::MaxMultiAddressSize;
pub type MultiAddress<T> = BoundedVec<u8, MaxMultiAddressSize<T>>;

// StorageData is the type of the unit in which we measure data size. We define its required traits in the
// pallet configuration so the runtime can use any type that implements them.
pub type StorageData<T> = <T as crate::Config>::StorageData;

// MerklePatriciaRoot is the type of the root of the Merkle Patricia Forest of a storage provider or a bucket.
// We define its required traits in the pallet configuration so the runtime can use any type that implements them.
pub type MerklePatriciaRoot<T> = <T as crate::Config>::MerklePatriciaRoot;

// Protocols is a vector of the protocols that (the runtime is aware of and) the main storage provider supports.
// Its maximum size is defined in the runtime configuration, as MaxProtocols.
pub type MaxProtocols<T> = <T as crate::Config>::MaxProtocols;
pub type Protocols<T> = BoundedVec<u8, MaxProtocols<T>>; // TODO: define a type for protocols

// MaxBsps is the maximum amount of backup storage providers that can exist. It is defined in the runtime configuration.
pub type MaxBsps<T> = <T as crate::Config>::MaxBsps;
