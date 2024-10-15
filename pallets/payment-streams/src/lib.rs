//! # Payment Streams Pallet
//!
//! This pallet provides the functionality to create, update, delete and charge payment streams.
//!
//! Notes: we took the decision of considering that another pallet is the one in charge of keeping track of both the current global price
//! and the accumulated price index since genesis. The alternative would be to keep those two things here, we believe any of the two approaches
//! are valid, but we decided to keep this pallet as simple as possible.
#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

pub mod types;
mod utils;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use frame_system::pallet_prelude::BlockNumberFor;
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
    use shp_traits::{ProofSubmittersInterface, ReadProvidersInterface, SystemMetricsInterface};
    use sp_runtime::traits::{AtLeast32BitUnsigned, Convert, MaybeDisplay, One, Saturating};

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
        type ProvidersPallet: ReadProvidersInterface<AccountId = Self::AccountId>
            + SystemMetricsInterface<ProvidedUnit = Self::Units>;

        /// The trait exposing data of which providers submitted valid proofs in which ticks
        type ProvidersProofSubmitters: ProofSubmittersInterface<
            ProviderId = <Self::ProvidersPallet as ReadProvidersInterface>::ProviderId,
            TickNumber = BlockNumberFor<Self>,
        >;

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

        /// The number of ticks that correspond to the deposit that a User has to pay to open a payment stream.
        /// This means that, from the balance of the User for which the payment stream is being created, the amount
        /// `NewStreamDeposit * rate` will be held as a deposit.
        /// In the case of dynamic-rate payment streams, `rate` will be `amount_provided * current_service_price`, where `current_service_price` has
        /// to be provided by the pallet using the `PaymentStreamsInterface` interface.
        #[pallet::constant]
        type NewStreamDeposit: Get<BlockNumberFor<Self>>;

        /// The number of ticks that a user will have to wait after it has been flagged as without funds to be able to clear that flag
        /// and be able to pay for services again. If there's any outstanding debt when the flag is cleared, it will be paid.
        #[pallet::constant]
        type UserWithoutFundsCooldown: Get<BlockNumberFor<Self>>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // Storage:

    /// A counter of blocks for which Providers can charge their streams.
    ///
    /// This counter is not necessarily the same as the block number, as the last chargeable info of Providers
    /// (and the global price index) are updated in the `on_poll` hook, which happens at the beginning of every block,
    /// so long as the block is not part of a [Multi-Block-Migration](https://github.com/paritytech/polkadot-sdk/pull/1781) (MBM).
    /// During MBMs, the block number increases, but `OnPollTicker` does not.
    #[pallet::storage]
    pub type OnPollTicker<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    /// The double mapping from a Provider, to its provided Users, to their fixed-rate payment streams.
    ///
    /// This is used to store and manage fixed-rate payment streams between Users and Providers.
    ///
    /// This storage is updated in:
    /// - [add_fixed_rate_payment_stream](crate::dispatchables::add_fixed_rate_payment_stream), which adds a new entry to the map.
    /// - [delete_fixed_rate_payment_stream](crate::dispatchables::delete_fixed_rate_payment_stream), which removes the corresponding entry from the map.
    /// - [update_fixed_rate_payment_stream](crate::dispatchables::update_fixed_rate_payment_stream), which updates the entry's `rate`.
    /// - [charge_payment_streams](crate::dispatchables::charge_payment_streams), which updates the entry's `last_charged_tick`.
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
    #[pallet::storage]
    pub type DynamicRatePaymentStreams<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ProviderIdFor<T>,
        Blake2_128Concat,
        T::AccountId,
        DynamicRatePaymentStream<T>,
    >;

    /// The mapping from a Provider to its last chargeable price index (for dynamic-rate payment streams) and last chargeable tick (for fixed-rate payment streams).
    ///
    /// This is used to keep track of the last chargeable price index and tick number for each Provider, so this pallet can charge the payment streams correctly.
    ///
    /// This storage is updated in:
    /// - [update_last_chargeable_info](crate::PaymentManager::update_last_chargeable_info), which updates the entry's `last_chargeable_tick` and `price_index`.
    #[pallet::storage]
    pub type LastChargeableInfo<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        ProviderIdFor<T>,
        ProviderLastChargeableInfo<T>,
        ValueQuery,
    >;

    /// The last tick from the Providers Proof Submitters pallet that was registered.
    ///
    /// This is used to keep track of the last tick from the Providers Proof Submitters pallet, that this pallet
    /// registered. For the tick in this storage element, this pallet already knows the Providers that submitted
    /// a valid proof.
    #[pallet::storage]
    pub type LastSubmittersTickRegistered<T: Config> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    /// The mapping from a user to if it has been flagged for not having enough funds to pay for its requested services.
    ///
    /// This is used to flag users that do not have enough funds to pay for their requested services, so other Providers
    /// can stop providing services to them.
    ///
    /// This storage is updated in:
    /// - [charge_payment_streams](crate::dispatchables::charge_payment_streams), which emits a `UserWithoutFunds` event and sets the user's entry in this map if it does not
    /// have enough funds, and clears the entry if it was set and the user has enough funds.
    #[pallet::storage]
    pub type UsersWithoutFunds<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberFor<T>>;

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

    /// The current price per unit per tick of the provided service, used to calculate the amount to charge for dynamic-rate payment streams.
    ///
    /// This is updated each tick using the formula that considers current system capacity (total storage of the system) and system availability (total storage available).
    ///
    /// This storage is updated in:
    /// - [do_update_current_price_per_unit_per_tick](crate::utils::do_update_current_price_per_unit_per_tick), which updates the current price per unit per tick.
    #[pallet::storage]
    pub type CurrentPricePerUnitPerTick<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// The accumulated price index since genesis, used to calculate the amount to charge for dynamic-rate payment streams.
    ///
    /// This is equivalent to what it would have cost to provide one unit of the provided service since the beginning of the network.
    /// We use this to calculate the amount to charge for dynamic-rate payment streams, by checking out the difference between the index
    /// when the payment stream was last charged, and the index at the last chargeable tick.
    ///
    /// This storage is updated in:
    /// - [do_update_price_index](crate::utils::do_update_price_index), which updates the accumulated price index, adding to it the current price.
    #[pallet::storage]
    pub type AccumulatedPriceIndex<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    // Genesis config:

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub current_price: BalanceOf<T>,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            let current_price = One::one();

            CurrentPricePerUnitPerTick::<T>::put(current_price);

            Self { current_price }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            CurrentPricePerUnitPerTick::<T>::put(self.current_price);
        }
    }

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
        /// the Provider that received the funds, the tick up to which it was charged and the amount that was charged.
        PaymentStreamCharged {
            user_account: T::AccountId,
            provider_id: ProviderIdFor<T>,
            amount: BalanceOf<T>,
            last_tick_charged: BlockNumberFor<T>,
            charged_at_tick: BlockNumberFor<T>,
        },
        /// Event emitted when a Provider's last chargeable tick and price index are updated. Provides information about the Provider of the stream,
        /// the tick number of the last chargeable tick and the price index at that tick.
        LastChargeableInfoUpdated {
            provider_id: ProviderIdFor<T>,
            last_chargeable_tick: BlockNumberFor<T>,
            last_chargeable_price_index: BalanceOf<T>,
        },
        /// Event emitted when a Provider is correctly trying to charge a User and that User does not have enough funds to pay for their services.
        /// This event is emitted to flag the user and let the network know that the user is not paying for the requested services, so other Providers can
        /// stop providing services to that user.
        UserWithoutFunds { who: T::AccountId },
        /// Event emitted when a User that has been flagged as not having enough funds to pay for their contracted services has paid all its outstanding debt.
        UserPaidDebts { who: T::AccountId },
        /// Event emitted when a User that has been flagged as not having enough funds to pay for their contracted services has waited the cooldown period,
        /// correctly paid all their outstanding debt and can now contract new services again.
        UserSolvent { who: T::AccountId },
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
        /// Error thrown when the tick number of when the payment stream was last charged is greater than the tick number of the last chargeable tick
        LastChargedGreaterThanLastChargeable,
        /// Error thrown when the new last chargeable tick number that is trying to be set is greater than the current tick number or smaller than the previous last chargeable tick number
        InvalidLastChargeableBlockNumber,
        /// Error thrown when the new last chargeable price index that is trying to be set is greater than the current price index or smaller than the previous last chargeable price index
        InvalidLastChargeablePriceIndex,
        /// Error thrown when charging a payment stream would result in an overflow of the balance type
        ChargeOverflow,
        /// Error thrown when trying to operate when the User has been flagged for not having enough funds.
        UserWithoutFunds,
        /// Error thrown when a user that has not been flagged as without funds tries to use the extrinsic to pay its outstanding debt
        UserNotFlaggedAsWithoutFunds,
        /// Error thrown when a user tries to clear the flag of being without funds before the cooldown period has passed
        CooldownPeriodNotPassed,
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

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// This hook is used to update the last chargeable info of the Providers that submitted a valid proof.
        ///
        /// It will be called at the beginning of every block, if the block is not being part of a
        /// [Multi-Block-Migration](https://github.com/paritytech/polkadot-sdk/pull/1781) (MBM).
        /// For more information on the lifecycle of the block and its hooks, see the [Substrate
        /// documentation](https://paritytech.github.io/polkadot-sdk/master/frame_support/traits/trait.Hooks.html#method.on_poll).
        fn on_poll(_n: BlockNumberFor<T>, weight: &mut sp_weights::WeightMeter) {
            // TODO: Benchmark computational weight cost of this hook.

            // Update the current tick since we are executing the `on_poll` hook.
            let mut last_tick = OnPollTicker::<T>::get();
            last_tick.saturating_inc();
            OnPollTicker::<T>::set(last_tick);

            // Update the last chargeable info of Providers that have sent a valid proof in the previous tick
            Self::do_update_last_chargeable_info(last_tick, weight);

            // Update the current global price and the global price index of the system
            Self::do_update_current_price_per_unit_per_tick(weight);
            Self::do_update_price_index(weight);
        }
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
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was executed by the root origin
            ensure_root(origin)?;

            // Execute checks and logic, update storage
            Self::do_create_dynamic_rate_payment_stream(
                &provider_id,
                &user_account,
                amount_provided,
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
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was executed by the root origin
            ensure_root(origin)?;

            // Execute checks and logic, update storage
            Self::do_update_dynamic_rate_payment_stream(
                &provider_id,
                &user_account,
                new_amount_provided,
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
        ///    2. Get the difference between the last charged tick number and the last chargeable tick number of the stream
        ///    3. Calculate the amount to charge doing `rate * difference`
        ///    4. Charge the user (if the user does not have enough funds, it gets flagged and a `UserWithoutFunds` event is emitted)
        ///    5. Update the last charged tick number of the payment stream
        /// 4. If there is a dynamic-rate payment stream:
        ///    1. Get the amount provided by the Provider   
        ///    2. Get the difference between price index when the stream was last charged and the price index at the last chargeable tick
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
                <T::ProvidersPallet as ReadProvidersInterface>::get_provider_id(provider_account)
                    .ok_or(Error::<T>::NotAProvider)?;

            // Execute checks and logic, update storage
            let (amount_charged, last_tick_charged) =
                Self::do_charge_payment_streams(&provider_id, &user_account)?;

            // Get the last tick to add it to the event
            let charged_at_tick = Self::get_current_tick();

            // Emit the corresponding event (we always emit it even if the charged amount was 0)
            Self::deposit_event(Event::<T>::PaymentStreamCharged {
                user_account,
                provider_id: provider_id,
                amount: amount_charged,
                last_tick_charged,
                charged_at_tick,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows a user flagged as without funds to pay all remaining payment streams to be able to recover
        /// its deposits.
        ///
        /// The dispatch origin for this call must be Signed.
        /// The origin must be the User that has been flagged as without funds.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the user has been flagged as without funds.
        /// 3. Release the user's funds that were held as a deposit for each payment stream.
        /// 4. Get all payment streams of the user and charge them, paying the Providers for the services.
        /// 5. Delete all payment streams of the user.
        ///
        /// Emits a 'UserPaidDebts' event when successful.
        ///
        /// Notes: this extrinsic iterates over all payment streams of the user and charges them, so it can be expensive in terms of weight.
        /// The fee to execute it should be high enough to compensate for the weight of the extrinsic, without being too high that the user
        /// finds more convenient to wait for Providers to get its deposits one by one instead.
        #[pallet::call_index(7)]
        #[pallet::weight(Weight::from_parts(100_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
        pub fn pay_outstanding_debt(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer
            let user_account = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            Self::do_pay_outstanding_debt(&user_account)?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::UserPaidDebts { who: user_account });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows a user flagged as without funds long ago enough to clear this flag from its account,
        /// allowing it to begin contracting and paying for services again. If there's any outstanding debt, it will be charged and cleared.
        ///
        /// The dispatch origin for this call must be Signed.
        /// The origin must be the User that has been flagged as without funds.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the user has been flagged as without funds.
        /// 3. Check that the cooldown period has passed since the user was flagged as without funds.
        /// 4. Check if there's any outstanding debt and charge it. This is done by:
        ///   a. Releasing any remaining funds held as a deposit for each payment stream.
        ///   b. Getting all payment streams of the user and charging them, paying the Providers for the services.
        ///   c. Returning the User any remaining funds.
        ///   d. Deleting all payment streams of the user.
        /// 5. Unflag the user as without funds.
        ///
        /// Emits a 'UserSolvent' event when successful.
        ///
        /// Notes: this extrinsic iterates over all remaining payment streams of the user and charges them, so it can be expensive in terms of weight.
        /// The fee to execute it should be high enough to compensate for the weight of the extrinsic, without being too high that the user
        /// finds more convenient to wait for Providers to get its deposits one by one instead.
        #[pallet::call_index(8)]
        #[pallet::weight(Weight::from_parts(100_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
        pub fn clear_insolvent_flag(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer
            let user_account = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            Self::do_clear_insolvent_flag(&user_account)?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::UserSolvent { who: user_account });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }
    }
}

/// Helper functions (getters, setters, etc.) for this pallet
impl<T: Config> Pallet<T> {
    /// A helper function to get the information of the last chargeable tick and price index of a Provider
    pub fn get_last_chargeable_info(
        provider_id: &ProviderIdFor<T>,
    ) -> ProviderLastChargeableInfo<T> {
        LastChargeableInfo::<T>::get(provider_id)
    }

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

    /// A helper function to get the current Tick of the system
    pub fn get_current_tick() -> BlockNumberFor<T> {
        OnPollTicker::<T>::get()
    }
}
