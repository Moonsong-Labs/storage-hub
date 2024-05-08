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
    pub fn do_create_payment_stream(
        bsp_account: &T::AccountId,
        user_account: &T::AccountId,
        rate: BalanceOf<T>,
    ) -> DispatchResult {
        // Get the BSP ID of the BSP account
        let bsp_id = <T::Providers as storage_hub_traits::ProvidersInterface>::get_provider(
            bsp_account.clone(),
        )
        .ok_or(Error::<T>::NotABackupStorageProvider)?;

        // Check that a payment stream between that BSP and user does not exist yet
        ensure!(
            !PaymentStreams::<T>::contains_key(bsp_id, user_account),
            Error::<T>::PaymentStreamAlreadyExists
        );

        // Check that the user is not flagged as without funds
        ensure!(
            !UsersWithoutFunds::<T>::contains_key(user_account),
            Error::<T>::UserWithoutFunds
        );

        // Check if the user is already registered and, if not, try to hold the deposit
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
            bsp_id,
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
        bsp_account: &T::AccountId,
        user_account: &T::AccountId,
        new_rate: BalanceOf<T>,
    ) -> DispatchResult {
        // Get the BSP ID of the BSP account
        let bsp_id = <T::Providers as storage_hub_traits::ProvidersInterface>::get_provider(
            bsp_account.clone(),
        )
        .ok_or(Error::<T>::NotABackupStorageProvider)?;

        // Ensure that the new rate is not 0 (should use remove_payment_stream instead)
        ensure!(new_rate != Zero::zero(), Error::<T>::UpdateRateToZero);

        // Check that a payment stream between that BSP and user exists
        ensure!(
            PaymentStreams::<T>::contains_key(bsp_id, user_account),
            Error::<T>::PaymentStreamNotFound
        );

        // Get the information of the payment stream
        let mut payment_stream = PaymentStreams::<T>::get(bsp_id, user_account)
            .ok_or(Error::<T>::PaymentStreamNotFound)?;

        // Verify that the new rate is different from the current one
        ensure!(
            payment_stream.rate != new_rate,
            Error::<T>::UpdateRateToSameRate
        );

        // Charge the payment stream with the old rate before updating it to prevent abuse
        let amount_charged = Self::do_charge_payment_stream(&bsp_account, user_account)?;
        if amount_charged > Zero::zero() {
            // We emit a payment charged event only if the user had to pay for its storage before the payment stream was updated
            Self::deposit_event(Event::<T>::PaymentCharged {
                user_account: user_account.clone(),
                backup_storage_provider_id: bsp_id,
                amount: amount_charged,
            });
        }

        // Check that the user is not flagged as without funds (after charging, to make sure it makes sense to update the payment stream)
        ensure!(
            !UsersWithoutFunds::<T>::contains_key(user_account),
            Error::<T>::UserWithoutFunds
        );

        // Update the payment stream in the PaymentStreams mapping
        payment_stream.rate = new_rate;
        PaymentStreams::<T>::insert(bsp_id, user_account, payment_stream);

        Ok(())
    }

    /// This function holds the logic that checks if a payment stream can be removed and, if so, removes the payment stream from the PaymentStreams mapping,
    /// decreases the user's payment streams count and, if the user has no more payment streams, releases the deposit.
    pub fn do_delete_payment_stream(
        bsp_account: &T::AccountId,
        user_account: &T::AccountId,
    ) -> DispatchResult {
        // Get the BSP ID of the BSP account
        let bsp_id = <T::Providers as storage_hub_traits::ProvidersInterface>::get_provider(
            bsp_account.clone(),
        )
        .ok_or(Error::<T>::NotABackupStorageProvider)?;

        // Check that a payment stream between that BSP and user exists
        ensure!(
            PaymentStreams::<T>::contains_key(bsp_id, user_account),
            Error::<T>::PaymentStreamNotFound
        );

        // Charge the payment stream before deletion to make sure the storage provided by the provider is paid in full for its duration
        let amount_charged = Self::do_charge_payment_stream(&bsp_account, user_account)?;
        if amount_charged > Zero::zero() {
            // We emit a payment charged event only if the user had to pay for its storage before deleting the payment stream
            Self::deposit_event(Event::<T>::PaymentCharged {
                user_account: user_account.clone(),
                backup_storage_provider_id: bsp_id,
                amount: amount_charged,
            });
        }

        // TODO: What do we do when a user is flagged as without funds? Does the provider assume the loss and we remove the payment stream?
        // Check that the user is not flagged as without funds (after charging, to make sure it makes sense to delete the payment stream)
        ensure!(
            !UsersWithoutFunds::<T>::contains_key(user_account),
            Error::<T>::UserWithoutFunds
        );

        // Remove the payment stream from the PaymentStreams mapping
        PaymentStreams::<T>::remove(bsp_id, user_account);

        // Decrease the user's payment streams count
        let mut user_payment_streams_count = RegisteredUsers::<T>::get(user_account);
        user_payment_streams_count = user_payment_streams_count
            .checked_sub(1)
            .ok_or(ArithmeticError::Underflow)?;
        RegisteredUsers::<T>::insert(user_account, user_payment_streams_count);

        // If the user has no more payment streams, release the deposit
        if user_payment_streams_count == 0 {
            // Release the deposit from the user
            T::NativeBalance::release_all(
                &HoldReason::PaymentStreamStorageDeposit.into(),
                &user_account,
                Precision::Exact,
            )?;
        }

        Ok(())
    }

    /// This function holds the logic that checks if a payment stream can be charged and, if so, charges the payment stream from the user's balance.
    /// The charge is calculated as: rate * time_passed where time_passed is the time between the last valid proof submitted and the last charged proof of this payment stream.
    /// As such, the last charged proof can't be greater than the last valid proof, and if they are equal then no charge is made.
    ///
    /// Note: right now, the other functions receive the BSP's account ID because it is needed to transfer the user payment to it in this function, but the idea is to change the
    /// BSP struct to hold a `payment_address` where the payment should be received, and then the functions of this pallet will receive the BSP's ID instead.
    pub fn do_charge_payment_stream(
        bsp_account: &T::AccountId,
        user_account: &T::AccountId,
    ) -> Result<BalanceOf<T>, DispatchError> {
        // Get the BSP ID of the BSP account
        let bsp_id = <T::Providers as storage_hub_traits::ProvidersInterface>::get_provider(
            bsp_account.clone(),
        )
        .ok_or(Error::<T>::NotABackupStorageProvider)?;

        // Check that a payment stream between that BSP and user exists
        ensure!(
            PaymentStreams::<T>::contains_key(bsp_id, user_account),
            Error::<T>::PaymentStreamNotFound
        );

        // Get the information of the payment stream
        let mut payment_stream = PaymentStreams::<T>::get(bsp_id, user_account)
            .ok_or(Error::<T>::PaymentStreamNotFound)?;

        // Note: No need to check if a new proof has been submitted since the last charge as the only consecuence of that is charging 0 to the user,
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
            // Flag it in the UsersWithoutFunds mapping and emit the UserWithoutFunds event
            UsersWithoutFunds::<T>::insert(user_account, ());
            Self::deposit_event(Event::<T>::UserWithoutFunds {
                who: user_account.clone(),
            });

            // Return 0 as the amount that has been charged
            Ok(Zero::zero())
        } else {
            // If the user does have enough funds to pay for its storage:

            // Charge the payment stream from the user's balance
            T::NativeBalance::transfer(
                user_account,
                bsp_account,
                amount_to_charge,
                Preservation::Preserve,
            )?;

            // Set the last charge to the current block number
            payment_stream.last_charged_proof = payment_stream.last_valid_proof;
            PaymentStreams::<T>::insert(bsp_id, user_account, payment_stream);

            // Return the amount that has been charged
            Ok(amount_to_charge)
        }
    }
}

impl<T: pallet::Config> PaymentStreamsInterface for pallet::Pallet<T> {
    type AccountId = T::AccountId;
    type Balance = T::NativeBalance;
    type BlockNumber = BlockNumberFor<T>;
    type PaymentStream = PaymentStream<T>;

    fn create_payment_stream(
        bsp_account: &Self::AccountId,
        user_account: &Self::AccountId,
        rate: <Self::Balance as Inspect<Self::AccountId>>::Balance,
    ) -> DispatchResult {
        // Execute the logic to create a payment stream
        Self::do_create_payment_stream(bsp_account, user_account, rate)?;

        // Get the BSP ID of the BSP account
        let bsp_id = <T::Providers as storage_hub_traits::ProvidersInterface>::get_provider(
            bsp_account.clone(),
        )
        .ok_or(Error::<T>::NotABackupStorageProvider)?;

        // Emit the corresponding event
        Self::deposit_event(Event::<T>::PaymentStreamCreated {
            user_account: user_account.clone(),
            backup_storage_provider_id: bsp_id,
            rate,
        });

        // Return a successful DispatchResult
        Ok(())
    }

    fn update_payment_stream(
        bsp_account: &Self::AccountId,
        user_account: &Self::AccountId,
        new_rate: <Self::Balance as Inspect<Self::AccountId>>::Balance,
    ) -> DispatchResult {
        // Execute the logic to update a payment stream
        Self::do_update_payment_stream(bsp_account, user_account, new_rate)?;

        // Get the BSP ID of the BSP account
        let bsp_id = <T::Providers as storage_hub_traits::ProvidersInterface>::get_provider(
            bsp_account.clone(),
        )
        .ok_or(Error::<T>::NotABackupStorageProvider)?;

        // Emit the corresponding event
        Self::deposit_event(Event::<T>::PaymentStreamUpdated {
            user_account: user_account.clone(),
            backup_storage_provider_id: bsp_id,
            new_rate,
        });

        // Return a successful DispatchResult
        Ok(())
    }

    fn delete_payment_stream(
        bsp_account: &Self::AccountId,
        user_account: &Self::AccountId,
    ) -> DispatchResult {
        // Execute the logic to delete a payment stream
        Self::do_delete_payment_stream(bsp_account, user_account)?;

        // Get the BSP ID of the BSP account
        let bsp_id = <T::Providers as storage_hub_traits::ProvidersInterface>::get_provider(
            bsp_account.clone(),
        )
        .ok_or(Error::<T>::NotABackupStorageProvider)?;

        // Emit the corresponding event
        Self::deposit_event(Event::<T>::PaymentStreamRemoved {
            user_account: user_account.clone(),
            backup_storage_provider_id: bsp_id,
        });

        // Return a successful DispatchResult
        Ok(())
    }

    fn update_last_valid_proof(
        bsp_account: &Self::AccountId,
        user_account: &Self::AccountId,
        last_valid_proof_block: Self::BlockNumber,
    ) -> DispatchResult {
        let bsp_id = <T::Providers as storage_hub_traits::ProvidersInterface>::get_provider(
            bsp_account.clone(),
        )
        .ok_or(Error::<T>::NotABackupStorageProvider)?;
        let mut payment_stream = PaymentStreams::<T>::get(bsp_id, user_account)
            .ok_or(Error::<T>::PaymentStreamNotFound)?;
        payment_stream.last_valid_proof = last_valid_proof_block;
        PaymentStreams::<T>::insert(bsp_id, user_account, payment_stream);
        Ok(())
    }

    fn get_payment_stream_info(
        bsp_account: &Self::AccountId,
        user_account: &Self::AccountId,
    ) -> Option<Self::PaymentStream> {
        let bsp_id = <T::Providers as storage_hub_traits::ProvidersInterface>::get_provider(
            bsp_account.clone(),
        )?;
        PaymentStreams::<T>::get(bsp_id, user_account)
    }
}
