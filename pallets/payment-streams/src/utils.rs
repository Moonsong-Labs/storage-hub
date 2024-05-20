use frame_support::ensure;
use frame_support::pallet_prelude::DispatchResult;
use frame_support::sp_runtime::{
    traits::{CheckedMul, CheckedSub, Zero},
    ArithmeticError, DispatchError,
};
use frame_support::traits::{
    fungible::{Inspect, InspectHold, Mutate, MutateHold},
    tokens::{Fortitude, Precision, Preservation},
    Get,
};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::traits::Convert;
use storage_hub_traits::ProvidersInterface;

use crate::*;
use storage_hub_traits::PaymentStreamsInterface;

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
    /// This function holds the logic that checks if a payment stream can be created and, if so, stores the payment stream in the PaymentStreams mapping
    /// and holds the necessary balance from the sender if it's its first payment stream
    ///
    /// Note: Maybe we should add a check to make sure the user has enough balance to pay for at least X amount of blocks?
    pub fn do_create_payment_stream(
        sp_id: &ProviderIdFor<T>,
        user_account: &T::AccountId,
        rate: BalanceOf<T>,
    ) -> DispatchResult {
        // Check that the given ID belongs to an actual provider
        ensure!(
            <T::ProvidersPallet as ProvidersInterface>::is_provider(*sp_id),
            Error::<T>::NotAProvider
        );

        // Check that a payment stream between that SP and user does not exist yet
        ensure!(
            !PaymentStreams::<T>::contains_key(sp_id, user_account),
            Error::<T>::PaymentStreamAlreadyExists
        );

        // Check that the user is not flagged as without funds
        ensure!(
            !UsersWithoutFunds::<T>::contains_key(user_account),
            Error::<T>::UserWithoutFunds
        );

        // Check if the user is already registered and, if not, try to hold the deposit
        // TODO: The deposit held should be proportional to the rate of the payment stream (so we hold X amount of blocks of storage)
        // TODO: Also, we should hold the deposit each time a user creates or updates a payment stream, not only when creating its first one like here
        let mut user_payment_streams_count = RegisteredUsers::<T>::get(user_account);
        if user_payment_streams_count == 0 {
            // Check that the user has enough balance to pay the deposit
            let user_balance = T::NativeBalance::reducible_balance(
                &user_account,
                Preservation::Preserve,
                Fortitude::Polite,
            );
            let deposit = T::NewUserDeposit::get();
            ensure!(user_balance >= deposit, Error::<T>::CannotHoldDeposit);

            // Check if we can hold the deposit from the user
            ensure!(
                T::NativeBalance::can_hold(
                    &HoldReason::PaymentStreamStorageDeposit.into(),
                    &user_account,
                    deposit
                ),
                Error::<T>::CannotHoldDeposit
            );

            // Hold the deposit from the user
            T::NativeBalance::hold(
                &HoldReason::PaymentStreamStorageDeposit.into(),
                &user_account,
                deposit,
            )?;
        }

        // Add one to the user's payment streams count
        user_payment_streams_count = user_payment_streams_count
            .checked_add(1)
            .ok_or(ArithmeticError::Overflow)?;
        RegisteredUsers::<T>::insert(user_account, user_payment_streams_count);

        // Store the new payment stream in the PaymentStreams mapping
        // We initiate the last_valid_proof and last_charged_proof with the current block number to be able to keep track of the
        // time passed since the payment stream was originally created
        PaymentStreams::<T>::insert(
            sp_id,
            user_account,
            PaymentStream {
                rate,
                last_valid_proof: frame_system::Pallet::<T>::block_number(),
                last_charged_proof: frame_system::Pallet::<T>::block_number(),
            },
        );

        Ok(())
    }

    /// This function holds the logic that checks if a payment stream can be updated and, if so, updates the payment stream in the PaymentStreams mapping.
    pub fn do_update_payment_stream(
        sp_id: &ProviderIdFor<T>,
        user_account: &T::AccountId,
        new_rate: BalanceOf<T>,
    ) -> DispatchResult {
        // Check that the given ID belongs to an actual provider
        ensure!(
            <T::ProvidersPallet as ProvidersInterface>::is_provider(*sp_id),
            Error::<T>::NotAProvider
        );

        // Ensure that the new rate is not 0 (should use remove_payment_stream instead)
        ensure!(new_rate != Zero::zero(), Error::<T>::UpdateRateToZero);

        // Check that a payment stream between that BSP and user exists
        ensure!(
            PaymentStreams::<T>::contains_key(sp_id, user_account),
            Error::<T>::PaymentStreamNotFound
        );

        // Get the information of the payment stream
        let payment_stream = PaymentStreams::<T>::get(sp_id, user_account)
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
        let amount_charged = Self::do_charge_payment_stream(&sp_id, user_account)?;
        if amount_charged > Zero::zero() {
            // We emit a payment charged event only if the user had to pay for its storage before the payment stream was updated
            Self::deposit_event(Event::<T>::PaymentStreamCharged {
                user_account: user_account.clone(),
                storage_provider_id: *sp_id,
                amount: amount_charged,
            });
        }

        // TODO: We should check if the new rate is lower or higher than the current one, and release or hold the difference in deposit accordingly

        // Update the payment stream in the PaymentStreams mapping
        PaymentStreams::<T>::mutate(sp_id, user_account, |payment_stream| {
            match payment_stream {
                Some(payment_stream) => {
                    payment_stream.rate = new_rate;
                    Ok(())
                }
                None => {
                    // This should never happen as we already checked that the payment stream exists
                    return Err(Error::<T>::PaymentStreamNotFound);
                }
            }
        })?;

        Ok(())
    }

    /// This function holds the logic that checks if a payment stream can be removed and, if so, removes the payment stream from the PaymentStreams mapping,
    /// decreases the user's payment streams count and releases the deposit of that payment stream.
    pub fn do_delete_payment_stream(
        sp_id: &ProviderIdFor<T>,
        user_account: &T::AccountId,
    ) -> DispatchResult {
        // Check that the given ID belongs to an actual provider
        ensure!(
            <T::ProvidersPallet as ProvidersInterface>::is_provider(*sp_id),
            Error::<T>::NotAProvider
        );

        // Check that a payment stream between that BSP and user exists
        ensure!(
            PaymentStreams::<T>::contains_key(sp_id, user_account),
            Error::<T>::PaymentStreamNotFound
        );

        // TODO: What do we do when a user is flagged as without funds? Does the provider assume the loss and we remove the payment stream?
        // Check that the user is not flagged as without funds
        ensure!(
            !UsersWithoutFunds::<T>::contains_key(user_account),
            Error::<T>::UserWithoutFunds
        );

        // Charge the payment stream before deletion to make sure the storage provided by the provider is paid in full for its duration
        let amount_charged = Self::do_charge_payment_stream(&sp_id, user_account)?;
        if amount_charged > Zero::zero() {
            // We emit a payment charged event only if the user had to pay for its storage before deleting the payment stream
            Self::deposit_event(Event::<T>::PaymentStreamCharged {
                user_account: user_account.clone(),
                storage_provider_id: *sp_id,
                amount: amount_charged,
            });
        }

        // Remove the payment stream from the PaymentStreams mapping
        PaymentStreams::<T>::remove(sp_id, user_account);

        // Decrease the user's payment streams count
        let mut user_payment_streams_count = RegisteredUsers::<T>::get(user_account);
        user_payment_streams_count = user_payment_streams_count
            .checked_sub(1)
            .ok_or(ArithmeticError::Underflow)?;
        RegisteredUsers::<T>::insert(user_account, user_payment_streams_count);

        // If the user has no more payment streams, release the deposit
        // TODO: We should actually release the deposit of that specific payment stream. We might have to keep track of the deposit held for each payment stream
        if user_payment_streams_count == 0 {
            // Release the deposit from the user
            T::NativeBalance::release_all(
                &HoldReason::PaymentStreamStorageDeposit.into(),
                &user_account,
                Precision::Exact,
            )?;

            // Remove the user from the UsersWithoutFunds mapping
            UsersWithoutFunds::<T>::remove(user_account);
        }

        Ok(())
    }

    /// This function holds the logic that checks if a payment stream can be charged and, if so, charges the payment stream from the user's balance.
    /// The charge is calculated as: rate * time_passed where time_passed is the time between the last valid proof submitted and the last charged proof of this payment stream.
    /// As such, the last charged proof can't be greater than the last valid proof, and if they are equal then no charge is made.
    ///
    /// TODO: Change charging system to utilize a price index instead of relying on the block number directly
    pub fn do_charge_payment_stream(
        sp_id: &ProviderIdFor<T>,
        user_account: &T::AccountId,
    ) -> Result<BalanceOf<T>, DispatchError> {
        // Check that the given ID belongs to an actual provider
        ensure!(
            <T::ProvidersPallet as ProvidersInterface>::is_provider(*sp_id),
            Error::<T>::NotAProvider
        );

        // Check that a payment stream between that BSP and user exists
        ensure!(
            PaymentStreams::<T>::contains_key(sp_id, user_account),
            Error::<T>::PaymentStreamNotFound
        );

        // Get the information of the payment stream
        let payment_stream = PaymentStreams::<T>::get(sp_id, user_account)
            .ok_or(Error::<T>::PaymentStreamNotFound)?;

        // Note: No need to check if a new proof has been submitted since the last charge as the only consequence of that is charging 0 to the user,
        // and not erroring out helps to be able to call this function without errors when updating or removing a payment stream.

        // Calculate the time passed between the last valid proof and the last charged proof
        let time_passed = expect_or_err!(payment_stream
            .last_valid_proof
            .checked_sub(&payment_stream.last_charged_proof), "Last valid proof should always be greater than or equal to the last charged proof, inconsistency error.",
            Error::<T>::LastChargeGreaterThanLastValidProof);

        // Convert it to the balance type (for math)
        let time_passed_balance_typed = T::BlockNumberToBalance::convert(time_passed);

        // Calculate the amount to charge
        let amount_to_charge = payment_stream
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

            // Return 0 as the amount that has been charged
            Ok(Zero::zero())
        } else {
            // If the user does have enough funds to pay for its storage:

            // Clear the user from the UsersWithoutFunds mapping
            // TODO: Design a more robust way of handling out-of-funds users
            UsersWithoutFunds::<T>::remove(user_account);

            // Get the payment account of the SP
            let sp_payment_account = expect_or_err!(
                <T::ProvidersPallet as ProvidersInterface>::get_provider_payment_account(*sp_id),
                "Storage Provider should exist and have a payment account if its ID exists.",
                Error::<T>::BspInconsistencyError
            );

            // Charge the payment stream from the user's balance
            T::NativeBalance::transfer(
                user_account,
                &sp_payment_account,
                amount_to_charge,
                Preservation::Preserve,
            )?;

            // Set the last charge to the block number of the last valid proof submitted
            PaymentStreams::<T>::mutate(sp_id, user_account, |payment_stream| {
                match payment_stream {
                    Some(payment_stream) => {
                        payment_stream.last_charged_proof = payment_stream.last_valid_proof;
                        Ok(())
                    }
                    None => {
                        // This should never happen as we already checked that the payment stream exists
                        return Err(Error::<T>::PaymentStreamNotFound);
                    }
                }
            })?;

            // Return the amount that has been charged
            Ok(amount_to_charge)
        }
    }
}

impl<T: pallet::Config> PaymentStreamsInterface for pallet::Pallet<T> {
    type AccountId = T::AccountId;
    type ProviderId = ProviderIdFor<T>;
    type Balance = T::NativeBalance;
    type BlockNumber = BlockNumberFor<T>;
    type PaymentStream = PaymentStream<T>;

    fn create_payment_stream(
        sp_id: &Self::ProviderId,
        user_account: &Self::AccountId,
        rate: <Self::Balance as Inspect<Self::AccountId>>::Balance,
    ) -> DispatchResult {
        // Execute the logic to create a payment stream
        Self::do_create_payment_stream(sp_id, user_account, rate)?;

        // Emit the corresponding event
        Self::deposit_event(Event::<T>::PaymentStreamCreated {
            user_account: user_account.clone(),
            storage_provider_id: *sp_id,
            rate,
        });

        // Return a successful DispatchResult
        Ok(())
    }

    fn update_payment_stream(
        sp_id: &Self::ProviderId,
        user_account: &Self::AccountId,
        new_rate: <Self::Balance as Inspect<Self::AccountId>>::Balance,
    ) -> DispatchResult {
        // Execute the logic to update a payment stream
        Self::do_update_payment_stream(sp_id, user_account, new_rate)?;

        // Emit the corresponding event
        Self::deposit_event(Event::<T>::PaymentStreamUpdated {
            user_account: user_account.clone(),
            storage_provider_id: *sp_id,
            new_rate,
        });

        // Return a successful DispatchResult
        Ok(())
    }

    fn delete_payment_stream(
        sp_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> DispatchResult {
        // Execute the logic to delete a payment stream
        Self::do_delete_payment_stream(sp_id, user_account)?;

        // Emit the corresponding event
        Self::deposit_event(Event::<T>::PaymentStreamDeleted {
            user_account: user_account.clone(),
            storage_provider_id: *sp_id,
        });

        // Return a successful DispatchResult
        Ok(())
    }

    fn update_last_valid_proof(
        sp_id: &Self::ProviderId,
        user_account: &Self::AccountId,
        last_valid_proof_block: Self::BlockNumber,
    ) -> DispatchResult {
        // Check that the given ID belongs to an actual provider
        ensure!(
            <T::ProvidersPallet as ProvidersInterface>::is_provider(*sp_id),
            Error::<T>::NotAProvider
        );

        // Ensure that the last valid proof block that is being submitted is not greater than the current block number
        ensure!(
            last_valid_proof_block <= frame_system::Pallet::<T>::block_number(),
            Error::<T>::InvalidLastValidProofBlockNumber
        );

        // Get the information of the payment stream to update
        let payment_stream = PaymentStreams::<T>::get(sp_id, user_account)
            .ok_or(Error::<T>::PaymentStreamNotFound)?;

        // Ensure that the new last valid proof block is greater than the last valid proof block of the payment stream
        ensure!(
            last_valid_proof_block > payment_stream.last_valid_proof,
            Error::<T>::InvalidLastValidProofBlockNumber
        );

        // Update the last valid proof block of the payment stream
        PaymentStreams::<T>::mutate(sp_id, user_account, |payment_stream| {
            match payment_stream {
                Some(payment_stream) => {
                    payment_stream.last_valid_proof = last_valid_proof_block;
                    Ok(())
                }
                None => {
                    // This should never happen as we already checked that the payment stream exists
                    return Err(Error::<T>::PaymentStreamNotFound);
                }
            }
        })?;

        // Return a successful DispatchResult
        Ok(())
    }

    fn get_payment_stream_info(
        sp_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> Option<Self::PaymentStream> {
        // Return the payment stream information
        PaymentStreams::<T>::get(sp_id, user_account)
    }
}
