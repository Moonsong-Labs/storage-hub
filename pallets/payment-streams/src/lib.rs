//! # Storage Providers Pallet
//!
//! This pallet provides the functionality to manage Main Storage Providers (MSPs)
//! and Backup Storage Providers (BSPs) in a decentralized storage network.
//!
//! The functionality allows users to sign up and sign off as MSPs or BSPs and change
//! their parameters. This is the way that users can offer their storage capacity to
//! the network and get rewarded for it.
#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

mod types;
mod utils;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub use pallet::*;
pub use scale_info::Type;
use types::{BackupStorageProviderId, BalanceOf, PaymentStream};

#[frame_support::pallet]
pub mod pallet {
    use super::types::*;
    use frame_support::{
        dispatch::DispatchResultWithPostInfo, pallet_prelude::*, traits::fungible::*,
        Blake2_128Concat,
    };
    use frame_system::pallet_prelude::{BlockNumberFor, *};
    use sp_runtime::traits::Convert;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Type to access the Balances pallet (using the fungible trait from frame_support)
        type NativeBalance: Inspect<Self::AccountId>
            + Mutate<Self::AccountId>
            + hold::Inspect<Self::AccountId, Reason = Self::RuntimeHoldReason>
            // , Reason = Self::HoldReason> We will probably have to hold deposits
            + hold::Mutate<Self::AccountId, Reason = Self::RuntimeHoldReason>
            + freeze::Inspect<Self::AccountId>
            + freeze::Mutate<Self::AccountId>;

        /// The trait for reading storage provider data.
        type Providers: storage_hub_traits::ProvidersInterface<
            Balance = Self::NativeBalance,
            AccountId = Self::AccountId,
            Provider = <Self as frame_system::Config>::Hash,
            MerkleHash = <Self as frame_system::Config>::Hash,
        >;

        /// The overarching hold reason
        type RuntimeHoldReason: From<HoldReason>;

        /// A converter to be able to convert the block number type to the balance type for charging (multiplying time (blocks) by rate (balance))
        type BlockNumberToBalance: Convert<BlockNumberFor<Self>, BalanceOf<Self>>;

        /// The amounts of funds to hold when a user first registers to the network (TODO: Again, this should not be done here but since we don't
        /// have it elsewhere, we will leave it here for now)
        #[pallet::constant]
        type NewUserDeposit: Get<BalanceOf<Self>>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // Storage:

    /// The mapping from a Backup Storage Provider to its provided users to their payment streams.
    ///
    /// This is used to get the payment stream of a user for a specific Backup Storage Provider.
    ///
    /// This storage is updated in:
    /// - [add_payment_stream](crate::dispatchables::add_payment_stream), which adds a new entry to the map.
    /// - [remove_payment_stream](crate::dispatchables::remove_payment_stream), which removes the corresponding entry from the map.
    /// - [update_payment_stream](crate::dispatchables::update_payment_stream), which updates the entry's `rate`.
    /// - [charge_payment](crate::dispatchables::charge_payment), which updates the entry's `last_charge`.
    /// - [update_valid_proof](crate::dispatchables::update_valid_proof), which updates the entry's `last_valid_proof`.
    #[pallet::storage]
    pub type PaymentStreams<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        BackupStorageProviderId<T>,
        Blake2_128Concat,
        T::AccountId,
        PaymentStream<T>,
    >;

    /// The mapping from a user to if it has been flagged for not having enough funds to pay for its storage.
    ///
    /// This is used to flag users that do not have enough funds to pay for their storage, so other Backup Storage Providers
    /// can stop providing storage to them.
    ///
    /// This storage is updated in:
    /// - [charge_payment](crate::dispatchables::charge_payment), which emits a `UserWithoutFunds` event if the user does not have enough funds.
    #[pallet::storage]
    pub type UsersWithoutFunds<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, ()>;

    /// The mapping from a user to if it has been registered to the network and the amount of payment streams it has.
    ///
    /// This is used to check if a user has already been registered to the network and his deposit has been held.
    ///
    /// This storage is updated in:
    /// - [add_payment_stream](crate::dispatchables::add_payment_stream), which holds the deposit of the user and adds one to this storage.
    /// - [remove_payment_stream](crate::dispatchables::remove_payment_stream), which removes one from this storage and if it's 0 releases its deposit.
    #[pallet::storage]
    pub type RegisteredUsers<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

    // Events & Errors:

    /// The events that can be emitted by this pallet
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event emitted when a payment stream is created. Provides information about the user that created the stream,
        /// the Backup Storage Provider that the stream is for, and the rate of the stream.
        PaymentStreamCreated {
            user_account: T::AccountId,
            backup_storage_provider_id: BackupStorageProviderId<T>,
            rate: BalanceOf<T>,
        },
        /// Event emitted when a payment stream is updated. Provides information about the user that updated the stream,
        /// the Backup Storage Provider that the stream is for, and the new rate of the stream.
        PaymentStreamUpdated {
            user_account: T::AccountId,
            backup_storage_provider_id: BackupStorageProviderId<T>,
            new_rate: BalanceOf<T>,
        },
        /// Event emitted when a payment stream is removed. Provides information about the user that removed the stream,
        /// and the Backup Storage Provider that the stream was for.
        PaymentStreamRemoved {
            user_account: T::AccountId,
            backup_storage_provider_id: BackupStorageProviderId<T>,
        },
        /// Event emitted when a payment is charged. Provides information about the user that was charged,
        /// the Backup Storage Provider that received the funds, and the amount that was charged.
        PaymentCharged {
            user_account: T::AccountId,
            backup_storage_provider_id: BackupStorageProviderId<T>,
            amount: BalanceOf<T>,
        },
        /// Event emitted when a payment stream's last valid proof is updated. Provides information about the user that the stream is for,
        /// the Backup Storage Provider that provided the proof, and the new block number of the last valid proof.
        ValidProofUpdated {
            user_account: T::AccountId,
            backup_storage_provider_id: BackupStorageProviderId<T>,
        },
        /// Event emitted when a BSP is correctly trying to charge a user and that user does not have enough funds to pay for their storage
        /// This event is emitted to flag the user and let the network know that the user is not paying for their storage, so other BSPs can
        /// stop providing storage to that user.
        UserWithoutFunds { who: T::AccountId },
    }

    /// The errors that can be thrown by this pallet to inform users about what went wrong
    #[pallet::error]
    pub enum Error<T> {
        /// Error thrown when a user of this pallet tries to add a payment stream that already exists.
        PaymentStreamAlreadyExists,
        /// Error thrown when a user of this pallet tries to update, remove or charge a payment stream that does not exist.
        PaymentStreamNotFound,
        /// Error thrown when a user tries to charge a payment stream and it's not a registered Backup Storage Provider
        NotABackupStorageProvider,
        /// Error thrown when the system can't hold funds from the user as a deposit for the storage used in this pallet
        CannotHoldDeposit,
        /// Error thrown when trying to update the rate of a payment stream to the same rate as before
        UpdateRateToSameRate,
        /// Error thrown when trying to update the rate of a payment stream to 0 (should use remove_payment_stream instead)
        UpdateRateToZero,
        /// Error thrown when the block of the last charged proof of a payment stream is greater than the block of the last valid proof
        LastChargeGreaterThanLastValidProof,
        /// Error thrown when charging a payment stream would result in an overflow of the balance type (TODO: maybe we should use saturating arithmetic instead)
        ChargeOverflow,
        /// Error thrown when a payment stream is being created or updated and the user has been flagged for not having enough funds.
        UserWithoutFunds,
    }

    /// This enum holds the HoldReasons for this pallet, allowing the runtime to identify each held balance with different reasons separately
    ///
    /// This allows us to hold tokens and be able to identify in the future that those held tokens were
    /// held because of this pallet
    #[pallet::composite_enum]
    pub enum HoldReason {
        /// Deposit that a user has to pay to open payment streams (TODO: This should go on user registration I believe, not in this pallet)
        PaymentStreamStorageDeposit,
        // TODO: Only for testing, remove this for production
        AnotherUnrelatedHold,
    }

    /// The hooks that this pallet utilizes (TODO: Check this, we might not need any)
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    /// Dispatchables (extrinsics) exposed by this pallet
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Dispatchable extrinsic that allows users to add a payment stream to a Backup Storage Provider and user.
        ///
        /// The dispatch origin for this call must be Root (Payment streams should only be added by traits in other pallets,
        /// this extrinsic is for testing only).
        ///
        /// Parameters:
        /// - `bsp_id`: The Backup Storage Provider ID that the payment stream is for.
        /// - `user_account`: The user ID that the payment stream is for.
        /// - `rate`: The initial rate of the payment stream.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was executed by the root origin
        /// 2. Check that the payment stream does not already exist
        /// 3. Check that the user has enough funds to pay the deposit (TODO: This should be done in the user registration, not here)
        /// 4. Hold the deposit from the user
        /// 5. Update the Payment Streams storage to add the new payment stream
        ///
        /// Emits `PaymentStreamCreated` event when successful.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn create_payment_stream(
            origin: OriginFor<T>,
            bsp_account: T::AccountId,
            user_account: T::AccountId,
            rate: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was executed by the root origin
            ensure_root(origin)?;

            // Execute checks and logic, update storage
            Self::do_create_payment_stream(&bsp_account, &user_account, rate)?;

            // Get the BSP ID of the BSP account
            let bsp_id =
                <T::Providers as storage_hub_traits::ProvidersInterface>::get_provider(bsp_account)
                    .ok_or(Error::<T>::NotABackupStorageProvider)?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::PaymentStreamCreated {
                user_account,
                backup_storage_provider_id: bsp_id,
                rate,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows users to update a payment stream from a Backup Storage Provider and user.
        ///
        /// The dispatch origin for this call must be Root (Payment streams should only be updated by traits in other pallets,
        /// this extrinsic is for testing only).
        ///
        /// Parameters:
        /// - `bsp_id`: The Backup Storage Provider ID that the payment stream is for.
        /// - `user_account`: The user ID that the payment stream is for.
        /// - `new_rate`: The new rate of the payment stream.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was executed by the root origin
        /// 2. Check that the payment stream exists
        /// 3. Update the Payment Streams storage to update the payment stream
        ///
        /// Emits `PaymentStreamUpdated` event when successful.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn update_payment_stream(
            origin: OriginFor<T>,
            bsp_account: T::AccountId,
            user_account: T::AccountId,
            new_rate: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was executed by the root origin
            ensure_root(origin)?;

            // Execute checks and logic, update storage
            Self::do_update_payment_stream(&bsp_account, &user_account, new_rate)?;

            // Get the BSP ID of the BSP account
            let bsp_id =
                <T::Providers as storage_hub_traits::ProvidersInterface>::get_provider(bsp_account)
                    .ok_or(Error::<T>::NotABackupStorageProvider)?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::PaymentStreamUpdated {
                user_account,
                backup_storage_provider_id: bsp_id,
                new_rate,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows users to remove a payment stream from a Backup Storage Provider and user.
        ///
        /// The dispatch origin for this call must be Root (Payment streams should only be removed by traits in other pallets,
        /// this extrinsic is for testing only).
        ///
        /// Parameters:
        /// - `bsp_id`: The Backup Storage Provider ID that the payment stream is for.
        /// - `user_account`: The user ID that the payment stream is for.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was executed by the root origin
        /// 2. Check that the payment stream exists
        /// 3. Update the Payment Streams storage to remove the payment stream
        ///
        /// Emits `PaymentStreamRemoved` event when successful.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn delete_payment_stream(
            origin: OriginFor<T>,
            bsp_account: T::AccountId,
            user_account: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was executed by the root origin
            ensure_root(origin)?;

            // Execute checks and logic, update storage
            Self::do_delete_payment_stream(&bsp_account, &user_account)?;

            // Get the BSP ID of the BSP account
            let bsp_id =
                <T::Providers as storage_hub_traits::ProvidersInterface>::get_provider(bsp_account)
                    .ok_or(Error::<T>::NotABackupStorageProvider)?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::PaymentStreamRemoved {
                user_account,
                backup_storage_provider_id: bsp_id,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows Backup Storage Providers to charge a payment stream from a user.
        ///
        /// The dispatch origin for this call must be Signed.
        /// The origin must be the Backup Storage Provider that has a payment stream with the user.
        ///
        /// Parameters:
        /// - `user_account`: The user ID that the payment stream is for.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the payment stream between the signer (BSP) and the user exists
        /// 3. Get the rate of the payment stream
        /// 4. Get the difference between the last charge and the last proof of the stream
        /// 5. Calculate the amount to charge
        /// 6. Charge the user (if the user does not have enough funds, it gets flagged and a `UserWithoutFunds` event is emitted)
        /// 7. Update the last charge of the payment stream
        ///
        /// Emits `PaymentCharged` event when successful.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
        pub fn charge_payment_stream(
            origin: OriginFor<T>,
            user_account: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer
            let bsp_account = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            let amount = Self::do_charge_payment_stream(&bsp_account, &user_account)?;

            // Get the BSP ID of the signer
            let bsp_id =
                <T::Providers as storage_hub_traits::ProvidersInterface>::get_provider(bsp_account)
                    .ok_or(Error::<T>::NotABackupStorageProvider)?;

            // Emit the corresponding event (we always emit it even if the charged amount was 0, to inform that the charge was successful)
            Self::deposit_event(Event::<T>::PaymentCharged {
                user_account,
                backup_storage_provider_id: bsp_id,
                amount,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }
    }
}

/// Helper functions (getters, setters, etc.) for this pallet
impl<T: Config> Pallet<T> {
    /// A helper function to get the information of a payment stream
    pub fn get_payment_stream_info(
        bsp_id: &BackupStorageProviderId<T>,
        user_account: &T::AccountId,
    ) -> Result<PaymentStream<T>, Error<T>> {
        PaymentStreams::<T>::get(bsp_id, user_account).ok_or(Error::<T>::PaymentStreamNotFound)
    }

    /// A helper function to get all users that have a payment stream with a Backup Storage Provider
    pub fn get_users_with_payment_stream_with_bsp(
        bsp_id: &BackupStorageProviderId<T>,
    ) -> Vec<T::AccountId> {
        PaymentStreams::<T>::iter_prefix(bsp_id)
            .map(|(user_account, _)| user_account)
            .collect()
    }

    /// A helper function that gets all payment streams of a Backup Storage Provider
    pub fn get_payment_streams_of_bsp(
        bsp_id: &BackupStorageProviderId<T>,
    ) -> Vec<(T::AccountId, PaymentStream<T>)> {
        PaymentStreams::<T>::iter_prefix(bsp_id).collect()
    }

    /// A helper function that gets all payment streams of a user
    pub fn get_payment_streams_of_user(
        user_account: &T::AccountId,
    ) -> Vec<(BackupStorageProviderId<T>, PaymentStream<T>)> {
        PaymentStreams::<T>::iter()
            .filter(|(_, user, _)| user == user_account)
            .map(|(bsp_id, _, stream)| (bsp_id, stream))
            .collect()
    }

    /// A helper function that gets the amount of open payment streams of a user
    pub fn get_amount_of_payment_streams_of_user(user_account: &T::AccountId) -> u32 {
        RegisteredUsers::<T>::get(user_account)
    }

    /// A helper function that returns if a user has been flagged for not having enough funds
    pub fn is_user_without_funds(user_account: &T::AccountId) -> bool {
        UsersWithoutFunds::<T>::contains_key(user_account)
    }
}
