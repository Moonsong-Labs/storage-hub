use frame_support::ensure;
use frame_support::pallet_prelude::DispatchResult;
use frame_support::sp_runtime::{
    traits::{CheckedAdd, CheckedMul, CheckedSub, Zero},
    ArithmeticError, BoundedVec, DispatchError,
};
use frame_support::traits::{
    fungible::{Inspect, InspectHold, Mutate, MutateHold},
    tokens::{Fortitude, Precision, Preservation},
    Get,
};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_payment_streams_runtime_api::GetUsersWithDebtOverThresholdError;
use shp_constants::GIGAUNIT;
use shp_traits::{
    PaymentStreamsInterface, PricePerGigaUnitPerTickInterface, ProofSubmittersInterface,
    ReadProvidersInterface, ReadUserSolvencyInterface, SystemMetricsInterface,
    TreasuryCutCalculator,
};
use sp_runtime::{
    traits::{CheckedDiv, Convert, One},
    Saturating,
};

use crate::{weights::WeightInfo, *};

macro_rules! expect_or_err {
    // Handle Option type
    ($optional:expr, $error_msg:expr, $error_type:path) => {{
        match $optional {
            Some(value) => value,
            None => {
                #[cfg(test)]
                unreachable!($error_msg);

                #[allow(unreachable_code)]
                {
                    Err($error_type)?
                }
            }
        }
    }};
    // Handle boolean type
    ($condition:expr, $error_msg:expr, $error_type:path, bool) => {{
        if !$condition {
            #[cfg(test)]
            unreachable!($error_msg);

            #[allow(unreachable_code)]
            {
                Err($error_type)?
            }
        }
    }};
}

impl<T> Pallet<T>
where
    T: pallet::Config,
{
    /// This function holds the logic that checks if a fixed-rate payment stream can be created and, if so, stores the payment
    /// stream in the FixedRatePaymentStreams mapping and holds the deposit from the User.
    pub fn do_create_fixed_rate_payment_stream(
        provider_id: &ProviderIdFor<T>,
        user_account: &T::AccountId,
        rate: BalanceOf<T>,
    ) -> DispatchResult {
        // Check that the given ID belongs to an actual Provider
        ensure!(
            <T::ProvidersPallet as ReadProvidersInterface>::is_provider(*provider_id),
            Error::<T>::NotAProvider
        );

        // We do not allow creating a payment stream if the provider is insolvent
        if <T::ProvidersPallet as ReadProvidersInterface>::is_provider_insolvent(*provider_id) {
            return Err(Error::<T>::ProviderInsolvent.into());
        }

        // Check that the given rate is not 0
        ensure!(rate != Zero::zero(), Error::<T>::RateCantBeZero);

        // Check that a fixed-rate payment stream between that Provider and User does not exist yet
        ensure!(
            !FixedRatePaymentStreams::<T>::contains_key(provider_id, user_account),
            Error::<T>::PaymentStreamAlreadyExists
        );

        // Check that the User is not flagged as without funds
        ensure!(
            !UsersWithoutFunds::<T>::contains_key(user_account),
            Error::<T>::UserWithoutFunds
        );

        // Check that the user has enough balance to pay the deposit
        let user_balance = T::NativeBalance::reducible_balance(
            &user_account,
            Preservation::Preserve,
            Fortitude::Polite,
        );
        let deposit = rate
            .checked_mul(&T::BlockNumberToBalance::convert(T::NewStreamDeposit::get()))
            .ok_or(ArithmeticError::Overflow)?
            .checked_add(&T::BaseDeposit::get())
            .ok_or(ArithmeticError::Overflow)?;
        ensure!(user_balance >= deposit, Error::<T>::CannotHoldDeposit);

        // Check if we can hold the deposit from the user
        ensure!(
            T::NativeBalance::can_hold(
                &HoldReason::PaymentStreamDeposit.into(),
                &user_account,
                deposit
            ),
            Error::<T>::CannotHoldDeposit
        );

        // Check if adding one to the user's payment streams count would overflow
        ensure!(
            RegisteredUsers::<T>::get(user_account)
                .checked_add(1)
                .is_some(),
            ArithmeticError::Overflow
        );

        // Hold the deposit from the user
        T::NativeBalance::hold(
            &HoldReason::PaymentStreamDeposit.into(),
            &user_account,
            deposit,
        )?;

        // Add one to the user's payment streams count
        RegisteredUsers::<T>::mutate(user_account, |user_payment_streams_count| {
            *user_payment_streams_count = user_payment_streams_count
                .checked_add(1)
                .ok_or(ArithmeticError::Overflow)?;
            Ok::<(), DispatchError>(())
        })?;

        // Store the new fixed-rate payment stream in the FixedRatePaymentStreams mapping
        // We initiate the `last_charged_tick` and `last_chargeable_tick` with the current tick number to be able to keep track of the
        // time passed since the payment stream was originally created
        FixedRatePaymentStreams::<T>::insert(
            provider_id,
            user_account,
            FixedRatePaymentStream {
                rate,
                last_charged_tick: OnPollTicker::<T>::get(),
                user_deposit: deposit,
                out_of_funds_tick: None,
            },
        );

        Ok(())
    }

    /// This function holds the logic that checks if a fixed-rate payment stream can be updated and, if so, updates it in the FixedRatePaymentStreams mapping.
    pub fn do_update_fixed_rate_payment_stream(
        provider_id: &ProviderIdFor<T>,
        user_account: &T::AccountId,
        new_rate: BalanceOf<T>,
    ) -> DispatchResult {
        // Check that the given ID belongs to an actual Provider
        ensure!(
            <T::ProvidersPallet as ReadProvidersInterface>::is_provider(*provider_id),
            Error::<T>::NotAProvider
        );

        // Ensure that the new rate is not 0 (should use remove_fixed_rate_payment_stream instead)
        ensure!(new_rate != Zero::zero(), Error::<T>::RateCantBeZero);

        // Check that a fixed-rate payment stream between the Provider and User exists
        ensure!(
            FixedRatePaymentStreams::<T>::contains_key(provider_id, user_account),
            Error::<T>::PaymentStreamNotFound
        );

        // Get the information of the payment stream
        let payment_stream = FixedRatePaymentStreams::<T>::get(provider_id, user_account)
            .ok_or(Error::<T>::PaymentStreamNotFound)?;

        // Verify that the new rate is different from the current one
        ensure!(
            payment_stream.rate != new_rate,
            Error::<T>::UpdateRateToSameRate
        );

        // Check that the user is not flagged as without funds
        ensure!(
            !UsersWithoutFunds::<T>::contains_key(user_account),
            Error::<T>::UserWithoutFunds
        );

        // Charge the payment stream with the old rate before updating it to prevent abuse
        let (amount_charged, last_tick_charged) =
            Self::do_charge_payment_streams(&provider_id, user_account)?;
        if amount_charged > Zero::zero() {
            let charged_at_tick = Self::get_current_tick();

            // We emit a payment charged event only if the user had to pay before the payment stream could be updated
            Self::deposit_event(Event::PaymentStreamCharged {
                user_account: user_account.clone(),
                provider_id: *provider_id,
                amount: amount_charged,
                last_tick_charged,
                charged_at_tick,
            });
        }

        // Update the user's deposit based on the new rate
        let new_deposit = new_rate
            .checked_mul(&T::BlockNumberToBalance::convert(T::NewStreamDeposit::get()))
            .ok_or(ArithmeticError::Overflow)?
            .checked_add(&T::BaseDeposit::get())
            .ok_or(ArithmeticError::Overflow)?;
        Self::update_user_deposit(&user_account, payment_stream.user_deposit, new_deposit)?;

        // Update the payment stream in the FixedRatePaymentStreams mapping
        FixedRatePaymentStreams::<T>::mutate(provider_id, user_account, |payment_stream| {
            let payment_stream = expect_or_err!(
                payment_stream,
                "Payment stream should exist if it was found before.",
                Error::<T>::PaymentStreamNotFound
            );
            payment_stream.rate = new_rate;
            payment_stream.user_deposit = new_deposit;
            Ok::<(), DispatchError>(())
        })?;

        Ok(())
    }

    /// This function holds the logic that checks if a fixed-rate payment stream can be deleted and, if so, removes it from the FixedRatePaymentStreams mapping,
    /// decreases the user's payment streams count and releases the deposit of that payment stream.
    pub fn do_delete_fixed_rate_payment_stream(
        provider_id: &ProviderIdFor<T>,
        user_account: &T::AccountId,
    ) -> DispatchResult {
        // Check that the given ID belongs to an actual Provider
        ensure!(
            <T::ProvidersPallet as ReadProvidersInterface>::is_provider(*provider_id),
            Error::<T>::NotAProvider
        );

        // Check that a payment stream between that Provider and User exists
        ensure!(
            FixedRatePaymentStreams::<T>::contains_key(provider_id, user_account),
            Error::<T>::PaymentStreamNotFound
        );

        // Charge the payment stream before deletion to make sure the services provided by the Provider is paid in full for its duration
        // We only charge if the Provider is solvent
        if !<T::ProvidersPallet as ReadProvidersInterface>::is_provider_insolvent(*provider_id) {
            let (amount_charged, last_tick_charged) =
                Self::do_charge_payment_streams(&provider_id, user_account)?;
            if amount_charged > Zero::zero() {
                let charged_at_tick = Self::get_current_tick();

                // We emit a payment charged event only if the user had to pay before being able to delete the payment stream
                Self::deposit_event(Event::PaymentStreamCharged {
                    user_account: user_account.clone(),
                    provider_id: *provider_id,
                    amount: amount_charged,
                    last_tick_charged,
                    charged_at_tick,
                });
            }
        }

        // The payment stream may have been deleted when charged if the user was out of funds.
        // If that's not the case, we clear it here.
        if FixedRatePaymentStreams::<T>::get(provider_id, user_account).is_some() {
            // Release the deposit of this payment stream to the User
            let deposit = FixedRatePaymentStreams::<T>::get(provider_id, user_account)
                .ok_or(Error::<T>::PaymentStreamNotFound)?
                .user_deposit;
            T::NativeBalance::release(
                &HoldReason::PaymentStreamDeposit.into(),
                &user_account,
                deposit,
                Precision::Exact,
            )?;

            // Remove the payment stream from the FixedRatePaymentStreams mapping
            FixedRatePaymentStreams::<T>::remove(provider_id, user_account);

            // Decrease the user's payment streams count
            let mut user_payment_streams_count = RegisteredUsers::<T>::get(user_account);
            user_payment_streams_count = user_payment_streams_count
                .checked_sub(1)
                .ok_or(ArithmeticError::Underflow)?;
            RegisteredUsers::<T>::insert(user_account, user_payment_streams_count);
        }

        Ok(())
    }

    /// This function holds the logic that checks if a dynamic-rate payment stream can be created and, if so, stores the payment
    /// stream in the DynamicRatePaymentStreams mapping and holds the deposit from the User.
    pub fn do_create_dynamic_rate_payment_stream(
        provider_id: &ProviderIdFor<T>,
        user_account: &T::AccountId,
        amount_provided: UnitsProvidedFor<T>,
    ) -> DispatchResult {
        // Check that the given ID belongs to an actual Provider
        ensure!(
            <T::ProvidersPallet as ReadProvidersInterface>::is_provider(*provider_id),
            Error::<T>::NotAProvider
        );

        // We do not allow creating a payment stream if the provider is insolvent
        if <T::ProvidersPallet as ReadProvidersInterface>::is_provider_insolvent(*provider_id) {
            return Err(Error::<T>::ProviderInsolvent.into());
        }

        // Check that the given amount provided is not 0
        ensure!(
            amount_provided != Zero::zero(),
            Error::<T>::AmountProvidedCantBeZero
        );

        // Check that a dynamic-rate payment stream between that Provider and User does not exist yet
        ensure!(
            !DynamicRatePaymentStreams::<T>::contains_key(provider_id, user_account),
            Error::<T>::PaymentStreamAlreadyExists
        );

        // Check that the User is not flagged as without funds
        ensure!(
            !UsersWithoutFunds::<T>::contains_key(user_account),
            Error::<T>::UserWithoutFunds
        );

        // Check that the user has enough balance to pay the deposit
        // Deposit is: `(amount_provided * current_price_per_giga_unit_per_tick * NewStreamDeposit) / giga_units ` where:
        // - `amount_provided` is the amount of units of something (for example, storage) that are provided by the Provider to the User
        // - `current_price_per_giga_unit_per_tick` is the current price of a giga-unit of something (per tick)
        // - `NewStreamDeposit` is a runtime constant that represents the number of ticks that the deposit should cover
        // - `GIGAUNIT` is the number of units in a giga-unit (1024 * 1024 * 1024 = 1_073_741_824)
        // As an example, if the `current_price_per_giga_unit_per_tick` is 10000 and the `NewStreamDeposit` is 100 ticks,
        // the minimum amount provided would be 1074 units and the deposit would increase by 1 every 1074 units.
        let user_balance = T::NativeBalance::reducible_balance(
            &user_account,
            Preservation::Preserve,
            Fortitude::Polite,
        );
        let current_price_per_giga_unit_per_tick = CurrentPricePerGigaUnitPerTick::<T>::get();
        let deposit = current_price_per_giga_unit_per_tick
            .checked_mul(&amount_provided.into())
            .ok_or(ArithmeticError::Overflow)?
            .checked_mul(&T::BlockNumberToBalance::convert(T::NewStreamDeposit::get()))
            .ok_or(ArithmeticError::Overflow)?
            .checked_div(&GIGAUNIT.into())
            .unwrap_or_default()
            .checked_add(&T::BaseDeposit::get())
            .ok_or(ArithmeticError::Overflow)?;
        ensure!(user_balance >= deposit, Error::<T>::CannotHoldDeposit);

        // Check if we can hold the deposit from the User
        ensure!(
            T::NativeBalance::can_hold(
                &HoldReason::PaymentStreamDeposit.into(),
                &user_account,
                deposit
            ),
            Error::<T>::CannotHoldDeposit
        );

        // Check if adding one to the user's payment streams count would overflow
        // NOTE: We check this BEFORE holding the deposit, as for some weird reason the `hold` function does NOT revert when the extrinsic fails !?!?
        ensure!(
            RegisteredUsers::<T>::get(user_account)
                .checked_add(1)
                .is_some(),
            ArithmeticError::Overflow
        );

        // Hold the deposit from the User
        T::NativeBalance::hold(
            &HoldReason::PaymentStreamDeposit.into(),
            &user_account,
            deposit,
        )?;

        // Add one to the user's payment streams count
        RegisteredUsers::<T>::mutate(user_account, |user_payment_streams_count| {
            *user_payment_streams_count = user_payment_streams_count
                .checked_add(1)
                .ok_or(ArithmeticError::Overflow)?;
            Ok::<(), DispatchError>(())
        })?;

        // Store the new dynamic-rate payment stream in the DynamicRatePaymentStreams mapping
        // We initiate the `price_index_when_last_charged` with the current price index to be able to keep track of the
        // price changes since the payment stream was originally created.
        DynamicRatePaymentStreams::<T>::insert(
            provider_id,
            user_account,
            DynamicRatePaymentStream {
                amount_provided,
                price_index_when_last_charged: AccumulatedPriceIndex::<T>::get(),
                user_deposit: deposit,
                out_of_funds_tick: None,
            },
        );

        Ok(())
    }

    /// This function holds the logic that checks if a dynamic-rate payment stream can be updated and, if so, updates it in the DynamicRatePaymentStreams mapping.
    /// The function also takes care of releasing or holding the difference in deposit from the User depending on the new amount provided.
    pub fn do_update_dynamic_rate_payment_stream(
        provider_id: &ProviderIdFor<T>,
        user_account: &T::AccountId,
        new_amount_provided: UnitsProvidedFor<T>,
    ) -> DispatchResult {
        // Check that the given ID belongs to an actual Provider
        ensure!(
            <T::ProvidersPallet as ReadProvidersInterface>::is_provider(*provider_id),
            Error::<T>::NotAProvider
        );

        // Ensure that the new amount provided is not 0 (should use remove_dynamic_rate_payment_stream instead)
        ensure!(
            new_amount_provided != Zero::zero(),
            Error::<T>::AmountProvidedCantBeZero
        );

        // Check that a dynamic-rate payment stream between the Provider and User exists
        ensure!(
            DynamicRatePaymentStreams::<T>::contains_key(provider_id, user_account),
            Error::<T>::PaymentStreamNotFound
        );

        // Get the information of the payment stream
        let payment_stream = DynamicRatePaymentStreams::<T>::get(provider_id, user_account)
            .ok_or(Error::<T>::PaymentStreamNotFound)?;

        // Verify that the new amount provided is different from the current one
        ensure!(
            payment_stream.amount_provided != new_amount_provided,
            Error::<T>::UpdateAmountToSameAmount
        );

        // Check that the user is not flagged as without funds
        ensure!(
            !UsersWithoutFunds::<T>::contains_key(user_account),
            Error::<T>::UserWithoutFunds
        );

        // Charge the payment stream with the old amount before updating it to prevent abuse
        let (amount_charged, last_tick_charged) =
            Self::do_charge_payment_streams(&provider_id, user_account)?;
        if amount_charged > Zero::zero() {
            let charged_at_tick = Self::get_current_tick();

            // We emit a payment charged event only if the user had to pay before the payment stream could be updated
            Self::deposit_event(Event::PaymentStreamCharged {
                user_account: user_account.clone(),
                provider_id: *provider_id,
                amount: amount_charged,
                last_tick_charged,
                charged_at_tick,
            });
        }

        // Update the user's deposit based on the new amount provided
        let current_price_per_giga_unit_per_tick = CurrentPricePerGigaUnitPerTick::<T>::get();
        let new_deposit = new_amount_provided
            .into()
            .checked_mul(&current_price_per_giga_unit_per_tick)
            .ok_or(ArithmeticError::Overflow)?
            .checked_mul(&T::BlockNumberToBalance::convert(T::NewStreamDeposit::get()))
            .ok_or(ArithmeticError::Overflow)?
            .checked_div(&GIGAUNIT.into())
            .unwrap_or_default()
            .checked_add(&T::BaseDeposit::get())
            .ok_or(ArithmeticError::Overflow)?;
        Self::update_user_deposit(&user_account, payment_stream.user_deposit, new_deposit)?;

        // Update the payment stream in the DynamicRatePaymentStreams mapping
        DynamicRatePaymentStreams::<T>::mutate(provider_id, user_account, |payment_stream| {
            let payment_stream = expect_or_err!(
                payment_stream,
                "Payment stream should exist if it was found before.",
                Error::<T>::PaymentStreamNotFound
            );
            payment_stream.amount_provided = new_amount_provided;
            payment_stream.user_deposit = new_deposit;
            Ok::<(), DispatchError>(())
        })?;

        Ok(())
    }

    /// This function holds the logic that checks if a dynamic-rate payment stream can be deleted and, if so, removes it from the DynamicRatePaymentStreams mapping,
    /// decreases the user's payment streams count and releases the deposit of that payment stream.
    pub fn do_delete_dynamic_rate_payment_stream(
        provider_id: &ProviderIdFor<T>,
        user_account: &T::AccountId,
    ) -> DispatchResult {
        // Check that the given ID belongs to an actual Provider
        ensure!(
            <T::ProvidersPallet as ReadProvidersInterface>::is_provider(*provider_id),
            Error::<T>::NotAProvider
        );

        // Check that a dynamic-rate payment stream between that Provider and User exists
        ensure!(
            DynamicRatePaymentStreams::<T>::contains_key(provider_id, user_account),
            Error::<T>::PaymentStreamNotFound
        );

        // Charge the payment stream before deletion to make sure the services provided by the Provider is paid in full for its duration
        // We only charge if the Provider is solvent
        if !<T::ProvidersPallet as ReadProvidersInterface>::is_provider_insolvent(*provider_id) {
            let (amount_charged, last_tick_charged) =
                Self::do_charge_payment_streams(&provider_id, user_account)?;
            if amount_charged > Zero::zero() {
                let charged_at_tick = Self::get_current_tick();

                // We emit a payment charged event only if the user had to pay before being able to delete the payment stream
                Self::deposit_event(Event::PaymentStreamCharged {
                    user_account: user_account.clone(),
                    provider_id: *provider_id,
                    amount: amount_charged,
                    last_tick_charged,
                    charged_at_tick,
                });
            }
        }

        // The payment stream may have been deleted when charged if the user was out of funds.
        // If that's not the case, we clear it here.
        if DynamicRatePaymentStreams::<T>::get(provider_id, user_account).is_some() {
            // Release the deposit of this payment stream to the User
            let deposit = DynamicRatePaymentStreams::<T>::get(provider_id, user_account)
                .ok_or(Error::<T>::PaymentStreamNotFound)?
                .user_deposit;
            T::NativeBalance::release(
                &HoldReason::PaymentStreamDeposit.into(),
                &user_account,
                deposit,
                Precision::Exact,
            )?;

            // Remove the payment stream from the DynamicRatePaymentStreams mapping
            DynamicRatePaymentStreams::<T>::remove(provider_id, user_account);

            // Decrease the user's payment streams count
            let mut user_payment_streams_count = RegisteredUsers::<T>::get(user_account);
            user_payment_streams_count = user_payment_streams_count
                .checked_sub(1)
                .ok_or(ArithmeticError::Underflow)?;
            RegisteredUsers::<T>::insert(user_account, user_payment_streams_count);
        }

        Ok(())
    }

    /// This function holds the logic that checks if any payment stream exists between a Provider and a User and, if so,
    /// charges the payment stream/s from the User's balance.
    /// For fixed-rate payment streams, the charge is calculated as: `rate * time_passed` where `time_passed` is the time between the last chargeable tick and
    /// the last charged tick of this payment stream. As such, the last charged tick can't ever be greater than the last chargeable tick, and if they are equal then no charge is made.
    /// For dynamic-rate payment streams, the charge is calculated as: `amount_provided * (price_index_when_last_charged - price_index_at_last_chargeable_tick)`. In this case,
    /// the price index at the last charged tick can't ever be greater than the price index at the last chargeable tick, and if they are equal then no charge is made.
    pub fn do_charge_payment_streams(
        provider_id: &ProviderIdFor<T>,
        user_account: &T::AccountId,
    ) -> Result<(BalanceOf<T>, BlockNumberFor<T>), DispatchError> {
        // Check that the provider is not insolvent
        ensure!(
            !<T::ProvidersPallet as ReadProvidersInterface>::is_provider_insolvent(*provider_id),
            Error::<T>::ProviderInsolvent
        );

        // Check that the given ID belongs to an actual Provider
        ensure!(
            <T::ProvidersPallet as ReadProvidersInterface>::is_provider(*provider_id),
            Error::<T>::NotAProvider
        );

        // Check that a payment stream between that Provider and User exists
        ensure!(
            FixedRatePaymentStreams::<T>::contains_key(provider_id, user_account)
                || DynamicRatePaymentStreams::<T>::contains_key(provider_id, user_account),
            Error::<T>::PaymentStreamNotFound
        );

        // Get the information of the fixed-rate payment stream (if it exists)
        let fixed_rate_payment_stream =
            FixedRatePaymentStreams::<T>::get(provider_id, user_account);

        // Get the information of the dynamic-rate payment stream (if it exists)
        let dynamic_rate_payment_stream =
            DynamicRatePaymentStreams::<T>::get(provider_id, user_account);

        // Note: No need to check if the last chargeable tick/price index at last chargeable tick has been updated since the last charge,
        // as the only consequence of that is charging 0 to the user.
        // Not erroring out in this situation helps to be able to call this function without errors when updating or removing a payment stream.

        // Initiate the variable that will hold the total amount that has been charged
        let mut total_amount_charged: BalanceOf<T> = Zero::zero();

        // Get the last chargeable info for this provider
        let last_chargeable_info = Self::get_last_chargeable_info_with_privilege(provider_id);
        let last_chargeable_tick = last_chargeable_info.last_chargeable_tick;

        // If the fixed-rate payment stream exists:
        if let Some(fixed_rate_payment_stream) = fixed_rate_payment_stream {
            // Check if the user is flagged as without funds to execute the correct charging logic
            match UsersWithoutFunds::<T>::get(user_account) {
                Some(_) => {
                    // If the user has been flagged as without funds, manage it accordingly
                    Self::manage_user_without_funds(
                        &provider_id,
                        &user_account,
                        &PaymentStream::FixedRatePaymentStream(fixed_rate_payment_stream),
                    )?;
                }
                None => {
                    // If the user hasn't been flagged as without funds, charge the payment stream
                    // Calculate the time passed between the last chargeable tick and the last charged tick
                    if let Some(time_passed) = last_chargeable_tick
                        .checked_sub(&fixed_rate_payment_stream.last_charged_tick)
                    {
                        // Convert it to the balance type (for math)
                        let time_passed_balance_typed =
                            T::BlockNumberToBalance::convert(time_passed);

                        // Calculate the amount to charge
                        let amount_to_charge = fixed_rate_payment_stream
                            .rate
                            .checked_mul(&time_passed_balance_typed)
                            .ok_or(Error::<T>::ChargeOverflow)?;

                        // Check the free balance of the user
                        let user_balance = T::NativeBalance::reducible_balance(
                            &user_account,
                            Preservation::Preserve,
                            Fortitude::Polite,
                        );

                        // If the user does not have enough balance to pay for its storage:
                        if user_balance < amount_to_charge {
                            // Check if this payment stream was already flagged as without funds and, if so, how many ticks have passed since then
                            let out_of_funds_tick = fixed_rate_payment_stream.out_of_funds_tick;
                            let current_tick = OnPollTicker::<T>::get();
                            let ticks_since_out_of_funds = match out_of_funds_tick {
                                Some(tick) => current_tick.saturating_sub(tick),
                                None => {
                                    // If this payment stream wasn't flagged as without funds before, flag it now with the current tick
                                    FixedRatePaymentStreams::<T>::mutate(
                                        provider_id,
                                        user_account,
                                        |payment_stream| {
                                            let payment_stream = expect_or_err!(
                                            payment_stream,
                                            "Payment stream should exist if it was found before.",
                                            Error::<T>::PaymentStreamNotFound
                                        );
                                            payment_stream.out_of_funds_tick = Some(current_tick);
                                            Ok::<(), DispatchError>(())
                                        },
                                    )?;
                                    Zero::zero()
                                }
                            };

                            // If the user has missed payment for more than the allowed number of ticks (which is the number of ticks
                            // that the user deposited when it was created), consider it as without funds.
                            if ticks_since_out_of_funds >= T::NewStreamDeposit::get() {
                                Self::manage_user_without_funds(
                                    &provider_id,
                                    &user_account,
                                    &PaymentStream::FixedRatePaymentStream(
                                        fixed_rate_payment_stream,
                                    ),
                                )?;
                            }
                        } else {
                            // If the user does have enough funds to pay for its storage:

                            // Clear the out-of-funds flag from the payment stream
                            FixedRatePaymentStreams::<T>::mutate(
                                provider_id,
                                user_account,
                                |payment_stream| {
                                    let payment_stream = expect_or_err!(
                                        payment_stream,
                                        "Payment stream should exist if it was found before.",
                                        Error::<T>::PaymentStreamNotFound
                                    );
                                    payment_stream.out_of_funds_tick = None;
                                    Ok::<(), DispatchError>(())
                                },
                            )?;

                            // Get the payment account of the SP
                            let provider_payment_account = expect_or_err!(
                            	<T::ProvidersPallet as ReadProvidersInterface>::get_payment_account(
                                	*provider_id
                            	),
                            	"Provider should exist and have a payment account if its ID exists.",
                            	Error::<T>::ProviderInconsistencyError
                        	);

                            // Check if the total amount charged would overflow
                            ensure!(
                                total_amount_charged
                                    .checked_add(&amount_to_charge)
                                    .is_some(),
                                ArithmeticError::Overflow
                            );

                            // Get, from the total amount to charge, the cut for the treasury and the cut for the provider
                            let total_provided_amount =
                                <T::ProvidersPallet as SystemMetricsInterface>::get_total_capacity(
                                );
                            let used_provided_amount = <T::ProvidersPallet as SystemMetricsInterface>::get_total_used_capacity();
                            let treasury_cut = <T::TreasuryCutCalculator as TreasuryCutCalculator>::calculate_treasury_cut(total_provided_amount, used_provided_amount, amount_to_charge);
                            let provider_cut = amount_to_charge.saturating_sub(treasury_cut); // Treasury cut should always be less than the amount to charge, so this will never be 0.

                            // Charge the payment stream from the user's balance
                            T::NativeBalance::transfer(
                                user_account,
                                &provider_payment_account,
                                provider_cut,
                                Preservation::Preserve,
                            )?;

                            // Send the rest of the funds to the treasury
                            T::NativeBalance::transfer(
                                user_account,
                                &T::TreasuryAccount::get(),
                                treasury_cut,
                                Preservation::Preserve,
                            )?;

                            // Set the last charged tick to the tick number of the last chargeable tick
                            FixedRatePaymentStreams::<T>::mutate(
                                provider_id,
                                user_account,
                                |payment_stream| {
                                    let payment_stream = expect_or_err!(
                                        payment_stream,
                                        "Payment stream should exist if it was found before.",
                                        Error::<T>::PaymentStreamNotFound
                                    );
                                    payment_stream.last_charged_tick = last_chargeable_tick;
                                    Ok::<(), DispatchError>(())
                                },
                            )?;

                            // Update the total amount charged:
                            total_amount_charged = total_amount_charged
                                .checked_add(&amount_to_charge)
                                .ok_or(Error::<T>::ChargeOverflow)?;
                        }
                    }
                }
            }
        }

        // If the dynamic-rate payment stream exists:
        if let Some(dynamic_rate_payment_stream) = dynamic_rate_payment_stream {
            match UsersWithoutFunds::<T>::get(user_account) {
                Some(_) => {
                    // If the user has been flagged as without funds, manage it accordingly
                    Self::manage_user_without_funds(
                        &provider_id,
                        &user_account,
                        &PaymentStream::DynamicRatePaymentStream(dynamic_rate_payment_stream),
                    )?;
                }
                None => {
                    // Calculate the difference between the last charged price index and the price index at the last chargeable tick
                    // Note: If the last chargeable price index is less than the last charged price index, we charge 0 to the user, because that would be an impossible state.

                    let price_index_at_last_chargeable_tick = last_chargeable_info.price_index;

                    if let Some(price_index_difference) = price_index_at_last_chargeable_tick
                        .checked_sub(&dynamic_rate_payment_stream.price_index_when_last_charged)
                    {
                        // Calculate the amount to charge
                        let amount_to_charge = price_index_difference
                            .checked_mul(&dynamic_rate_payment_stream.amount_provided.into())
                            .ok_or(Error::<T>::ChargeOverflow)?
                            .checked_div(&GIGAUNIT.into())
                            .ok_or(ArithmeticError::Underflow)?;

                        // Check the free balance of the user
                        let user_balance = T::NativeBalance::reducible_balance(
                            &user_account,
                            Preservation::Preserve,
                            Fortitude::Polite,
                        );

                        // If the user does not have enough balance to pay for its storage:
                        if user_balance < amount_to_charge {
                            // Check if this payment stream was already flagged as without funds and, if so, how many ticks have passed since then
                            let out_of_funds_tick = dynamic_rate_payment_stream.out_of_funds_tick;
                            let current_tick = OnPollTicker::<T>::get();
                            let ticks_since_out_of_funds = match out_of_funds_tick {
                                Some(tick) => current_tick.saturating_sub(tick),
                                None => {
                                    // If this payment stream wasn't flagged as without funds before, flag it now with the current tick
                                    DynamicRatePaymentStreams::<T>::mutate(
                                        provider_id,
                                        user_account,
                                        |payment_stream| {
                                            let payment_stream = expect_or_err!(
                                            payment_stream,
                                            "Payment stream should exist if it was found before.",
                                            Error::<T>::PaymentStreamNotFound
                                        );
                                            payment_stream.out_of_funds_tick = Some(current_tick);
                                            Ok::<(), DispatchError>(())
                                        },
                                    )?;
                                    Zero::zero()
                                }
                            };

                            // If the user has missed payment for more than the allowed number of ticks (which is the number of ticks
                            // that the user deposited when it was created), consider it as without funds.
                            if ticks_since_out_of_funds >= T::NewStreamDeposit::get() {
                                Self::manage_user_without_funds(
                                    &provider_id,
                                    &user_account,
                                    &PaymentStream::DynamicRatePaymentStream(
                                        dynamic_rate_payment_stream,
                                    ),
                                )?;
                            }
                        } else {
                            // If the user does have enough funds to pay for its storage:

                            // Clear the out-of-funds flag from the payment stream
                            DynamicRatePaymentStreams::<T>::mutate(
                                provider_id,
                                user_account,
                                |payment_stream| {
                                    let payment_stream = expect_or_err!(
                                        payment_stream,
                                        "Payment stream should exist if it was found before.",
                                        Error::<T>::PaymentStreamNotFound
                                    );
                                    payment_stream.out_of_funds_tick = None;
                                    Ok::<(), DispatchError>(())
                                },
                            )?;

                            // Get the payment account of the SP
                            let provider_payment_account = expect_or_err!(
                            <T::ProvidersPallet as ReadProvidersInterface>::get_payment_account(
                                *provider_id
                            ),
                            "Provider should exist and have a payment account if its ID exists.",
                            Error::<T>::ProviderInconsistencyError
                        );

                            // Check if the total amount charged would overflow
                            // NOTE: We check this BEFORE transferring the amount to the provider.
                            ensure!(
                                total_amount_charged
                                    .checked_add(&amount_to_charge)
                                    .is_some(),
                                ArithmeticError::Overflow
                            );

                            // Get, from the total amount to charge, the cut for the treasury and the cut for the provider
                            let total_provided_amount =
                                <T::ProvidersPallet as SystemMetricsInterface>::get_total_capacity(
                                );
                            let used_provided_amount = <T::ProvidersPallet as SystemMetricsInterface>::get_total_used_capacity();
                            let treasury_cut = <T::TreasuryCutCalculator as TreasuryCutCalculator>::calculate_treasury_cut(total_provided_amount, used_provided_amount, amount_to_charge);
                            let provider_cut = amount_to_charge.saturating_sub(treasury_cut); // Treasury cut should always be less than the amount to charge, so this will never be 0.

                            // Charge the payment stream from the user's balance
                            T::NativeBalance::transfer(
                                user_account,
                                &provider_payment_account,
                                provider_cut,
                                Preservation::Preserve,
                            )?;

                            // Send the rest of the funds to the treasury
                            T::NativeBalance::transfer(
                                user_account,
                                &T::TreasuryAccount::get(),
                                treasury_cut,
                                Preservation::Preserve,
                            )?;

                            // Set the last charged price index to be the price index of the last chargeable tick
                            DynamicRatePaymentStreams::<T>::mutate(
                                provider_id,
                                user_account,
                                |payment_stream| {
                                    let payment_stream = expect_or_err!(
                                        payment_stream,
                                        "Payment stream should exist if it was found before.",
                                        Error::<T>::PaymentStreamNotFound
                                    );
                                    payment_stream.price_index_when_last_charged =
                                        price_index_at_last_chargeable_tick;
                                    Ok::<(), DispatchError>(())
                                },
                            )?;

                            // Update the total amount charged:
                            total_amount_charged = total_amount_charged
                                .checked_add(&amount_to_charge)
                                .ok_or(Error::<T>::ChargeOverflow)?;
                        }
                    }
                }
            }
        }

        Ok((total_amount_charged, last_chargeable_tick))
    }

    /// This function holds the logic that checks, for each User in the `user_accounts` array, if they have any
    /// payment streams with the given Provider and, if so, charges them.
    pub fn do_charge_multiple_users_payment_streams(
        provider_id: &ProviderIdFor<T>,
        user_accounts: &BoundedVec<T::AccountId, T::MaxUsersToCharge>,
    ) -> DispatchResult {
        // Get the current tick
        let current_tick = Self::get_current_tick();

        // For each User in the array, charge their payment stream with the given Provider
        // and emit a PaymentStreamCharged event if the User had to pay.
        for user_account in user_accounts.iter() {
            let (amount_charged, last_tick_charged) =
                Self::do_charge_payment_streams(provider_id, user_account)?;

            if amount_charged > Zero::zero() {
                Self::deposit_event(Event::PaymentStreamCharged {
                    user_account: user_account.clone(),
                    provider_id: *provider_id,
                    amount: amount_charged,
                    last_tick_charged,
                    charged_at_tick: current_tick,
                });
            }
        }

        Ok(())
    }

    /// This function holds the logic that checks if a user has outstanding debt and, if so, pays it by transferring each contracted Provider
    /// the amount owed, deleting the corresponding payment stream and decreasing the user's payment streams count until all outstanding debt is paid
    /// or all the Providers in the provided list have been paid. It returns true if the user has paid all the outstanding debt, false otherwise.
    ///
    /// Note: This could be achieved by calling `manage_user_without_funds` for each payment stream, but this function is more efficient as it
    /// avoids repeating the same checks and operations for each payment stream, such as releasing all the deposit of the user at once instead
    /// of doing it for each payment stream.
    pub fn do_pay_outstanding_debt(
        user_account: &T::AccountId,
        providers: Vec<ProviderIdFor<T>>,
    ) -> Result<bool, DispatchError> {
        // Check that the user is flagged as without funds
        ensure!(
            UsersWithoutFunds::<T>::contains_key(user_account),
            Error::<T>::UserNotFlaggedAsWithoutFunds
        );

        // Release the total deposit amount that the user has deposited
        let total_deposit_released = T::NativeBalance::release_all(
            &HoldReason::PaymentStreamDeposit.into(),
            &user_account,
            Precision::Exact,
        )?;

        // Get the system metrics to calculate the treasury cut for each payment stream.
        let total_provided_amount =
            <T::ProvidersPallet as SystemMetricsInterface>::get_total_capacity();
        let used_provided_amount =
            <T::ProvidersPallet as SystemMetricsInterface>::get_total_used_capacity();

        // Keep track of the deposit that corresponds to the payment streams that have been paid
        let mut total_deposit_paid: BalanceOf<T> = Zero::zero();

        // For each Provider in the list, pay the outstanding debt of the user
        for provider_id in providers {
            // Get the fixed-rate payment stream of the user with the given Provider
            let fixed_rate_payment_stream =
                FixedRatePaymentStreams::<T>::get(provider_id, user_account);

            // Get the dynamic-rate payment stream of the user with the given Provider
            let dynamic_rate_payment_stream =
                DynamicRatePaymentStreams::<T>::get(provider_id, user_account);

            // If the fixed-rate payment stream exists:
            if let Some(fixed_rate_payment_stream) = fixed_rate_payment_stream {
                // Get the amount that should be charged for this payment stream
                let last_chargeable_info =
                    Self::get_last_chargeable_info_with_privilege(&provider_id);
                let amount_to_charge = fixed_rate_payment_stream
                    .rate
                    .checked_mul(&T::BlockNumberToBalance::convert(
                        last_chargeable_info
                            .last_chargeable_tick
                            .saturating_sub(fixed_rate_payment_stream.last_charged_tick),
                    ))
                    .ok_or(ArithmeticError::Overflow)?;

                // If the amount to charge is greater than the deposit, just charge the deposit
                let amount_to_charge = amount_to_charge.min(fixed_rate_payment_stream.user_deposit);

                // Get the cut for the treasury and the cut for the provider
                let treasury_cut =
                    <T::TreasuryCutCalculator as TreasuryCutCalculator>::calculate_treasury_cut(
                        total_provided_amount,
                        used_provided_amount,
                        amount_to_charge,
                    );
                let provider_cut = amount_to_charge.saturating_sub(treasury_cut); // Treasury cut should always be less than the amount to charge, so this will never be 0.

                // Transfer the provider's cut from the user to the Provider
                let provider_payment_account = expect_or_err!(
                    <T::ProvidersPallet as ReadProvidersInterface>::get_payment_account(
                        provider_id
                    ),
                    "Provider should exist and have a payment account if its ID exists.",
                    Error::<T>::ProviderInconsistencyError
                );
                T::NativeBalance::transfer(
                    user_account,
                    &provider_payment_account,
                    provider_cut,
                    Preservation::Preserve,
                )?;

                // Send the rest of the funds to the treasury
                T::NativeBalance::transfer(
                    user_account,
                    &T::TreasuryAccount::get(),
                    treasury_cut,
                    Preservation::Preserve,
                )?;

                // Update the total deposit paid
                total_deposit_paid = total_deposit_paid
                    .checked_add(&fixed_rate_payment_stream.user_deposit)
                    .ok_or(ArithmeticError::Overflow)?;

                // Remove the payment stream from the FixedRatePaymentStreams mapping
                FixedRatePaymentStreams::<T>::remove(provider_id, user_account);

                // Decrease the user's payment streams count
                let mut user_payment_streams_count = RegisteredUsers::<T>::get(user_account);
                user_payment_streams_count = user_payment_streams_count
                    .checked_sub(1)
                    .ok_or(ArithmeticError::Underflow)?;
                RegisteredUsers::<T>::insert(user_account, user_payment_streams_count);
            }

            // If the dynamic-rate payment stream exists:
            if let Some(dynamic_rate_payment_stream) = dynamic_rate_payment_stream {
                // Get the amount that should be charged for this payment stream
                let price_index_at_last_chargeable_tick =
                    Self::get_last_chargeable_info_with_privilege(&provider_id).price_index;
                let amount_to_charge = price_index_at_last_chargeable_tick
                    .saturating_sub(dynamic_rate_payment_stream.price_index_when_last_charged)
                    .checked_mul(&dynamic_rate_payment_stream.amount_provided.into())
                    .ok_or(ArithmeticError::Overflow)?
                    .checked_div(&GIGAUNIT.into())
                    .ok_or(ArithmeticError::Underflow)?;

                // If the amount to charge is greater than the deposit, just charge the deposit
                let amount_to_charge =
                    amount_to_charge.min(dynamic_rate_payment_stream.user_deposit);

                // Get the cut for the treasury and the cut for the provider
                let treasury_cut =
                    <T::TreasuryCutCalculator as TreasuryCutCalculator>::calculate_treasury_cut(
                        total_provided_amount,
                        used_provided_amount,
                        amount_to_charge,
                    );
                let provider_cut = amount_to_charge.saturating_sub(treasury_cut); // Treasury cut should always be less than the amount to charge, so this will never be 0.

                // Transfer the provider's cut from the user to the Provider
                let provider_payment_account = expect_or_err!(
                    <T::ProvidersPallet as ReadProvidersInterface>::get_payment_account(
                        provider_id
                    ),
                    "Provider should exist and have a payment account if its ID exists.",
                    Error::<T>::ProviderInconsistencyError
                );
                T::NativeBalance::transfer(
                    user_account,
                    &provider_payment_account,
                    provider_cut,
                    Preservation::Preserve,
                )?;

                // Send the rest of the funds to the treasury
                T::NativeBalance::transfer(
                    user_account,
                    &T::TreasuryAccount::get(),
                    treasury_cut,
                    Preservation::Preserve,
                )?;

                // Update the total deposit paid
                total_deposit_paid = total_deposit_paid
                    .checked_add(&dynamic_rate_payment_stream.user_deposit)
                    .ok_or(ArithmeticError::Overflow)?;

                // Remove the payment stream from the DynamicRatePaymentStreams mapping
                DynamicRatePaymentStreams::<T>::remove(provider_id, user_account);

                // Decrease the user's payment streams count
                let mut user_payment_streams_count = RegisteredUsers::<T>::get(user_account);
                user_payment_streams_count = user_payment_streams_count
                    .checked_sub(1)
                    .ok_or(ArithmeticError::Underflow)?;
                RegisteredUsers::<T>::insert(user_account, user_payment_streams_count);
            }
        }

        // Hold the difference between the total deposit release and the total deposit used to pay the payment streams
        let difference = total_deposit_released
            .checked_sub(&total_deposit_paid)
            .ok_or(ArithmeticError::Underflow)?;
        T::NativeBalance::hold(
            &HoldReason::PaymentStreamDeposit.into(),
            &user_account,
            difference,
        )?;

        // Check if the user has paid all the outstanding debt, which is equivalent to having no remaining payment streams
        let remaining_payment_streams = RegisteredUsers::<T>::get(user_account);

        // If the user has paid all the outstanding debt, return true
        if remaining_payment_streams == 0 {
            Ok(true)
        } else {
            // If the user has not paid all the outstanding debt, return false
            Ok(false)
        }
    }

    pub fn do_clear_insolvent_flag(user_account: &T::AccountId) -> DispatchResult {
        // Check that the user is flagged as without funds
        ensure!(
            UsersWithoutFunds::<T>::contains_key(user_account),
            Error::<T>::UserNotFlaggedAsWithoutFunds
        );

        // Check that enough time has passed since the user was flagged as without funds
        let current_tick = OnPollTicker::<T>::get();
        let out_of_funds_tick = expect_or_err!(
            UsersWithoutFunds::<T>::get(user_account),
            "User should be flagged as without funds if it was found before.",
            Error::<T>::UserNotFlaggedAsWithoutFunds
        );
        ensure!(
            current_tick.saturating_sub(out_of_funds_tick) >= T::UserWithoutFundsCooldown::get(),
            Error::<T>::CooldownPeriodNotPassed
        );

        // Make sure the user has no remaining payment streams
        let user_payment_streams_count = RegisteredUsers::<T>::get(user_account);
        ensure!(
            user_payment_streams_count == 0,
            Error::<T>::UserHasRemainingDebt
        );

        // Remove the user from the UsersWithoutFunds mapping
        UsersWithoutFunds::<T>::remove(user_account);

        Ok(())
    }

    /// This function gets the Providers that submitted a valid proof in the last tick using the `ProofSubmittersInterface`,
    /// and updates the last chargeable tick and last chargeable price index of those Providers. It is bounded by the maximum
    /// amount of Providers that can submit a proof in a given tick, which is represented by the bounded binary tree set received from
    /// the `get_proof_submitters_for_tick` function of the `ProofSubmittersInterface` trait.
    pub fn do_update_last_chargeable_info(
        n: BlockNumberFor<T>,
        meter: &mut sp_weights::WeightMeter,
    ) {
        // Get the current tick of the pallet that implements the `ProofSubmittersInterface` trait
        let current_tick_of_proof_submitters =
            <T::ProvidersProofSubmitters as ProofSubmittersInterface>::get_current_tick();

        // Since this is the current tick and should have been just updated by the pallet that implements the `ProofSubmittersInterface` trait,
        // it should not have any valid proof submitters yet, so the required tick for the processing of proof submitters is the previous one.
        let tick_to_process = current_tick_of_proof_submitters.saturating_sub(One::one());

        // Check to see if the tick registered as the last processed one by this pallet is the same as the tick to process.
        // If it is, this tick was already processed and there's nothing to do.
        let last_processed_tick = LastSubmittersTickRegistered::<T>::get();
        if last_processed_tick >= tick_to_process {
            return;
        }
        // If it's not greater (which should never happen) nor equal, it has to be exactly one less than the tick to process. If it's not,
        // there's an inconsistency in the tick processing and we emit an event to signal it.
        else if last_processed_tick != tick_to_process.saturating_sub(One::one()) {
            Self::deposit_event(Event::InconsistentTickProcessing {
                last_processed_tick,
                tick_to_process,
            });
        }

        // Get the Providers that submitted a valid proof in the last tick of the pallet that implements the `ProofSubmittersInterface` trait, if there are any.
        let maybe_proof_submitters =
            <T::ProvidersProofSubmitters as ProofSubmittersInterface>::get_proof_submitters_for_tick(&tick_to_process);

        // Initialize the variable that holds how many Providers were processed in this call.
        let mut amount_of_providers_processed: u32 = 0;

        // If there are any Providers to process, process them, updating their last chargeable info.
        if let Some(proof_submitters_to_process) = maybe_proof_submitters {
            // Update the last chargeable info of all Providers that submitted a valid proof in the tick to process.
            for provider_id in &proof_submitters_to_process {
                // Update the last chargeable tick and last chargeable price index of the Provider.
                // The last chargeable tick is set to the current tick of THIS PALLET. That means that if the tick from
                // the pallet that implements the `ProofSubmittersInterface` trait is stalled for some time (and this pallet
                // continues to increment its tick), when the stalled tick starts to increment again this pallet will
                // allow Providers to charge for the time that during which the tick was stalled, since they would have
                // been storing the data during that time even though they have not submitted proofs.
                let accumulated_price_index = AccumulatedPriceIndex::<T>::get();
                LastChargeableInfo::<T>::mutate(provider_id, |provider_info| {
                    provider_info.last_chargeable_tick = n;
                    provider_info.price_index = accumulated_price_index;
                });
                Self::deposit_event(Event::LastChargeableInfoUpdated {
                    provider_id: *provider_id,
                    last_chargeable_tick: n,
                    last_chargeable_price_index: accumulated_price_index,
                });
            }

            // Get the amount of Providers just processed (the amount of processed Providers fits in a u32).
            amount_of_providers_processed += proof_submitters_to_process.len() as u32;
        }

        // Get the weight that was used to process this amount of Providers.
        let weight_needed =
            T::WeightInfo::update_providers_last_chargeable_info(amount_of_providers_processed);

        // Consume the used weight.
        meter.consume(weight_needed);

        // Finally, update the last processed tick of the pallet that implements the `ProofSubmittersInterface` trait to the one that was just processed.
        LastSubmittersTickRegistered::<T>::put(tick_to_process);
    }

    pub fn do_update_price_index(meter: &mut sp_weights::WeightMeter) {
        // Get the current price
        let current_price = CurrentPricePerGigaUnitPerTick::<T>::get();

        // Add it to the accumulated price index
        AccumulatedPriceIndex::<T>::mutate(|price_index| {
            *price_index = price_index.saturating_add(current_price);
        });

        // Get the weight required by this function
        let required_weight = T::WeightInfo::price_index_update();

        // Consume the required weight
        meter.consume(required_weight);
    }

    /// This function advances the current tick and returns the previous and now-current tick.
    pub fn do_advance_tick(
        meter: &mut sp_weights::WeightMeter,
    ) -> (BlockNumberFor<T>, BlockNumberFor<T>) {
        // Get the current tick
        let current_tick = OnPollTicker::<T>::get();

        // Increment the current tick
        let next_tick = current_tick.saturating_add(One::one());

        // Update the current tick
        OnPollTicker::<T>::set(next_tick);

        // Get the weight required by this function
        let required_weight = T::WeightInfo::tick_update();

        // Consume the required weight
        meter.consume(required_weight);

        // Return the previous tick (`current_tick`) and the now-current one (`next_tick`)
        (current_tick, next_tick)
    }

    /// This function holds the logic that updates the deposit of a User based on the new deposit that should be held from them.
    ///
    /// It checks if the new deposit is greater or smaller than the old deposit and holds/releases the difference in deposit from the User accordingly.
    pub fn update_user_deposit(
        user_account: &T::AccountId,
        old_deposit: BalanceOf<T>,
        new_deposit: BalanceOf<T>,
    ) -> DispatchResult {
        if new_deposit < old_deposit {
            // Calculate the difference in deposit (`(amount_provided - new_amount_provided) * current_price * NewStreamDeposit`)
            let difference_in_deposit = old_deposit
                .checked_sub(&new_deposit)
                .ok_or(ArithmeticError::Underflow)?;

            // Release the difference in deposit from the user
            T::NativeBalance::release(
                &HoldReason::PaymentStreamDeposit.into(),
                &user_account,
                difference_in_deposit,
                Precision::Exact,
            )?;
        } else if new_deposit > old_deposit {
            // Calculate the difference in deposit (`(new_amount_provided - amount_provided) * current_price * NewStreamDeposit`)
            let difference_in_deposit = new_deposit
                .checked_sub(&old_deposit)
                .ok_or(ArithmeticError::Underflow)?;

            // Check if we can hold the difference in deposit from the user
            ensure!(
                T::NativeBalance::can_hold(
                    &HoldReason::PaymentStreamDeposit.into(),
                    &user_account,
                    difference_in_deposit
                ),
                Error::<T>::CannotHoldDeposit
            );

            // Hold the difference in deposit from the user
            T::NativeBalance::hold(
                &HoldReason::PaymentStreamDeposit.into(),
                &user_account,
                difference_in_deposit,
            )?;
        }

        Ok(())
    }

    /// This function holds the logic that has to be executed when it is detected that a Provider is trying to charge a user that
    /// has been or should be flagged as without funds.
    ///
    /// It releases the deposit of the payment stream from the user and transfers it to the Provider to pay for the unpaid services,
    /// deletes the payment stream, decreases the user's payment streams count and flags the user as without funds if they still have
    /// remaining payment streams and hadn't been flagged before. If the user has no more payment streams, it removes the user from the
    /// UsersWithoutFunds mapping, since it is no longer considered insolvent.
    pub fn manage_user_without_funds(
        provider_id: &ProviderIdFor<T>,
        user_account: &T::AccountId,
        payment_stream: &PaymentStream<T>,
    ) -> DispatchResult {
        // Get the amount that should be charged for this payment stream
        let last_chargeable_info = Self::get_last_chargeable_info_with_privilege(&provider_id);

        // Get the user deposit and amount to charge from the payment stream
        let (deposit, amount_to_charge) = match payment_stream {
            PaymentStream::FixedRatePaymentStream(fixed_rate_payment_stream) => {
                let amount_to_charge = fixed_rate_payment_stream
                    .rate
                    .checked_mul(&T::BlockNumberToBalance::convert(
                        last_chargeable_info
                            .last_chargeable_tick
                            .saturating_sub(fixed_rate_payment_stream.last_charged_tick),
                    ))
                    .ok_or(ArithmeticError::Overflow)?;

                // If the amount to charge is greater than the deposit, just charge the deposit
                let amount_to_charge = amount_to_charge.min(fixed_rate_payment_stream.user_deposit);

                (fixed_rate_payment_stream.user_deposit, amount_to_charge)
            }
            PaymentStream::DynamicRatePaymentStream(dynamic_rate_payment_stream) => {
                // Get the amount that should be charged for this payment stream
                let price_index_at_last_chargeable_tick = last_chargeable_info.price_index;
                let amount_to_charge = price_index_at_last_chargeable_tick
                    .saturating_sub(dynamic_rate_payment_stream.price_index_when_last_charged)
                    .checked_mul(&dynamic_rate_payment_stream.amount_provided.into())
                    .ok_or(ArithmeticError::Overflow)?
                    .checked_div(&GIGAUNIT.into())
                    .ok_or(ArithmeticError::Underflow)?;

                // If the amount to charge is greater than the deposit, just charge the deposit
                let amount_to_charge =
                    amount_to_charge.min(dynamic_rate_payment_stream.user_deposit);

                (dynamic_rate_payment_stream.user_deposit, amount_to_charge)
            }
        };

        // Get, from the total amount to charge, the cut for the treasury and the cut for the provider
        let total_provided_amount =
            <T::ProvidersPallet as SystemMetricsInterface>::get_total_capacity();
        let used_provided_amount =
            <T::ProvidersPallet as SystemMetricsInterface>::get_total_used_capacity();
        let treasury_cut =
            <T::TreasuryCutCalculator as TreasuryCutCalculator>::calculate_treasury_cut(
                total_provided_amount,
                used_provided_amount,
                amount_to_charge,
            );
        let provider_cut = amount_to_charge.saturating_sub(treasury_cut); // Treasury cut should always be less than the amount to charge, so this will never be 0.

        // Release the deposit from the user to pay for their services
        let provider_payment_account = expect_or_err!(
            <T::ProvidersPallet as ReadProvidersInterface>::get_payment_account(*provider_id),
            "Provider should exist and have a payment account if its ID exists.",
            Error::<T>::ProviderInconsistencyError
        );
        T::NativeBalance::release(
            &HoldReason::PaymentStreamDeposit.into(),
            &user_account,
            deposit,
            Precision::Exact,
        )?;

        // Send the provider's cut to the provider
        T::NativeBalance::transfer(
            user_account,
            &provider_payment_account,
            provider_cut,
            Preservation::Preserve,
        )?;

        // Send the rest of the funds to the treasury
        T::NativeBalance::transfer(
            user_account,
            &T::TreasuryAccount::get(),
            treasury_cut,
            Preservation::Preserve,
        )?;

        // If the stream is a fixed-rate payment stream, remove it from the FixedRatePaymentStreams mapping
        if let PaymentStream::FixedRatePaymentStream(_) = payment_stream {
            FixedRatePaymentStreams::<T>::remove(provider_id, user_account);
        } else {
            // Else if it's a dynamic-rate payment stream, remove it from the DynamicRatePaymentStreams mapping
            DynamicRatePaymentStreams::<T>::remove(provider_id, user_account);
        }

        // Decrease the user's payment streams count
        let mut user_payment_streams_count = RegisteredUsers::<T>::get(user_account);
        user_payment_streams_count = user_payment_streams_count
            .checked_sub(1)
            .ok_or(ArithmeticError::Underflow)?;
        RegisteredUsers::<T>::insert(user_account, user_payment_streams_count);

        // Add the user to the UsersWithoutFunds mapping and emit the UserWithoutFunds event. If the user has no remaining
        // payment streams, emit the UserPaidAllDebts event as well.
        // Note: once a user is flagged as without funds, it is considered insolvent by the system and every Provider
        // will be incentivised to stop providing services to that user.
        // To be unflagged, the user will have to pay its remaining debt and wait the cooldown period, after which it will
        // need to execute the `clear_insolvent_flag` extrinsic. If it hasn't paid its debt by then, the extrinsic will
        // pay it for the user using the payment streams' deposits.
        if !UsersWithoutFunds::<T>::contains_key(user_account) {
            UsersWithoutFunds::<T>::insert(user_account, frame_system::Pallet::<T>::block_number());
            Self::deposit_event(Event::UserWithoutFunds {
                who: user_account.clone(),
            });
        }
        if user_payment_streams_count == 0 {
            Self::deposit_event(Event::UserPaidAllDebts {
                who: user_account.clone(),
            });
        }

        Ok(())
    }
}

impl<T: pallet::Config> PaymentStreamsInterface for pallet::Pallet<T> {
    type AccountId = T::AccountId;
    type ProviderId = ProviderIdFor<T>;
    type Balance = T::NativeBalance;
    type TickNumber = BlockNumberFor<T>;
    type Units = UnitsProvidedFor<T>;
    type FixedRatePaymentStream = FixedRatePaymentStream<T>;
    type DynamicRatePaymentStream = DynamicRatePaymentStream<T>;

    fn create_fixed_rate_payment_stream(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
        rate: <Self::Balance as Inspect<Self::AccountId>>::Balance,
    ) -> DispatchResult {
        // Execute the logic to create a fixed-rate payment stream
        Self::do_create_fixed_rate_payment_stream(provider_id, user_account, rate)?;

        // Emit the corresponding event
        Self::deposit_event(Event::FixedRatePaymentStreamCreated {
            user_account: user_account.clone(),
            provider_id: *provider_id,
            rate,
        });

        // Return a successful DispatchResult
        Ok(())
    }

    fn update_fixed_rate_payment_stream(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
        new_rate: <Self::Balance as Inspect<Self::AccountId>>::Balance,
    ) -> DispatchResult {
        // Execute the logic to update a fixed-rate payment stream
        Self::do_update_fixed_rate_payment_stream(provider_id, user_account, new_rate)?;

        // Emit the corresponding event
        Self::deposit_event(Event::FixedRatePaymentStreamUpdated {
            user_account: user_account.clone(),
            provider_id: *provider_id,
            new_rate,
        });

        // Return a successful DispatchResult
        Ok(())
    }

    fn delete_fixed_rate_payment_stream(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> DispatchResult {
        // Execute the logic to delete a fixed-rate payment stream
        Self::do_delete_fixed_rate_payment_stream(provider_id, user_account)?;

        // Emit the corresponding event
        Self::deposit_event(Event::FixedRatePaymentStreamDeleted {
            user_account: user_account.clone(),
            provider_id: *provider_id,
        });

        // Return a successful DispatchResult
        Ok(())
    }

    fn get_fixed_rate_payment_stream_info(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> Option<Self::FixedRatePaymentStream> {
        // Return the payment stream information
        FixedRatePaymentStreams::<T>::get(provider_id, user_account)
    }

    fn get_inner_fixed_rate_payment_stream_value(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> Option<<Self::Balance as Inspect<Self::AccountId>>::Balance> {
        FixedRatePaymentStreams::<T>::get(provider_id, user_account).map(|stream| stream.rate)
    }

    fn fixed_rate_payment_stream_exists(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> bool {
        FixedRatePaymentStreams::<T>::contains_key(provider_id, user_account)
    }

    fn create_dynamic_rate_payment_stream(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
        amount_provided: &Self::Units,
    ) -> DispatchResult {
        // Execute the logic to create a dynamic-rate payment stream
        Self::do_create_dynamic_rate_payment_stream(provider_id, user_account, *amount_provided)?;

        // Emit the corresponding event
        Self::deposit_event(Event::DynamicRatePaymentStreamCreated {
            user_account: user_account.clone(),
            provider_id: *provider_id,
            amount_provided: *amount_provided,
        });

        // Return a successful DispatchResult
        Ok(())
    }

    fn update_dynamic_rate_payment_stream(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
        new_amount_provided: &Self::Units,
    ) -> DispatchResult {
        // Execute the logic to update a dynamic-rate payment stream
        Self::do_update_dynamic_rate_payment_stream(
            &provider_id,
            &user_account,
            *new_amount_provided,
        )?;

        // Emit the corresponding event
        Self::deposit_event(Event::DynamicRatePaymentStreamUpdated {
            user_account: user_account.clone(),
            provider_id: *provider_id,
            new_amount_provided: *new_amount_provided,
        });

        // Return a successful DispatchResult
        Ok(())
    }

    fn delete_dynamic_rate_payment_stream(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> DispatchResult {
        // Execute the logic to delete a dynamic-rate payment stream
        Self::do_delete_dynamic_rate_payment_stream(&provider_id, &user_account)?;

        // Emit the corresponding event
        Self::deposit_event(Event::DynamicRatePaymentStreamDeleted {
            user_account: user_account.clone(),
            provider_id: *provider_id,
        });

        // Return a successful DispatchResult
        Ok(())
    }

    fn get_dynamic_rate_payment_stream_info(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> Option<Self::DynamicRatePaymentStream> {
        // Return the payment stream information
        DynamicRatePaymentStreams::<T>::get(provider_id, user_account)
    }

    fn get_dynamic_rate_payment_stream_amount_provided(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> Option<Self::Units> {
        // Return the amount provided by the user in the dynamic-rate payment stream
        DynamicRatePaymentStreams::<T>::get(provider_id, user_account)
            .map(|stream| stream.amount_provided)
    }

    fn has_active_payment_stream_with_user(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> bool {
        FixedRatePaymentStreams::<T>::contains_key(provider_id, user_account)
            || DynamicRatePaymentStreams::<T>::contains_key(provider_id, user_account)
    }

    fn has_active_payment_stream(provider_id: &Self::ProviderId) -> bool {
        Self::provider_has_payment_streams(provider_id)
    }

    fn add_privileged_provider(provider_id: &Self::ProviderId) {
        PrivilegedProviders::<T>::insert(provider_id, ());
    }

    fn remove_privileged_provider(provider_id: &Self::ProviderId) {
        PrivilegedProviders::<T>::remove(provider_id);
    }

    fn current_tick() -> BlockNumberFor<T> {
        OnPollTicker::<T>::get()
    }
}

impl<T: pallet::Config> ReadUserSolvencyInterface for pallet::Pallet<T> {
    type AccountId = T::AccountId;

    fn is_user_insolvent(user_account: &Self::AccountId) -> bool {
        UsersWithoutFunds::<T>::contains_key(user_account)
    }
}

impl<T: pallet::Config> PricePerGigaUnitPerTickInterface for pallet::Pallet<T> {
    type PricePerGigaUnitPerTick = BalanceOf<T>;

    fn get_price_per_giga_unit_per_tick() -> Self::PricePerGigaUnitPerTick {
        CurrentPricePerGigaUnitPerTick::<T>::get()
    }

    fn set_price_per_giga_unit_per_tick(price_index: Self::PricePerGigaUnitPerTick) {
        CurrentPricePerGigaUnitPerTick::<T>::put(price_index);
    }
}

/// Runtime API implementation for the PaymentStreams pallet
impl<T> Pallet<T>
where
    T: pallet::Config,
{
    pub fn get_users_with_debt_over_threshold(
        provider_id: &ProviderIdFor<T>,
        threshold: BalanceOf<T>,
    ) -> Result<Vec<T::AccountId>, GetUsersWithDebtOverThresholdError> {
        // Check if the Provider ID received belongs to an actual Provider
        ensure!(
            <T::ProvidersPallet as ReadProvidersInterface>::is_provider(*provider_id),
            GetUsersWithDebtOverThresholdError::ProviderNotRegistered
        );

        // Check if the Provider has any payment streams active
        let provider_has_payment_streams =
            !Self::get_fixed_rate_payment_streams_of_provider(provider_id).is_empty()
                || !Self::get_dynamic_rate_payment_streams_of_provider(provider_id).is_empty();

        ensure!(
            provider_has_payment_streams,
            GetUsersWithDebtOverThresholdError::ProviderWithoutPaymentStreams
        );

        // Get the last chargeable info of the Provider
        let last_chargeable_info = Self::get_last_chargeable_info_with_privilege(provider_id);

        // Get all the users that have a payment stream with this Provider
        let users_of_provider = Self::get_users_with_payment_stream_with_provider(provider_id);

        // Initialize the vector that will hold the users with debt over the threshold
        let mut users_with_debt_over_threshold = Vec::new();

        // For each user, check if they have a debt over the threshold and if so, add them to the vector
        for user in users_of_provider {
            if !UsersWithoutFunds::<T>::contains_key(&user) {
                let mut debt: BalanceOf<T> = Zero::zero();

                if let Some(dynamic_stream) =
                    DynamicRatePaymentStreams::<T>::get(provider_id, &user)
                {
                    let price_index_difference = last_chargeable_info
                        .price_index
                        .saturating_sub(dynamic_stream.price_index_when_last_charged);
                    let amount_to_charge = price_index_difference
                        .checked_mul(&dynamic_stream.amount_provided.into())
                        .ok_or(GetUsersWithDebtOverThresholdError::AmountToChargeOverflow)?
                        .checked_div(&GIGAUNIT.into())
                        .ok_or(GetUsersWithDebtOverThresholdError::AmountToChargeUnderflow)?;
                    debt = debt
                        .checked_add(&amount_to_charge)
                        .ok_or(GetUsersWithDebtOverThresholdError::DebtOverflow)?;
                }

                if let Some(fixed_stream) = FixedRatePaymentStreams::<T>::get(provider_id, &user) {
                    let time_passed = last_chargeable_info
                        .last_chargeable_tick
                        .saturating_sub(fixed_stream.last_charged_tick);
                    let amount_to_charge = fixed_stream
                        .rate
                        .checked_mul(&T::BlockNumberToBalance::convert(time_passed))
                        .ok_or(GetUsersWithDebtOverThresholdError::AmountToChargeOverflow)?;
                    debt = debt
                        .checked_add(&amount_to_charge)
                        .ok_or(GetUsersWithDebtOverThresholdError::DebtOverflow)?;
                }

                if debt >= threshold {
                    users_with_debt_over_threshold.push(user);
                }
            }
        }

        Ok(users_with_debt_over_threshold)
    }

    pub fn get_users_of_payment_streams_of_provider(
        provider_id: &ProviderIdFor<T>,
    ) -> Vec<T::AccountId> {
        let mut payment_streams = Vec::new();

        // Check if the Provider has any payment streams active
        let provider_has_payment_streams =
            !Self::get_fixed_rate_payment_streams_of_provider(provider_id).is_empty()
                || !Self::get_dynamic_rate_payment_streams_of_provider(provider_id).is_empty();

        if provider_has_payment_streams {
            // Get the fixed-rate payment streams of the Provider
            let fixed_rate_payment_streams =
                FixedRatePaymentStreams::<T>::iter_prefix(provider_id).map(|(user, _)| user);
            payment_streams.extend(fixed_rate_payment_streams);

            // Get the dynamic-rate payment streams of the Provider
            let dynamic_rate_payment_streams =
                DynamicRatePaymentStreams::<T>::iter_prefix(provider_id).map(|(user, _)| user);
            payment_streams.extend(dynamic_rate_payment_streams);
        }

        payment_streams
    }

    /// This function is called by the runtime API that allows anyone to get the count of users that have
    /// at least one payment stream with a provider.
    /// It returns the count as a u32, avoiding vector allocation and serialization overhead.
    pub fn get_number_of_active_users_of_provider(provider_id: &ProviderIdFor<T>) -> u32 {
        let fixed_count = FixedRatePaymentStreams::<T>::iter_prefix(provider_id).count();
        let dynamic_count = DynamicRatePaymentStreams::<T>::iter_prefix(provider_id).count();
        (fixed_count + dynamic_count) as u32
    }

    /// This function is called by the runtime API that allows anyone to get the list of Providers that have a
    /// at least one payment stream with a user.
    /// It returns a vector of Provider IDs, without duplicates.
    pub fn get_providers_with_payment_streams_with_user(
        user_account: &T::AccountId,
    ) -> Vec<ProviderIdFor<T>> {
        let mut providers = Vec::new();

        // Get all the payment streams of the user
        let fixed_rate_payment_streams = Self::get_fixed_rate_payment_streams_of_user(user_account);
        let dynamic_rate_payment_streams =
            Self::get_dynamic_rate_payment_streams_of_user(user_account);

        // Get the Providers of the fixed-rate payment streams
        for (provider_id, _) in fixed_rate_payment_streams {
            if !providers.contains(&provider_id) {
                providers.push(provider_id);
            }
        }

        // Get the Providers of the dynamic-rate payment streams
        for (provider_id, _) in dynamic_rate_payment_streams {
            if !providers.contains(&provider_id) {
                providers.push(provider_id);
            }
        }

        providers
    }

    /// Returns the [`ProviderLastChargeableInfo`] of a Provider, which includes the last chargeable tick and the last chargeable price index.
    pub fn get_last_chargeable_info_with_privilege(
        provider_id: &ProviderIdFor<T>,
    ) -> ProviderLastChargeableInfo<T> {
        // If this is a Privileged Provider, then it is allowed to charge up to the current tick.
        if let Some(_) = PrivilegedProviders::<T>::get(provider_id) {
            return ProviderLastChargeableInfo {
                last_chargeable_tick: Self::get_current_tick(),
                price_index: Default::default(),
            };
        }

        return LastChargeableInfo::<T>::get(provider_id);
    }
}
