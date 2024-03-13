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

    // Events & Errors:

    /// The events that can be emitted by this pallet
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event documentation should end with an array that provides descriptive names for event
        /// parameters. [something, who]
        SomethingStored(u32, T::AccountId),
    }

    /// The errors that can be thrown by this pallet to inform users about what went wrong
    #[pallet::error]
    pub enum Error<T> {
        /// Error names should be descriptive.
        NoneValue,
        /// Errors should have helpful documentation associated with them.
        StorageOverflow,
    }

    /// The hooks that this pallet utilizes (TODO: Check this, we might not need any)
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    /// Dispatchables (extrinsics) exposed by this pallet
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// A dispatchable function that allows users to sign up as a main storage provider.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn msp_sign_up(
            origin: OriginFor<T>,
            total_data: StorageData<T>,
            multiaddress: MultiAddress<T>,
            value_prop: ValueProposition<T>,
        ) -> DispatchResultWithPostInfo {
            // todo!("Logic to sign up a MSP.");
            // This extrinsic should:
            // 1. Check that the extrinsic was signed and get the signer.
            // 2. Check that the signer is not already registered as either a MSP or BSP
            // 3. Check that the multiaddress is valid (size and any other relevant checks)
            // 3b. Any value proposition checks??? TODO: Ask
            // 4. Check that the data to be stored is greater than the minimum required by the runtime. TODO: Ask if this applies to MSPs
            // 5. Calculate how much deposit will the signer have to pay using the amount of data it wants to store
            // 6. Check that the signer has enough funds to pay the deposit
            // 7. Hold the deposit from the signer
            // 8. Update the MSP storage to add the signer as an MSP
            // 9. Emit an event confirming that the registration of the MSP has been successful

            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://docs.substrate.io/v3/runtime/origins
            let who = ensure_signed(origin)?;

            let msp_info = MainStorageProvider {
                total_data,
                data_used: StorageData::<T>::default(),
                multiaddress,
                value_prop,
            };
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
            total_data: StorageData<T>,
            multiaddress: MultiAddress<T>,
        ) -> DispatchResultWithPostInfo {
            // todo!("Logic to sign up a BSP.");
            // This extrinsic should:
            // 1. Check that the extrinsic was signed and get the signer.
            // 2. Check that, by adding this new BSP, we won't exceed the max amount of BSPs allowed
            // 3. Check that the signer is not already registered as either a MSP or BSP
            // 4. Check that the multiaddress is valid (size and any other relevant checks)
            // 5. Check that the data to be stored is greater than the minimum required by the runtime
            // 6. Calculate how much deposit will the signer have to pay using the amount of data it wants to store
            // 7. Check that the signer has enough funds to pay the deposit
            // 8. Hold the deposit from the signer
            // 9. Update the BSP storage to add the signer as an BSP
            // 10. Update the total capacity of all BSPs, adding the new capacity (factoring by redundancy)
            // 11. Add this BSP to the vector of BSPs to draw from for proofs
            // 12. Emit an event confirming that the registration of the BSP has been successful

            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://docs.substrate.io/v3/runtime/origins
            let who = ensure_signed(origin)?;

            let bsp_info = BackupStorageProvider {
                total_data,
                data_used: StorageData::<T>::default(),
                multiaddress,
            };
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
            // This extrinsic should:
            // 1. Check that the extrinsic was signed and get the signer.
            // 2. Check that the signer is registered as a MSP
            // 3. Check that the MSP has no storage assigned to it
            // 4. Update the MSPs storage, removing the signer as an MSP
            // 5. Return the deposit to the signer
            // 6. Emit an event confirming that the sign off of the MSP has been successful

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
            // This extrinsic should:
            // 1. Check that the extrinsic was signed and get the signer.
            // 2. Check that the signer is registered as a BSP
            // 3. Check that the BSP has no storage assigned to it (TODO: not entirely sure how this would work, ask)
            // 4. Update the BSPs storage, removing the signer as an BSP
            // 5. Update the total capacity of all BSPs, removing the capacity of the signer (factoring by redundancy)
            // 6. Remove this BSP from the vector of BSPs to draw from for proofs
            // 7. Return the deposit to the signer
            // 8. Emit an event confirming that the sign off of the BSP has been successful

            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://docs.substrate.io/v3/runtime/origins
            let who = ensure_signed(origin)?;

            // Update storage.
            <Bsps<T>>::remove(&who);

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// A dispatchable function that allows users to change their amount of stored data
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn change_total_data(
            origin: OriginFor<T>,
            new_total_data: StorageData<T>,
        ) -> DispatchResultWithPostInfo {
            // TODO: design a way (with timelock probably) to allow a SP to change its stake
            // This extrinsic should:
            // 1. Check that the extrinsic was signed and get the signer.
            // 2. Check that the signer is registered as a SP
            // 3. Check that enough time has passed since the last time the SP changed its stake
            // 4. Check that the new total data is greater than the minimum required by the runtime
            // 5. Check that the new total data is greater than the data used by this SP
            // 6. Check to see if the new total data is greater or less than the current total data
            // 	a. If the new total data is greater than the current total data:
            //		i. Calculate how much deposit will the signer have to pay using the amount of data it wants to store
            // 		ii. Check that the signer has enough funds to pay this extra deposit
            // 		iii. Hold the extra deposit from the signer
            // 	b. If the new total data is less than the current total data, return the extra deposit to the signer
            // 7. Update the SPs storage to change the total data
            // 8. Emit an event confirming that the change of the total data has been successful

            Ok(().into())
        }
    }
}

/// Helper functions (getters, setters, etc.) for this pallet
use crate::types::*;
impl<T: Config> Pallet<T> {
    /// A helper function to get the total capacity of a storage provider.
    pub fn get_total_capacity(who: &T::AccountId) -> StorageData<T> {
        if let Some(m) = Msps::<T>::get(who) {
            m.total_data
        } else if let Some(b) = Bsps::<T>::get(who) {
            b.total_data
        } else {
            StorageData::<T>::default()
        }
    }
}
