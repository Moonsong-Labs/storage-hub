#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

mod types;
mod utils;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use super::types::*;
	use codec::HasCompact;
	use frame_support::{
		dispatch::{DispatchResult, DispatchResultWithPostInfo},
		pallet_prelude::*,
		sp_runtime::traits::{AtLeast32Bit, CheckEqual, MaybeDisplay, SimpleBitOps},
		traits::fungible::*,
		Blake2_128Concat,
	};
	use frame_system::pallet_prelude::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Type to access the Balances pallet (using the fungible trait from frame_support)
		type NativeBalance: Inspect<Self::AccountId>
			+ Mutate<Self::AccountId>
			+ hold::Inspect<Self::AccountId>
			// , Reason = Self::HoldReason> We will probably have to hold deposits
			+ hold::Mutate<Self::AccountId>
			+ freeze::Inspect<Self::AccountId>
			+ freeze::Mutate<Self::AccountId>;

		/// Data type for the measurement of storage size
		type StorageData: Parameter
			+ Member
			+ MaybeSerializeDeserialize
			+ Default
			+ MaybeDisplay
			+ AtLeast32Bit
			+ Copy
			+ MaxEncodedLen
			+ HasCompact;

		/// Data type of the Merkle Patricia roots that the runtime will store
		type MerklePatriciaRoot: Parameter
			+ Member
			+ MaybeSerializeDeserialize
			+ MaybeDisplay
			+ SimpleBitOps
			+ Ord
			+ Default
			+ Copy
			+ CheckEqual
			+ AsRef<[u8]>
			+ AsMut<[u8]>
			+ MaxEncodedLen;

		/// The minimum amount that an account has to deposit to become a storage provider.
		#[pallet::constant]
		type SpMinDeposit: Get<BalanceOf<Self>>;

		/// The amount that a BSP receives as allocation of storage capacity when it deposits SpMinDeposit.
		#[pallet::constant]
		type SpMinCapacity: Get<StorageData<Self>>;

		/// The slope of the collateral vs storage capacity curve. In other terms, how many tokens a Storage Provider should add as collateral to increase its storage capacity in one unit of StorageData.
		#[pallet::constant]
		type DepositPerData: Get<BalanceOf<Self>>;

		/// The maximum amount of BSPs that can exist.
		#[pallet::constant]
		type MaxBsps: Get<u32>;

		/// The maximum size of a multiaddress.
		#[pallet::constant]
		type MaxMultiAddressSize: Get<u32>;

		/// The maximum number of protocols the MSP can support (at least within the runtime).
		#[pallet::constant]
		type MaxProtocols: Get<u32>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	// Storage:

	/// The mapping from an account id to a main storage provider. Returns `None` if the account isnt a main storage provider.
	#[pallet::storage]
	pub type Msps<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, MainStorageProvider<T>>;

	/// The mapping from an account id to a backup storage provider. Returns `None` if the account isnt a backup storage provider.
	#[pallet::storage]
	pub type Bsps<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BackupStorageProvider<T>>;

	/// The total amount of storage capacity all BSPs have. Remember redundancy!
	#[pallet::storage]
	#[pallet::getter(fn total_bsps_capacity)] // TODO: remove this and add an explicit getter
	pub type TotalBspsCapacity<T: Config> = StorageValue<_, StorageData<T>>;

	/// A vector of account IDs that are BSPs (to draw from for proofs)
	#[pallet::storage]
	pub type BspsVec<T: Config> = StorageValue<_, BoundedVec<T::AccountId, T::MaxBsps>, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// A dispatchable function that allows users to sign up as a main storage provider.
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn msp_sign_up(
			origin: OriginFor<T>,
			data_stored: StorageData<T>,
			multiaddress: MultiAddress<T>,
			value_prop: ValueProposition<T>,
		) -> DispatchResultWithPostInfo {
			// todo!("Logic to sign up a MSP.");

			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/v3/runtime/origins
			let who = ensure_signed(origin)?;

			let msp_info = MainStorageProvider { data_stored, multiaddress, value_prop };
			// Update storage.
			Self::do_msp_sign_up(&who, msp_info)?;

			// Emit an event.
			Self::deposit_event(Event::SomethingStored(123, who));
			// Return a successful DispatchResultWithPostInfo
			Ok(().into())
		}

		/// A dispatchable function that allows users to sign up as a backup storage provider.
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn bsp_sign_up(
			origin: OriginFor<T>,
			data_stored: StorageData<T>,
			multiaddress: MultiAddress<T>,
		) -> DispatchResultWithPostInfo {
			// todo!("Logic to sign up a BSP.");

			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/v3/runtime/origins
			let who = ensure_signed(origin)?;

			let bsp_info = BackupStorageProvider { data_stored, multiaddress };
			// Update storage.
			Self::do_bsp_sign_up(&who, bsp_info)?;

			// Return a successful DispatchResultWithPostInfo
			Ok(().into())
		}

		/// A dispatchable function that allows users to sign off as a main storage provider.
		#[pallet::call_index(2)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn msp_sign_off(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			// todo!("Logic to sign off a MSP.");

			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/v3/runtime/origins
			let who = ensure_signed(origin)?;

			// Update storage.
			<Msps<T>>::remove(&who);

			// Return a successful DispatchResultWithPostInfo
			Ok(().into())
		}

		/// A dispatchable function that allows users to sign off as a backup storage provider.
		#[pallet::call_index(3)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn bsp_sign_off(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			// todo!("Logic to sign off a BSP.");

			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/v3/runtime/origins
			let who = ensure_signed(origin)?;

			// Update storage.
			<Bsps<T>>::remove(&who);

			// Return a successful DispatchResultWithPostInfo
			Ok(().into())
		}
	}
}

use crate::types::*;
impl<T: Config> Pallet<T> {
	/// A helper function to get the total capacity of a storage provider.
	pub fn get_total_capacity(who: &T::AccountId) -> StorageData<T> {
		if let Some(m) = Msps::<T>::get(who) {
			m.data_stored
		} else if let Some(b) = Bsps::<T>::get(who) {
			b.data_stored
		} else {
			StorageData::<T>::default()
		}
	}
}
