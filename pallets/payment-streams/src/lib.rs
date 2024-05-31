//! # Payment Streams Pallet
//!
//! This pallet provides the functionality to create, update, delete and charge payment streams.
//!
//! Notes: we took the decision of considering that another pallet is the one in charge of keeping track of both the current global price
//! and the accumulated price index since genesis. The alternative would be to keep those two things here, we believe any of the two approaches
//! are valid, but we decided to keep this pallet as simple as possible.
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
use scale_info::prelude::vec::Vec;
pub use scale_info::Type;
use types::*;

#[frame_support::pallet]
pub mod pallet {
    use super::types::*;
    use codec::HasCompact;
    use frame_support::{
        dispatch::DispatchResultWithPostInfo, pallet_prelude::*, traits::fungible::*,
        Blake2_128Concat,
    };
    use frame_system::pallet_prelude::{BlockNumberFor, *};
    use shp_traits::ProvidersInterface;
    use sp_runtime::traits::{AtLeast32BitUnsigned, Convert, MaybeDisplay, Saturating};

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Type to access the Balances pallet (using the fungible trait from frame_support)
        type NativeBalance: Inspect<Self::AccountId>
            + Mutate<Self::AccountId>
            + hold::Inspect<Self::AccountId, Reason = Self::RuntimeHoldReason>
            + hold::Mutate<Self::AccountId, Reason = Self::RuntimeHoldReason>;

        /// The trait for reading provider data.
        type ProvidersPallet: ProvidersInterface<AccountId = Self::AccountId>;

        /// The overarching hold reason
        type RuntimeHoldReason: From<HoldReason>;

        /// A converter to be able to convert the block number type to the balance type for charging (multiplying time (blocks) by rate (balance))
        type BlockNumberToBalance: Convert<BlockNumberFor<Self>, BalanceOf<Self>>;

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
            + Into<BalanceOf<Self>>;

        /// The number of blocks that correspond to the deposit that a User has to pay to open a payment stream.
        /// This means that, from the balance of the User for which the payment stream is being created, the amount
        /// `NewStreamDeposit * rate` will be held as a deposit.
        /// In the case of dynamic-rate payment streams, `rate` will be `amount_provided * current_service_price`, where `current_service_price` has
        /// to be provided by the pallet using the `PaymentStreamsInterface` interface.
        #[pallet::constant]
        type NewStreamDeposit: Get<BlockNumberFor<Self>>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // Storage:

    /// The double mapping from a Provider, to its provided Users, to their fixed-rate payment streams.
    ///
    /// This is used to store and manage fixed-rate payment streams between Users and Providers.
    ///
    /// This storage is updated in:
    /// - [add_fixed_rate_payment_stream](crate::dispatchables::add_fixed_rate_payment_stream), which adds a new entry to the map.
    /// - [delete_fixed_rate_payment_stream](crate::dispatchables::delete_fixed_rate_payment_stream), which removes the corresponding entry from the map.
    /// - [update_fixed_rate_payment_stream](crate::dispatchables::update_fixed_rate_payment_stream), which updates the entry's `rate`.
    /// - [charge_payment_streams](crate::dispatchables::charge_payment_streams), which updates the entry's `last_charged_block`.
    /// - [update_last_chargeable_block](crate::dispatchables::update_last_chargeable_block), which updates the entry's `last_chargeable_block`.
    #[pallet::storage]
    pub type FixedRatePaymentStreams<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ProviderIdFor<T>,
        Blake2_128Concat,
        T::AccountId,
        FixedRatePaymentStream<T>,
    >;

    /// The double mapping from a Provider, to its provided Users, to their dynamic-rate payment streams.
    ///
    /// This is used to store and manage dynamic-rate payment streams between Users and Providers.
    ///
    /// This storage is updated in:
    /// - [add_dynamic_rate_payment_stream](crate::dispatchables::add_dynamic_rate_payment_stream), which adds a new entry to the map.
    /// - [delete_dynamic_rate_payment_stream](crate::dispatchables::delete_dynamic_rate_payment_stream), which removes the corresponding entry from the map.
    /// - [update_dynamic_rate_payment_stream](crate::dispatchables::update_dynamic_rate_payment_stream), which updates the entry's `amount_provided`.
    /// - [charge_payment_streams](crate::dispatchables::charge_payment_streams), which updates the entry's `price_index_when_last_charged`.
    /// - [update_last_chargeable_block](crate::dispatchables::update_last_chargeable_block), which updates the entry's `price_index_at_last_chargeable_block`.
    #[pallet::storage]
    pub type DynamicRatePaymentStreams<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ProviderIdFor<T>,
        Blake2_128Concat,
        T::AccountId,
        DynamicRatePaymentStream<T>,
    >;

    /// The mapping from a user to if it has been flagged for not having enough funds to pay for its requested services.
    ///
    /// This is used to flag users that do not have enough funds to pay for their requested services, so other Providers
    /// can stop providing services to them.
    ///
    /// This storage is updated in:
    /// - [charge_payment_streams](crate::dispatchables::charge_payment_streams), which emits a `UserWithoutFunds` event and sets the user's entry in this map if it does not
    /// have enough funds, and clears the entry if it was set and the user has enough funds.
    #[pallet::storage]
    pub type UsersWithoutFunds<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, ()>;

    /// The mapping from a user to if it has been registered to the network and the amount of payment streams it has.
    ///
    /// Since users have to provide a deposit to be able to open each payment stream, this is used to keep track of the amount of payment streams
    /// that a user has and it is also useful to check if a user has registered to the network.
    ///
    /// This storage is updated in:
    /// - [add_fixed_rate_payment_stream](crate::dispatchables::add_fixed_rate_payment_stream), which holds the deposit of the user and adds one to this storage.
    /// - [add_dynamic_rate_payment_stream](crate::dispatchables::add_dynamic_rate_payment_stream), which holds the deposit of the user and adds one to this storage.
    /// - [remove_fixed_rate_payment_stream](crate::dispatchables::remove_fixed_rate_payment_stream), which removes one from this storage and releases the deposit.
    /// - [remove_dynamic_rate_payment_stream](crate::dispatchables::remove_dynamic_rate_payment_stream), which removes one from this storage and releases the deposit.
    #[pallet::storage]
    pub type RegisteredUsers<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

    // Events & Errors:

    /// The events that can be emitted by this pallet
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event emitted when a fixed-rate payment stream is created. Provides information about the Provider and User of the stream
        /// and its initial rate.
        FixedRatePaymentStreamCreated {
            user_account: T::AccountId,
            provider_id: ProviderIdFor<T>,
            rate: BalanceOf<T>,
        },
        /// Event emitted when a fixed-rate payment stream is updated. Provides information about the User and Provider of the stream
        /// and the new rate of the stream.
        FixedRatePaymentStreamUpdated {
            user_account: T::AccountId,
            provider_id: ProviderIdFor<T>,
            new_rate: BalanceOf<T>,
        },
        /// Event emitted when a fixed-rate payment stream is removed. Provides information about the User and Provider of the stream.
        FixedRatePaymentStreamDeleted {
            user_account: T::AccountId,
            provider_id: ProviderIdFor<T>,
        },
        /// Event emitted when a dynamic-rate payment stream is created. Provides information about the User and Provider of the stream
        /// and the initial amount provided.
        DynamicRatePaymentStreamCreated {
            user_account: T::AccountId,
            provider_id: ProviderIdFor<T>,
            amount_provided: UnitsProvidedFor<T>,
        },
        /// Event emitted when a dynamic-rate payment stream is updated. Provides information about the User and Provider of the stream
        /// and the new amount provided.
        DynamicRatePaymentStreamUpdated {
            user_account: T::AccountId,
            provider_id: ProviderIdFor<T>,
            new_amount_provided: UnitsProvidedFor<T>,
        },
        /// Event emitted when a dynamic-rate payment stream is removed. Provides information about the User and Provider of the stream.
        DynamicRatePaymentStreamDeleted {
            user_account: T::AccountId,
            provider_id: ProviderIdFor<T>,
        },
        /// Event emitted when a payment is charged. Provides information about the user that was charged,
        /// the Provider that received the funds, and the amount that was charged.
        PaymentStreamCharged {
            user_account: T::AccountId,
            provider_id: ProviderIdFor<T>,
            amount: BalanceOf<T>,
        },
        /// Event emitted when a payment stream's last chargeable block is updated. Provides information about the User and Provider of the stream
        /// and the block number of the last chargeable block.
        LastChargeableBlockUpdated {
            user_account: T::AccountId,
            provider_id: ProviderIdFor<T>,
            last_chargeable_block: BlockNumberFor<T>,
        },
        /// Event emitted when a Provider is correctly trying to charge a User and that User does not have enough funds to pay for their services.
        /// This event is emitted to flag the user and let the network know that the user is not paying for the requested services, so other Providers can
        /// stop providing services to that user.
        UserWithoutFunds { who: T::AccountId },
    }

    /// The errors that can be thrown by this pallet to inform users about what went wrong
    #[pallet::error]
    pub enum Error<T> {
        /// Error thrown when a user of this pallet tries to add a payment stream that already exists.
        PaymentStreamAlreadyExists,
        /// Error thrown when a user of this pallet tries to update, remove or charge a payment stream that does not exist.
        PaymentStreamNotFound,
        /// Error thrown when a user tries to charge a payment stream and it's not a registered Provider
        NotAProvider,
        /// Error thrown when failing to get the payment account of a registered Provider
        ProviderInconsistencyError,
        /// Error thrown when the system can't hold funds from the User as a deposit for creating a new payment stream
        CannotHoldDeposit,
        /// Error thrown when trying to update the rate of a fixed-rate payment stream to the same rate as before
        UpdateRateToSameRate,
        /// Error thrown when trying to update the amount provided of a dynamic-rate payment stream to the same amount as before
        UpdateAmountToSameAmount,
        /// Error thrown when trying to create a new fixed-rate payment stream with rate 0 or update the rate of an existing one to 0 (should use remove_fixed_rate_payment_stream instead)
        RateCantBeZero,
        /// Error thrown when trying to create a new dynamic-rate payment stream with amount provided 0 or update the amount provided of an existing one to 0 (should use remove_dynamic_rate_payment_stream instead)
        AmountProvidedCantBeZero,
        /// Error thrown when the block number of when the payment stream was last charged is greater than the block number of the last chargeable block
        LastChargedGreaterThanLastChargeable,
        /// Error thrown when the new last chargeable block number that is trying to be set by the PaymentManager is greater than the current block number or smaller than the previous last chargeable block number
        InvalidLastChargeableBlockNumber,
        /// Error thrown when the new last chargeable price index that is trying to be set by the PaymentManager is greater than the current price index or smaller than the previous last chargeable price index
        InvalidLastChargeablePriceIndex,
        /// Error thrown when charging a payment stream would result in an overflow of the balance type (TODO: maybe we should use saturating arithmetic instead)
        ChargeOverflow,
        /// Error thrown when trying to operate when the User has been flagged for not having enough funds.
        UserWithoutFunds,
    }

    /// This enum holds the HoldReasons for this pallet, allowing the runtime to identify each held balance with different reasons separately
    ///
    /// This allows us to hold tokens and be able to identify in the future that those held tokens were
    /// held because of this pallet
    #[pallet::composite_enum]
    pub enum HoldReason {
        /// Deposit that a user has to pay to open payment streams
        PaymentStreamDeposit,
        // Only for testing, another unrelated hold reason
        #[cfg(test)]
        AnotherUnrelatedHold,
    }

    /// Dispatchables (extrinsics) exposed by this pallet
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Dispatchable extrinsic that allows root to add a fixed-rate payment stream from a User to a Provider.
        ///
        /// The dispatch origin for this call must be Root (Payment streams should only be added by traits in other pallets,
        /// this extrinsic is for manual testing).
        ///
        /// Parameters:
        /// - `provider_id`: The Provider ID that the payment stream is for.
        /// - `user_account`: The User Account ID that the payment stream is for.
        /// - `rate`: The initial rate of the payment stream.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was executed by the root origin
        /// 2. Check that the payment stream does not already exist
        /// 3. Check that the User has enough funds to pay the deposit
        /// 4. Hold the deposit from the User
        /// 5. Update the Payment Streams storage to add the new payment stream
        ///
        /// Emits `FixedRatePaymentStreamCreated` event when successful.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn create_fixed_rate_payment_stream(
            origin: OriginFor<T>,
            provider_id: ProviderIdFor<T>,
            user_account: T::AccountId,
            rate: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was executed by the root origin
            ensure_root(origin)?;

            // Execute checks and logic, update storage
            Self::do_create_fixed_rate_payment_stream(&provider_id, &user_account, rate)?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::FixedRatePaymentStreamCreated {
                user_account,
                provider_id: provider_id,
                rate,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows root to update an existing fixed-rate payment stream between a User and a Provider.
        ///
        /// The dispatch origin for this call must be Root (Payment streams should only be added by traits in other pallets,
        /// this extrinsic is for manual testing).
        ///
        /// Parameters:
        /// - `provider_id`: The Provider ID that the payment stream is for.
        /// - `user_account`: The User Account ID that the payment stream is for.
        /// - `new_rate`: The new rate of the payment stream.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was executed by the root origin
        /// 2. Check that the payment stream exists
        /// 3. Update the Payment Streams storage to update the payment stream
        ///
        /// Emits `FixedRatePaymentStreamUpdated` event when successful.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn update_fixed_rate_payment_stream(
            origin: OriginFor<T>,
            provider_id: ProviderIdFor<T>,
            user_account: T::AccountId,
            new_rate: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was executed by the root origin
            ensure_root(origin)?;

            // Execute checks and logic, update storage
            Self::do_update_fixed_rate_payment_stream(&provider_id, &user_account, new_rate)?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::FixedRatePaymentStreamUpdated {
                user_account,
                provider_id: provider_id,
                new_rate,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows root to delete an existing fixed-rate payment stream between a User and a Provider.
        ///
        /// The dispatch origin for this call must be Root (Payment streams should only be added by traits in other pallets,
        /// this extrinsic is for manual testing).
        ///
        /// Parameters:
        /// - `provider_id`: The Provider ID that the payment stream is for.
        /// - `user_account`: The User Account ID that the payment stream is for.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was executed by the root origin
        /// 2. Check that the payment stream exists
        /// 3. Update the Payment Streams storage to remove the payment stream
        ///
        /// Emits `FixedRatePaymentStreamDeleted` event when successful.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn delete_fixed_rate_payment_stream(
            origin: OriginFor<T>,
            provider_id: ProviderIdFor<T>,
            user_account: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was executed by the root origin
            ensure_root(origin)?;

            // Execute checks and logic, update storage
            Self::do_delete_fixed_rate_payment_stream(&provider_id, &user_account)?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::FixedRatePaymentStreamDeleted {
                user_account,
                provider_id: provider_id,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows root to add a dynamic-rate payment stream from a User to a Provider.
        ///
        /// The dispatch origin for this call must be Root (Payment streams should only be added by traits in other pallets,
        /// this extrinsic is for manual testing).
        ///
        /// Parameters:
        /// - `provider_id`: The Provider ID that the payment stream is for.
        /// - `user_account`: The User Account ID that the payment stream is for.
        /// - `amount_provided`: The initial amount provided by the Provider.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was executed by the root origin
        /// 2. Check that the payment stream does not already exist
        /// 3. Check that the User has enough funds to pay the deposit
        /// 4. Hold the deposit from the User
        /// 5. Update the Payment Streams storage to add the new payment stream
        ///
        /// Emits `DynamicRatePaymentStreamCreated` event when successful.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn create_dynamic_rate_payment_stream(
            origin: OriginFor<T>,
            provider_id: ProviderIdFor<T>,
            user_account: T::AccountId,
            amount_provided: UnitsProvidedFor<T>,
            current_price: BalanceOf<T>,
            current_accumulated_price_index: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was executed by the root origin
            ensure_root(origin)?;

            // Execute checks and logic, update storage
            Self::do_create_dynamic_rate_payment_stream(
                &provider_id,
                &user_account,
                amount_provided,
                current_price,
                current_accumulated_price_index,
            )?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::DynamicRatePaymentStreamCreated {
                user_account,
                provider_id: provider_id,
                amount_provided,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows root to update an existing dynamic-rate payment stream between a User and a Provider.
        ///
        /// The dispatch origin for this call must be Root (Payment streams should only be added by traits in other pallets,
        /// this extrinsic is for manual testing).
        ///
        /// Parameters:
        /// - `provider_id`: The Provider ID that the payment stream is for.
        /// - `user_account`: The User Account ID that the payment stream is for.
        /// - `new_amount_provided`: The new amount provided by the Provider.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was executed by the root origin
        /// 2. Check that the payment stream exists
        /// 3. Update the Payment Streams storage to update the payment stream
        ///
        /// Emits `DynamicRatePaymentStreamUpdated` event when successful.
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn update_dynamic_rate_payment_stream(
            origin: OriginFor<T>,
            provider_id: ProviderIdFor<T>,
            user_account: T::AccountId,
            new_amount_provided: UnitsProvidedFor<T>,
            current_price: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was executed by the root origin
            ensure_root(origin)?;

            // Execute checks and logic, update storage
            Self::do_update_dynamic_rate_payment_stream(
                &provider_id,
                &user_account,
                new_amount_provided,
                current_price,
            )?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::DynamicRatePaymentStreamUpdated {
                user_account,
                provider_id,
                new_amount_provided,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows root to delete an existing dynamic-rate payment stream between a User and a Provider.
        ///
        /// The dispatch origin for this call must be Root (Payment streams should only be added by traits in other pallets,
        /// this extrinsic is for manual testing).
        ///
        /// Parameters:
        /// - `provider_id`: The Provider ID that the payment stream is for.
        /// - `user_account`: The User Account ID that the payment stream is for.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was executed by the root origin
        /// 2. Check that the payment stream exists
        /// 3. Update the Payment Streams storage to remove the payment stream
        ///
        /// Emits `DynamicRatePaymentStreamDeleted` event when successful.
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn delete_dynamic_rate_payment_stream(
            origin: OriginFor<T>,
            provider_id: ProviderIdFor<T>,
            user_account: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was executed by the root origin
            ensure_root(origin)?;

            // Execute checks and logic, update storage
            Self::do_delete_dynamic_rate_payment_stream(&provider_id, &user_account)?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::DynamicRatePaymentStreamDeleted {
                user_account,
                provider_id,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows Providers to charge a payment stream from a User.
        ///
        /// The dispatch origin for this call must be Signed.
        /// The origin must be the Provider that has at least one type of payment stream with the User.
        ///
        /// Parameters:
        /// - `user_account`: The User Account ID that the payment stream is for.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that a payment stream between the signer (Provider) and the User exists
        /// 3. If there is a fixed-rate payment stream:
        ///    1. Get the rate of the payment stream
        ///    2. Get the difference between the last charged block number and the last chargeable block number of the stream
        ///    3. Calculate the amount to charge doing `rate * difference`
        ///    4. Charge the user (if the user does not have enough funds, it gets flagged and a `UserWithoutFunds` event is emitted)
        ///    5. Update the last charged block number of the payment stream
        /// 4. If there is a dynamic-rate payment stream:
        ///    1. Get the amount provided by the Provider
        ///    2. Get the difference between price index when the stream was last charged and the price index at the last chargeable block
        ///    3. Calculate the amount to charge doing `amount_provided * difference`
        ///    4. Charge the user (if the user does not have enough funds, it gets flagged and a `UserWithoutFunds` event is emitted)
        ///    5. Update the price index when the stream was last charged of the payment stream
        ///
        /// Emits a `PaymentStreamCharged` event when successful.
        ///
        /// Notes: a Provider could have both a fixed-rate and a dynamic-rate payment stream with a User. If that's the case, this extrinsic
        /// will try to charge both and the amount charged will be the sum of the amounts charged for each payment stream.
        #[pallet::call_index(6)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
        pub fn charge_payment_streams(
            origin: OriginFor<T>,
            user_account: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer
            let provider_account = ensure_signed(origin)?;

            // Get the Provider ID of the signer
            let provider_id =
                <T::ProvidersPallet as ProvidersInterface>::get_provider_id(provider_account)
                    .ok_or(Error::<T>::NotAProvider)?;

            // Execute checks and logic, update storage
            let amount_charged = Self::do_charge_payment_streams(&provider_id, &user_account)?;

            // Emit the corresponding event (we always emit it even if the charged amount was 0)
            Self::deposit_event(Event::<T>::PaymentStreamCharged {
                user_account,
                provider_id: provider_id,
                amount: amount_charged,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }
    }
}

/// Helper functions (getters, setters, etc.) for this pallet
impl<T: Config> Pallet<T> {
    /// A helper function to get the information of a fixed-rate payment stream
    pub fn get_fixed_rate_payment_stream_info(
        provider_id: &ProviderIdFor<T>,
        user_account: &T::AccountId,
    ) -> Result<FixedRatePaymentStream<T>, Error<T>> {
        FixedRatePaymentStreams::<T>::get(provider_id, user_account)
            .ok_or(Error::<T>::PaymentStreamNotFound)
    }

    /// A helper function to get the information of a dynamic-rate payment stream
    pub fn get_dynamic_rate_payment_stream_info(
        provider_id: &ProviderIdFor<T>,
        user_account: &T::AccountId,
    ) -> Result<DynamicRatePaymentStream<T>, Error<T>> {
        DynamicRatePaymentStreams::<T>::get(provider_id, user_account)
            .ok_or(Error::<T>::PaymentStreamNotFound)
    }

    /// A helper function to get all users that have a payment stream with a Provider
    /// Note: users with both a fixed-rate and a dynamic-rate payment stream are duplicated in the result
    pub fn get_users_with_payment_stream_with_provider(
        provider_id: &ProviderIdFor<T>,
    ) -> Vec<T::AccountId> {
        let fixed_rate_users: Vec<T::AccountId> =
            FixedRatePaymentStreams::<T>::iter_prefix(provider_id)
                .map(|(user_account, _)| user_account)
                .collect();
        let dynamic_rate_users: Vec<T::AccountId> =
            DynamicRatePaymentStreams::<T>::iter_prefix(provider_id)
                .map(|(user_account, _)| user_account)
                .collect();
        fixed_rate_users
            .into_iter()
            .chain(dynamic_rate_users.into_iter())
            .collect()
    }

    /// A helper function that gets all fixed-rate payment streams of a Provider
    pub fn get_fixed_rate_payment_streams_of_provider(
        provider_id: &ProviderIdFor<T>,
    ) -> Vec<(T::AccountId, FixedRatePaymentStream<T>)> {
        FixedRatePaymentStreams::<T>::iter_prefix(provider_id).collect()
    }

    /// A helper function that gets all dynamic-rate payment streams of a Provider
    pub fn get_dynamic_rate_payment_streams_of_provider(
        provider_id: &ProviderIdFor<T>,
    ) -> Vec<(T::AccountId, DynamicRatePaymentStream<T>)> {
        DynamicRatePaymentStreams::<T>::iter_prefix(provider_id).collect()
    }

    /// A helper function that gets all fixed-rate payment streams of a User
    pub fn get_fixed_rate_payment_streams_of_user(
        user_account: &T::AccountId,
    ) -> Vec<(ProviderIdFor<T>, FixedRatePaymentStream<T>)> {
        FixedRatePaymentStreams::<T>::iter()
            .filter(|(_, user, _)| user == user_account)
            .map(|(provider_id, _, stream)| (provider_id, stream))
            .collect()
    }

    /// A helper function that gets all dynamic-rate payment streams of a User
    pub fn get_dynamic_rate_payment_streams_of_user(
        user_account: &T::AccountId,
    ) -> Vec<(ProviderIdFor<T>, DynamicRatePaymentStream<T>)> {
        DynamicRatePaymentStreams::<T>::iter()
            .filter(|(_, user, _)| user == user_account)
            .map(|(provider_id, _, stream)| (provider_id, stream))
            .collect()
    }

    /// A helper function that gets the amount of open payment streams of a user
    pub fn get_payment_streams_count_of_user(user_account: &T::AccountId) -> u32 {
        RegisteredUsers::<T>::get(user_account)
    }

    /// A helper function that returns if a user has been flagged for not having enough funds
    pub fn is_user_without_funds(user_account: &T::AccountId) -> bool {
        UsersWithoutFunds::<T>::contains_key(user_account)
    }
}
