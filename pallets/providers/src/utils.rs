use crate::types::{Bucket, MainStorageProvider, MultiAddress, StorageProvider};
use crate::*;
use codec::{Decode, Encode};
use frame_support::{
    dispatch::{DispatchResultWithPostInfo, Pays},
    ensure,
    pallet_prelude::DispatchResult,
    sp_runtime::{
        traits::{CheckedAdd, CheckedMul, CheckedSub, One, Saturating, Zero},
        ArithmeticError, BoundedVec, DispatchError,
    },
    traits::{
        fungible::{Inspect, InspectHold, MutateHold},
        tokens::{Fortitude, Precision, Preservation, Restriction},
        Get, Randomness,
    },
};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_storage_providers_runtime_api::{
    GetBspInfoError, QueryAvailableStorageCapacityError, QueryEarliestChangeCapacityBlockError,
    QueryStorageProviderCapacityError,
};
use shp_file_metadata::FileMetadata;
use shp_traits::{
    MutateBucketsInterface, MutateChallengeableProvidersInterface, MutateProvidersInterface,
    MutateStorageProvidersInterface, PaymentStreamsInterface, ProofSubmittersInterface,
    ReadBucketsInterface, ReadChallengeableProvidersInterface, ReadProvidersInterface,
    ReadStorageProvidersInterface, SystemMetricsInterface,
};
use sp_std::vec::Vec;
use types::{ProviderId, StorageProviderId};

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
    /// This function holds the logic that checks if a user can request to sign up as a Main Storage Provider
    /// and, if so, stores the request in the SignUpRequests mapping
    pub fn do_request_msp_sign_up(msp_info: &MainStorageProvider<T>) -> DispatchResult {
        // todo!("If this comment is present, it means this function is still incomplete even though it compiles.")

        let who = &msp_info.owner_account;

        // Check that the user does not have a pending sign up request
        ensure!(
            SignUpRequests::<T>::get(&who).is_none(),
            Error::<T>::SignUpRequestPending
        );

        // Check that the account is not already registered either as a Main Storage Provider or a Backup Storage Provider
        ensure!(
            AccountIdToMainStorageProviderId::<T>::get(who).is_none()
                && AccountIdToBackupStorageProviderId::<T>::get(who).is_none(),
            Error::<T>::AlreadyRegistered
        );

        // Check that the multiaddresses vector is not empty (SPs have to register with at least one)
        ensure!(
            !msp_info.multiaddresses.is_empty(),
            Error::<T>::NoMultiAddress
        );

        // TODO: Check that the multiaddresses are valid
        /* for multiaddress in msp_info.multiaddresses.iter() {
            let multiaddress_vec = multiaddress.to_vec();
            let valid_multiaddress = Multiaddr::try_from(multiaddress_vec);
            match valid_multiaddress {
                Ok(_) => (),
                Err(_) => return Err(Error::<T>::InvalidMultiAddress.into()),
            }
        } */

        // Check that the data to be stored is bigger than the minimum required by the runtime
        ensure!(
            msp_info.capacity >= T::SpMinCapacity::get(),
            Error::<T>::StorageTooLow
        );

        // Calculate how much deposit will the signer have to pay to register with this amount of data
        let capacity_over_minimum = msp_info
            .capacity
            .checked_sub(&T::SpMinCapacity::get())
            .ok_or(Error::<T>::StorageTooLow)?;
        let deposit_for_capacity_over_minimum = T::DepositPerData::get()
            .checked_mul(&capacity_over_minimum.into())
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;
        let deposit = T::SpMinDeposit::get()
            .checked_add(&deposit_for_capacity_over_minimum)
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;

        // Check if the user has enough balance to pay the deposit
        let user_balance =
            T::NativeBalance::reducible_balance(who, Preservation::Preserve, Fortitude::Polite);
        ensure!(user_balance >= deposit, Error::<T>::NotEnoughBalance);

        // Check if we can hold the deposit from the user
        ensure!(
            T::NativeBalance::can_hold(&HoldReason::StorageProviderDeposit.into(), who, deposit),
            Error::<T>::CannotHoldDeposit
        );

        // Hold the deposit from the user
        T::NativeBalance::hold(&HoldReason::StorageProviderDeposit.into(), who, deposit)?;

        // Store the sign up request in the SignUpRequests mapping
        SignUpRequests::<T>::insert(
            who,
            (
                StorageProvider::MainStorageProvider(msp_info.clone()),
                frame_system::Pallet::<T>::block_number(),
            ),
        );

        Ok(())
    }

    /// This function holds the logic that checks if a user can request to sign up as a Backup Storage Provider
    /// and, if so, stores the request in the SignUpRequests mapping
    pub fn do_request_bsp_sign_up(bsp_info: &BackupStorageProvider<T>) -> DispatchResult {
        // todo!("If this comment is present, it means this function is still incomplete even though it compiles.")

        let who = &bsp_info.owner_account;

        // Check that the user does not have a pending sign up request
        ensure!(
            SignUpRequests::<T>::get(&who).is_none(),
            Error::<T>::SignUpRequestPending
        );

        // Check that the account is not already registered either as a Main Storage Provider or a Backup Storage Provider
        ensure!(
            AccountIdToMainStorageProviderId::<T>::get(who).is_none()
                && AccountIdToBackupStorageProviderId::<T>::get(who).is_none(),
            Error::<T>::AlreadyRegistered
        );

        // Check that the multiaddresses vector is not empty (SPs have to register with at least one)
        ensure!(
            !bsp_info.multiaddresses.is_empty(),
            Error::<T>::NoMultiAddress
        );

        // TODO: Check that the multiaddresses are valid
        /* for multiaddress in bsp_info.multiaddresses.iter() {
            let multiaddress_vec = multiaddress.to_vec();
            let valid_multiaddress = Multiaddr::try_from(multiaddress_vec);
            match valid_multiaddress {
                Ok(_) => (),
                Err(_) => return Err(Error::<T>::InvalidMultiAddress.into()),
            }
        } */

        // Check that the data to be stored is bigger than the minimum required by the runtime
        ensure!(
            bsp_info.capacity >= T::SpMinCapacity::get(),
            Error::<T>::StorageTooLow
        );

        // Calculate how much deposit will the signer have to pay to register with this amount of data
        let capacity_over_minimum = bsp_info
            .capacity
            .checked_sub(&T::SpMinCapacity::get())
            .ok_or(Error::<T>::StorageTooLow)?;
        let deposit_for_capacity_over_minimum = T::DepositPerData::get()
            .checked_mul(&capacity_over_minimum.into())
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;
        let deposit = T::SpMinDeposit::get()
            .checked_add(&deposit_for_capacity_over_minimum)
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;

        // Check if the user has enough balance to pay the deposit
        let user_balance =
            T::NativeBalance::reducible_balance(who, Preservation::Preserve, Fortitude::Polite);
        ensure!(user_balance >= deposit, Error::<T>::NotEnoughBalance);

        // Check if we can hold the deposit from the user
        ensure!(
            T::NativeBalance::can_hold(&HoldReason::StorageProviderDeposit.into(), who, deposit),
            Error::<T>::CannotHoldDeposit
        );

        // Hold the deposit from the user
        T::NativeBalance::hold(&HoldReason::StorageProviderDeposit.into(), who, deposit)?;

        // Store the sign up request in the SignUpRequests mapping
        SignUpRequests::<T>::insert(
            who,
            (
                StorageProvider::BackupStorageProvider(bsp_info.clone()),
                frame_system::Pallet::<T>::block_number(),
            ),
        );

        Ok(())
    }

    /// This function holds the logic that checks if a user can cancel a sign up request as a Storage Provider
    /// and, if so, removes the request from the SignUpRequests mapping
    pub fn do_cancel_sign_up(who: &T::AccountId) -> DispatchResult {
        // Check that the signer has requested to sign up as a Storage Provider
        SignUpRequests::<T>::get(who).ok_or(Error::<T>::SignUpNotRequested)?;

        // Remove the sign up request from the SignUpRequests mapping
        SignUpRequests::<T>::remove(who);

        // Return the deposit to the signer
        // We return all held funds as there's no possibility of the user having another _valid_ hold with this pallet
        T::NativeBalance::release_all(
            &HoldReason::StorageProviderDeposit.into(),
            who,
            frame_support::traits::tokens::Precision::Exact,
        )?;

        Ok(())
    }

    /// This function dispatches the logic to confirm the sign up of a user as a Storage Provider
    /// It checks if the user has requested to sign up, and if so, it dispatches the corresponding logic
    /// according to the type of Storage Provider that the user is trying to sign up as
    pub fn do_confirm_sign_up(who: &T::AccountId) -> DispatchResult {
        // Check that the signer has requested to sign up as a Storage Provider
        let (sp, request_block) =
            SignUpRequests::<T>::get(who).ok_or(Error::<T>::SignUpNotRequested)?;

        // Get the ProviderId by using the AccountId as the seed for a random generator
        let (sp_id, block_number_when_random) =
            T::ProvidersRandomness::random(who.encode().as_ref());

        // Check that the maximum block number after which the randomness is invalid is greater than or equal to the block number when the
        // request was made to ensure that the randomness was not known when the request was made
        ensure!(
            block_number_when_random >= request_block,
            Error::<T>::RandomnessNotValidYet
        );

        // Check what type of Storage Provider the signer is trying to sign up as and dispatch the corresponding logic
        match sp {
            StorageProvider::MainStorageProvider(msp_info) => {
                Self::do_msp_sign_up(who, sp_id, &msp_info, request_block)?;
            }
            StorageProvider::BackupStorageProvider(bsp_info) => {
                Self::do_bsp_sign_up(who, sp_id, &bsp_info, request_block)?;
            }
        }

        Ok(())
    }

    /// This function holds the logic that confirms the sign up of a user as a Main Storage Provider
    /// It updates the storage to add the new Main Storage Provider, increments the counter of Main Storage Providers,
    /// and removes the sign up request from the SignUpRequests mapping
    pub fn do_msp_sign_up(
        who: &T::AccountId,
        msp_id: MainStorageProviderId<T>,
        msp_info: &MainStorageProvider<T>,
        request_block: BlockNumberFor<T>,
    ) -> DispatchResult {
        // Check that the current block number is not greater than the block number when the request was made plus the maximum amount of
        // blocks that we allow the user to wait for valid randomness (should be at least more than an epoch if using BABE's RandomnessFromOneEpochAgo)
        // We do this to ensure that a user cannot wait indefinitely for randomness that suits them
        ensure!(
            frame_system::Pallet::<T>::block_number()
                < request_block + T::MaxBlocksForRandomness::get(),
            Error::<T>::SignUpRequestExpired
        );

        // Insert the MainStorageProviderId into the mapping
        AccountIdToMainStorageProviderId::<T>::insert(who, msp_id);

        // Save the MainStorageProvider information in storage
        MainStorageProviders::<T>::insert(&msp_id, msp_info);

        // Increment the counter of Main Storage Providers registered
        let new_amount_of_msps = MspCount::<T>::get()
            .checked_add(&T::SpCount::one())
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;
        MspCount::<T>::set(new_amount_of_msps);

        // Remove the sign up request from the SignUpRequests mapping
        SignUpRequests::<T>::remove(who);

        // Emit the corresponding event
        Self::deposit_event(Event::<T>::MspSignUpSuccess {
            who: who.clone(),
            multiaddresses: msp_info.multiaddresses.clone(),
            capacity: msp_info.capacity,
            value_prop: msp_info.value_prop.clone(),
        });

        Ok(())
    }

    /// This function holds the logic that confirms the sign up of a user as a Backup Storage Provider
    /// It updates the storage to add the new Backup Storage Provider, increments the counter of Backup Storage Providers,
    /// increments the total capacity of the network (which is the sum of all BSPs capacities), and removes the sign up request
    /// from the SignUpRequests mapping
    pub fn do_bsp_sign_up(
        who: &T::AccountId,
        bsp_id: BackupStorageProviderId<T>,
        bsp_info: &BackupStorageProvider<T>,
        request_block: BlockNumberFor<T>,
    ) -> DispatchResult {
        // Check that the current block number is not greater than the block number when the request was made plus the maximum amount of
        // blocks that we allow the user to wait for valid randomness (should be at least more than an epoch if using BABE's RandomnessFromOneEpochAgo)
        // We do this to ensure that a user cannot wait indefinitely for randomness that suits them
        ensure!(
            frame_system::Pallet::<T>::block_number()
                < request_block + T::MaxBlocksForRandomness::get(),
            Error::<T>::SignUpRequestExpired
        );

        // Insert the BackupStorageProviderId into the mapping
        AccountIdToBackupStorageProviderId::<T>::insert(who, bsp_id);

        // Save the BackupStorageProvider information in storage
        BackupStorageProviders::<T>::insert(&bsp_id, bsp_info.clone());

        // Increment the total capacity of the network (which is the sum of all BSPs capacities)
        TotalBspsCapacity::<T>::mutate(|n| match n.checked_add(&bsp_info.capacity) {
            Some(new_total_bsp_capacity) => {
                *n = new_total_bsp_capacity;
                Ok(())
            }
            None => Err(DispatchError::Arithmetic(ArithmeticError::Overflow)),
        })?;

        // Increment the counter of Backup Storage Providers registered
        let new_amount_of_bsps = BspCount::<T>::get()
            .checked_add(&T::SpCount::one())
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;
        BspCount::<T>::set(new_amount_of_bsps);

        // Remove the sign up request from the SignUpRequests mapping
        SignUpRequests::<T>::remove(who);

        // Increase global reputation weight
        GlobalBspsReputationWeight::<T>::mutate(|n| {
            *n = n.saturating_add(bsp_info.reputation_weight);
        });

        // Emit the corresponding event
        Self::deposit_event(Event::<T>::BspSignUpSuccess {
            who: who.clone(),
            multiaddresses: bsp_info.multiaddresses.clone(),
            capacity: bsp_info.capacity,
        });

        Ok(())
    }

    /// This function holds the logic that checks if a user can sign off as a Main Storage Provider
    /// and, if so, updates the storage to remove the user as a Main Storage Provider, decrements the counter of Main Storage Providers,
    /// and returns the deposit to the user
    pub fn do_msp_sign_off(who: &T::AccountId) -> DispatchResult {
        // Check that the signer is registered as a MSP and get its info
        let msp_id =
            AccountIdToMainStorageProviderId::<T>::get(who).ok_or(Error::<T>::NotRegistered)?;

        let msp = expect_or_err!(
            MainStorageProviders::<T>::get(&msp_id),
            "MSP is registered (has a MSP ID), it should also have metadata",
            Error::<T>::SpRegisteredButDataNotFound
        );

        // Check that the MSP has no storage assigned to it (no buckets or data used by it)
        ensure!(
            msp.capacity_used == T::StorageDataUnit::zero(),
            Error::<T>::StorageStillInUse
        );

        // Update the MSPs storage, removing the signer as an MSP
        AccountIdToMainStorageProviderId::<T>::remove(who);
        MainStorageProviders::<T>::remove(&msp_id);

        // Return the deposit to the signer (if all funds cannot be returned, it will fail and revert with the reason)
        T::NativeBalance::release_all(
            &HoldReason::StorageProviderDeposit.into(),
            who,
            frame_support::traits::tokens::Precision::Exact,
        )?;

        // Decrement the storage that holds total amount of MSPs currently in the system
        MspCount::<T>::mutate(|n| {
            let new_amount_of_msps = n.checked_sub(&T::SpCount::one());
            match new_amount_of_msps {
                Some(new_amount_of_msps) => {
                    *n = new_amount_of_msps;
                    Ok(())
                }
                None => Err(DispatchError::Arithmetic(ArithmeticError::Underflow)),
            }
        })?;

        Ok(())
    }

    /// This function holds the logic that checks if a user can sign off as a Backup Storage Provider
    /// and, if so, updates the storage to remove the user as a Backup Storage Provider, decrements the counter of Backup Storage Providers,
    /// decrements the total capacity of the network (which is the sum of all BSPs capacities), and returns the deposit to the user
    pub fn do_bsp_sign_off(who: &T::AccountId) -> DispatchResult {
        // Check that the signer is registered as a BSP and get its info
        let bsp_id =
            AccountIdToBackupStorageProviderId::<T>::get(who).ok_or(Error::<T>::NotRegistered)?;

        let bsp = expect_or_err!(
            BackupStorageProviders::<T>::get(&bsp_id),
            "BSP is registered (has a BSP ID), it should also have metadata",
            Error::<T>::SpRegisteredButDataNotFound
        );

        // Check that the BSP has no storage assigned to it (it is not currently storing any files)
        ensure!(
            bsp.capacity_used == T::StorageDataUnit::zero(),
            Error::<T>::StorageStillInUse
        );

        // Update the BSPs storage, removing the signer as an BSP
        AccountIdToBackupStorageProviderId::<T>::remove(who);
        BackupStorageProviders::<T>::remove(&bsp_id);

        // Update the total capacity of the network (which is the sum of all BSPs capacities)
        TotalBspsCapacity::<T>::mutate(|n| match n.checked_sub(&bsp.capacity) {
            Some(new_total_bsp_capacity) => {
                *n = new_total_bsp_capacity;
                Ok(())
            }
            None => Err(DispatchError::Arithmetic(ArithmeticError::Underflow)),
        })?;

        // Return the deposit to the signer (if all funds cannot be returned, it will fail and revert with the reason)
        T::NativeBalance::release_all(
            &HoldReason::StorageProviderDeposit.into(),
            who,
            frame_support::traits::tokens::Precision::Exact,
        )?;

        // Decrement the storage that holds total amount of BSPs currently in the system
        BspCount::<T>::mutate(|n| {
            let new_amount_of_bsps = n.checked_sub(&T::SpCount::one());
            match new_amount_of_bsps {
                Some(new_amount_of_bsps) => {
                    *n = new_amount_of_bsps;
                    Ok(())
                }
                None => Err(DispatchError::Arithmetic(ArithmeticError::Underflow)),
            }
        })?;

        // Decrease global reputation weight
        GlobalBspsReputationWeight::<T>::mutate(|n| {
            *n = n.saturating_sub(bsp.reputation_weight);
        });

        Ok(())
    }

    /// This function is in charge of dispatching the logic to change the capacity of a Storage Provider
    /// It checks if the signer is registered as a SP and dispatches the corresponding function
    /// that checks if the user can change its capacity and, if so, updates the storage to reflect the new capacity
    pub fn do_change_capacity(
        who: &T::AccountId,
        new_capacity: StorageDataUnit<T>,
    ) -> Result<StorageDataUnit<T>, DispatchError> {
        // Check that the new capacity is not zero (there are specific functions to sign off as a SP)
        ensure!(
            new_capacity != T::StorageDataUnit::zero(),
            Error::<T>::NewCapacityCantBeZero
        );

        // Check that the signer is registered as a SP and dispatch the corresponding function, getting its old capacity
        let old_capacity = if let Some(msp_id) = AccountIdToMainStorageProviderId::<T>::get(who) {
            Self::do_change_capacity_msp(who, msp_id, new_capacity)?
        } else if let Some(bsp_id) = AccountIdToBackupStorageProviderId::<T>::get(who) {
            Self::do_change_capacity_bsp(who, bsp_id, new_capacity)?
        } else {
            return Err(Error::<T>::NotRegistered.into());
        };

        Ok(old_capacity)
    }

    /// This function holds the logic that checks if a user can change its capacity as a Main Storage Provider
    /// and, if so, updates the storage to reflect the new capacity, modifying the user's deposit accordingly
    /// and returning the old capacity if successful
    pub fn do_change_capacity_msp(
        account_id: &T::AccountId,
        msp_id: MainStorageProviderId<T>,
        new_capacity: StorageDataUnit<T>,
    ) -> Result<StorageDataUnit<T>, DispatchError> {
        // Check that the MSP is registered and get its info
        let mut msp = MainStorageProviders::<T>::get(&msp_id).ok_or(Error::<T>::NotRegistered)?;

        // Check that the new capacity is different from the current capacity
        ensure!(
            new_capacity != msp.capacity,
            Error::<T>::NewCapacityEqualsCurrentCapacity
        );

        // Check that enough time has passed since the last capacity change
        ensure!(
            frame_system::Pallet::<T>::block_number()
                >= msp.last_capacity_change + T::MinBlocksBetweenCapacityChanges::get(),
            Error::<T>::NotEnoughTimePassed
        );

        // Check that the new capacity is bigger than the minimum required by the runtime
        ensure!(
            new_capacity >= T::SpMinCapacity::get(),
            Error::<T>::StorageTooLow
        );

        // Check that the new capacity is bigger than the current used capacity by the MSP
        ensure!(
            new_capacity >= msp.capacity_used,
            Error::<T>::NewCapacityLessThanUsedStorage
        );

        // Calculate how much deposit will the signer have to pay to register with this amount of data
        let capacity_over_minimum = new_capacity
            .checked_sub(&T::SpMinCapacity::get())
            .ok_or(Error::<T>::StorageTooLow)?;
        let deposit_for_capacity_over_minimum = T::DepositPerData::get()
            .checked_mul(&capacity_over_minimum.into())
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;
        let new_deposit = T::SpMinDeposit::get()
            .checked_add(&deposit_for_capacity_over_minimum)
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;

        // Check how much has the MSP already deposited for the current capacity
        let current_deposit = T::NativeBalance::balance_on_hold(
            &HoldReason::StorageProviderDeposit.into(),
            account_id,
        );

        // Check if the new deposit is bigger or smaller than the current deposit
        // Note: we do not check directly capacities as, for example, a bigger new_capacity could entail a smaller deposit
        // because of changes in storage pricing, so we check the difference in deposits instead
        if new_deposit > current_deposit {
            // If the new deposit is bigger than the current deposit, more balance has to be held from the user
            Self::hold_balance(account_id, current_deposit, new_deposit)?;
        } else if new_deposit < current_deposit {
            // If the new deposit is smaller than the current deposit, some balance has to be released to the user
            Self::release_balance(account_id, current_deposit, new_deposit)?;
        }

        // Get the MSP's old capacity
        let old_capacity = msp.capacity;

        // Update the MSP's storage, modifying the capacity and the last capacity change block number
        msp.capacity = new_capacity;
        msp.last_capacity_change = frame_system::Pallet::<T>::block_number();
        MainStorageProviders::<T>::insert(&msp_id, msp);

        // Return the old capacity
        Ok(old_capacity)
    }

    /// This function holds the logic that checks if a user can change its capacity as a Backup Storage Provider
    /// and, if so, updates the storage to reflect the new capacity, modifying the user's deposit accordingly
    /// and returning the old capacity if successful
    pub fn do_change_capacity_bsp(
        account_id: &T::AccountId,
        bsp_id: BackupStorageProviderId<T>,
        new_capacity: StorageDataUnit<T>,
    ) -> Result<StorageDataUnit<T>, DispatchError> {
        // Check that the BSP is registered and get its info
        let mut bsp = BackupStorageProviders::<T>::get(&bsp_id).ok_or(Error::<T>::NotRegistered)?;

        // Check that the new capacity is different from the current capacity
        ensure!(
            new_capacity != bsp.capacity,
            Error::<T>::NewCapacityEqualsCurrentCapacity
        );

        // Check that enough time has passed since the last capacity change
        ensure!(
            frame_system::Pallet::<T>::block_number()
                >= bsp.last_capacity_change + T::MinBlocksBetweenCapacityChanges::get(),
            Error::<T>::NotEnoughTimePassed
        );

        // Check that the new capacity is bigger than the minimum required by the runtime
        ensure!(
            new_capacity >= T::SpMinCapacity::get(),
            Error::<T>::StorageTooLow
        );

        // Check that the new capacity is bigger than the current used capacity by the BSP
        ensure!(
            new_capacity >= bsp.capacity_used,
            Error::<T>::NewCapacityLessThanUsedStorage
        );

        // Calculate how much deposit will the signer have to pay to register with this amount of data
        let capacity_over_minimum = new_capacity
            .checked_sub(&T::SpMinCapacity::get())
            .ok_or(Error::<T>::StorageTooLow)?;
        let deposit_for_capacity_over_minimum = T::DepositPerData::get()
            .checked_mul(&capacity_over_minimum.into())
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;
        let new_deposit = T::SpMinDeposit::get()
            .checked_add(&deposit_for_capacity_over_minimum)
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;

        // Check how much has the used already deposited for the current capacity
        let current_deposit = T::NativeBalance::balance_on_hold(
            &HoldReason::StorageProviderDeposit.into(),
            account_id,
        );

        // Check if the new deposit is bigger or smaller than the current deposit
        // Note: we do not check directly capacities as, for example, a bigger new_capacity could entail a smaller deposit
        // because of changes in storage pricing, so we check the difference in deposits instead
        if new_deposit > current_deposit {
            // If the new deposit is bigger than the current deposit, more balance has to be held from the user
            Self::hold_balance(account_id, current_deposit, new_deposit)?;
        } else if new_deposit < current_deposit {
            // If the new deposit is smaller than the current deposit, some balance has to be released to the user
            Self::release_balance(account_id, current_deposit, new_deposit)?;
        }

        // Get the BSP's old capacity
        let old_capacity = bsp.capacity;

        // Update the total capacity of the network (which is the sum of all BSPs capacities)
        if new_capacity > old_capacity {
            // If the new capacity is bigger than the old capacity, get the difference doing new_capacity - old_capacity
            let difference = new_capacity
                .checked_sub(&old_capacity)
                .ok_or(DispatchError::Arithmetic(ArithmeticError::Underflow))?;
            // Increment the total capacity of the network by the difference
            TotalBspsCapacity::<T>::mutate(|n| match n.checked_add(&difference) {
                Some(new_total_bsp_capacity) => {
                    *n = new_total_bsp_capacity;
                    Ok(())
                }
                None => Err(DispatchError::Arithmetic(ArithmeticError::Overflow)),
            })?;
        } else {
            // If the new capacity is smaller than the old capacity, get the difference doing old_capacity - new_capacity
            let difference = old_capacity
                .checked_sub(&new_capacity)
                .ok_or(DispatchError::Arithmetic(ArithmeticError::Underflow))?;
            // Decrement the total capacity of the network
            TotalBspsCapacity::<T>::mutate(|n| match n.checked_sub(&difference) {
                Some(new_total_bsp_capacity) => {
                    *n = new_total_bsp_capacity;
                    Ok(())
                }
                None => Err(DispatchError::Arithmetic(ArithmeticError::Underflow)),
            })?;
        }

        // Update the BSP's storage, modifying the capacity and the last capacity change block number
        bsp.capacity = new_capacity;
        bsp.last_capacity_change = frame_system::Pallet::<T>::block_number();
        BackupStorageProviders::<T>::insert(&bsp_id, bsp);

        // Return the old capacity
        Ok(old_capacity)
    }

    /// Slash a Storage Provider.
    ///
    /// The amount slashed is calculated as the product of the [`SlashAmountPerChunkOfStorageData`] and the accrued failed proof submissions.
    /// The amount is then slashed from the Storage Provider's held deposit and transferred to the treasury.
    ///
    /// This will return an error when the Storage Provider is not slashable. In the context of the StorageHub protocol,
    /// a Storage Provider is slashable when the proofs-dealer pallet has marked them as such.
    ///
    /// Successfully slashing a Storage Provider should be a free operation.
    pub(crate) fn do_slash(provider_id: &HashId<T>) -> DispatchResultWithPostInfo {
        let account_id = if let Some(provider) = MainStorageProviders::<T>::get(provider_id) {
            provider.owner_account
        } else if let Some(provider) = BackupStorageProviders::<T>::get(provider_id) {
            provider.owner_account
        } else {
            return Err(Error::<T>::ProviderNotSlashable.into());
        };

        // Calculate slashable amount.
        // Doubling the slash for each failed proof submission is necessary since it is more probabilistic for a Storage Provider to have
        // responded with two file key proofs given a random or custom challenge.
        let slashable_amount = Self::compute_worst_case_scenario_slashable_amount(provider_id)?;

        let amount_slashed = T::NativeBalance::transfer_on_hold(
            &HoldReason::StorageProviderDeposit.into(),
            &account_id,
            &T::Treasury::get(),
            slashable_amount,
            Precision::BestEffort,
            Restriction::Free,
            Fortitude::Polite,
        )?;

        // Clear the accrued failed proof submissions for the Storage Provider
        <T::ProvidersProofSubmitters as ProofSubmittersInterface>::clear_accrued_failed_proof_submissions(&provider_id);

        // Provider held funds have been completely depleted.
        if amount_slashed <= slashable_amount {
            // TODO: Force sign off the provider.
        }

        Self::deposit_event(Event::<T>::Slashed {
            provider_id: *provider_id,
            amount_slashed,
        });

        Ok(Pays::No.into())
    }

    fn hold_balance(
        account_id: &T::AccountId,
        previous_deposit: BalanceOf<T>,
        new_deposit: BalanceOf<T>,
    ) -> DispatchResult {
        // Get the user's reducible balance
        let user_balance = T::NativeBalance::reducible_balance(
            account_id,
            Preservation::Preserve,
            Fortitude::Polite,
        );

        // Get the difference between the new deposit and the current deposit
        let difference = new_deposit
            .checked_sub(&previous_deposit)
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Underflow))?;

        // Check if the user has enough balance to pay the difference
        ensure!(user_balance >= difference, Error::<T>::NotEnoughBalance);

        // Check if we can hold the difference from the user
        ensure!(
            T::NativeBalance::can_hold(
                &HoldReason::StorageProviderDeposit.into(),
                account_id,
                difference,
            ),
            Error::<T>::CannotHoldDeposit
        );

        // Hold the difference from the user
        T::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            account_id,
            difference,
        )?;

        Ok(())
    }

    fn release_balance(
        account_id: &T::AccountId,
        previous_deposit: BalanceOf<T>,
        new_deposit: BalanceOf<T>,
    ) -> DispatchResult {
        // Get the difference between the current deposit and the new deposit
        let difference = previous_deposit
            .checked_sub(&new_deposit)
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Underflow))?;

        // Release the difference from the user
        T::NativeBalance::release(
            &HoldReason::StorageProviderDeposit.into(),
            account_id,
            difference,
            Precision::Exact,
        )?;

        Ok(())
    }

    /// Compute the worst case scenario slashable amount for a Storage Provider.
    ///
    /// Every failed proof submission counts as for two files which should have been proven due to the low probability of a challenge
    /// being an exact match to a file key stored by the Storage Provider. The StorageHub protocol requires the Storage Provider to
    /// submit a proof of storage for the neighbouring file keys of the missing challenged file key.
    ///
    /// The slashing amount is calculated based on an assumption that every file is the maximum size allowed by the protocol.
    pub fn compute_worst_case_scenario_slashable_amount(
        provider_id: &HashId<T>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let accrued_failed_submission_count = <T::ProvidersProofSubmitters as ProofSubmittersInterface>::get_accrued_failed_proof_submissions(&provider_id)
            .ok_or(Error::<T>::ProviderNotSlashable)?.into();

        Ok(T::SlashAmountPerMaxFileSize::get()
            .saturating_mul(accrued_failed_submission_count)
            .saturating_mul(2u32.into()))
    }
}

impl<T: Config> From<MainStorageProvider<T>> for BackupStorageProvider<T> {
    fn from(msp: MainStorageProvider<T>) -> Self {
        BackupStorageProvider {
            capacity: msp.capacity,
            capacity_used: msp.capacity_used,
            multiaddresses: msp.multiaddresses,
            root: T::DefaultMerkleRoot::get(),
            last_capacity_change: msp.last_capacity_change,
            owner_account: msp.owner_account,
            payment_account: msp.payment_account,
            reputation_weight: T::StartingReputationWeight::get(),
        }
    }
}

/// Implement the ReadBucketsInterface trait for the Storage Providers pallet.
impl<T: pallet::Config> ReadBucketsInterface for pallet::Pallet<T> {
    type AccountId = T::AccountId;
    type BucketId = BucketId<T>;
    type BucketNameLimit = T::BucketNameLimit;
    type ProviderId = HashId<T>;
    type ReadAccessGroupId = T::ReadAccessGroupId;
    type MerkleHash = MerklePatriciaRoot<T>;
    type StorageDataUnit = T::StorageDataUnit;

    fn bucket_exists(bucket_id: &Self::BucketId) -> bool {
        Buckets::<T>::contains_key(bucket_id)
    }

    fn derive_bucket_id(
        msp_id: &Self::ProviderId,
        owner: &Self::AccountId,
        bucket_name: BoundedVec<u8, Self::BucketNameLimit>,
    ) -> Self::BucketId {
        let concat = msp_id
            .encode()
            .into_iter()
            .chain(
                owner
                    .encode()
                    .into_iter()
                    .chain(bucket_name.encode().into_iter()),
            )
            .collect::<scale_info::prelude::vec::Vec<u8>>();

        <<T as frame_system::Config>::Hashing as sp_runtime::traits::Hash>::hash(&concat)
    }

    fn get_msp_of_bucket(bucket_id: &Self::BucketId) -> Option<Self::ProviderId> {
        Buckets::<T>::get(bucket_id).map(|bucket| bucket.msp_id)
    }

    fn get_read_access_group_id_of_bucket(
        bucket_id: &Self::BucketId,
    ) -> Result<Option<Self::ReadAccessGroupId>, DispatchError> {
        let bucket = Buckets::<T>::get(bucket_id).ok_or(Error::<T>::BucketNotFound)?;
        Ok(bucket.read_access_group_id)
    }

    fn is_bucket_owner(
        who: &Self::AccountId,
        bucket_id: &Self::BucketId,
    ) -> Result<bool, DispatchError> {
        let bucket = Buckets::<T>::get(bucket_id).ok_or(Error::<T>::BucketNotFound)?;
        Ok(&bucket.user_id == who)
    }

    fn is_bucket_private(bucket_id: &Self::BucketId) -> Result<bool, DispatchError> {
        let bucket = Buckets::<T>::get(bucket_id).ok_or(Error::<T>::BucketNotFound)?;
        Ok(bucket.private)
    }

    fn is_bucket_stored_by_msp(msp_id: &Self::ProviderId, bucket_id: &Self::BucketId) -> bool {
        if let Some(bucket) = Buckets::<T>::get(bucket_id) {
            bucket.msp_id == *msp_id
        } else {
            false
        }
    }

    fn get_root_bucket(bucket_id: &Self::BucketId) -> Option<Self::MerkleHash> {
        Buckets::<T>::get(bucket_id).map(|bucket| bucket.root)
    }

    fn get_bucket_size(bucket_id: &Self::BucketId) -> Result<Self::StorageDataUnit, DispatchError> {
        let bucket = Buckets::<T>::get(bucket_id).ok_or(Error::<T>::BucketNotFound)?;
        Ok(bucket.size)
    }

    fn get_msp_bucket(bucket_id: &Self::BucketId) -> Result<Self::ProviderId, DispatchError> {
        let bucket = Buckets::<T>::get(bucket_id).ok_or(Error::<T>::BucketNotFound)?;
        Ok(bucket.msp_id)
    }
}

/// Implement the MutateBucketsInterface trait for the Storage Providers pallet.
impl<T: pallet::Config> MutateBucketsInterface for pallet::Pallet<T> {
    type AccountId = T::AccountId;
    type BucketId = BucketId<T>;
    type ProviderId = HashId<T>;
    type ReadAccessGroupId = T::ReadAccessGroupId;
    type MerkleHash = MerklePatriciaRoot<T>;
    type StorageDataUnit = T::StorageDataUnit;

    fn add_bucket(
        provider_id: Self::ProviderId,
        user_id: Self::AccountId,
        bucket_id: Self::BucketId,
        privacy: bool,
        maybe_read_access_group_id: Option<Self::ReadAccessGroupId>,
    ) -> DispatchResult {
        // Check if bucket already exists
        ensure!(
            !Buckets::<T>::contains_key(&bucket_id),
            Error::<T>::BucketAlreadyExists
        );

        // Check if the MSP exists
        ensure!(
            MainStorageProviders::<T>::contains_key(&provider_id),
            Error::<T>::NotRegistered
        );

        let user_balance = T::NativeBalance::reducible_balance(
            &user_id,
            Preservation::Preserve,
            Fortitude::Polite,
        );

        let deposit = T::BucketDeposit::get();
        ensure!(user_balance >= deposit, Error::<T>::NotEnoughBalance);
        ensure!(
            T::NativeBalance::can_hold(&HoldReason::BucketDeposit.into(), &user_id, deposit),
            Error::<T>::CannotHoldDeposit
        );

        // Hold the bucket deposit
        T::NativeBalance::hold(&HoldReason::BucketDeposit.into(), &user_id, deposit)?;

        let bucket = Bucket {
            root: T::DefaultMerkleRoot::get(),
            msp_id: provider_id,
            private: privacy,
            read_access_group_id: maybe_read_access_group_id,
            user_id,
            size: T::StorageDataUnit::zero(),
        };

        Buckets::<T>::insert(&bucket_id, &bucket);

        MainStorageProviderIdsToBuckets::<T>::try_append(&provider_id, bucket_id)
            .map_err(|_| Error::<T>::AppendBucketToMspFailed)?;

        Ok(())
    }

    fn change_msp_bucket(bucket_id: &Self::BucketId, new_msp: &Self::ProviderId) -> DispatchResult {
        Buckets::<T>::try_mutate(bucket_id, |bucket| {
            let bucket = bucket.as_mut().ok_or(Error::<T>::BucketNotFound)?;
            bucket.msp_id = *new_msp;

            Ok(())
        })
    }

    fn change_root_bucket(bucket_id: Self::BucketId, new_root: Self::MerkleHash) -> DispatchResult {
        Buckets::<T>::try_mutate(&bucket_id, |bucket| {
            let bucket = bucket.as_mut().ok_or(Error::<T>::BucketNotFound)?;
            bucket.root = new_root;

            Ok(())
        })
    }

    fn remove_root_bucket(bucket_id: Self::BucketId) -> DispatchResult {
        let bucket = Buckets::<T>::take(&bucket_id).ok_or(Error::<T>::BucketNotFound)?;

        MainStorageProviderIdsToBuckets::<T>::mutate_exists(
            &bucket.msp_id,
            |buckets| match buckets {
                Some(b) => {
                    b.retain(|b| b != &bucket_id);

                    if b.is_empty() {
                        *buckets = None;
                    }
                }
                _ => {}
            },
        );

        // Release the bucket deposit hold
        T::NativeBalance::release(
            &HoldReason::BucketDeposit.into(),
            &bucket.user_id,
            T::BucketDeposit::get(),
            Precision::Exact,
        )?;

        Ok(())
    }

    fn update_bucket_privacy(bucket_id: Self::BucketId, privacy: bool) -> DispatchResult {
        Buckets::<T>::try_mutate(&bucket_id, |maybe_bucket| {
            let bucket = maybe_bucket.as_mut().ok_or(Error::<T>::BucketNotFound)?;
            bucket.private = privacy;

            Ok(())
        })
    }

    fn update_bucket_read_access_group_id(
        bucket_id: Self::BucketId,
        maybe_read_access_group_id: Option<Self::ReadAccessGroupId>,
    ) -> DispatchResult {
        Buckets::<T>::try_mutate(&bucket_id, |maybe_bucket| {
            let bucket = maybe_bucket.as_mut().ok_or(Error::<T>::BucketNotFound)?;
            bucket.read_access_group_id = maybe_read_access_group_id;

            Ok(())
        })
    }

    fn increase_bucket_size(
        bucket_id: &Self::BucketId,
        delta: Self::StorageDataUnit,
    ) -> DispatchResult {
        Buckets::<T>::try_mutate(&bucket_id, |maybe_bucket| {
            let bucket = maybe_bucket.as_mut().ok_or(Error::<T>::BucketNotFound)?;
            bucket.size = bucket.size.saturating_add(delta);

            Ok(())
        })
    }

    fn decrease_bucket_size(
        bucket_id: &Self::BucketId,
        delta: Self::StorageDataUnit,
    ) -> DispatchResult {
        Buckets::<T>::try_mutate(&bucket_id, |maybe_bucket| {
            let bucket = maybe_bucket.as_mut().ok_or(Error::<T>::BucketNotFound)?;
            bucket.size = bucket.size.saturating_sub(delta);

            Ok(())
        })
    }
}

/// Implement the ReadStorageProvidersInterface trait for the Storage Providers pallet.
impl<T: pallet::Config> ReadStorageProvidersInterface for pallet::Pallet<T> {
    type ProviderId = HashId<T>;
    type StorageDataUnit = T::StorageDataUnit;
    type SpCount = T::SpCount;
    type MultiAddress = MultiAddress<T>;
    type MaxNumberOfMultiAddresses = T::MaxMultiAddressAmount;
    type ReputationWeight = T::ReputationWeightType;

    fn is_bsp(who: &Self::ProviderId) -> bool {
        BackupStorageProviders::<T>::contains_key(&who)
    }

    fn is_msp(who: &Self::ProviderId) -> bool {
        MainStorageProviders::<T>::contains_key(&who)
    }

    fn get_global_bsps_reputation_weight() -> Self::ReputationWeight {
        GlobalBspsReputationWeight::<T>::get()
    }

    fn get_bsp_reputation_weight(
        who: &Self::ProviderId,
    ) -> Result<Self::ReputationWeight, DispatchError> {
        if let Some(bsp) = BackupStorageProviders::<T>::get(who) {
            Ok(bsp.reputation_weight)
        } else {
            Err(Error::<T>::NotRegistered.into())
        }
    }

    fn get_number_of_bsps() -> Self::SpCount {
        Self::get_bsp_count()
    }

    fn get_capacity(who: &Self::ProviderId) -> Self::StorageDataUnit {
        if let Some(bsp) = BackupStorageProviders::<T>::get(who) {
            bsp.capacity
        } else if let Some(msp) = MainStorageProviders::<T>::get(who) {
            msp.capacity
        } else {
            Zero::zero()
        }
    }

    fn get_used_capacity(who: &Self::ProviderId) -> Self::StorageDataUnit {
        if let Some(bsp) = BackupStorageProviders::<T>::get(who) {
            bsp.capacity_used
        } else if let Some(msp) = MainStorageProviders::<T>::get(who) {
            msp.capacity_used
        } else {
            Zero::zero()
        }
    }

    fn available_capacity(who: &Self::ProviderId) -> Self::StorageDataUnit {
        if let Some(bsp) = BackupStorageProviders::<T>::get(who) {
            bsp.capacity.saturating_sub(bsp.capacity_used)
        } else if let Some(msp) = MainStorageProviders::<T>::get(who) {
            msp.capacity.saturating_sub(msp.capacity_used)
        } else {
            Zero::zero()
        }
    }

    fn get_bsp_multiaddresses(
        who: &Self::ProviderId,
    ) -> Result<BoundedVec<Self::MultiAddress, Self::MaxNumberOfMultiAddresses>, DispatchError>
    {
        if let Some(bsp) = BackupStorageProviders::<T>::get(who) {
            Ok(BoundedVec::from(bsp.multiaddresses))
        } else {
            Err(Error::<T>::NotRegistered.into())
        }
    }
}

/// Implement the MutateStorageProvidersInterface trait for the Storage Providers pallet.
impl<T: pallet::Config> MutateStorageProvidersInterface for pallet::Pallet<T> {
    type ProviderId = HashId<T>;
    type StorageDataUnit = T::StorageDataUnit;

    fn decrease_capacity_used(
        provider_id: &Self::ProviderId,
        delta: Self::StorageDataUnit,
    ) -> DispatchResult {
        if MainStorageProviders::<T>::contains_key(&provider_id) {
            let mut msp =
                MainStorageProviders::<T>::get(&provider_id).ok_or(Error::<T>::NotRegistered)?;
            msp.capacity_used = msp.capacity_used.saturating_sub(delta);
            MainStorageProviders::<T>::insert(&provider_id, msp);
        } else if BackupStorageProviders::<T>::contains_key(&provider_id) {
            let mut bsp =
                BackupStorageProviders::<T>::get(&provider_id).ok_or(Error::<T>::NotRegistered)?;
            bsp.capacity_used = bsp.capacity_used.saturating_sub(delta);
            BackupStorageProviders::<T>::insert(&provider_id, bsp);
            UsedBspsCapacity::<T>::mutate(|n| match n.checked_sub(&delta) {
                Some(new_total_bsp_capacity) => {
                    *n = new_total_bsp_capacity;
                    Ok(())
                }
                None => Err(DispatchError::Arithmetic(ArithmeticError::Underflow)),
            })?;
        } else {
            return Err(Error::<T>::NotRegistered.into());
        }
        Ok(())
    }

    fn increase_capacity_used(
        provider_id: &Self::ProviderId,
        delta: Self::StorageDataUnit,
    ) -> DispatchResult {
        if MainStorageProviders::<T>::contains_key(&provider_id) {
            let mut msp =
                MainStorageProviders::<T>::get(&provider_id).ok_or(Error::<T>::NotRegistered)?;

            let new_used_capacity = msp.capacity_used.saturating_add(delta);
            if msp.capacity < new_used_capacity {
                return Err(Error::<T>::NewUsedCapacityExceedsStorageCapacity.into());
            }
            msp.capacity_used = new_used_capacity;
            MainStorageProviders::<T>::insert(&provider_id, msp);
        } else if BackupStorageProviders::<T>::contains_key(&provider_id) {
            let mut bsp =
                BackupStorageProviders::<T>::get(&provider_id).ok_or(Error::<T>::NotRegistered)?;
            bsp.capacity_used = bsp.capacity_used.saturating_add(delta);
            BackupStorageProviders::<T>::insert(&provider_id, bsp);
            UsedBspsCapacity::<T>::mutate(|n| match n.checked_add(&delta) {
                Some(new_total_bsp_capacity) => {
                    *n = new_total_bsp_capacity;
                    Ok(())
                }
                None => Err(DispatchError::Arithmetic(ArithmeticError::Overflow)),
            })?;
        } else {
            return Err(Error::<T>::NotRegistered.into());
        }
        Ok(())
    }
}

/// Implement the ReadProvidersInterface for the Storage Providers pallet.
impl<T: pallet::Config> ReadProvidersInterface for pallet::Pallet<T> {
    type AccountId = T::AccountId;
    type Balance = T::NativeBalance;
    type MerkleHash = MerklePatriciaRoot<T>;
    type ProviderId = HashId<T>;

    fn get_default_root() -> Self::MerkleHash {
        T::DefaultMerkleRoot::get()
    }

    fn get_owner_account(who: Self::ProviderId) -> Option<Self::AccountId> {
        if let Some(bsp) = BackupStorageProviders::<T>::get(&who) {
            Some(bsp.owner_account)
        } else if let Some(msp) = MainStorageProviders::<T>::get(&who) {
            Some(msp.owner_account)
        } else if let Some(bucket) = Buckets::<T>::get(&who) {
            let msp_for_bucket = bucket.msp_id;
            if let Some(msp) = MainStorageProviders::<T>::get(&msp_for_bucket) {
                Some(msp.owner_account)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn get_payment_account(who: Self::ProviderId) -> Option<Self::AccountId> {
        if let Some(bsp) = BackupStorageProviders::<T>::get(&who) {
            Some(bsp.payment_account)
        } else if let Some(msp) = MainStorageProviders::<T>::get(&who) {
            Some(msp.payment_account)
        } else {
            None
        }
    }

    fn get_provider_id(who: Self::AccountId) -> Option<Self::ProviderId> {
        if let Some(bsp_id) = AccountIdToBackupStorageProviderId::<T>::get(who.clone()) {
            Some(bsp_id)
        } else if let Some(msp_id) = AccountIdToMainStorageProviderId::<T>::get(who) {
            Some(msp_id)
        } else {
            None
        }
    }

    fn get_root(who: Self::ProviderId) -> Option<Self::MerkleHash> {
        if let Some(bucket) = Buckets::<T>::get(&who) {
            Some(bucket.root)
        } else if let Some(bsp) = BackupStorageProviders::<T>::get(&who) {
            Some(bsp.root)
        } else {
            None
        }
    }

    fn get_stake(
        who: Self::ProviderId,
    ) -> Option<<Self::Balance as frame_support::traits::fungible::Inspect<Self::AccountId>>::Balance>
    {
        if let Some(bucket) = Buckets::<T>::get(&who) {
            match MainStorageProviders::<T>::get(bucket.msp_id) {
                Some(related_msp) => Some(T::NativeBalance::balance_on_hold(
                    &HoldReason::BucketDeposit.into(),
                    &related_msp.owner_account,
                )),
                None => None,
            }
        } else if let Some(bsp) = BackupStorageProviders::<T>::get(&who) {
            Some(T::NativeBalance::balance_on_hold(
                &HoldReason::StorageProviderDeposit.into(),
                &bsp.owner_account,
            ))
        } else {
            None
        }
    }

    fn is_provider(who: Self::ProviderId) -> bool {
        BackupStorageProviders::<T>::contains_key(&who)
            || MainStorageProviders::<T>::contains_key(&who)
            || Buckets::<T>::contains_key(&who)
    }
}

/// Implement the MutateProvidersInterface for the Storage Providers pallet.
impl<T: pallet::Config> MutateProvidersInterface for pallet::Pallet<T> {
    type MerkleHash = MerklePatriciaRoot<T>;
    type ProviderId = HashId<T>;

    fn update_root(who: Self::ProviderId, new_root: Self::MerkleHash) -> DispatchResult {
        if let Some(bucket) = Buckets::<T>::get(&who) {
            Buckets::<T>::insert(
                &who,
                Bucket {
                    root: new_root,
                    ..bucket
                },
            );
        } else if let Some(bsp) = BackupStorageProviders::<T>::get(&who) {
            BackupStorageProviders::<T>::insert(
                &who,
                BackupStorageProvider {
                    root: new_root,
                    ..bsp
                },
            );
        } else {
            return Err(Error::<T>::NotRegistered.into());
        }
        Ok(())
    }
}

/// Implement the ReadChallengeableProvidersInterface for the Storage Providers pallet.
impl<T: pallet::Config> ReadChallengeableProvidersInterface for pallet::Pallet<T> {
    type AccountId = T::AccountId;
    type Balance = T::NativeBalance;
    type MerkleHash = MerklePatriciaRoot<T>;
    type ProviderId = HashId<T>;

    fn get_default_root() -> Self::MerkleHash {
        T::DefaultMerkleRoot::get()
    }

    fn get_owner_account(who: Self::ProviderId) -> Option<Self::AccountId> {
        if let Some(bsp) = BackupStorageProviders::<T>::get(&who) {
            Some(bsp.owner_account)
        } else {
            None
        }
    }

    fn get_provider_id(who: Self::AccountId) -> Option<Self::ProviderId> {
        if let Some(bsp_id) = AccountIdToBackupStorageProviderId::<T>::get(who.clone()) {
            Some(bsp_id)
        } else {
            None
        }
    }

    fn get_root(who: Self::ProviderId) -> Option<Self::MerkleHash> {
        if let Some(bsp) = BackupStorageProviders::<T>::get(&who) {
            Some(bsp.root)
        } else {
            None
        }
    }

    fn get_stake(
        who: Self::ProviderId,
    ) -> Option<<Self::Balance as frame_support::traits::fungible::Inspect<Self::AccountId>>::Balance>
    {
        if let Some(bsp) = BackupStorageProviders::<T>::get(&who) {
            Some(T::NativeBalance::balance_on_hold(
                &HoldReason::StorageProviderDeposit.into(),
                &bsp.owner_account,
            ))
        } else {
            None
        }
    }

    fn is_provider(who: Self::ProviderId) -> bool {
        BackupStorageProviders::<T>::contains_key(&who)
    }
}

/// Implement the MutateChallengeableProvidersInterface for the Storage Providers pallet.
impl<T: pallet::Config> MutateChallengeableProvidersInterface for pallet::Pallet<T> {
    type MerkleHash = MerklePatriciaRoot<T>;
    type ProviderId = HashId<T>;

    fn update_root(who: Self::ProviderId, new_root: Self::MerkleHash) -> DispatchResult {
        if let Some(bsp) = BackupStorageProviders::<T>::get(&who) {
            BackupStorageProviders::<T>::insert(
                &who,
                BackupStorageProvider {
                    root: new_root,
                    ..bsp
                },
            );
        } else {
            return Err(Error::<T>::NotRegistered.into());
        }
        Ok(())
    }

    fn update_provider_after_key_removal(
        who: &Self::ProviderId,
        removed_trie_value: &Vec<u8>,
    ) -> DispatchResult {
        // Get the removed file's metadata
        let file_metadata: FileMetadata<
            { shp_constants::H_LENGTH },
            { shp_constants::FILE_CHUNK_SIZE },
            { shp_constants::FILE_SIZE_TO_CHALLENGES },
        > = FileMetadata::decode(&mut removed_trie_value.as_slice())
            .map_err(|_| Error::<T>::InvalidEncodedFileMetadata)?;

        // Get the file size as a StorageDataUnit type and the owner as an AccountId type
        let file_size = StorageDataUnit::<T>::from(file_metadata.file_size);
        let owner = T::AccountId::decode(&mut file_metadata.owner.as_slice())
            .map_err(|_| Error::<T>::InvalidEncodedAccountId)?;

        // Decrease the used capacity of the provider
        Self::decrease_capacity_used(who, file_size)?;

        // Update the provider's payment stream with the user
        let previous_amount_provided =
            <T::PaymentStreams as PaymentStreamsInterface>::get_dynamic_rate_payment_stream_amount_provided(
                who,
                &owner,
            )
            .ok_or(Error::<T>::PaymentStreamNotFound)?;
        let new_amount_provided = previous_amount_provided.saturating_sub(file_size);
        <T::PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
            who,
            &owner,
            &new_amount_provided,
        )?;

        Ok(())
    }
}

/// Implement the SystemMetricsInterface for the Storage Providers pallet.
impl<T: pallet::Config> SystemMetricsInterface for pallet::Pallet<T> {
    type ProvidedUnit = StorageDataUnit<T>;

    fn get_total_capacity() -> Self::ProvidedUnit {
        Self::get_total_bsp_capacity()
    }

    fn get_total_used_capacity() -> Self::ProvidedUnit {
        Self::get_used_bsp_capacity()
    }
}

/// Runtime API implementation for the Storage Providers pallet.
impl<T> Pallet<T>
where
    T: pallet::Config,
{
    pub fn get_bsp_info(
        bsp_id: &BackupStorageProviderId<T>,
    ) -> Result<BackupStorageProvider<T>, GetBspInfoError> {
        BackupStorageProviders::<T>::get(bsp_id).ok_or(GetBspInfoError::BspNotRegistered)
    }

    pub fn get_storage_provider_id(who: &T::AccountId) -> Option<StorageProviderId<T>> {
        if let Some(bsp_id) = AccountIdToBackupStorageProviderId::<T>::get(who) {
            Some(StorageProviderId::BackupStorageProvider(bsp_id))
        } else if let Some(msp_id) = AccountIdToMainStorageProviderId::<T>::get(who) {
            Some(StorageProviderId::MainStorageProvider(msp_id))
        } else {
            None
        }
    }

    pub fn query_storage_provider_capacity(
        provider_id: &ProviderId<T>,
    ) -> Result<StorageDataUnit<T>, QueryStorageProviderCapacityError> {
        if MainStorageProviders::<T>::contains_key(provider_id) {
            let msp = MainStorageProviders::<T>::get(provider_id)
                .ok_or(QueryStorageProviderCapacityError::ProviderNotRegistered)?;
            Ok(msp.capacity)
        } else if BackupStorageProviders::<T>::contains_key(provider_id) {
            let bsp = BackupStorageProviders::<T>::get(provider_id)
                .ok_or(QueryStorageProviderCapacityError::ProviderNotRegistered)?;
            Ok(bsp.capacity)
        } else {
            Err(QueryStorageProviderCapacityError::ProviderNotRegistered)
        }
    }

    pub fn query_available_storage_capacity(
        provider_id: &ProviderId<T>,
    ) -> Result<StorageDataUnit<T>, QueryAvailableStorageCapacityError> {
        if MainStorageProviders::<T>::contains_key(provider_id) {
            let msp = MainStorageProviders::<T>::get(provider_id)
                .ok_or(QueryAvailableStorageCapacityError::ProviderNotRegistered)?;
            Ok(msp.capacity.saturating_sub(msp.capacity_used))
        } else if BackupStorageProviders::<T>::contains_key(provider_id) {
            let bsp = BackupStorageProviders::<T>::get(provider_id)
                .ok_or(QueryAvailableStorageCapacityError::ProviderNotRegistered)?;
            Ok(bsp.capacity.saturating_sub(bsp.capacity_used))
        } else {
            Err(QueryAvailableStorageCapacityError::ProviderNotRegistered)
        }
    }

    pub fn query_earliest_change_capacity_block(
        provider_id: &BackupStorageProviderId<T>,
    ) -> Result<BlockNumberFor<T>, QueryEarliestChangeCapacityBlockError> {
        let bsp = BackupStorageProviders::<T>::get(provider_id)
            .ok_or(QueryEarliestChangeCapacityBlockError::ProviderNotRegistered)?;
        Ok(bsp.last_capacity_change + T::MinBlocksBetweenCapacityChanges::get())
    }

    pub fn get_worst_case_scenario_slashable_amount(
        provider_id: &ProviderId<T>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        Self::compute_worst_case_scenario_slashable_amount(provider_id)
    }

    pub fn get_slash_amount_per_max_file_size() -> BalanceOf<T> {
        T::SlashAmountPerMaxFileSize::get()
    }
}
