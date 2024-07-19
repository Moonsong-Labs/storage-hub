use frame_support::ensure;
use frame_support::pallet_prelude::DispatchResult;
use frame_support::sp_runtime::{
    traits::{CheckedAdd, CheckedMul, CheckedSub, Zero},
    ArithmeticError, DispatchError,
};
use frame_support::traits::{
    fungible::{Inspect, InspectHold, Mutate, MutateHold},
    tokens::{Fortitude, Precision, Preservation},
    Get,
};
use frame_system::pallet_prelude::BlockNumberFor;
use shp_traits::{
    PaymentStreamsInterface, ProvidersInterface, ReadProofSubmittersInterface,
    ReadProvidersInterface, SystemMetricsInterface,
};
use sp_runtime::{
    traits::{Convert, One},
    Saturating,
};

use crate::*;

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
            <T::ProvidersPallet as ProvidersInterface>::is_provider(*provider_id),
            Error::<T>::NotAProvider
        );

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
        // NOTE: We check this BEFORE holding the deposit, as for some weird reason the `hold` function does NOT revert when the extrinsic fails !?!?
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
        // We initiate the `last_charged_block` and `last_chargeable_block` with the current block number to be able to keep track of the
        // time passed since the payment stream was originally created
        FixedRatePaymentStreams::<T>::insert(
            provider_id,
            user_account,
            FixedRatePaymentStream {
                rate,
                last_charged_block: frame_system::Pallet::<T>::block_number(),
                user_deposit: deposit,
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
            <T::ProvidersPallet as ProvidersInterface>::is_provider(*provider_id),
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
        let amount_charged = Self::do_charge_payment_streams(&provider_id, user_account)?;
        if amount_charged > Zero::zero() {
            // We emit a payment charged event only if the user had to pay before the payment stream could be updated
            Self::deposit_event(Event::<T>::PaymentStreamCharged {
                user_account: user_account.clone(),
                provider_id: *provider_id,
                amount: amount_charged,
            });
        }

        // Update the user's deposit based on the new rate
        let new_deposit = new_rate
            .checked_mul(&T::BlockNumberToBalance::convert(T::NewStreamDeposit::get()))
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
            <T::ProvidersPallet as ProvidersInterface>::is_provider(*provider_id),
            Error::<T>::NotAProvider
        );

        // Check that a payment stream between that Provider and User exists
        ensure!(
            FixedRatePaymentStreams::<T>::contains_key(provider_id, user_account),
            Error::<T>::PaymentStreamNotFound
        );

        // TODO: What do we do when a user is flagged as without funds? Does the provider assume the loss and we remove the payment stream?
        // Check that the user is not flagged as without funds
        ensure!(
            !UsersWithoutFunds::<T>::contains_key(user_account),
            Error::<T>::UserWithoutFunds
        );

        // Charge the payment stream before deletion to make sure the services provided by the Provider is paid in full for its duration
        let amount_charged = Self::do_charge_payment_streams(&provider_id, user_account)?;
        if amount_charged > Zero::zero() {
            // We emit a payment charged event only if the user had to pay before being able to delete the payment stream
            Self::deposit_event(Event::<T>::PaymentStreamCharged {
                user_account: user_account.clone(),
                provider_id: *provider_id,
                amount: amount_charged,
            });
        }

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

        Ok(())
    }

    /// This function holds the logic that checks if a dynamic-rate payment stream can be created and, if so, stores the payment
    /// stream in the DynamicRatePaymentStreams mapping and holds the deposit from the User.
    pub fn do_create_dynamic_rate_payment_stream(
        provider_id: &ProviderIdFor<T>,
        user_account: &T::AccountId,
        amount_provided: UnitsProvidedFor<T>,
        current_price: BalanceOf<T>,
        current_accumulated_price_index: BalanceOf<T>,
    ) -> DispatchResult {
        // Check that the given ID belongs to an actual Provider
        ensure!(
            <T::ProvidersPallet as ProvidersInterface>::is_provider(*provider_id),
            Error::<T>::NotAProvider
        );

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
        // Deposit is: `amount_provided * current_price * NewStreamDeposit` where:
        // - `amount_provided` is the amount of units of something (for example, storage) that are provided by the Provider to the User
        // - `current_price` is the current price of the units of something (per unit per block)
        // - `NewStreamDeposit` is a runtime constant that represents the number of blocks that the deposit should cover
        let user_balance = T::NativeBalance::reducible_balance(
            &user_account,
            Preservation::Preserve,
            Fortitude::Polite,
        );
        let deposit = current_price
            .checked_mul(&amount_provided.into())
            .ok_or(ArithmeticError::Overflow)?
            .checked_mul(&T::BlockNumberToBalance::convert(T::NewStreamDeposit::get()))
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
        // We initiate the `price_index_when_last_charged` and `price_index_at_last_chargeable_block` with the current price index to be able to keep track of the
        // price changes since the payment stream was originally created.
        DynamicRatePaymentStreams::<T>::insert(
            provider_id,
            user_account,
            DynamicRatePaymentStream {
                amount_provided,
                price_index_when_last_charged: current_accumulated_price_index,
                user_deposit: deposit,
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
        current_price: BalanceOf<T>,
    ) -> DispatchResult {
        // Check that the given ID belongs to an actual Provider
        ensure!(
            <T::ProvidersPallet as ProvidersInterface>::is_provider(*provider_id),
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
        let amount_charged = Self::do_charge_payment_streams(&provider_id, user_account)?;
        if amount_charged > Zero::zero() {
            // We emit a payment charged event only if the user had to pay before the payment stream could be updated
            Self::deposit_event(Event::<T>::PaymentStreamCharged {
                user_account: user_account.clone(),
                provider_id: *provider_id,
                amount: amount_charged,
            });
        }

        // Update the user's deposit based on the new amount provided
        let new_deposit = new_amount_provided
            .into()
            .checked_mul(&current_price)
            .ok_or(ArithmeticError::Overflow)?
            .checked_mul(&T::BlockNumberToBalance::convert(T::NewStreamDeposit::get()))
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
            <T::ProvidersPallet as ProvidersInterface>::is_provider(*provider_id),
            Error::<T>::NotAProvider
        );

        // Check that a dynamic-rate payment stream between that Provider and User exists
        ensure!(
            DynamicRatePaymentStreams::<T>::contains_key(provider_id, user_account),
            Error::<T>::PaymentStreamNotFound
        );

        // TODO: What do we do when a user is flagged as without funds? Does the provider assume the loss and we remove the payment stream?
        // Check that the user is not flagged as without funds
        ensure!(
            !UsersWithoutFunds::<T>::contains_key(user_account),
            Error::<T>::UserWithoutFunds
        );

        // Charge the payment stream before deletion to make sure the services provided by the Provider is paid in full for its duration
        let amount_charged = Self::do_charge_payment_streams(&provider_id, user_account)?;
        if amount_charged > Zero::zero() {
            // We emit a payment charged event only if the user had to pay before being able to delete the payment stream
            Self::deposit_event(Event::<T>::PaymentStreamCharged {
                user_account: user_account.clone(),
                provider_id: *provider_id,
                amount: amount_charged,
            });
        }

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

        Ok(())
    }

    /// This function holds the logic that checks if any payment stream exists between a Provider and a User and, if so,
    /// charges the payment stream/s from the User's balance.
    /// For fixed-rate payment streams, the charge is calculated as: `rate * time_passed` where `time_passed` is the time between the last chargeable block and
    /// the last charged block of this payment stream.  As such, the last charged block can't ever be greater than the last chargeable block, and if they are equal then no charge is made.
    /// For dynamic-rate payment streams, the charge is calculated as: `amount_provided * (price_index_when_last_charged - price_index_at_last_chargeable_block)`. In this case,
    /// the price index at the last charged block can't ever be greater than the price index at the last chargeable block, and if they are equal then no charge is made.
    /// TODO: Maybe add a way to pass an array of users to charge them all at once?
    pub fn do_charge_payment_streams(
        provider_id: &ProviderIdFor<T>,
        user_account: &T::AccountId,
    ) -> Result<BalanceOf<T>, DispatchError> {
        // Check that the given ID belongs to an actual Provider
        ensure!(
            <T::ProvidersPallet as ProvidersInterface>::is_provider(*provider_id),
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

        // Note: No need to check if the last chargeable block/price index at last chargeable block has been updated since the last charge,
        // as the only consequence of that is charging 0 to the user.
        // Not erroring out in this situation helps to be able to call this function without errors when updating or removing a payment stream.

        // Initiate the variable that will hold the total amount that has been charged
        let mut total_amount_charged: BalanceOf<T> = Zero::zero();

        // If the fixed-rate payment stream exists:
        if let Some(fixed_rate_payment_stream) = fixed_rate_payment_stream {
            // Calculate the time passed between the last chargeable block and the last charged block
            let last_chargeable_block =
                LastChargeableInfo::<T>::get(provider_id).last_chargeable_block;
            if let Some(time_passed) =
                last_chargeable_block.checked_sub(&fixed_rate_payment_stream.last_charged_block)
            {
                // Convert it to the balance type (for math)
                let time_passed_balance_typed = T::BlockNumberToBalance::convert(time_passed);

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
                    // TODO: Probably just charge what the user has and then flag it
                    // Flag it in the UsersWithoutFunds mapping and emit the UserWithoutFunds event
                    UsersWithoutFunds::<T>::insert(user_account, ());
                    Self::deposit_event(Event::<T>::UserWithoutFunds {
                        who: user_account.clone(),
                    });
                } else {
                    // If the user does have enough funds to pay for its storage:

                    // Clear the user from the UsersWithoutFunds mapping
                    // TODO: Design a more robust way of handling out-of-funds users
                    UsersWithoutFunds::<T>::remove(user_account);

                    // Get the payment account of the SP
                    let provider_payment_account = expect_or_err!(
                    <T::ProvidersPallet as ReadProvidersInterface>::get_provider_payment_account(
                        *provider_id
                    ),
                    "Provider should exist and have a payment account if its ID exists.",
                    Error::<T>::ProviderInconsistencyError
                );

                    // Check if the total amount charged would overflow
                    // NOTE: We check this BEFORE transferring the amount to the provider, as the `transfer` function does NOT revert when the extrinsic fails !?!?
                    ensure!(
                        total_amount_charged
                            .checked_add(&amount_to_charge)
                            .is_some(),
                        ArithmeticError::Overflow
                    );

                    // Charge the payment stream from the user's balance
                    T::NativeBalance::transfer(
                        user_account,
                        &provider_payment_account,
                        amount_to_charge,
                        Preservation::Preserve,
                    )?;

                    // Set the last charged block to the block number of the last chargeable block
                    FixedRatePaymentStreams::<T>::mutate(
                        provider_id,
                        user_account,
                        |payment_stream| {
                            let payment_stream = expect_or_err!(
                                payment_stream,
                                "Payment stream should exist if it was found before.",
                                Error::<T>::PaymentStreamNotFound
                            );
                            payment_stream.last_charged_block = last_chargeable_block;
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

        // If the dynamic-rate payment stream exists:
        if let Some(dynamic_rate_payment_stream) = dynamic_rate_payment_stream {
            // Calculate the difference between the last charged price index and the price index at the last chargeable block
            // Note: If the last chargeable price index is less than the last charged price index, we charge 0 to the user, because that would be an impossible state.

            let price_index_at_last_chargeable_block =
                LastChargeableInfo::<T>::get(provider_id).price_index;
            if let Some(price_index_difference) = price_index_at_last_chargeable_block
                .checked_sub(&dynamic_rate_payment_stream.price_index_when_last_charged)
            {
                // Calculate the amount to charge
                let amount_to_charge = price_index_difference
                    .checked_mul(&dynamic_rate_payment_stream.amount_provided.into())
                    .ok_or(Error::<T>::ChargeOverflow)?;

                // Check the free balance of the user
                let user_balance = T::NativeBalance::reducible_balance(
                    &user_account,
                    Preservation::Preserve,
                    Fortitude::Polite,
                );

                // If the user does not have enough balance to pay for its storage:
                if user_balance < amount_to_charge {
                    // TODO: Probably just charge what the user has and then flag it
                    // Flag it in the UsersWithoutFunds mapping and emit the UserWithoutFunds event
                    UsersWithoutFunds::<T>::insert(user_account, ());
                    Self::deposit_event(Event::<T>::UserWithoutFunds {
                        who: user_account.clone(),
                    });
                } else {
                    // If the user does have enough funds to pay for its storage:

                    // Clear the user from the UsersWithoutFunds mapping
                    // TODO: Design a more robust way of handling out-of-funds users
                    UsersWithoutFunds::<T>::remove(user_account);

                    // Get the payment account of the SP
                    let provider_payment_account = expect_or_err!(
                    <T::ProvidersPallet as ReadProvidersInterface>::get_provider_payment_account(
                        *provider_id
                    ),
                    "Provider should exist and have a payment account if its ID exists.",
                    Error::<T>::ProviderInconsistencyError
                );

                    // Check if the total amount charged would overflow
                    // NOTE: We check this BEFORE transferring the amount to the provider, as the `transfer` function does NOT revert when the extrinsic fails !?!?
                    ensure!(
                        total_amount_charged
                            .checked_add(&amount_to_charge)
                            .is_some(),
                        ArithmeticError::Overflow
                    );

                    // Charge the payment stream from the user's balance
                    T::NativeBalance::transfer(
                        user_account,
                        &provider_payment_account,
                        amount_to_charge,
                        Preservation::Preserve,
                    )?;

                    // Set the last charged price index to be the price index of the last chargeable block
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
                                price_index_at_last_chargeable_block;
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

        Ok(total_amount_charged)
    }

    /// This function gets the Providers that submitted a valid proof in the last tick using the `ReadProofSubmittersInterface`,
    /// and updates the last chargeable block and last chargeable price index of those Providers.
    pub fn do_update_last_chargeable_info(
        n: BlockNumberFor<T>,
        weight: &mut frame_support::weights::WeightMeter,
    ) {
        // Get last block's number
        let n = n.saturating_sub(One::one());
        // Get the Providers that submitted a valid proof in the last tick, if there are any
        let proof_submitters =
            <T::ProvidersProofSubmitters as ReadProofSubmittersInterface>::get_proof_submitters_for_tick(&n);
        weight.consume(T::DbWeight::get().reads(1));

        // If there are any proof submitters in the last tick
        if let Some(proof_submitters) = proof_submitters {
            // Update all Providers
            // Iterate through the proof submitters and update their last chargeable block and last chargeable price index
            for provider_id in proof_submitters {
                // Update the last chargeable block and last chargeable price index of the Provider
                LastChargeableInfo::<T>::mutate(provider_id, |provider_info| {
                    provider_info.last_chargeable_block = n;
                    provider_info.price_index = AccumulatedPriceIndex::<T>::get();
                });
                weight.consume(T::DbWeight::get().reads_writes(1, 1));
            }

            // TODO: What happens if we do not have enough weight? It should never happen so we should have a way to just reserve the
            // needed weight in the block for this, such as what `on_initialize` does.
        }
    }

    /// This functions calculates the current price of services provided for dynamic-rate streams and updates it in storage.
    pub fn do_update_current_price_per_unit_per_block(
        weight: &mut frame_support::weights::WeightMeter,
    ) {
        // Get the total used capacity of the network
        let _total_used_capacity =
            <T::ProvidersPallet as SystemMetricsInterface>::get_total_used_capacity();
        weight.consume(T::DbWeight::get().reads(1));

        // Get the total capacity of the network
        let _total_capacity = <T::ProvidersPallet as SystemMetricsInterface>::get_total_capacity();
        weight.consume(T::DbWeight::get().reads(1));

        // Calculate the current price per unit per block
        // TODO: Once the curve of price per unit per block is defined, implement it here
        let current_price_per_unit_per_block: BalanceOf<T> =
            CurrentPricePerUnitPerBlock::<T>::get();

        // Update it in storage
        CurrentPricePerUnitPerBlock::<T>::put(current_price_per_unit_per_block);
        weight.consume(T::DbWeight::get().writes(1));
    }

    pub fn do_update_price_index(weight: &mut frame_support::weights::WeightMeter) {
        // Get the current price
        let current_price = CurrentPricePerUnitPerBlock::<T>::get();
        weight.consume(T::DbWeight::get().reads(1));

        // Add it to the accumulated price index
        AccumulatedPriceIndex::<T>::mutate(|price_index| {
            *price_index = price_index.saturating_add(current_price);
        });
        weight.consume(T::DbWeight::get().reads_writes(1, 1));
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
}

impl<T: pallet::Config> PaymentStreamsInterface for pallet::Pallet<T> {
    type AccountId = T::AccountId;
    type ProviderId = ProviderIdFor<T>;
    type Balance = T::NativeBalance;
    type BlockNumber = BlockNumberFor<T>;
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
        Self::deposit_event(Event::<T>::FixedRatePaymentStreamCreated {
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
        Self::deposit_event(Event::<T>::FixedRatePaymentStreamUpdated {
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
        Self::deposit_event(Event::<T>::FixedRatePaymentStreamDeleted {
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

    fn create_dynamic_rate_payment_stream(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
        amount_provided: &Self::Units,
        current_price: <Self::Balance as Inspect<Self::AccountId>>::Balance,
        current_accumulated_price_index: <Self::Balance as Inspect<Self::AccountId>>::Balance,
    ) -> DispatchResult {
        // Execute the logic to create a dynamic-rate payment stream
        Self::do_create_dynamic_rate_payment_stream(
            provider_id,
            user_account,
            *amount_provided,
            current_price,
            current_accumulated_price_index,
        )?;

        // Emit the corresponding event
        Self::deposit_event(Event::<T>::DynamicRatePaymentStreamCreated {
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
        current_price: <Self::Balance as Inspect<Self::AccountId>>::Balance,
    ) -> DispatchResult {
        // Execute the logic to update a dynamic-rate payment stream
        Self::do_update_dynamic_rate_payment_stream(
            &provider_id,
            &user_account,
            *new_amount_provided,
            current_price,
        )?;

        // Emit the corresponding event
        Self::deposit_event(Event::<T>::DynamicRatePaymentStreamUpdated {
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
        Self::deposit_event(Event::<T>::DynamicRatePaymentStreamDeleted {
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
}
