use crate::*;
use codec::Encode;
use frame_support::{
    ensure,
    pallet_prelude::DispatchResult,
    sp_runtime::{
        traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, One, Saturating, Zero},
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
    GetBspInfoError, GetStakeError, QueryAvailableStorageCapacityError,
    QueryEarliestChangeCapacityBlockError, QueryMspIdOfBucketIdError,
    QueryProviderMultiaddressesError, QueryStorageProviderCapacityError,
};
use shp_constants::GIGAUNIT;
use shp_traits::{
    FileMetadataInterface, MutateBucketsInterface, MutateChallengeableProvidersInterface,
    MutateProvidersInterface, MutateStorageProvidersInterface, PaymentStreamsInterface,
    ProofSubmittersInterface, ReadBucketsInterface, ReadChallengeableProvidersInterface,
    ReadProvidersInterface, ReadStorageProvidersInterface, ReadUserSolvencyInterface,
    SystemMetricsInterface,
};
use sp_arithmetic::{rational::MultiplyRational, Rounding::NearestPrefUp};
use sp_runtime::traits::ConvertBack;
use sp_std::vec::Vec;
use types::{
    Bucket, Commitment, ExpirationItem, MainStorageProvider, MainStorageProviderSignUpRequest,
    MultiAddress, Multiaddresses, ProviderIdFor, RateDeltaParam, SignUpRequestSpParams,
    StorageDataUnitAndBalanceConverter, StorageProviderId, TopUpMetadata, ValuePropIdFor,
    ValueProposition, ValuePropositionWithId,
};

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
    pub fn do_request_msp_sign_up(
        sign_up_request: MainStorageProviderSignUpRequest<T>,
    ) -> DispatchResult {
        let who = sign_up_request.msp_info.owner_account.clone();

        // Check that the user does not have a pending sign up request
        ensure!(
            SignUpRequests::<T>::get(&who).is_none(),
            Error::<T>::SignUpRequestPending
        );

        // Check that the account is not already registered either as a Main Storage Provider or a Backup Storage Provider
        ensure!(
            AccountIdToMainStorageProviderId::<T>::get(&who).is_none()
                && AccountIdToBackupStorageProviderId::<T>::get(&who).is_none(),
            Error::<T>::AlreadyRegistered
        );

        // Check that the multiaddresses vector is not empty (SPs have to register with at least one)
        ensure!(
            !sign_up_request.msp_info.multiaddresses.is_empty(),
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
            sign_up_request.msp_info.capacity >= T::SpMinCapacity::get(),
            Error::<T>::StorageTooLow
        );

        // Calculate how much deposit will the signer have to pay to register with this amount of data
        let deposit = Self::compute_deposit_needed_for_capacity(sign_up_request.msp_info.capacity)?;

        // Check if the user has enough balance to pay the deposit
        let user_balance =
            T::NativeBalance::reducible_balance(&who, Preservation::Preserve, Fortitude::Polite);
        ensure!(user_balance >= deposit, Error::<T>::NotEnoughBalance);

        // Check if we can hold the deposit from the user
        ensure!(
            T::NativeBalance::can_hold(&HoldReason::StorageProviderDeposit.into(), &who, deposit),
            Error::<T>::CannotHoldDeposit
        );

        // Hold the deposit from the user
        T::NativeBalance::hold(&HoldReason::StorageProviderDeposit.into(), &who, deposit)?;

        // Store the sign up request in the SignUpRequests mapping
        SignUpRequests::<T>::insert(
            who,
            SignUpRequest::<T> {
                sp_sign_up_request: SignUpRequestSpParams::MainStorageProvider(sign_up_request),
                at: frame_system::Pallet::<T>::block_number(),
            },
        );

        Ok(())
    }

    /// This function holds the logic that checks if a user can request to sign up as a Backup Storage Provider
    /// and, if so, stores the request in the SignUpRequests mapping
    pub fn do_request_bsp_sign_up(bsp_info: &BackupStorageProvider<T>) -> DispatchResult {
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
        let deposit = Self::compute_deposit_needed_for_capacity(bsp_info.capacity)?;

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
            SignUpRequest::<T> {
                sp_sign_up_request: SignUpRequestSpParams::BackupStorageProvider(bsp_info.clone()),
                at: frame_system::Pallet::<T>::block_number(),
            },
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
        let sign_up_request =
            SignUpRequests::<T>::get(who).ok_or(Error::<T>::SignUpNotRequested)?;

        // Get the ProviderId by using the AccountId as the seed for a random generator
        let (sp_id, block_number_when_random) =
            T::ProvidersRandomness::random(who.encode().as_ref());

        // Check that the maximum block number after which the randomness is invalid is greater than or equal to the block number when the
        // request was made to ensure that the randomness was not known when the request was made
        ensure!(
            block_number_when_random >= sign_up_request.at,
            Error::<T>::RandomnessNotValidYet
        );

        // Check what type of Storage Provider the signer is trying to sign up as and dispatch the corresponding logic
        match sign_up_request.sp_sign_up_request {
            SignUpRequestSpParams::MainStorageProvider(msp_params) => {
                Self::do_msp_sign_up(who, sp_id, msp_params, sign_up_request.at)?;
            }
            SignUpRequestSpParams::BackupStorageProvider(bsp_params) => {
                Self::do_bsp_sign_up(who, sp_id, &bsp_params, sign_up_request.at)?;
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
        sign_up_request: MainStorageProviderSignUpRequest<T>,
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
        MainStorageProviders::<T>::insert(&msp_id, sign_up_request.msp_info.clone());

        let (_, value_prop) = Self::do_add_value_prop(
            who,
            sign_up_request
                .value_prop
                .price_per_giga_unit_of_data_per_block,
            sign_up_request.value_prop.commitment,
            sign_up_request.value_prop.bucket_data_limit,
        )?;

        let value_prop_id = value_prop.derive_id();
        // Save the ValueProposition information in storage
        MainStorageProviderIdsToValuePropositions::<T>::insert(&msp_id, value_prop_id, &value_prop);

        // Increment the counter of Main Storage Providers registered
        let new_amount_of_msps = MspCount::<T>::get()
            .checked_add(&T::SpCount::one())
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;
        MspCount::<T>::set(new_amount_of_msps);

        // Remove the sign up request from the SignUpRequests mapping
        SignUpRequests::<T>::remove(who);

        <T::PaymentStreams as PaymentStreamsInterface>::add_privileged_provider(&msp_id)?;

        // Emit the corresponding event
        Self::deposit_event(Event::<T>::MspSignUpSuccess {
            who: who.clone(),
            msp_id,
            multiaddresses: sign_up_request.msp_info.multiaddresses.clone(),
            capacity: sign_up_request.msp_info.capacity,
            value_prop: ValuePropositionWithId {
                id: value_prop_id,
                value_prop: value_prop.clone(),
            },
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
            bsp_id,
            root: bsp_info.root,
            multiaddresses: bsp_info.multiaddresses.clone(),
            capacity: bsp_info.capacity,
        });

        Ok(())
    }

    /// This function holds the logic that checks if a user can sign off as a Main Storage Provider
    /// and, if so, updates the storage to remove the user as a Main Storage Provider, decrements the counter of Main Storage Providers,
    /// and returns the deposit to the user
    pub fn do_msp_sign_off(who: &T::AccountId) -> Result<MainStorageProviderId<T>, DispatchError> {
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
                    Ok(msp_id)
                }
                None => Err(DispatchError::Arithmetic(ArithmeticError::Underflow)),
            }
        })?;

        <T::PaymentStreams as PaymentStreamsInterface>::remove_privileged_provider(&msp_id)?;

        Ok(msp_id)
    }

    /// This function holds the logic that checks if a user can sign off as a Backup Storage Provider
    /// and, if so, updates the storage to remove the user as a Backup Storage Provider, decrements the counter of Backup Storage Providers,
    /// decrements the total capacity of the network (which is the sum of all BSPs capacities), and returns the deposit to the user
    pub fn do_bsp_sign_off(
        who: &T::AccountId,
    ) -> Result<BackupStorageProviderId<T>, DispatchError> {
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

        // Check that the sign off period since the BSP signed up has passed
        ensure!(
            frame_system::Pallet::<T>::block_number()
                >= bsp.sign_up_block + T::BspSignUpLockPeriod::get(),
            Error::<T>::SignOffPeriodNotPassed
        );

        // Update the BSPs storage, removing the signer as an BSP
        AccountIdToBackupStorageProviderId::<T>::remove(who);
        BackupStorageProviders::<T>::remove(&bsp_id);

        // Update the total capacity of the network (which is the sum of all BSPs capacities)
        TotalBspsCapacity::<T>::mutate(|n| match n.checked_sub(&bsp.capacity) {
            Some(new_total_bsp_capacity) => {
                *n = new_total_bsp_capacity;
                Ok(bsp_id)
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
                    Ok(bsp_id)
                }
                None => Err(DispatchError::Arithmetic(ArithmeticError::Underflow)),
            }
        })?;

        // Decrease global reputation weight
        GlobalBspsReputationWeight::<T>::mutate(|n| {
            *n = n.saturating_sub(bsp.reputation_weight);
        });

        Ok(bsp_id)
    }

    /// This function is in charge of dispatching the logic to change the capacity of a Storage Provider
    /// It checks if the signer is registered as a SP and dispatches the corresponding function
    /// that checks if the user can change its capacity and, if so, updates the storage to reflect the new capacity
    pub fn do_change_capacity(
        who: &T::AccountId,
        new_capacity: StorageDataUnit<T>,
    ) -> Result<(StorageProviderId<T>, StorageDataUnit<T>), DispatchError> {
        // Check that the new capacity is not zero (there are specific functions to sign off as a SP)
        ensure!(
            new_capacity != T::StorageDataUnit::zero(),
            Error::<T>::NewCapacityCantBeZero
        );

        // Check that the signer is registered as a SP and dispatch the corresponding function, getting its old capacity
        let old_capacity = if let Some(msp_id) = AccountIdToMainStorageProviderId::<T>::get(who) {
            // Check if MSP is insolvent
            ensure!(
                InsolventProviders::<T>::get(StorageProviderId::<T>::MainStorageProvider(msp_id))
                    .is_none(),
                Error::<T>::OperationNotAllowedForInsolventProvider
            );

            (
                StorageProviderId::MainStorageProvider(msp_id),
                Self::do_change_capacity_msp(who, msp_id, new_capacity)?,
            )
        } else if let Some(bsp_id) = AccountIdToBackupStorageProviderId::<T>::get(who) {
            // Check if BSP is insolvent
            ensure!(
                InsolventProviders::<T>::get(StorageProviderId::<T>::BackupStorageProvider(bsp_id))
                    .is_none(),
                Error::<T>::OperationNotAllowedForInsolventProvider
            );

            (
                StorageProviderId::BackupStorageProvider(bsp_id),
                Self::do_change_capacity_bsp(who, bsp_id, new_capacity)?,
            )
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
        let new_deposit = Self::compute_deposit_needed_for_capacity(new_capacity)?;

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

        let new_deposit = Self::compute_deposit_needed_for_capacity(new_capacity)?;

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

    /// This function holds the logic that checks if a user can add a new multiaddress to its storage
    /// and, if so, updates the storage to reflect the new multiaddress and returns the provider id if successful
    pub fn do_add_multiaddress(
        who: &T::AccountId,
        new_multiaddress: &MultiAddress<T>,
    ) -> Result<ProviderIdFor<T>, DispatchError> {
        // Check that the account is a registered Provider and modify the Provider's storage accordingly
        let provider_id = if let Some(msp_id) = AccountIdToMainStorageProviderId::<T>::get(who) {
            // Check if MSP is insolvent
            ensure!(
                InsolventProviders::<T>::get(StorageProviderId::<T>::MainStorageProvider(msp_id))
                    .is_none(),
                Error::<T>::OperationNotAllowedForInsolventProvider
            );

            // If the provider is a MSP, add the new multiaddress to the MSP's storage,
            // making sure the multiaddress did not exist previously
            let mut msp =
                MainStorageProviders::<T>::get(&msp_id).ok_or(Error::<T>::NotRegistered)?;
            ensure!(
                !msp.multiaddresses.contains(new_multiaddress),
                Error::<T>::MultiAddressAlreadyExists
            );
            msp.multiaddresses
                .try_push(new_multiaddress.clone())
                .map_err(|_| Error::<T>::MultiAddressesMaxAmountReached)?;
            MainStorageProviders::<T>::insert(&msp_id, msp);
            msp_id
        } else if let Some(bsp_id) = AccountIdToBackupStorageProviderId::<T>::get(who) {
            // Check if BSP is insolvent
            ensure!(
                InsolventProviders::<T>::get(StorageProviderId::<T>::BackupStorageProvider(bsp_id))
                    .is_none(),
                Error::<T>::OperationNotAllowedForInsolventProvider
            );

            // If the provider is a BSP, add the new multiaddress to the BSP's storage,
            // making sure the multiaddress did not exist previously
            let mut bsp =
                BackupStorageProviders::<T>::get(&bsp_id).ok_or(Error::<T>::NotRegistered)?;
            ensure!(
                !bsp.multiaddresses.contains(new_multiaddress),
                Error::<T>::MultiAddressAlreadyExists
            );
            bsp.multiaddresses
                .try_push(new_multiaddress.clone())
                .map_err(|_| Error::<T>::MultiAddressesMaxAmountReached)?;
            BackupStorageProviders::<T>::insert(&bsp_id, bsp);
            bsp_id
        } else {
            return Err(Error::<T>::NotRegistered.into());
        };

        Ok(provider_id)
    }

    /// This function holds the logic that checks if a user can remove a multiaddress from its storage
    /// and, if so, updates the storage to reflect the removal of the multiaddress and returns the provider id if successful
    pub fn do_remove_multiaddress(
        who: &T::AccountId,
        multiaddress: &MultiAddress<T>,
    ) -> Result<ProviderIdFor<T>, DispatchError> {
        // Check that the account is a registered Provider and modify the Provider's storage accordingly
        let provider_id = if let Some(msp_id) = AccountIdToMainStorageProviderId::<T>::get(who) {
            // If the provider is a MSP, remove the multiaddress from the MSP's storage.
            // but only if it's not the only multiaddress left
            let mut msp =
                MainStorageProviders::<T>::get(&msp_id).ok_or(Error::<T>::NotRegistered)?;

            ensure!(
                msp.multiaddresses.len() > 1,
                Error::<T>::LastMultiAddressCantBeRemoved
            );

            let multiaddress_index = msp
                .multiaddresses
                .iter()
                .position(|addr| addr == multiaddress)
                .ok_or(Error::<T>::MultiAddressNotFound)?;
            msp.multiaddresses.remove(multiaddress_index);

            MainStorageProviders::<T>::insert(&msp_id, msp);

            msp_id
        } else if let Some(bsp_id) = AccountIdToBackupStorageProviderId::<T>::get(who) {
            // If the provider is a BSP, remove the multiaddress from the BSP's storage.
            // but only if it's not the only multiaddress left
            let mut bsp =
                BackupStorageProviders::<T>::get(&bsp_id).ok_or(Error::<T>::NotRegistered)?;

            ensure!(
                bsp.multiaddresses.len() > 1,
                Error::<T>::LastMultiAddressCantBeRemoved
            );

            let multiaddress_index = bsp
                .multiaddresses
                .iter()
                .position(|addr| addr == multiaddress)
                .ok_or(Error::<T>::MultiAddressNotFound)?;
            bsp.multiaddresses.remove(multiaddress_index);

            BackupStorageProviders::<T>::insert(&bsp_id, bsp);

            bsp_id
        } else {
            return Err(Error::<T>::NotRegistered.into());
        };

        Ok(provider_id)
    }

    /// Slash a storage provider based on accrued failed proof submissions.
    ///
    /// Calculates the slashable amount and slashes the provider's held deposit, consequentially reducing the provider's capacity.
    /// If the provider's capacity drops below their used capacity after slashing, we will hold the required amount needed to cover the used capacity deficit
    /// if they have enough free balance. If they don't have enough free balance, we will initiate a grace period for manual top-up.
    ///
    /// # Events
    ///
    /// - `Slashed`: Emitted when the provider is slashed, indicating the amount slashed.
    /// - `TopUpFulfilled`: Emitted when the provider's held deposit is topped up to match the used capacity, indicating the amount topped up.
    /// - `AwaitingTopUp`: Emitted if there is a capacity deficit (i.e. the provider's capacity is falls below the used capacity) and therefore are required to top up their held deposit.
    /// This can be done manually by executing the `top_up_deposit` extrinsic.
    pub(crate) fn do_slash(provider_id: &ProviderIdFor<T>) -> DispatchResult {
        let typed_provider_id = if MainStorageProviders::<T>::get(provider_id).is_some() {
            StorageProviderId::MainStorageProvider(*provider_id)
        } else {
            StorageProviderId::BackupStorageProvider(*provider_id)
        };

        // Check if the provider is insolvent
        ensure!(
            InsolventProviders::<T>::get(&typed_provider_id).is_none(),
            Error::<T>::OperationNotAllowedForInsolventProvider
        );

        let (account_id, _capacity, used_capacity) = Self::get_provider_details(*provider_id)?;

        // Calculate slashable amount for the current number of accrued failed proof submissions
        let slashable_amount = Self::compute_worst_case_scenario_slashable_amount(provider_id)?;

        // Clear the accrued failed proof submissions for the Storage Provider
        <T::ProvidersProofSubmitters as ProofSubmittersInterface>::clear_accrued_failed_proof_submissions(&provider_id);

        // Slash the held deposit since there's not enough free balance
        let actual_slashed = T::NativeBalance::transfer_on_hold(
            &HoldReason::StorageProviderDeposit.into(),
            &account_id,
            &T::Treasury::get(),
            slashable_amount,
            Precision::BestEffort,
            Restriction::Free,
            Fortitude::Force,
        )?;

        // Calculate the new capacity after slashing the held deposit
        let new_decreased_capacity = Self::compute_capacity_from_held_deposit(actual_slashed)?;

        // Decrease capacity by the amount slashed from the held deposit
        let mut final_capacity = new_decreased_capacity;

        // Slash amount could be 0, but this is still emitted as a signal for the provider and users to be aware
        Self::deposit_event(Event::<T>::Slashed {
            provider_id: *provider_id,
            amount: actual_slashed,
        });

        // Capacity needed for the provider to remain active
        let needed_capacity = used_capacity.max(T::SpMinCapacity::get());

        // Held deposit needed for required capacity
        let required_held_amt = Self::compute_deposit_needed_for_capacity(needed_capacity)?;

        // Needed balance to be held to increase capacity back to `needed_capacity`
        let held_deposit_difference =
            required_held_amt.saturating_sub(T::NativeBalance::balance_on_hold(
                &HoldReason::StorageProviderDeposit.into(),
                &account_id,
            ));

        // Short circuit there is nothing left to do if the provider's held deposit covers the `needed_capacity`
        if held_deposit_difference == BalanceOf::<T>::zero() {
            return Ok(());
        }

        // At this point, we know the provider is running with a capacity deficit
        // Try to hold the required amount from provider's free balance
        if T::NativeBalance::can_hold(
            &HoldReason::StorageProviderDeposit.into(),
            &account_id,
            held_deposit_difference,
        ) {
            // Hold the required amount
            T::NativeBalance::hold(
                &HoldReason::StorageProviderDeposit.into(),
                &account_id,
                held_deposit_difference,
            )?;

            // Increase capacity up to the used capacity
            final_capacity = needed_capacity;

            // Remove provider from this storage so when the grace period ends and we process the provider top up expiration item,
            // they will not be slashed
            AwaitingTopUpFromProviders::<T>::remove(&typed_provider_id);

            Self::deposit_event(Event::<T>::TopUpFulfilled {
                provider_id: *provider_id,
                amount: held_deposit_difference,
            });
        } else {
            // Cannot hold enough balance, start tracking grace period and awaited top up

            // Queue provider top up expiration
            let block_number_expiry = Self::enqueue_expiration_item(
                ExpirationItem::ProviderTopUp(typed_provider_id.clone()),
            )?;

            let top_up_metadata = TopUpMetadata {
                started_at:
                    <T::PaymentStreams as shp_traits::PaymentStreamsInterface>::current_tick(),
                end_block_grace_period: block_number_expiry,
            };

            AwaitingTopUpFromProviders::<T>::insert(
                typed_provider_id.clone(),
                top_up_metadata.clone(),
            );

            // Signal to the provider that they need to top up their held deposit to match the current used capacity
            Self::deposit_event(Event::<T>::AwaitingTopUp {
                provider_id: *provider_id,
                top_up_metadata,
            });
        }

        // Update the provider's capacity
        match &typed_provider_id {
            StorageProviderId::MainStorageProvider(provider_id) => {
                let mut provider =
                    MainStorageProviders::<T>::get(provider_id).ok_or(Error::<T>::NotRegistered)?;
                provider.capacity = final_capacity;
                MainStorageProviders::<T>::insert(provider_id, provider);
            }
            StorageProviderId::BackupStorageProvider(provider_id) => {
                let mut provider = BackupStorageProviders::<T>::get(provider_id)
                    .ok_or(Error::<T>::NotRegistered)?;
                provider.capacity = final_capacity;
                BackupStorageProviders::<T>::insert(*provider_id, provider);
            }
        }

        Ok(())
    }

    /// Allows a storage provider to manually top up their held deposit to restore capacity up to their currently used capacity.
    ///
    /// The provider must be within a grace period due to insufficient capacity.
    /// Holds the required amount from the provider's free balance to match their used capacity.
    ///
    /// This will error out if the provider is not registered or lacks sufficient balance.
    pub(crate) fn do_top_up_deposit(account_id: &T::AccountId) -> DispatchResult {
        let provider_id = AccountIdToMainStorageProviderId::<T>::get(account_id)
            .or(AccountIdToBackupStorageProviderId::<T>::get(account_id))
            .ok_or(Error::<T>::NotRegistered)?;

        let typed_provider_id = if MainStorageProviders::<T>::get(&provider_id).is_some() {
            StorageProviderId::MainStorageProvider(provider_id)
        } else {
            StorageProviderId::BackupStorageProvider(provider_id)
        };

        // Check if the provider is insolvent
        ensure!(
            InsolventProviders::<T>::get(&typed_provider_id).is_none(),
            Error::<T>::OperationNotAllowedForInsolventProvider
        );

        let (account_id, _capacity, used_capacity) = Self::get_provider_details(provider_id)?;

        // Capacity needed for the provider to remain active
        let needed_capacity = used_capacity.max(T::SpMinCapacity::get());

        // Additional balance needed to be held to match the used capacity
        let required_held_amt = Self::compute_deposit_needed_for_capacity(needed_capacity)?;

        // Needed balance to be held to increase capacity back to `needed_capacity`
        let held_deposit_difference =
            required_held_amt.saturating_sub(T::NativeBalance::balance_on_hold(
                &HoldReason::StorageProviderDeposit.into(),
                &account_id,
            ));

        // Early return if the provider's held deposit covers the `needed_capacity`
        if held_deposit_difference == BalanceOf::<T>::zero() {
            return Ok(());
        }

        // Check if the provider has enough free balance to top up the slashed amount
        ensure!(
            T::NativeBalance::can_hold(
                &HoldReason::StorageProviderDeposit.into(),
                &account_id,
                held_deposit_difference,
            ),
            Error::<T>::CannotHoldDeposit
        );

        // Hold the slashable amount from the free balance
        T::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &account_id,
            held_deposit_difference,
        )?;

        // Update the provider's capacity in storage
        match typed_provider_id {
            StorageProviderId::MainStorageProvider(provider_id) => {
                let mut provider =
                    MainStorageProviders::<T>::get(provider_id).ok_or(Error::<T>::NotRegistered)?;
                provider.capacity = needed_capacity;
                MainStorageProviders::<T>::insert(provider_id, provider);
            }
            StorageProviderId::BackupStorageProvider(provider_id) => {
                let mut provider = BackupStorageProviders::<T>::get(provider_id)
                    .ok_or(Error::<T>::NotRegistered)?;
                provider.capacity = needed_capacity;
                BackupStorageProviders::<T>::insert(provider_id, provider);
            }
        }

        // Remove provider from this storage so when the grace period ends and we process the provider top up expiration item,
        // they will not be slashed
        AwaitingTopUpFromProviders::<T>::remove(typed_provider_id);

        // Signal that the slashed amount has been topped up
        Self::deposit_event(Event::<T>::TopUpFulfilled {
            provider_id,
            amount: held_deposit_difference,
        });

        Ok(())
    }

    pub(crate) fn do_add_value_prop(
        who: &T::AccountId,
        price_per_giga_unit_of_data_per_block: BalanceOf<T>,
        commitment: Commitment<T>,
        bucket_data_limit: StorageDataUnit<T>,
    ) -> Result<(MainStorageProviderId<T>, ValueProposition<T>), DispatchError> {
        let msp_id =
            AccountIdToMainStorageProviderId::<T>::get(who).ok_or(Error::<T>::NotRegistered)?;

        // Check if MSP is insolvent
        ensure!(
            InsolventProviders::<T>::get(StorageProviderId::<T>::MainStorageProvider(msp_id))
                .is_none(),
            Error::<T>::OperationNotAllowedForInsolventProvider
        );

        let value_prop = ValueProposition::<T>::new(
            price_per_giga_unit_of_data_per_block,
            commitment,
            bucket_data_limit,
        );
        let value_prop_id = value_prop.derive_id();

        if MainStorageProviderIdsToValuePropositions::<T>::contains_key(&msp_id, &value_prop_id) {
            return Err(Error::<T>::ValuePropositionAlreadyExists.into());
        }

        MainStorageProviderIdsToValuePropositions::<T>::insert(&msp_id, value_prop_id, &value_prop);

        Ok((msp_id, value_prop))
    }

    pub(crate) fn do_make_value_prop_unavailable(
        who: &T::AccountId,
        value_prop_id: ValuePropIdFor<T>,
    ) -> Result<MainStorageProviderId<T>, DispatchError> {
        let msp_id =
            AccountIdToMainStorageProviderId::<T>::get(who).ok_or(Error::<T>::NotRegistered)?;

        MainStorageProviderIdsToValuePropositions::<T>::try_mutate_exists(
            &msp_id,
            value_prop_id,
            |value_prop| {
                let value_prop = value_prop
                    .as_mut()
                    .ok_or(Error::<T>::ValuePropositionNotFound)?;

                value_prop.available = false;

                Ok(msp_id)
            },
        )
    }

    pub(crate) fn do_delete_provider(provider_id: &ProviderIdFor<T>) -> Result<(), DispatchError> {
        ensure!(
            Self::can_delete_provider(provider_id),
            Error::<T>::DeleteProviderConditionsNotMet
        );

        // Delete provider data
        if let Some(msp) = MainStorageProviders::<T>::get(provider_id) {
            InsolventProviders::<T>::remove(StorageProviderId::<T>::MainStorageProvider(
                *provider_id,
            ));
            MainStorageProviders::<T>::remove(&provider_id);
            AccountIdToMainStorageProviderId::<T>::remove(msp.owner_account);
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
            MainStorageProviderIdsToValuePropositions::<T>::drain_prefix(&provider_id);
            MainStorageProviderIdsToBuckets::<T>::drain_prefix(&provider_id);

            Self::deposit_event(Event::<T>::MspDeleted {
                provider_id: *provider_id,
            });
        } else if let Some(bsp) = BackupStorageProviders::<T>::get(provider_id) {
            InsolventProviders::<T>::remove(StorageProviderId::<T>::BackupStorageProvider(
                *provider_id,
            ));
            BackupStorageProviders::<T>::remove(&provider_id);
            AccountIdToBackupStorageProviderId::<T>::remove(bsp.owner_account);
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
            TotalBspsCapacity::<T>::mutate(|n| {
                let new_total_bsp_capacity = n.checked_sub(&bsp.capacity);
                match new_total_bsp_capacity {
                    Some(new_total_bsp_capacity) => {
                        *n = new_total_bsp_capacity;
                        Ok(())
                    }
                    None => Err(DispatchError::Arithmetic(ArithmeticError::Underflow)),
                }
            })?;
            UsedBspsCapacity::<T>::mutate(|n| {
                let new_used_bsp_capacity = n.checked_sub(&bsp.capacity_used);
                match new_used_bsp_capacity {
                    Some(new_used_bsp_capacity) => {
                        *n = new_used_bsp_capacity;
                        Ok(())
                    }
                    None => Err(DispatchError::Arithmetic(ArithmeticError::Underflow)),
                }
            })?;
            GlobalBspsReputationWeight::<T>::mutate(|n| {
                *n = n.saturating_sub(bsp.reputation_weight);
            });

            Self::deposit_event(Event::<T>::BspDeleted {
                provider_id: *provider_id,
            });
        }

        Ok(())
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
    /// The slashing amount is calculated as the product of the [`SlashAmountPerMaxFileSize`](Config::SlashAmountPerMaxFileSize)
    /// (an assumption that most file sizes fall below this large arbitrary size) and the accrued failed proof submissions multiplied
    /// by `2` to account for the worst case scenario where the provider would have proved two file keys surrounding the challenged file key.
    pub fn compute_worst_case_scenario_slashable_amount(
        provider_id: &ProviderIdFor<T>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let accrued_failed_submission_count = <T::ProvidersProofSubmitters as ProofSubmittersInterface>::get_accrued_failed_proof_submissions(&provider_id)
            .ok_or(Error::<T>::ProviderNotSlashable)?.into();

        Ok(T::SlashAmountPerMaxFileSize::get()
            .saturating_mul(accrued_failed_submission_count)
            .saturating_mul(2u32.into()))
    }

    /// Adjust the fixed rate payment stream between a user and an MSP based on the [`RateDeltaParam`].
    ///
    /// Handles creating, updating, or deleting the fixed rate payment stream storage.
    pub fn apply_delta_fixed_rate_payment_stream(
        msp_id: &MainStorageProviderId<T>,
        bucket_id: &BucketId<T>,
        user_id: &T::AccountId,
        delta: RateDeltaParam<T>,
    ) -> Result<(), DispatchError> {
        // If the user in insolvent (inactive in the system), proceed to delete the payment stream if it still exists
        if <T::PaymentStreams as ReadUserSolvencyInterface>::is_user_insolvent(&user_id) {
            if <T::PaymentStreams as PaymentStreamsInterface>::has_active_payment_stream_with_user(
                &msp_id, &user_id,
            ) {
                <T::PaymentStreams as PaymentStreamsInterface>::delete_fixed_rate_payment_stream(
                    &msp_id, &user_id,
                )?;
            }
        } else {
            // If the user is solvent (active in the system), we can proceed with the rate adjustment
            let current_rate = <T::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(
					&msp_id,
					&user_id,
				)
				.unwrap_or_default();

            let bucket = Buckets::<T>::get(&bucket_id).ok_or(Error::<T>::BucketNotFound)?;

            ensure!(
                bucket.value_prop_id.is_some(),
                Error::<T>::BucketHasNoValueProposition
            );

            let value_prop = MainStorageProviderIdsToValuePropositions::<T>::get(
                &msp_id,
                &bucket.value_prop_id.unwrap(),
            )
            .ok_or(Error::<T>::ValuePropositionNotFound)?;

            let zero_sized_bucket_rate = T::ZeroSizeBucketFixedRate::get();

            match delta {
                RateDeltaParam::NewBucket => {
                    // Get the rate of the new bucket to add.
                    // If the bucket size is zero, the rate is the fixed rate of a zero sized bucket.
                    // Otherwise, the rate is the fixed rate of a zero sized bucket plus the rate according to the bucket size.
                    // Since the value proposition is in price per giga unit of data per block, we need to convert the price to price per unit of data per block
                    // and that could mean that, since it's an integer division, the rate could be zero. In that case, we saturate to the zero sized bucket rate.
                    let bucket_rate = if bucket.size.is_zero() {
                        zero_sized_bucket_rate
                    } else {
                        value_prop
                            .price_per_giga_unit_of_data_per_block
                            .multiply_rational(bucket.size.into(), GIGAUNIT.into(), NearestPrefUp)
                            .ok_or(ArithmeticError::Overflow)?
                            .checked_add(&zero_sized_bucket_rate)
                            .ok_or(ArithmeticError::Overflow)?
                    };

                    let new_rate = current_rate
                        .checked_add(&bucket_rate)
                        .ok_or(ArithmeticError::Overflow)?;

                    if <T::PaymentStreams as PaymentStreamsInterface>::fixed_rate_payment_stream_exists(
							&msp_id, &user_id,
						) {
							<T::PaymentStreams as PaymentStreamsInterface>::update_fixed_rate_payment_stream(
							&msp_id,
							&user_id,
							new_rate,
						)?;
						} else {
							<T::PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
								&msp_id,
								&user_id,
								new_rate,
							)?;
						}
                }
                RateDeltaParam::RemoveBucket => {
                    // Get the current rate of the bucket to remove.
                    // If the bucket size is zero, the rate is the fixed rate of a zero sized bucket.
                    // Otherwise, the rate is the fixed rate of a zero sized bucket plus the rate according to the bucket size.
                    // Since the value proposition is in price per giga unit of data per block, we need to convert the price to price per unit of data per block
                    // and that could mean that, since it's an integer division, the rate could be zero. In that case, we saturate to the zero sized bucket rate.
                    let bucket_rate = if bucket.size.is_zero() {
                        zero_sized_bucket_rate
                    } else {
                        value_prop
                            .price_per_giga_unit_of_data_per_block
                            .multiply_rational(bucket.size.into(), GIGAUNIT.into(), NearestPrefUp)
                            .ok_or(ArithmeticError::Overflow)?
                            .checked_add(&zero_sized_bucket_rate)
                            .ok_or(ArithmeticError::Overflow)?
                    };

                    let new_rate = current_rate.saturating_sub(bucket_rate);

                    if new_rate.is_zero() {
                        <T::PaymentStreams as PaymentStreamsInterface>::delete_fixed_rate_payment_stream(
							&msp_id,
							&user_id,
						)?;
                    } else {
                        <T::PaymentStreams as PaymentStreamsInterface>::update_fixed_rate_payment_stream(
								&msp_id,
								&user_id,
								new_rate,
							)?;
                    }
                }
                RateDeltaParam::Increase(delta) => {
                    // Get the current bucket rate, which is the rate of a zero sized bucket plus the rate according to the bucket size.
                    let bucket_rate = value_prop
                        .price_per_giga_unit_of_data_per_block
                        .multiply_rational(bucket.size.into(), GIGAUNIT.into(), NearestPrefUp)
                        .ok_or(ArithmeticError::Overflow)?
                        .checked_add(&zero_sized_bucket_rate)
                        .ok_or(ArithmeticError::Overflow)?;

                    // Calculate the new bucket's size.
                    let new_bucket_size = bucket
                        .size
                        .checked_add(&delta)
                        .ok_or(ArithmeticError::Overflow)?;

                    // Ensure the new bucket size does not exceed the bucket data limit of associated value proposition
                    ensure!(
                        new_bucket_size <= value_prop.bucket_data_limit,
                        Error::<T>::BucketSizeExceedsLimit
                    );

                    // Calculate what would be the new bucket rate with the new size.
                    let new_bucket_rate = value_prop
                        .price_per_giga_unit_of_data_per_block
                        .multiply_rational(new_bucket_size.into(), GIGAUNIT.into(), NearestPrefUp)
                        .ok_or(ArithmeticError::Overflow)?
                        .checked_add(&zero_sized_bucket_rate)
                        .ok_or(ArithmeticError::Overflow)?;

                    // Get the delta rate, which is the difference between the old and new rates for this bucket.
                    let delta_rate = new_bucket_rate
                        .checked_sub(&bucket_rate)
                        .ok_or(ArithmeticError::Underflow)?;

                    // If the rate has changed, update the payment stream.
                    if !delta_rate.is_zero() {
                        // Since this is an increase, add the delta rate to the current rate.
                        let new_rate = current_rate
                            .checked_add(&delta_rate)
                            .ok_or(ArithmeticError::Overflow)?;

                        <T::PaymentStreams as PaymentStreamsInterface>::update_fixed_rate_payment_stream(
							&msp_id, &user_id, new_rate,
							)?;
                    }
                }
                RateDeltaParam::Decrease(delta) => {
                    // Get the current bucket rate, which is the rate of a zero sized bucket plus the rate according to the bucket size.
                    let bucket_rate = value_prop
                        .price_per_giga_unit_of_data_per_block
                        .multiply_rational(bucket.size.into(), GIGAUNIT.into(), NearestPrefUp)
                        .ok_or(ArithmeticError::Overflow)?
                        .checked_add(&zero_sized_bucket_rate)
                        .ok_or(ArithmeticError::Overflow)?;

                    // Calculate the new bucket's size.
                    let new_bucket_size = bucket
                        .size
                        .checked_sub(&delta)
                        .ok_or(ArithmeticError::Underflow)?;

                    // Calculate what would be the new bucket rate with the new size.
                    let new_bucket_rate = value_prop
                        .price_per_giga_unit_of_data_per_block
                        .multiply_rational(new_bucket_size.into(), GIGAUNIT.into(), NearestPrefUp)
                        .ok_or(ArithmeticError::Overflow)?
                        .checked_add(&zero_sized_bucket_rate)
                        .ok_or(ArithmeticError::Overflow)?;

                    // Get the delta rate, which is the difference between the old and new rates for this bucket.
                    let delta_rate = bucket_rate
                        .checked_sub(&new_bucket_rate)
                        .ok_or(ArithmeticError::Underflow)?;

                    // If the rate has changed, update the payment stream.
                    if !delta_rate.is_zero() {
                        // Since this is a decrease, subtract the delta rate from the current rate.
                        let new_rate = current_rate.saturating_sub(delta_rate);

                        <T::PaymentStreams as PaymentStreamsInterface>::update_fixed_rate_payment_stream(
							&msp_id, &user_id, new_rate,
							)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Compute the deposit needed for a given capacity.
    pub(crate) fn compute_deposit_needed_for_capacity(
        capacity: T::StorageDataUnit,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let capacity_over_minimum = capacity
            .checked_sub(&T::SpMinCapacity::get())
            .ok_or(Error::<T>::StorageTooLow)?;
        let deposit_for_capacity_over_minimum = T::DepositPerData::get()
            .checked_mul(&capacity_over_minimum.into())
            .ok_or(ArithmeticError::Overflow)?;
        T::SpMinDeposit::get()
            .checked_add(&deposit_for_capacity_over_minimum)
            .ok_or(ArithmeticError::Overflow.into())
    }

    /// Computes the capacity corresponding to a given held deposit.
    /// This is the inverse of `compute_deposit_needed_for_capacity` but returns 0 if the held deposit is less than the minimum required instead of an error.
    pub(crate) fn compute_capacity_from_held_deposit(
        held_deposit: BalanceOf<T>,
    ) -> Result<T::StorageDataUnit, DispatchError> {
        // Subtract the minimum deposit to get the excess deposit
        let deposit_over_minimum = match held_deposit.checked_sub(&T::SpMinDeposit::get()) {
            Some(d) => d,
            // A held deposit smaller than the minimum required will result in a capacity of 0
            None => return Ok(T::StorageDataUnit::zero()),
        };

        // Calculate the capacity over the minimum
        let capacity_over_minimum = if deposit_over_minimum >= BalanceOf::<T>::one() {
            let storage_data_units = deposit_over_minimum
                .checked_div(&T::DepositPerData::get())
                .ok_or(ArithmeticError::Underflow)?;

            StorageDataUnitAndBalanceConverter::<T>::convert_back(storage_data_units)
        } else {
            T::StorageDataUnit::zero()
        };

        // Add the minimum capacity to get the total capacity
        let total_capacity = T::SpMinCapacity::get()
            .checked_add(&capacity_over_minimum)
            .ok_or(ArithmeticError::Overflow)?;

        Ok(total_capacity)
    }

    fn get_provider_details(
        provider_id: ProviderIdFor<T>,
    ) -> Result<(T::AccountId, StorageDataUnit<T>, StorageDataUnit<T>), DispatchError>
    where
        T: pallet::Config,
    {
        if let Some(provider) = MainStorageProviders::<T>::get(provider_id) {
            Ok((
                provider.owner_account,
                provider.capacity,
                provider.capacity_used,
            ))
        } else if let Some(provider) = BackupStorageProviders::<T>::get(provider_id) {
            Ok((
                provider.owner_account,
                provider.capacity,
                provider.capacity_used,
            ))
        } else {
            return Err(Error::<T>::NotRegistered.into());
        }
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
            sign_up_block: msp.sign_up_block,
        }
    }
}

/**************** Interface Implementations ****************/

/// Implement the ReadBucketsInterface trait for the Storage Providers pallet.
impl<T: pallet::Config> ReadBucketsInterface for pallet::Pallet<T> {
    type AccountId = T::AccountId;
    type BucketId = BucketId<T>;
    type BucketNameLimit = T::BucketNameLimit;
    type ProviderId = ProviderIdFor<T>;
    type ReadAccessGroupId = T::ReadAccessGroupId;
    type MerkleHash = MerklePatriciaRoot<T>;
    type StorageDataUnit = T::StorageDataUnit;

    fn bucket_exists(bucket_id: &Self::BucketId) -> bool {
        Buckets::<T>::contains_key(bucket_id)
    }

    fn derive_bucket_id(
        owner: &Self::AccountId,
        bucket_name: BoundedVec<u8, Self::BucketNameLimit>,
    ) -> Self::BucketId {
        let concat = owner
            .encode()
            .into_iter()
            .chain(bucket_name.encode().into_iter())
            .collect::<scale_info::prelude::vec::Vec<u8>>();

        <<T as crate::Config>::ProviderIdHashing as sp_runtime::traits::Hash>::hash(&concat)
    }

    fn get_msp_of_bucket(
        bucket_id: &Self::BucketId,
    ) -> Result<Option<Self::ProviderId>, DispatchError> {
        let bucket = Buckets::<T>::get(bucket_id).ok_or(Error::<T>::BucketNotFound)?;
        Ok(bucket.msp_id)
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
            bucket.msp_id == Some(*msp_id)
        } else {
            false
        }
    }

    fn get_root_bucket(bucket_id: &Self::BucketId) -> Option<Self::MerkleHash> {
        Buckets::<T>::get(bucket_id).map(|bucket| bucket.root)
    }

    fn get_bucket_owner(bucket_id: &Self::BucketId) -> Result<Self::AccountId, DispatchError> {
        let bucket = Buckets::<T>::get(bucket_id).ok_or(Error::<T>::BucketNotFound)?;
        Ok(bucket.user_id)
    }

    fn get_bucket_size(bucket_id: &Self::BucketId) -> Result<Self::StorageDataUnit, DispatchError> {
        let bucket = Buckets::<T>::get(bucket_id).ok_or(Error::<T>::BucketNotFound)?;
        Ok(bucket.size)
    }

    fn get_msp_bucket(
        bucket_id: &Self::BucketId,
    ) -> Result<Option<Self::ProviderId>, DispatchError> {
        let bucket = Buckets::<T>::get(bucket_id).ok_or(Error::<T>::BucketNotFound)?;
        Ok(bucket.msp_id)
    }
}

/// Implement the MutateBucketsInterface trait for the Storage Providers pallet.
impl<T: pallet::Config> MutateBucketsInterface for pallet::Pallet<T> {
    type AccountId = T::AccountId;
    type BucketId = BucketId<T>;
    type ProviderId = ProviderIdFor<T>;
    type ReadAccessGroupId = T::ReadAccessGroupId;
    type MerkleHash = MerklePatriciaRoot<T>;
    type StorageDataUnit = T::StorageDataUnit;
    type ValuePropId = ValuePropIdFor<T>;

    fn add_bucket(
        provider_id: Option<Self::ProviderId>,
        user_id: Self::AccountId,
        bucket_id: Self::BucketId,
        privacy: bool,
        maybe_read_access_group_id: Option<Self::ReadAccessGroupId>,
        value_prop_id: Option<Self::ValuePropId>,
    ) -> DispatchResult {
        // Check if bucket already exists
        ensure!(
            !Buckets::<T>::contains_key(&bucket_id),
            Error::<T>::BucketAlreadyExists
        );

        let user_balance = T::NativeBalance::reducible_balance(
            &user_id,
            Preservation::Preserve,
            Fortitude::Polite,
        );

        if let Some(provider_id) = provider_id {
            // Check if the MSP exists
            ensure!(
                MainStorageProviders::<T>::contains_key(&provider_id),
                Error::<T>::NotRegistered
            );

            if let Some(value_prop_id) = value_prop_id {
                let value_prop = MainStorageProviderIdsToValuePropositions::<T>::get(
                    &provider_id,
                    &value_prop_id,
                )
                .ok_or(Error::<T>::ValuePropositionNotFound)?;

                ensure!(
                    value_prop.available,
                    Error::<T>::ValuePropositionNotAvailable
                );
            }
        }

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
            user_id: user_id.clone(),
            size: T::StorageDataUnit::zero(),
            value_prop_id,
        };

        Buckets::<T>::insert(&bucket_id, &bucket);

        if let Some(provider_id) = provider_id {
            MainStorageProviderIdsToBuckets::<T>::insert(provider_id, bucket_id, ());

            Self::apply_delta_fixed_rate_payment_stream(
                &provider_id,
                &bucket_id,
                &user_id,
                RateDeltaParam::NewBucket,
            )?;
        }

        Ok(())
    }

    fn assign_msp_to_bucket(
        bucket_id: &Self::BucketId,
        new_msp: &Self::ProviderId,
    ) -> DispatchResult {
        Buckets::<T>::try_mutate(bucket_id, |bucket| {
            let bucket = bucket.as_mut().ok_or(Error::<T>::BucketNotFound)?;

            if let Some(msp_id) = bucket.msp_id {
                if msp_id == *new_msp {
                    return Err(Error::<T>::MspAlreadyAssignedToBucket.into());
                }

                Self::apply_delta_fixed_rate_payment_stream(
                    &msp_id,
                    bucket_id,
                    &bucket.user_id,
                    RateDeltaParam::RemoveBucket,
                )?;

                MainStorageProviderIdsToBuckets::<T>::remove(msp_id, bucket_id);
            }

            bucket.msp_id = Some(*new_msp);

            Self::apply_delta_fixed_rate_payment_stream(
                new_msp,
                bucket_id,
                &bucket.user_id,
                RateDeltaParam::NewBucket,
            )?;

            MainStorageProviderIdsToBuckets::<T>::insert(*new_msp, bucket_id, ());

            Ok::<_, DispatchError>(())
        })
    }

    fn unassign_msp_from_bucket(bucket_id: &Self::BucketId) -> DispatchResult {
        Buckets::<T>::try_mutate(bucket_id, |bucket| {
            let bucket = bucket.as_mut().ok_or(Error::<T>::BucketNotFound)?;

            // MSP should exist within the context of this execution.
            let msp_id = bucket
                .msp_id
                .ok_or(Error::<T>::BucketMustHaveMspForOperation)?;

            bucket.msp_id = None;

            Self::apply_delta_fixed_rate_payment_stream(
                &msp_id,
                bucket_id,
                &bucket.user_id,
                RateDeltaParam::RemoveBucket,
            )?;

            MainStorageProviderIdsToBuckets::<T>::remove(msp_id, bucket_id);

            Ok::<_, DispatchError>(())
        })
    }

    fn change_root_bucket(bucket_id: Self::BucketId, new_root: Self::MerkleHash) -> DispatchResult {
        Buckets::<T>::try_mutate(&bucket_id, |bucket| {
            let bucket = bucket.as_mut().ok_or(Error::<T>::BucketNotFound)?;

            Self::deposit_event(Event::<T>::BucketRootChanged {
                bucket_id,
                old_root: bucket.root,
                new_root,
            });

            bucket.root = new_root;

            Ok(())
        })
    }

    fn remove_root_bucket(bucket_id: Self::BucketId) -> DispatchResult {
        let bucket = Buckets::<T>::get(&bucket_id).ok_or(Error::<T>::BucketNotFound)?;

        // Check if the bucket is empty
        ensure!(
            bucket.root == T::DefaultMerkleRoot::get(),
            Error::<T>::BucketNotEmpty
        );

        if let Some(msp_id) = bucket.msp_id {
            Self::apply_delta_fixed_rate_payment_stream(
                &msp_id,
                &bucket_id,
                &bucket.user_id,
                RateDeltaParam::RemoveBucket,
            )?;

            MainStorageProviderIdsToBuckets::<T>::remove(msp_id, &bucket_id);
        };

        Buckets::<T>::remove(&bucket_id);

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

            // First, try to update the fixed rate payment stream with the new rate, since
            // this function uses the current bucket size to calculate it
            if let Some(msp_id) = bucket.msp_id {
                Self::apply_delta_fixed_rate_payment_stream(
                    &msp_id,
                    bucket_id,
                    &bucket.user_id,
                    RateDeltaParam::Increase(delta),
                )?;
            }

            // Then, if that was successful, update the bucket size
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

            // First, try to update the fixed rate payment stream with the new rate, since
            // this function uses the current bucket size to calculate it
            if let Some(msp_id) = bucket.msp_id {
                Self::apply_delta_fixed_rate_payment_stream(
                    &msp_id,
                    bucket_id,
                    &bucket.user_id,
                    RateDeltaParam::Decrease(delta),
                )?;
            }

            // Then, if that was successful, update the bucket size
            bucket.size = bucket.size.saturating_sub(delta);

            Ok(())
        })
    }
}

/// Implement the ReadStorageProvidersInterface trait for the Storage Providers pallet.
impl<T: pallet::Config> ReadStorageProvidersInterface for pallet::Pallet<T> {
    type ProviderId = ProviderIdFor<T>;
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
    type ProviderId = ProviderIdFor<T>;
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
    type ProviderId = ProviderIdFor<T>;

    fn get_default_root() -> Self::MerkleHash {
        T::DefaultMerkleRoot::get()
    }

    fn get_owner_account(who: Self::ProviderId) -> Option<Self::AccountId> {
        if let Some(bsp) = BackupStorageProviders::<T>::get(&who) {
            Some(bsp.owner_account)
        } else if let Some(msp) = MainStorageProviders::<T>::get(&who) {
            Some(msp.owner_account)
        } else if let Some(bucket) = Buckets::<T>::get(&who) {
            let msp_id = bucket.msp_id?;

            if let Some(msp) = MainStorageProviders::<T>::get(&msp_id) {
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
        if let Some(msp) = MainStorageProviders::<T>::get(&who) {
            Some(T::NativeBalance::balance_on_hold(
                &HoldReason::StorageProviderDeposit.into(),
                &msp.owner_account,
            ))
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

    fn is_provider_insolvent(who: Self::ProviderId) -> bool {
        let is_provider_insolvent =
            InsolventProviders::<T>::get(&StorageProviderId::<T>::MainStorageProvider(who))
                .is_some()
                || InsolventProviders::<T>::get(&StorageProviderId::<T>::BackupStorageProvider(
                    who,
                ))
                .is_some();

        // While provider is being awaited for top up, it is still considered insolvent, it's just that
        // it can get out of this state.
        let is_provider_awaiting_topup =
            AwaitingTopUpFromProviders::<T>::get(&StorageProviderId::<T>::MainStorageProvider(who))
                .is_some()
                || AwaitingTopUpFromProviders::<T>::get(
                    &StorageProviderId::<T>::BackupStorageProvider(who),
                )
                .is_some();

        is_provider_insolvent || is_provider_awaiting_topup
    }
}

/// Implement the MutateProvidersInterface for the Storage Providers pallet.
impl<T: pallet::Config> MutateProvidersInterface for pallet::Pallet<T> {
    type MerkleHash = MerklePatriciaRoot<T>;
    type ProviderId = ProviderIdFor<T>;

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
    type ProviderId = ProviderIdFor<T>;

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

    fn get_min_stake(
    ) -> <Self::Balance as frame_support::traits::fungible::Inspect<Self::AccountId>>::Balance {
        T::SpMinDeposit::get()
    }
}

/// Implement the MutateChallengeableProvidersInterface for the Storage Providers pallet.
impl<T: pallet::Config> MutateChallengeableProvidersInterface for pallet::Pallet<T> {
    type MerkleHash = MerklePatriciaRoot<T>;
    type ProviderId = ProviderIdFor<T>;

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
        provider_id: &Self::ProviderId,
        removed_trie_value: &Vec<u8>,
    ) -> DispatchResult {
        // Get the removed file's metadata
        let file_metadata =
            <<T as crate::Config>::FileMetadataManager as FileMetadataInterface>::decode(
                removed_trie_value,
            )
            .map_err(|_| Error::<T>::InvalidEncodedFileMetadata)?;

        // Get the file size as a StorageDataUnit type and the owner as an AccountId type
        let file_size =
            <<T as crate::Config>::FileMetadataManager as FileMetadataInterface>::get_file_size(
                &file_metadata,
            );
        let owner =
            <<T as crate::Config>::FileMetadataManager as FileMetadataInterface>::get_file_owner(
                &file_metadata,
            )
            .map_err(|_| Error::<T>::InvalidEncodedAccountId)?;

        // Decrease the used capacity of the provider
        Self::decrease_capacity_used(provider_id, file_size)?;

        // If the user is insolvent, delete the payment stream between the user and the provider if it still exists.
        if <T::PaymentStreams as ReadUserSolvencyInterface>::is_user_insolvent(&owner) {
            if <T::PaymentStreams as PaymentStreamsInterface>::has_active_payment_stream_with_user(
                &provider_id,
                &owner,
            ) {
                <T::PaymentStreams as PaymentStreamsInterface>::delete_dynamic_rate_payment_stream(
                    &provider_id,
                    &owner,
                )?;
            }
        } else {
            // If the user is solvent, update the payment stream between the user and the provider.
            // If the new amount provided would be zero, delete it instead.
            let previous_amount_provided =
            <T::PaymentStreams as PaymentStreamsInterface>::get_dynamic_rate_payment_stream_amount_provided(
                provider_id,
                &owner,
            )
            .ok_or(Error::<T>::PaymentStreamNotFound)?;
            let new_amount_provided = previous_amount_provided.saturating_sub(file_size);
            if new_amount_provided.is_zero() {
                <T::PaymentStreams as PaymentStreamsInterface>::delete_dynamic_rate_payment_stream(
                    provider_id,
                    &owner,
                )?;
            } else {
                <T::PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                    provider_id,
                    &owner,
                    &new_amount_provided,
                )?;
            }
        }

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
        provider_id: &ProviderIdFor<T>,
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
        provider_id: &ProviderIdFor<T>,
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
        provider_id: &ProviderIdFor<T>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        Self::compute_worst_case_scenario_slashable_amount(provider_id)
    }

    pub fn get_slash_amount_per_max_file_size() -> BalanceOf<T> {
        T::SlashAmountPerMaxFileSize::get()
    }

    pub fn query_msp_id_of_bucket_id(
        bucket_id: &BucketId<T>,
    ) -> Result<Option<MainStorageProviderId<T>>, QueryMspIdOfBucketIdError> {
        let bucket =
            Buckets::<T>::get(bucket_id).ok_or(QueryMspIdOfBucketIdError::BucketNotFound)?;
        Ok(bucket.msp_id)
    }

    pub fn query_provider_multiaddresses(
        provider_id: &ProviderIdFor<T>,
    ) -> Result<Multiaddresses<T>, QueryProviderMultiaddressesError> {
        if let Some(bsp) = BackupStorageProviders::<T>::get(provider_id) {
            Ok(bsp.multiaddresses)
        } else if let Some(msp) = MainStorageProviders::<T>::get(provider_id) {
            Ok(msp.multiaddresses)
        } else {
            Err(QueryProviderMultiaddressesError::ProviderNotRegistered)
        }
    }

    pub fn query_value_propositions_for_msp(
        msp_id: &MainStorageProviderId<T>,
    ) -> Vec<ValuePropositionWithId<T>> {
        MainStorageProviderIdsToValuePropositions::<T>::iter_prefix(msp_id)
            .map(|(id, vp)| ValuePropositionWithId { id, value_prop: vp })
            .collect::<Vec<ValuePropositionWithId<T>>>()
    }

    pub fn get_bsp_stake(
        bsp_id: &BackupStorageProviderId<T>,
    ) -> Result<BalanceOf<T>, GetStakeError> {
        let bsp =
            BackupStorageProviders::<T>::get(bsp_id).ok_or(GetStakeError::ProviderNotRegistered)?;

        let stake = T::NativeBalance::balance_on_hold(
            &HoldReason::StorageProviderDeposit.into(),
            &bsp.owner_account,
        );
        Ok(stake)
    }

    /// Determines if a provider can be deleted based on the following criteria:
    ///
    /// - Provider must be marked as insolvent
    /// - Provider must not have any payment streams
    pub fn can_delete_provider(provider_id: &ProviderIdFor<T>) -> bool {
        // Provider must be insolvent
        if !InsolventProviders::<T>::contains_key(StorageProviderId::<T>::MainStorageProvider(
            *provider_id,
        )) && !InsolventProviders::<T>::contains_key(
            StorageProviderId::<T>::BackupStorageProvider(*provider_id),
        ) {
            return false;
        }

        // Provider must not have any payment streams
        if <T::PaymentStreams as PaymentStreamsInterface>::has_active_payment_stream(provider_id) {
            return false;
        }

        true
    }

    /// Compute the next block number to insert an expiring item, and insert it in the corresponding expiration queue.
    ///
    /// This function attempts to insert a the expiration item at the next available block starting from
    /// the current next available block.
    pub(crate) fn enqueue_expiration_item(
        expiration_item: ExpirationItem<T>,
    ) -> Result<BlockNumberFor<T>, DispatchError> {
        let expiration_block = expiration_item.get_next_expiration_block()?;
        let new_expiration_block = expiration_item.try_append(expiration_block)?;
        expiration_item.set_next_expiration_block(new_expiration_block)?;

        Ok(new_expiration_block)
    }
}

mod hooks {
    use crate::{
        pallet,
        types::{ShTickGetter, StorageHubTickNumber},
        utils::StorageProviderId,
        AwaitingTopUpFromProviders, BackupStorageProviders, Event, HoldReason, InsolventProviders,
        MainStorageProviders, NextStartingShTickToCleanUp, Pallet, ProviderTopUpExpirations,
    };

    use frame_support::{
        traits::{
            fungible::{InspectHold, MutateHold},
            tokens::{Fortitude, Precision, Restriction},
            Get,
        },
        weights::WeightMeter,
    };
    use shp_traits::StorageHubTickGetter;
    use sp_runtime::{
        traits::{One, Zero},
        Saturating,
    };

    impl<T: pallet::Config> Pallet<T> {
        pub(crate) fn do_on_idle(mut meter: &mut WeightMeter) -> &mut WeightMeter {
            let db_weight = T::DbWeight::get();
            let current_sh_tick = ShTickGetter::<T>::get_current_tick();
            let mut sh_tick_to_clean = NextStartingShTickToCleanUp::<T>::get();

            while sh_tick_to_clean <= current_sh_tick && !meter.remaining().is_zero() {
                Self::process_block_expired_items(&mut sh_tick_to_clean, &mut meter);

                if meter.remaining().is_zero() {
                    break;
                }

                sh_tick_to_clean.saturating_accrue(StorageHubTickNumber::<T>::one());
            }

            // Update the next starting block for cleanup
            if sh_tick_to_clean > NextStartingShTickToCleanUp::<T>::get() {
                NextStartingShTickToCleanUp::<T>::put(sh_tick_to_clean);
                meter.consume(db_weight.writes(1));
            }

            meter
        }

        fn process_block_expired_items(
            tick_to_process: &mut StorageHubTickNumber<T>,
            meter: &mut WeightMeter,
        ) {
            let db_weight = T::DbWeight::get();
            let minimum_required_weight_processing_expired_items = db_weight.reads_writes(2, 1);

            // Check if there is enough remaining weight to process expired move bucket requests
            if !meter.can_consume(minimum_required_weight_processing_expired_items) {
                return;
            }

            // Remove expired move bucket requests if any existed and process them.
            let mut provider_top_up_expirations =
                ProviderTopUpExpirations::<T>::take(*tick_to_process);
            meter.consume(minimum_required_weight_processing_expired_items);

            // TODO: After benchmarking, we should check before this loop that there is enough remaining weight to
            // TODO: process all the expired move bucket requests. If not, we should return early.
            while let Some(typed_provider_id) = provider_top_up_expirations.pop() {
                Self::process_expired_provider_top_up_period(typed_provider_id, meter);
            }

            // If there are remaining items which were not processed, put them back in storage
            if !provider_top_up_expirations.is_empty() {
                ProviderTopUpExpirations::<T>::insert(tick_to_process, provider_top_up_expirations);
                meter.consume(db_weight.writes(1));
            }
        }

        fn process_expired_provider_top_up_period(
            typed_provider_id: StorageProviderId<T>,
            meter: &mut WeightMeter,
        ) {
            let db_weight = T::DbWeight::get();
            let potential_weight = db_weight.reads_writes(0, 2);

            if !meter.can_consume(potential_weight) {
                return;
            }

            // Clear awaiting top up storage
            let maybe_awaiting_top_up = AwaitingTopUpFromProviders::<T>::take(&typed_provider_id);

            // Mark the provider as insolvent if it was awaiting a top up
            // If the provider was not awaiting a top up, it means they already topped up either via an
            // automatic top up or a manual top up.
            if maybe_awaiting_top_up.is_some() {
                InsolventProviders::<T>::insert(typed_provider_id.clone(), ());

                Self::deposit_event(Event::ProviderInsolvent {
                    provider_id: *typed_provider_id.inner(),
                });

                let account_id = if let Some(bsp) =
                    BackupStorageProviders::<T>::get(&typed_provider_id.inner())
                {
                    bsp.owner_account
                } else if let Some(msp) = MainStorageProviders::<T>::get(&typed_provider_id.inner())
                {
                    msp.owner_account
                } else {
                    log::error!(
                        target: "runtime::providers",
                        "Could not slash any potentially remaining deposit for provider {:?} as it does not exist.",
                        typed_provider_id
                    );
                    return;
                };

                let held_deposit = T::NativeBalance::balance_on_hold(
                    &HoldReason::StorageProviderDeposit.into(),
                    &account_id,
                );

                if !held_deposit.is_zero() {
                    // Transfer all held deposit to treasury
                    if let Err(e) = T::NativeBalance::transfer_on_hold(
                        &HoldReason::StorageProviderDeposit.into(),
                        &account_id,
                        &T::Treasury::get(),
                        held_deposit,
                        Precision::BestEffort,
                        Restriction::Free,
                        Fortitude::Force,
                    ) {
                        log::error!(
                            target: "runtime::providers",
                            "Could not slash remaining deposit for provider {:?} due to error: {:?}",
                            typed_provider_id,
                            e
                        );
                    }
                }

                if let Err(e) =
                    <T::ProofDealer as shp_traits::ProofsDealerInterface>::stop_challenge_cycle(
                        &typed_provider_id.inner(),
                    )
                {
                    log::error!(
                        target: "runtime::providers",
                        "Could not stop challenge cycle for provider {:?} due to error: {:?}",
                        typed_provider_id,
                        e
                    );
                }
            }

            meter.consume(potential_weight);
        }
    }
}
