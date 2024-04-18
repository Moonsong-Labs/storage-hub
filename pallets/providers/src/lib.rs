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
    use frame_support::traits::Randomness;
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::*,
        sp_runtime::traits::{
            AtLeast32BitUnsigned, CheckEqual, MaybeDisplay, Saturating, SimpleBitOps,
        },
        traits::fungible::*,
        Blake2_128Concat,
    };
    use frame_system::pallet_prelude::{BlockNumberFor, *};
    use scale_info::prelude::fmt::Debug;
    use storage_hub_traits::SubscribeProvidersInterface;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Type to access randomness to salt AccountIds and get the corresponding HashId
        type ProvidersRandomness: Randomness<HashId<Self>, BlockNumberFor<Self>>;

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

        /// Data type for the measurement of storage size
        type StorageData: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Default
            + MaybeDisplay
            + AtLeast32BitUnsigned
            + Saturating
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

        /// Subscribers to important updates
        type Subscribers: SubscribeProvidersInterface;

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

        /// The maximum amount of blocks after which a sign up request expires so the randomness cannot be chosen
        #[pallet::constant]
        type MaxBlocksForRandomness: Get<BlockNumberFor<Self>>;

        /// The minimum amount of blocks between capacity changes for a SP
        #[pallet::constant]
        type MinBlocksBetweenCapacityChanges: Get<BlockNumberFor<Self>>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // Storage:

    /// The mapping from an AccountId that requested to sign up to a tuple of the metadata with type of the request, and the block
    /// number when the request was made.
    ///
    /// This is used for the two-step process of registering: when a user requests to register as a SP (either MSP or BSP),
    /// that request with the metadata and the deposit held is stored here. When the user confirms the sign up, the
    /// request is removed from this storage and the user is registered as a SP.
    #[pallet::storage]
    pub type SignUpRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, (StorageProvider<T>, BlockNumberFor<T>)>;

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
    pub type TotalBspsCapacity<T: Config> = StorageValue<_, StorageData<T>, ValueQuery>;

    // Events & Errors:

    /// The events that can be emitted by this pallet
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event emitted when a Main Storage Provider has requested to sign up successfully. Provides information about
        /// that MSP's account id, its multiaddresses, the total data it can store according to its stake, and its value proposition.
        MspRequestSignUpSuccess {
            who: T::AccountId,
            multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>>,
            capacity: StorageData<T>,
            value_prop: ValueProposition<T>,
        },

        /// Event emitted when a Main Storage Provider has confirmed its sign up successfully. Provides information about
        /// that MSP's account id, the total data it can store according to its stake, its multiaddress, and its value proposition.
        MspSignUpSuccess {
            who: T::AccountId,
            multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>>,
            capacity: StorageData<T>,
            value_prop: ValueProposition<T>,
        },

        /// Event emitted when a Backup Storage Provider has requested to sign up successfully. Provides information about
        /// that BSP's account id, its multiaddresses, and the total data it can store according to its stake.
        BspRequestSignUpSuccess {
            who: T::AccountId,
            multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>>,
            capacity: StorageData<T>,
        },

        /// Event emitted when a Backup Storage Provider has confirmed its sign up successfully. Provides information about
        /// that BSP's account id, the total data it can store according to its stake, and its multiaddress.
        BspSignUpSuccess {
            who: T::AccountId,
            multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>>,
            capacity: StorageData<T>,
        },

        /// Event emitted when a sign up request has been canceled successfully. Provides information about
        /// the account id of the user that canceled the request.
        SignUpRequestCanceled { who: T::AccountId },

        /// Event emitted when a Main Storage Provider has signed off successfully. Provides information about
        /// that MSP's account id.
        MspSignOffSuccess { who: T::AccountId },

        /// Event emitted when a Backup Storage Provider has signed off successfully. Provides information about
        /// that BSP's account id.
        BspSignOffSuccess { who: T::AccountId },

        /// Event emitted when a SP has changed its capacity successfully. Provides information about
        /// that SP's account id, its old total data that could store, and the new total data.
        CapacityChanged {
            who: T::AccountId,
            old_capacity: StorageData<T>,
            new_capacity: StorageData<T>,
            next_block_when_change_allowed: BlockNumberFor<T>,
        },
    }

    /// The errors that can be thrown by this pallet to inform users about what went wrong
    #[pallet::error]
    pub enum Error<T> {
        // Sign up errors:
        /// Error thrown when a user tries to sign up as a SP but is already registered as a MSP or BSP.
        AlreadyRegistered,
        /// Error thrown when a user tries to sign up as a BSP but the maximum amount of BSPs has been reached.
        MaxBspsReached,
        /// Error thrown when a user tries to sign up as a MSP but the maximum amount of MSPs has been reached.
        MaxMspsReached,
        /// Error thrown when a user tries to confirm a sign up that was not requested previously.
        SignUpNotRequested,
        /// Error thrown when a user tries to request to sign up when it already has a sign up request pending.
        SignUpRequestPending,
        /// Error thrown when a user tries to sign up without any multiaddress.
        NoMultiAddress,
        /// Error thrown when a user tries to sign up as a SP but any of the provided multiaddresses is invalid.
        InvalidMultiAddress,
        /// Error thrown when a user tries to sign up or change its capacity to store less storage than the minimum required by the runtime.
        StorageTooLow,

        // Deposit errors:
        /// Error thrown when a user does not have enough balance to pay the deposit that it would incur by signing up as a SP or changing its capacity.
        NotEnoughBalance,
        /// Error thrown when the runtime cannot hold the required deposit from the account to register it as a SP or change its capacity.
        CannotHoldDeposit,

        // Sign off errors:
        /// Error thrown when a user tries to sign off as a SP but still has used storage.
        StorageStillInUse,

        // Randomness errors:
        /// Error thrown when a user tries to confirm a sign up but the randomness is too fresh to be used yet.
        RandomnessNotValidYet,
        /// Error thrown when a user tries to confirm a sign up but too much time has passed since the request.
        SignUpRequestExpired,

        // Capacity change errors:
        /// Error thrown when a user tries to change its capacity to less than its used storage.
        NewCapacityLessThanUsedStorage,
        /// Error thrown when a user tries to change its capacity to the same value it already has.
        NewCapacityEqualsCurrentCapacity,
        /// Error thrown when a user tries to change its capacity to zero (there are specific extrinsics to sign off as a SP).
        NewCapacityCantBeZero,
        /// Error thrown when a SP tries to change its capacity but it has not been enough time since the last time it changed it.
        NotEnoughTimePassed,

        // General errors:
        /// Error thrown when a user tries to interact as a SP but is not registered as a MSP or BSP.
        NotRegistered,
        /// Error thrown when trying to get a root from a MSP without passing a User ID.
        NoUserId,
        /// Error thrown when trying to get a root from a MSP without passing a Bucket ID.
        NoBucketId,
        /// Error thrown when a user has a SP ID assigned to it but the SP data does not exist in storage (Inconsistency error).
        SpRegisteredButDataNotFound,
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
        /// A dispatchable function that allows users to request to sign up as a Main Storage Provider.
        ///
        /// This extrinsic will:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that, by registering this new MSP, we would not go over the MaxMsps limit
        /// 3. Check that the signer is not already registered as either a MSP or BSP
        /// 4. Check that the multiaddress is valid
        /// 5. Check that the data to be stored is greater than the minimum required by the runtime.
        /// 6. Calculate how much deposit will the signer have to pay using the amount of data it wants to store
        /// 7. Check that the signer has enough funds to pay the deposit
        /// 8. Hold the deposit from the signer
        /// 9. Update the Sign Up Requests storage to add the signer as requesting to sign up as a MSP
        /// 10. Emit an event confirming that the sign up request as MSP has been successful
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn request_msp_sign_up(
            origin: OriginFor<T>,
            capacity: StorageData<T>,
            multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>>,
            value_prop: ValueProposition<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Set up a structure with the information of the new MSP
            let msp_info = MainStorageProvider {
                buckets: BoundedVec::default(),
                capacity,
                data_used: StorageData::<T>::default(),
                multiaddresses: multiaddresses.clone(),
                value_prop: value_prop.clone(),
                last_capacity_change: frame_system::Pallet::<T>::block_number(),
            };

            // Sign up the new MSP (if possible), updating storage
            Self::do_request_msp_sign_up(&who, &msp_info)?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::MspRequestSignUpSuccess {
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
        /// 4. Check that the multiaddress is valid
        /// 5. Check that the data to be stored is greater than the minimum required by the runtime
        /// 6. Calculate how much deposit will the signer have to pay using the amount of data it wants to store
        /// 7. Check that the signer has enough funds to pay the deposit
        /// 8. Hold the deposit from the signer
        /// 9. Update the Sign Up Requests storage to add the signer as requesting to sign up as a BSP
        /// 10. Emit an event confirming that the sign up of the BSP has been successful
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn request_bsp_sign_up(
            origin: OriginFor<T>,
            capacity: StorageData<T>,
            multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Set up a structure with the information of the new BSP
            let bsp_info = BackupStorageProvider {
                capacity,
                data_used: StorageData::<T>::default(),
                multiaddresses: multiaddresses.clone(),
                root: MerklePatriciaRoot::<T>::default(),
                last_capacity_change: frame_system::Pallet::<T>::block_number(),
            };

            // Sign up the new BSP (if possible), updating storage
            Self::do_request_bsp_sign_up(&who, bsp_info)?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::BspRequestSignUpSuccess {
                who,
                multiaddresses,
                capacity,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// A dispatchable function that allows users to confirm their sign up as a Storage Provider, either MSP or BSP.
        ///
        /// This extrinsic will:
        /// 1. Check that the extrinsic was signed
        /// 2. Check that the account received has requested to register as a SP
        /// 3. Check that by registering this SP we would not go over the MaxMsps or MaxBsps limit
        /// 4. Check that the current randomness is sufficiently fresh to be used as a salt for that request
        /// 5. Check that the request has not expired
        /// 6. Register the signer as a MSP or BSP with the data provided in the request
        /// 7. Emit an event confirming that the sign up of the SP has been confirmed
        ///
        /// Notes:
        /// - This extrinsic could be called by the user itself or by a third party
        /// - The deposit that the user has to pay to register as a SP is held when the user requests to register as a SP
        /// - If this extrinsic is successful, it will be free for the caller, to incentive state debloating
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn confirm_sign_up(
            origin: OriginFor<T>,
            provider_account: Option<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage and emit event
            // We emit the event in the interior logic to not have to check again which type of sign up it is outside of it
            match provider_account {
                Some(provider_account) => Self::do_confirm_sign_up(&provider_account)?,
                None => Self::do_confirm_sign_up(&who)?,
            }

            // Return a successful DispatchResultWithPostInfo. If the extrinsic executed correctly, it will be free for the caller
            Ok(Pays::No.into())
        }

        /// A dispatchable function that allows a user with a pending Sign Up Request to cancel it, getting the deposit back.
        ///
        /// This extrinsic will:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the signer has requested to sign up as a SP
        /// 3. Delete the request from the Sign Up Requests storage
        /// 4. Return the deposit to the signer
        /// 5. Emit an event confirming that the cancellation of the sign up request has been successful
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn cancel_sign_up(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            Self::do_cancel_sign_up(&who)?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::SignUpRequestCanceled { who });

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
        /// 6. Decrement the storage that holds total amount of MSPs currently in the system
        /// 7. Emit an event confirming that the sign off of the MSP has been successful
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn msp_sign_off(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            Self::do_msp_sign_off(&who)?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::MspSignOffSuccess { who });

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
        /// 5. Update the total capacity of all BSPs, removing the capacity of the signer
        /// 6. Return the deposit to the signer
        /// 7. Decrement the storage that holds total amount of BSPs currently in the system
        /// 8. Emit an event confirming that the sign off of the BSP has been successful
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn bsp_sign_off(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            Self::do_bsp_sign_off(&who)?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::BspSignOffSuccess { who });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// A dispatchable function that allows users to change their amount of stored data
        ///
        /// This extrinsic will:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the signer is registered as a SP
        /// 3. Check that enough time has passed since the last time the SP changed its capacity
        /// 4. Check that the new capacity is greater than the minimum required by the runtime
        /// 5. Check that the new capacity is greater than the data used by this SP
        /// 6. Calculate the new deposit needed for this new capacity
        /// 7. Check to see if the new deposit needed is greater or less than the current deposit
        /// 	a. If the new deposit is greater than the current deposit:
        /// 		i. Check that the signer has enough funds to pay this extra deposit
        /// 		ii. Hold the extra deposit from the signer
        /// 	b. If the new deposit is less than the current deposit, return the held difference to the signer
        /// 7. Update the SPs storage to change the total data
        /// 8. If user is a BSP, update the total capacity of the network (sum of all capacities of BSPs)
        /// 8. Emit an event confirming that the change of the capacity has been successful
        #[pallet::call_index(6)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn change_capacity(
            origin: OriginFor<T>,
            new_capacity: StorageData<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            let old_capacity = Self::do_change_capacity(&who, new_capacity)?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::CapacityChanged {
                who,
                old_capacity,
                new_capacity,
                next_block_when_change_allowed: frame_system::Pallet::<T>::block_number()
                    + T::MinBlocksBetweenCapacityChanges::get(),
            });

            // Return a successful DispatchResultWithPostInfo
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
        #[pallet::call_index(7)]
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
    StorageProvider,
};
use frame_system::pallet_prelude::BlockNumberFor;
/// Helper functions (getters, setters, etc.) for this pallet
impl<T: Config> Pallet<T> {
    /// A helper function to get the information of a sign up request
    pub fn get_sign_up_request(
        who: &T::AccountId,
    ) -> Result<(StorageProvider<T>, BlockNumberFor<T>), Error<T>> {
        SignUpRequests::<T>::get(who).ok_or(Error::<T>::SignUpNotRequested)
    }

    /// A helper function to get the total capacity of a storage provider.
    pub fn get_total_capacity_of_sp(who: &T::AccountId) -> Result<StorageData<T>, Error<T>> {
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

    /// A helper function to get the total capacity of all BSPs.
    pub fn get_total_bsp_capacity() -> StorageData<T> {
        TotalBspsCapacity::<T>::get()
    }

    /// A helper function to get the total data used by a Main Storage Provider
    pub fn get_used_storage_of_msp(
        who: &MainStorageProviderId<T>,
    ) -> Result<StorageData<T>, Error<T>> {
        let msp = MainStorageProviders::<T>::get(who).ok_or(Error::<T>::NotRegistered)?;
        Ok(msp.data_used)
    }

    /// A helper function to get the total data used by a Backup Storage Provider
    pub fn get_used_storage_of_bsp(
        who: &BackupStorageProviderId<T>,
    ) -> Result<StorageData<T>, Error<T>> {
        let bsp = BackupStorageProviders::<T>::get(who).ok_or(Error::<T>::NotRegistered)?;
        Ok(bsp.data_used)
    }

    /// A helper function to get the total amount of BSPs that have registered
    pub fn get_bsp_count() -> T::SpCount {
        BspCount::<T>::get()
    }

    /// A helper function to get the total amount of MSPs that have registered
    pub fn get_msp_count() -> T::SpCount {
        MspCount::<T>::get()
    }
}
