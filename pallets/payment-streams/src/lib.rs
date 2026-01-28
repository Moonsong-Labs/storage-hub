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
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use frame_support::StorageDoubleMap;
use frame_system::pallet_prelude::BlockNumberFor;
pub use pallet::*;
use scale_info::prelude::vec::Vec;
pub use scale_info::Type;
use types::*;

#[frame_support::pallet]
pub mod pallet {
    use super::{types::*, weights::WeightInfo, Vec};
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

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: crate::weights::WeightInfo;

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

        /// The trait exposing the logic to calculate how much of the charged funds must go to the treasury.
        type TreasuryCutCalculator: shp_traits::TreasuryCutCalculator<
            Balance = BalanceOf<Self>,
            ProvidedUnit = Self::Units,
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

        /// The base deposit for a new payment stream. The actual deposit will be this constant + the deposit calculated using the `NewStreamDeposit` constant.
        #[pallet::constant]
        type BaseDeposit: Get<BalanceOf<Self>>;

        /// The number of ticks that correspond to the deposit that a User has to pay to open a payment stream.
        /// This means that, from the balance of the User for which the payment stream is being created, the amount
        /// `NewStreamDeposit * rate + BaseDeposit` will be held as a deposit.
        /// In the case of dynamic-rate payment streams, `rate` will be `amount_provided_in_giga_units * price_per_giga_unit_per_tick`, where `price_per_giga_unit_per_tick` is
        /// obtained from the `CurrentPricePerGigaUnitPerTick` storage.
        #[pallet::constant]
        type NewStreamDeposit: Get<BlockNumberFor<Self>>;

        /// The number of ticks that a user will have to wait after it has been flagged as without funds to be able to clear that flag
        /// and be able to pay for services again. If there's any outstanding debt when the flag is cleared, it will be paid.
        #[pallet::constant]
        type UserWithoutFundsCooldown: Get<BlockNumberFor<Self>>;

        /// The treasury account of the runtime, where a fraction of each payment goes.
        #[pallet::constant]
        type TreasuryAccount: Get<Self::AccountId>;

        /// The maximum amount of Users that a Provider can charge in a single extrinsic execution.
        /// This is used to prevent a Provider from charging too many Users in a single block, which could lead to a DoS attack.
        #[pallet::constant]
        type MaxUsersToCharge: Get<u32>;
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
    /// - [create_fixed_rate_payment_stream](crate::dispatchables::create_fixed_rate_payment_stream), which adds a new entry to the map.
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
    /// - [create_dynamic_rate_payment_stream](crate::dispatchables::create_dynamic_rate_payment_stream), which adds a new entry to the map.
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

    /// The last tick that was processed by this pallet from the Proof Submitters interface.
    ///
    /// This is used to keep track of the last tick processed by this pallet from the pallet that implements the from the ProvidersProofSubmitters interface.
    /// This is done to know the last tick for which this pallet has registered the Providers that submitted a valid proof and updated their last chargeable info.
    /// In the next `on_poll` hook execution, this pallet will update the last chargeable info of the Providers that submitted a valid proof in the tick that
    /// follows the one saved in this storage element.
    #[pallet::storage]
    pub type LastSubmittersTickRegistered<T: Config> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    /// The mapping from a user to if it has been flagged for not having enough funds to pay for its requested services.
    ///
    /// This is used to flag users that do not have enough funds to pay for their requested services, so other Providers
    /// can stop providing services to them.
    ///
    /// This storage is updated in:
    /// - [charge_payment_streams](crate::dispatchables::charge_payment_streams), which emits a `UserWithoutFunds` event and sets the user's entry in this map
    /// to that moment's tick number if it does not have enough funds.
    /// - [clear_insolvent_flag](crate::utils::clear_insolvent_flag), which clears the user's entry in this map if the cooldown period has passed and the user has paid all its outstanding debt.
    #[pallet::storage]
    pub type UsersWithoutFunds<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberFor<T>>;

    /// The mapping from a user to if it has been registered to the network and the amount of payment streams it has.
    ///
    /// Since users have to provide a deposit to be able to open each payment stream, this is used to keep track of the amount of payment streams
    /// that a user has and it is also useful to check if a user has registered to the network.
    ///
    /// This storage is updated in:
    /// - [create_fixed_rate_payment_stream](crate::dispatchables::create_fixed_rate_payment_stream), which holds the deposit of the user and adds one to this storage.
    /// - [create_dynamic_rate_payment_stream](crate::dispatchables::create_dynamic_rate_payment_stream), which holds the deposit of the user and adds one to this storage.
    /// - [remove_fixed_rate_payment_stream](crate::dispatchables::remove_fixed_rate_payment_stream), which removes one from this storage and releases the deposit.
    /// - [remove_dynamic_rate_payment_stream](crate::dispatchables::remove_dynamic_rate_payment_stream), which removes one from this storage and releases the deposit.
    #[pallet::storage]
    pub type RegisteredUsers<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

    /// The current price per gigaunit per tick of the provided service, used to calculate the amount to charge for dynamic-rate payment streams.
    ///
    /// This can be updated each tick by the system manager.
    ///
    /// It is in giga-units to allow for a more granular price per unit considering the limitations in decimal places that the Balance type might have.
    #[pallet::storage]
    pub type CurrentPricePerGigaUnitPerTick<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

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

    /// Mapping of Privileged Providers.
    ///
    /// Privileged Providers are those who are allowed to charge up to the current tick in
    /// fixed rate payment streams, regardless of their [`LastChargeableInfo`].
    #[pallet::storage]
    pub type PrivilegedProviders<T: Config> = StorageMap<_, Blake2_128Concat, ProviderIdFor<T>, ()>;

    // Genesis config:

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub current_price: BalanceOf<T>,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            let current_price = One::one();

            CurrentPricePerGigaUnitPerTick::<T>::put(current_price);

            Self { current_price }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            CurrentPricePerGigaUnitPerTick::<T>::put(self.current_price);
        }
    }

    // Events & Errors:

    /// # Event Encoding/Decoding Stability
    ///
    /// All event variants use explicit `#[codec(index = N)]` to ensure stable SCALE encoding/decoding
    /// across runtime upgrades.
    ///
    /// These indices must NEVER be changed or reused. Any breaking changes to errors must be
    /// introduced as new variants (append-only) to ensure backward and forward compatibility.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event emitted when a fixed-rate payment stream is created. Provides information about the Provider and User of the stream
        /// and its initial rate.
        #[codec(index = 0)]
        FixedRatePaymentStreamCreated {
            user_account: T::AccountId,
            provider_id: ProviderIdFor<T>,
            rate: BalanceOf<T>,
        },
        /// Event emitted when a fixed-rate payment stream is updated. Provides information about the User and Provider of the stream
        /// and the new rate of the stream.
        #[codec(index = 1)]
        FixedRatePaymentStreamUpdated {
            user_account: T::AccountId,
            provider_id: ProviderIdFor<T>,
            new_rate: BalanceOf<T>,
        },
        /// Event emitted when a fixed-rate payment stream is removed. Provides information about the User and Provider of the stream.
        #[codec(index = 2)]
        FixedRatePaymentStreamDeleted {
            user_account: T::AccountId,
            provider_id: ProviderIdFor<T>,
        },
        /// Event emitted when a dynamic-rate payment stream is created. Provides information about the User and Provider of the stream
        /// and the initial amount provided.
        #[codec(index = 3)]
        DynamicRatePaymentStreamCreated {
            user_account: T::AccountId,
            provider_id: ProviderIdFor<T>,
            amount_provided: UnitsProvidedFor<T>,
        },
        /// Event emitted when a dynamic-rate payment stream is updated. Provides information about the User and Provider of the stream
        /// and the new amount provided.
        #[codec(index = 4)]
        DynamicRatePaymentStreamUpdated {
            user_account: T::AccountId,
            provider_id: ProviderIdFor<T>,
            new_amount_provided: UnitsProvidedFor<T>,
        },
        /// Event emitted when a dynamic-rate payment stream is removed. Provides information about the User and Provider of the stream.
        #[codec(index = 5)]
        DynamicRatePaymentStreamDeleted {
            user_account: T::AccountId,
            provider_id: ProviderIdFor<T>,
        },
        /// Event emitted when a payment is charged. Provides information about the user that was charged,
        /// the Provider that received the funds, the tick up to which it was charged and the amount that was charged.
        #[codec(index = 6)]
        PaymentStreamCharged {
            user_account: T::AccountId,
            provider_id: ProviderIdFor<T>,
            amount: BalanceOf<T>,
            last_tick_charged: BlockNumberFor<T>,
            charged_at_tick: BlockNumberFor<T>,
        },
        /// Event emitted when multiple payment streams have been charged from a Provider. Provides information about
        /// the charged users, the Provider that received the funds and the tick when the charge happened.
        #[codec(index = 7)]
        UsersCharged {
            user_accounts: BoundedVec<T::AccountId, T::MaxUsersToCharge>,
            provider_id: ProviderIdFor<T>,
            charged_at_tick: BlockNumberFor<T>,
        },
        /// Event emitted when a Provider's last chargeable tick and price index are updated. Provides information about the Provider of the stream,
        /// the tick number of the last chargeable tick and the price index at that tick.
        #[codec(index = 8)]
        LastChargeableInfoUpdated {
            provider_id: ProviderIdFor<T>,
            last_chargeable_tick: BlockNumberFor<T>,
            last_chargeable_price_index: BalanceOf<T>,
        },
        /// Event emitted when a Provider is correctly trying to charge a User and that User does not have enough funds to pay for their services.
        /// This event is emitted to flag the user and let the network know that the user is not paying for the requested services, so other Providers can
        /// stop providing services to that user.
        #[codec(index = 9)]
        UserWithoutFunds { who: T::AccountId },
        /// Event emitted when a User that has been flagged as not having enough funds to pay for their contracted services has paid all its outstanding debt.
        #[codec(index = 10)]
        UserPaidAllDebts { who: T::AccountId },
        /// Event emitted when a User that has been flagged as not having enough funds to pay for their contracted services has paid some (but not all) of its outstanding debt.
        #[codec(index = 11)]
        UserPaidSomeDebts { who: T::AccountId },
        /// Event emitted when a User that has been flagged as not having enough funds to pay for their contracted services has waited the cooldown period,
        /// correctly paid all their outstanding debt and can now contract new services again.
        #[codec(index = 12)]
        UserSolvent { who: T::AccountId },
        /// Event emitted when the `on_poll` hook detects that the tick of the proof submitters that needs to process is not the one immediately after the last processed tick.
        #[codec(index = 13)]
        InconsistentTickProcessing {
            last_processed_tick: BlockNumberFor<T>,
            tick_to_process: BlockNumberFor<T>,
        },
    }

    /// # Error Encoding/Decoding Stability
    ///
    /// All error variants use explicit `#[codec(index = N)]` to ensure stable SCALE encoding/decoding
    /// across runtime upgrades.
    ///
    /// These indices must NEVER be changed or reused. Any breaking changes to errors must be
    /// introduced as new variants (append-only) to ensure backward and forward compatibility.
    #[pallet::error]
    pub enum Error<T> {
        /// Error thrown when a user of this pallet tries to add a payment stream that already exists.
        #[codec(index = 0)]
        PaymentStreamAlreadyExists,
        /// Error thrown when a user of this pallet tries to update, remove or charge a payment stream that does not exist.
        #[codec(index = 1)]
        PaymentStreamNotFound,
        /// Error thrown when a user tries to charge a payment stream and it's not a registered Provider
        #[codec(index = 2)]
        NotAProvider,
        /// Error thrown when failing to get the payment account of a registered Provider
        #[codec(index = 3)]
        ProviderInconsistencyError,
        /// Error thrown when the system can't hold funds from the User as a deposit for creating a new payment stream
        #[codec(index = 4)]
        CannotHoldDeposit,
        /// Error thrown when trying to update the rate of a fixed-rate payment stream to the same rate as before
        #[codec(index = 5)]
        UpdateRateToSameRate,
        /// Error thrown when trying to update the amount provided of a dynamic-rate payment stream to the same amount as before
        #[codec(index = 6)]
        UpdateAmountToSameAmount,
        /// Error thrown when trying to create a new fixed-rate payment stream with rate 0 or update the rate of an existing one to 0 (should use remove_fixed_rate_payment_stream instead)
        #[codec(index = 7)]
        RateCantBeZero,
        /// Error thrown when trying to create a new dynamic-rate payment stream with amount provided 0 or update the amount provided of an existing one to 0 (should use remove_dynamic_rate_payment_stream instead)
        #[codec(index = 8)]
        AmountProvidedCantBeZero,
        /// Error thrown when the tick number of when the payment stream was last charged is greater than the tick number of the last chargeable tick
        #[codec(index = 9)]
        LastChargedGreaterThanLastChargeable,
        /// Error thrown when the new last chargeable tick number that is trying to be set is greater than the current tick number or smaller than the previous last chargeable tick number
        #[codec(index = 10)]
        InvalidLastChargeableBlockNumber,
        /// Error thrown when the new last chargeable price index that is trying to be set is greater than the current price index or smaller than the previous last chargeable price index
        #[codec(index = 11)]
        InvalidLastChargeablePriceIndex,
        /// Error thrown when charging a payment stream would result in an overflow of the balance type
        #[codec(index = 12)]
        ChargeOverflow,
        /// Error thrown when trying to operate when the User has been flagged for not having enough funds.
        #[codec(index = 13)]
        UserWithoutFunds,
        /// Error thrown when a user that has not been flagged as without funds tries to use the extrinsic to pay its outstanding debt
        #[codec(index = 14)]
        UserNotFlaggedAsWithoutFunds,
        /// Error thrown when a user tries to clear the flag of being without funds before the cooldown period has passed
        #[codec(index = 15)]
        CooldownPeriodNotPassed,
        /// Error thrown when a user tries to clear the flag of being without funds before paying all its remaining debt
        #[codec(index = 16)]
        UserHasRemainingDebt,
        /// Error thrown when a charge is attempted when the provider is marked as insolvent
        #[codec(index = 17)]
        ProviderInsolvent,
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
        fn on_poll(_n: BlockNumberFor<T>, meter: &mut sp_weights::WeightMeter) {
            // Update the current tick since we are executing the `on_poll` hook
            let (previous_tick, _new_tick) = Self::do_advance_tick(meter);

            // Update the last chargeable info of Providers that have sent a valid proof in the previous tick
            Self::do_update_last_chargeable_info(previous_tick, meter);

            // Update the global price index of the system
            Self::do_update_price_index(meter);
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
        #[pallet::weight(T::WeightInfo::create_fixed_rate_payment_stream())]
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
            Self::deposit_event(Event::FixedRatePaymentStreamCreated {
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
        #[pallet::weight(T::WeightInfo::update_fixed_rate_payment_stream())]
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
            Self::deposit_event(Event::FixedRatePaymentStreamUpdated {
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
        #[pallet::weight(T::WeightInfo::delete_fixed_rate_payment_stream())]
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
            Self::deposit_event(Event::FixedRatePaymentStreamDeleted {
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
        #[pallet::weight(T::WeightInfo::create_dynamic_rate_payment_stream())]
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
            Self::deposit_event(Event::DynamicRatePaymentStreamCreated {
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
        #[pallet::weight(T::WeightInfo::update_dynamic_rate_payment_stream())]
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
            Self::deposit_event(Event::DynamicRatePaymentStreamUpdated {
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
        #[pallet::weight(T::WeightInfo::delete_dynamic_rate_payment_stream())]
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
            Self::deposit_event(Event::DynamicRatePaymentStreamDeleted {
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
        #[pallet::weight(T::WeightInfo::charge_payment_streams())]
        pub fn charge_payment_streams(
            origin: OriginFor<T>,
            user_account: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer
            let provider_account = ensure_signed(origin)?;

            // Get the Provider ID of the signer
            let provider_id =
                <T::ProvidersPallet as ReadProvidersInterface>::get_provider_id(&provider_account)
                    .ok_or(Error::<T>::NotAProvider)?;

            // Execute checks and logic, update storage
            let (amount_charged, last_tick_charged) =
                Self::do_charge_payment_streams(&provider_id, &user_account)?;

            // Get the last tick to add it to the event
            let charged_at_tick = Self::get_current_tick();

            // Emit the corresponding event (we always emit it even if the charged amount was 0)
            Self::deposit_event(Event::PaymentStreamCharged {
                user_account,
                provider_id: provider_id,
                amount: amount_charged,
                last_tick_charged,
                charged_at_tick,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows Providers to charge multiple User's payment streams.
        ///
        /// The dispatch origin for this call must be Signed.
        /// The origin must be the Provider that has at least one type of payment stream with each of the Users.
        ///
        /// Parameters:
        /// - `user_accounts`: The array of User Account IDs that have payment streams with the Provider.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the array of Users is not bigger than the maximum allowed.
        /// 3. Execute a for loop for each User in the array of User Account IDs, in which it:
        /// 	a. Checks that a payment stream between the signer (Provider) and the User exists
        /// 	b. If there is a fixed-rate payment stream:
        ///    		1. Get the rate of the payment stream
        ///    		2. Get the difference between the last charged tick number and the last chargeable tick number of the stream
        ///    		3. Calculate the amount to charge doing `rate * difference`
        ///    		4. Charge the user (if the user does not have enough funds, it gets flagged and a `UserWithoutFunds` event is emitted)
        ///    		5. Update the last charged tick number of the payment stream
        /// 	c. If there is a dynamic-rate payment stream:
        ///    		1. Get the amount provided by the Provider   
        ///    		2. Get the difference between price index when the stream was last charged and the price index at the last chargeable tick
        ///    		3. Calculate the amount to charge doing `amount_provided * difference`
        ///    		4. Charge the user (if the user does not have enough funds, it gets flagged and a `UserWithoutFunds` event is emitted)
        ///    		5. Update the price index when the stream was last charged of the payment stream
        ///
        /// Emits a `PaymentStreamCharged` per User that had to pay and a `UsersCharged` event when successful.
        ///
        /// Notes: a Provider could have both a fixed-rate and a dynamic-rate payment stream with a User. If that's the case, this extrinsic
        /// will try to charge both and the amount charged will be the sum of the amounts charged for each payment stream.
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::charge_multiple_users_payment_streams(user_accounts.len() as u32))]
        pub fn charge_multiple_users_payment_streams(
            origin: OriginFor<T>,
            user_accounts: BoundedVec<T::AccountId, T::MaxUsersToCharge>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer
            let provider_account = ensure_signed(origin)?;

            // Get the Provider ID of the signer
            let provider_id =
                <T::ProvidersPallet as ReadProvidersInterface>::get_provider_id(&provider_account)
                    .ok_or(Error::<T>::NotAProvider)?;

            // Execute checks and logic, update storage
            Self::do_charge_multiple_users_payment_streams(&provider_id, &user_accounts)?;

            // Get the last tick to add it to the event
            let charged_at_tick = Self::get_current_tick();

            // Emit the corresponding event (we always emit it even if the charged amount was 0)
            Self::deposit_event(Event::UsersCharged {
                user_accounts,
                provider_id,
                charged_at_tick,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows a user flagged as without funds to pay the Providers that still have payment streams
        /// with it, in order to recover as much of its deposits as possible.
        ///
        /// The dispatch origin for this call must be Signed.
        /// The origin must be the User that has been flagged as without funds.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the user has been flagged as without funds.
        /// 3. Release the user's funds that were held as a deposit for each payment stream to be paid.
        /// 4. Get the payment streams that the user has with the provided list of Providers, and pay them for the services.
        /// 5. Delete the charged payment streams of the user.
        ///
        /// Emits a 'UserPaidSomeDebts' event when successful if the user has remaining debts. If the user has successfully paid all its debts,
        /// it emits a 'UserPaidAllDebts' event.
        ///
        /// Notes: this extrinsic iterates over the provided list of Providers, getting the payment streams they have with the user and charging
        /// them, so the execution could get expensive. It's recommended to provide a list of Providers that the user actually has payment streams with,
        /// which can be obtained by calling the `get_providers_with_payment_streams_with_user` runtime API.
        /// There was an idea to limit the amount of Providers that can be received by this extrinsic using a constant in the configuration of this pallet,
        /// but the correct benchmarking of this extrinsic should be enough to avoid any potential abuse.
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::pay_outstanding_debt(providers.len().try_into().unwrap_or(u32::MAX)))]
        pub fn pay_outstanding_debt(
            origin: OriginFor<T>,
            providers: Vec<ProviderIdFor<T>>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer
            let user_account = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            let fully_paid = Self::do_pay_outstanding_debt(&user_account, providers)?;

            // Emit the corresponding event
            if fully_paid {
                Self::deposit_event(Event::UserPaidAllDebts { who: user_account });
            } else {
                Self::deposit_event(Event::UserPaidSomeDebts { who: user_account });
            }

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows a user flagged as without funds long ago enough to clear this flag from its account,
        /// allowing it to begin contracting and paying for services again. It should have previously paid all its outstanding debt.
        ///
        /// The dispatch origin for this call must be Signed.
        /// The origin must be the User that has been flagged as without funds.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the user has been flagged as without funds.
        /// 3. Check that the cooldown period has passed since the user was flagged as without funds.
        /// 4. Check that there's no remaining outstanding debt.
        /// 5. Unflag the user as without funds.
        ///
        /// Emits a 'UserSolvent' event when successful.
        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::clear_insolvent_flag())]
        pub fn clear_insolvent_flag(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer
            let user_account = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            Self::do_clear_insolvent_flag(&user_account)?;

            // Emit the corresponding event
            Self::deposit_event(Event::UserSolvent { who: user_account });

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

    /// A helper function to check if a provider has at least 1 payment stream with any user
    pub fn provider_has_payment_streams(provider_id: &ProviderIdFor<T>) -> bool {
        FixedRatePaymentStreams::<T>::contains_prefix(provider_id)
            || DynamicRatePaymentStreams::<T>::contains_prefix(provider_id)
    }

    /// A helper function that gets all fixed-rate payment streams of a Provider
    ///
    /// WARNING: Do not use this function unless you are sure of the amount of payment streams that the Provider has.
    /// Calling this during block execution could potentially result in a big unbounded weight consumption. This is meant
    /// to be used in a runtime API.
    pub fn get_fixed_rate_payment_streams_of_provider(
        provider_id: &ProviderIdFor<T>,
    ) -> Vec<(T::AccountId, FixedRatePaymentStream<T>)> {
        FixedRatePaymentStreams::<T>::iter_prefix(provider_id).collect()
    }

    /// A helper function that gets all dynamic-rate payment streams of a Provider
    ///
    /// WARNING: Do not use this function unless you are sure of the amount of payment streams that the Provider has.
    /// Calling this during block execution could potentially result in a big unbounded weight consumption. This is meant
    /// to be used in a runtime API.
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

    /// A helper function to get the current price per unit per tick of the system
    pub fn get_current_price_per_giga_unit_per_tick() -> BalanceOf<T> {
        CurrentPricePerGigaUnitPerTick::<T>::get()
    }

    /// A helper function to get the accumulated price index of the system
    pub fn get_accumulated_price_index() -> BalanceOf<T> {
        AccumulatedPriceIndex::<T>::get()
    }
}
