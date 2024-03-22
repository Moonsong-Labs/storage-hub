#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use types::{BackupStorageProviderId, MainStorageProviderId};

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
    use codec::{FullCodec, HasCompact};
    // TODO: use frame_support::traits::Randomness;
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::*,
        sp_runtime::traits::{AtLeast32BitUnsigned, CheckEqual, Hash, MaybeDisplay, SimpleBitOps},
        traits::fungible::*,
        Blake2_128Concat,
    };
    use frame_system::pallet_prelude::*;
    use scale_info::prelude::fmt::Debug;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// TODO: Type to access randomness to salt AccountIds and get the corresponding HashId
        //type ProvidersRandomness: Randomness<Self::HashId, BlockNumberFor<Self>>;

        /// Type to access the Balances pallet (using the fungible trait from frame_support)
        type NativeBalance: Inspect<Self::AccountId>
            + Mutate<Self::AccountId>
            + hold::Inspect<Self::AccountId, Reason = Self::RuntimeHoldReason>
            // , Reason = Self::HoldReason> We will probably have to hold deposits
            + hold::Mutate<Self::AccountId, Reason = Self::RuntimeHoldReason>
            + freeze::Inspect<Self::AccountId>
            + freeze::Mutate<Self::AccountId>;

        /// The overarching hold reason
        type RuntimeHoldReason: From<HoldReason>;

        /// The type of ID that uniquely identifies a Storage Provider (MSPs/BSPs) from an AccountId
        /// It is also used to identify a Bucket of data inside a MSP
        type HashId: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Debug
            + MaybeDisplay
            + SimpleBitOps
            + Ord
            + Default
            + Copy
            + CheckEqual
            + std::hash::Hash
            + AsRef<[u8]>
            + AsMut<[u8]>
            + MaxEncodedLen;

        /// The hashing system (algorithm) being used in the runtime (e.g. Blake2).
        type Hashing: Hash<Output = Self::HashId> + TypeInfo;

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
            + Into<BalanceOf<Self>>;

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

        /// The type of the identifier of the value proposition of a MSP (probably a hash of that value proposition)
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
        type MaxBsps: Get<Self::SpCount>;

        /// The maximum amount of MSPs that can exist.
        #[pallet::constant]
        type MaxMsps: Get<Self::SpCount>;

        // TODO: Change these next constants to a more generic type

        /// The maximum size of a multiaddress.
        #[pallet::constant]
        type MaxMultiAddressSize: Get<u32>;

        /// The maximum amount of multiaddresses that a Storage Provider can have.
        #[pallet::constant]
        type MaxMultiAddressAmount: Get<u32>;

        /// The maximum number of protocols the MSP can support (at least within the runtime).
        #[pallet::constant]
        type MaxProtocols: Get<u32>;

        /// The maximum amount of Buckets that a MSP can have.
        #[pallet::constant]
        type MaxBuckets: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // Storage:

    /// The mapping from an AccountId to a MainStorageProviderId
    ///
    /// This is used to get a Main Storage Provider's unique identifier to access its relevant data
    #[pallet::storage]
    pub type AccountIdToMainStorageProviderId<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, MainStorageProviderId<T>>;

    /// The mapping from a MainStorageProviderId to a MainStorageProvider
    ///
    /// This is used to get a Main Storage Provider's relevant data.
    /// It returns `None` if the Main Storage Provider ID does not correspond to any registered Main Storage Provider.
    #[pallet::storage]
    pub type MainStorageProviders<T: Config> =
        StorageMap<_, Blake2_128Concat, MainStorageProviderId<T>, MainStorageProvider<T>>;

    /// The mapping from a BucketId to that bucket's metadata
    ///
    /// This is used to get a bucket's relevant data, such as root, user ID, and MSP ID.
    /// It returns `None` if the Bucket ID does not correspond to any registered bucket.
    #[pallet::storage]
    pub type Buckets<T: Config> = StorageMap<_, Blake2_128Concat, BucketId<T>, Bucket<T>>;

    /// The mapping from an AccountId to a BackupStorageProviderId
    ///
    /// This is used to get a Backup Storage Provider's unique identifier to access its relevant data
    #[pallet::storage]
    pub type AccountIdToBackupStorageProviderId<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BackupStorageProviderId<T>>;

    /// The mapping from a BackupStorageProviderId to a BackupStorageProvider
    ///
    /// This is used to get a Backup Storage Provider's relevant data.
    /// It returns `None` if the Backup Storage Provider ID does not correspond to any registered Backup Storage Provider.
    #[pallet::storage]
    pub type BackupStorageProviders<T: Config> =
        StorageMap<_, Blake2_128Concat, BackupStorageProviderId<T>, BackupStorageProvider<T>>;

    /// The amount of Main Storage Providers that are currently registered in the runtime.
    #[pallet::storage]
    pub type MspCount<T: Config> = StorageValue<_, T::SpCount, ValueQuery>;

    /// The amount of Backup Storage Providers that are currently registered in the runtime.
    #[pallet::storage]
    pub type BspCount<T: Config> = StorageValue<_, T::SpCount, ValueQuery>;

    /// The total amount of storage capacity all BSPs have. Remember redundancy!
    #[pallet::storage]
    #[pallet::getter(fn total_bsps_capacity)] // TODO: remove this and add an explicit getter
    pub type TotalBspsCapacity<T: Config> = StorageValue<_, StorageData<T>, ValueQuery>;

    // Events & Errors:

    /// The events that can be emitted by this pallet
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event emitted when a Main Storage Provider has signed up successfully. Provides information about
        /// that MSP's account id, the total data it can store according to its stake, its multiaddress, and its value proposition.
        MspSignUpSuccess {
            who: T::AccountId,
            multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>>,
            capacity: StorageData<T>,
            value_prop: ValueProposition<T>,
        },

        /// Event emitted when a Backup Storage Provider has signed up successfully. Provides information about
        /// that BSP's account id, the total data it can store according to its stake, and its multiaddress.
        BspSignUpSuccess {
            who: T::AccountId,
            multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>>,
            capacity: StorageData<T>,
        },

        /// Event emitted when a Main Storage Provider has signed off successfully. Provides information about
        /// that MSP's account id.
        MspSignOffSuccess { who: T::AccountId },

        /// Event emitted when a Backup Storage Provider has signed off successfully. Provides information about
        /// that BSP's account id.
        BspSignOffSuccess { who: T::AccountId },

        /// Event emitted when a SP has changed is total data (stake) successfully. Provides information about
        /// that SP's account id, its old total data that could store, and the new total data.
        TotalDataChanged {
            who: T::AccountId,
            old_capacity: StorageData<T>,
            new_capacity: StorageData<T>,
        },
    }

    /// The errors that can be thrown by this pallet to inform users about what went wrong
    #[pallet::error]
    pub enum Error<T> {
        /// Error thrown when a user tries to sign up as a SP but is already registered as a MSP or BSP.
        AlreadyRegistered,
        /// Error thrown when a user tries to sign up or change its stake to store less storage than the minimum required by the runtime.
        StorageTooLow,
        /// Error thrown when a user does not have enough balance to pay the deposit that it would incur by signing up as a SP or changing its total data (stake).
        NotEnoughBalance,
        /// Error thrown when a user tries to sign up as a BSP but the maximum amount of BSPs has been reached.
        MaxBspsReached,
        /// Error thrown when a user tries to sign up as a MSP but the maximum amount of MSPs has been reached.
        MaxMspsReached,
        /// Error thrown when a user tries to sign off as a SP but is not registered as a MSP or BSP.
        NotRegistered,
        /// Error thrown when a user tries to sign off as a SP but still has used storage.
        StorageStillInUse,
        /// Error thrown when a SP tries to change its total data (stake) but it has not been enough time since the last time it changed it.
        NotEnoughTimePassed,
        /// Error thrown when trying to get a root from a MSP without passing a User ID
        NoUserId,
        /// Error thrown when trying to get a root from a MSP without passing a Bucket ID
        NoBucketId,
    }

    /// This enum holds the HoldReasons for this pallet, allowing the runtime to identify each held balance with different reasons separately
    ///
    /// This allows us to hold tokens and be able to identify in the future that those held tokens were
    /// held because of this pallet
    #[pallet::composite_enum]
    pub enum HoldReason {
        /// Deposit that a Storage Provider has to pay to be registered as such
        StorageProviderDeposit,
        // TODO: Only for testing, remove this for production
        AnotherUnrelatedHold,
    }

    /// The hooks that this pallet utilizes (TODO: Check this, we might not need any)
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    /// Dispatchables (extrinsics) exposed by this pallet
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// A dispatchable function that allows users to sign up as a Main Storage Provider.
        ///
        /// This extrinsic will:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that, by registering this new MSP, we would not go over the MaxMsps limit
        /// 3. Check that the signer is not already registered as either a MSP or BSP
        /// 4. Check that the multiaddress is valid (size and any other relevant checks)
        /// 4b. Any value proposition checks??? todo!("ask this")
        /// 5. Check that the data to be stored is greater than the minimum required by the runtime. todo!("Ask if this applies to MSPs")
        /// 6. Calculate how much deposit will the signer have to pay using the amount of data it wants to store
        /// 7. Check that the signer has enough funds to pay the deposit
        /// 8. Hold the deposit from the signer
        /// 9. Update the MSP storage to add the signer as an MSP
        /// 10. Increment the storage that holds total amount of MSPs currently in the system
        /// 11. Emit an event confirming that the registration of the MSP has been successful
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn msp_sign_up(
            origin: OriginFor<T>,
            capacity: StorageData<T>,
            multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>>,
            value_prop: ValueProposition<T>,
        ) -> DispatchResultWithPostInfo {
            // TODO: Logic to sign up an MSP

            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://docs.substrate.io/v3/runtime/origins
            let who = ensure_signed(origin)?;

            let msp_info = MainStorageProvider {
                buckets: BoundedVec::default(),
                capacity,
                data_used: StorageData::<T>::default(),
                multiaddresses: multiaddresses.clone(),
                value_prop: value_prop.clone(),
            };
            // Update storage.
            Self::do_msp_sign_up(&who, &msp_info)?;

            // Emit an event.
            Self::deposit_event(Event::<T>::MspSignUpSuccess {
                who,
                multiaddresses,
                capacity,
                value_prop,
            });
            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// A dispatchable function that allows users to sign up as a Backup Storage Provider.
        ///
        /// This extrinsic will:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that, by adding this new BSP, we won't exceed the max amount of BSPs allowed
        /// 3. Check that the signer is not already registered as either a MSP or BSP
        /// 4. Check that the multiaddress is valid (size and any other relevant checks)
        /// 5. Check that the data to be stored is greater than the minimum required by the runtime
        /// 6. Calculate how much deposit will the signer have to pay using the amount of data it wants to store
        /// 7. Check that the signer has enough funds to pay the deposit
        /// 8. Hold the deposit from the signer
        /// 9. Update the BSP storage to add the signer as an BSP
        /// 10. Update the total capacity of all BSPs, adding the new capacity (redundancy is factored in the capacity used)
        /// 11. Add this BSP to the vector of BSPs to draw from for proofs
        /// 12. Increment the storage that holds total amount of BSPs currently in the system
        /// 13. Emit an event confirming that the registration of the BSP has been successful
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn bsp_sign_up(
            origin: OriginFor<T>,
            capacity: StorageData<T>,
            multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>>,
        ) -> DispatchResultWithPostInfo {
            // TODO: Logic to sign up a BSP

            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://docs.substrate.io/v3/runtime/origins
            let who = ensure_signed(origin)?;

            let bsp_info = BackupStorageProvider {
                capacity,
                data_used: StorageData::<T>::default(),
                multiaddresses: multiaddresses.clone(),
                root: MerklePatriciaRoot::<T>::default(),
            };

            // Update storage.
            Self::do_bsp_sign_up(&who, bsp_info)?;

            // Emit an event.
            Self::deposit_event(Event::<T>::BspSignUpSuccess {
                who,
                multiaddresses,
                capacity,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// A dispatchable function that allows users to sign off as a Main Storage Provider.
        ///
        ///  This extrinsic should:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the signer is registered as a MSP
        /// 3. Check that the MSP has no storage assigned to it (no buckets or data used by it)
        /// 4. Update the MSPs storage, removing the signer as an MSP
        /// 5. Return the deposit to the signer
        /// 6. Decrement the storage that holds total amount of SPs currently in the system
        /// 7. Emit an event confirming that the sign off of the MSP has been successful
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn msp_sign_off(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            // TODO: Logic to sign off a MSP

            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://docs.substrate.io/v3/runtime/origins
            let who = ensure_signed(origin)?;

            let msp_id = AccountIdToMainStorageProviderId::<T>::get(&who)
                .ok_or(Error::<T>::NotRegistered)?;

            // Update storage.
            <MainStorageProviders<T>>::remove(&msp_id);

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// A dispatchable function that allows users to sign off as a Backup Storage Provider.
        ///
        /// This extrinsic will:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the signer is registered as a BSP
        /// 3. Check that the BSP has no storage assigned to it
        /// 4. Update the BSPs storage, removing the signer as an BSP
        /// 5. Update the total capacity of all BSPs, removing the capacity of the signer (factoring by redundancy)
        /// 6. Remove this BSP from the vector of BSPs to draw from for proofs
        /// 7. Return the deposit to the signer
        /// 8. Decrement the storage that holds total amount of SPs currently in the system
        /// 9. Emit an event confirming that the sign off of the BSP has been successful
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn bsp_sign_off(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            // TODO: Logic to sign off a BSP

            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://docs.substrate.io/v3/runtime/origins
            let who = ensure_signed(origin)?;

            let bsp_id = AccountIdToBackupStorageProviderId::<T>::get(&who)
                .ok_or(Error::<T>::NotRegistered)?;

            // Update storage.
            <BackupStorageProviders<T>>::remove(&bsp_id);

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// A dispatchable function that allows users to change their amount of stored data
        ///
        /// This extrinsic will:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the signer is registered as a SP
        /// 3. Check that enough time has passed since the last time the SP changed its stake
        /// 4. Check that the new total data is greater than the minimum required by the runtime
        /// 5. Check that the new total data is greater than the data used by this SP
        /// 6. Check to see if the new total data is greater or less than the current total data
        /// 	a. If the new total data is greater than the current total data:
        ///		i. Calculate how much deposit will the signer have to pay using the amount of data it wants to store
        /// 		ii. Check that the signer has enough funds to pay this extra deposit
        /// 		iii. Hold the extra deposit from the signer
        /// 	b. If the new total data is less than the current total data, return the extra deposit to the signer
        /// 7. Update the SPs storage to change the total data
        /// 8. Emit an event confirming that the change of the total data has been successful
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn change_capacity(
            _origin: OriginFor<T>,
            _new_capacity: StorageData<T>,
        ) -> DispatchResultWithPostInfo {
            // TODO: design a way (with timelock probably) to allow a SP to change its stake

            Ok(().into())
        }

        /// A dispatchable function only callable by an MSP that allows it to add a value proposition to its service
        ///
        /// This extrinsic will:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the signer is registered as a MSP
        /// 3. Check that the MSP has not reached the maximum amount of value propositions
        /// 4. Check that the value proposition is valid (size and any other relevant checks)
        /// 5. Update the MSPs storage to add the value proposition (with its identifier)
        /// 6. Emit an event confirming that the addition of the value proposition has been successful
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn add_value_prop(
            _origin: OriginFor<T>,
            _new_value_prop: ValueProposition<T>,
        ) -> DispatchResultWithPostInfo {
            // TODO: implement this

            Ok(().into())
        }
    }
}

use crate::types::{
    BackupStorageProvider, BalanceOf, BucketId, HashId, MerklePatriciaRoot, StorageData,
};
/// Helper functions (getters, setters, etc.) for this pallet
impl<T: Config> Pallet<T> {
    /// A helper function to get the total capacity of a storage provider.
    pub fn get_total_capacity(who: &T::AccountId) -> Result<StorageData<T>, Error<T>> {
        if let Some(m_id) = AccountIdToMainStorageProviderId::<T>::get(who) {
            let msp = MainStorageProviders::<T>::get(m_id).ok_or(Error::<T>::NotRegistered)?;
            Ok(msp.capacity)
        } else if let Some(b_id) = AccountIdToBackupStorageProviderId::<T>::get(who) {
            let bsp = BackupStorageProviders::<T>::get(b_id).ok_or(Error::<T>::NotRegistered)?;
            Ok(bsp.capacity)
        } else {
            Err(Error::<T>::NotRegistered)
        }
    }
}

// Trait definitions:

use frame_support::pallet_prelude::DispatchResult;

/// Interface to allow the File System pallet to modify the data used by the Storage Providers pallet.
pub trait StorageProvidersInterface<T: Config> {
    /// Change the used data of a Storage Provider (generic, MSP or BSP).
    fn change_data_used(who: &T::AccountId, data_change: T::StorageData) -> DispatchResult;

    /// Add a new Bucket as a Provider
    fn add_bucket(
        msp_id: MainStorageProviderId<T>,
        user_id: T::AccountId,
        bucket_id: BucketId<T>,
        bucket_root: MerklePatriciaRoot<T>,
    ) -> DispatchResult;

    /// Change the root of a bucket
    fn change_root_bucket(
        bucket_id: BucketId<T>,
        new_root: MerklePatriciaRoot<T>,
    ) -> DispatchResult;

    /// Change the root of a BSP
    fn change_root_bsp(
        bsp_id: BackupStorageProviderId<T>,
        new_root: MerklePatriciaRoot<T>,
    ) -> DispatchResult;

    /// Remove a root from a bucket of a MSP, removing the whole bucket from storage
    fn remove_root_bucket(bucket_id: BucketId<T>) -> DispatchResult;

    /// Remove a root from a BSP. It will remove the whole BSP from storage, so it should only be called when the BSP is being removed.
    /// todo!("If the only way to remove a BSP is by this pallet (bsp_sign_off), then is this function actually needed?")
    fn remove_root_bsp(who: &T::AccountId) -> DispatchResult;
}
