use core::cmp::max;

use frame_support::{ensure, pallet_prelude::DispatchResult, sp_runtime::BoundedVec, traits::Get};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::traits::CheckedAdd;

use crate::{
    pallet,
    types::{FileLocation, Fingerprint, MultiAddress, StorageRequestMetadata, StorageUnit},
    Error, NextAvailableExpirationInsertionBlock, Pallet, StorageRequestExpirations,
    StorageRequests,
};

macro_rules! expect_or_err {
    ($optional:expr, $error_msg:expr, $error_type:path) => {
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
    };
}

impl<T> Pallet<T>
where
    T: pallet::Config,
{
    pub(crate) fn do_request_storage(
        requested_by: T::AccountId,
        location: FileLocation<T>,
        fingerprint: Fingerprint<T>,
        size: StorageUnit<T>,
        user_multiaddr: MultiAddress<T>,
    ) -> DispatchResult {
        // TODO: Check user funds and lock them for the storage request.
        // TODO: Check storage capacity of chosen MSP (when we support MSPs)
        // TODO: Return error if the file is already stored and overwrite is false.
        let file_metadata = StorageRequestMetadata::<T> {
            requested_at: <frame_system::Pallet<T>>::block_number(),
            requested_by,
            fingerprint,
            size,
            user_multiaddr,
            bsps_volunteered: BoundedVec::default(),
            bsps_confirmed: BoundedVec::default(),
        };

        // TODO: if we add the overwrite flag, this would only fail if the overwrite flag is false.
        // Check that storage request is not already registered.
        ensure!(
            !<StorageRequests<T>>::contains_key(&location),
            Error::<T>::StorageRequestAlreadyRegistered
        );

        // Register storage request.
        <StorageRequests<T>>::insert(&location, file_metadata);

        let mut block_to_insert_expiration = Self::next_expiration_insertion_block_number();

        // Get current storage request expirations vec.
        let curr_storage_request_expirations =
            <StorageRequestExpirations<T>>::get(block_to_insert_expiration);

        // Check size of storage request expirations vec.
        if curr_storage_request_expirations.len() >= T::MaxExpiredStorageRequests::get() as usize {
            block_to_insert_expiration = match block_to_insert_expiration.checked_add(&1u8.into()) {
                Some(block) => block,
                None => {
                    return Err(Error::<T>::StorageRequestExpirationBlockOverflow.into());
                }
            };

            <NextAvailableExpirationInsertionBlock<T>>::set(block_to_insert_expiration);
        }

        expect_or_err!(
            // TODO: Verify that try_append gets an empty BoundedVec when appending a first element.
            <StorageRequestExpirations<T>>::try_append(block_to_insert_expiration, location).ok(),
            "Storage request expiration should have enough slots available since it was just checked.",
            Error::<T>::StorageRequestExpiredNoSlotAvailable
        );

        Ok(())
    }

    pub(crate) fn do_bsp_volunteer(
        who: T::AccountId,
        location: FileLocation<T>,
        _fingerprint: Fingerprint<T>,
    ) -> DispatchResult {
        // TODO: Verify BSP has enough storage capacity to store the file
        // TODO: Check that sender is a registered storage provider

        // Check that the storage request exists.
        ensure!(
            <StorageRequests<T>>::contains_key(&location),
            Error::<T>::StorageRequestNotRegistered
        );

        // Get storage request metadata.
        let mut file_metadata = expect_or_err!(
            <StorageRequests<T>>::get(&location),
            "Storage request should exist",
            Error::<T>::StorageRequestNotRegistered
        );

        // Check if the BSP is not already registered for this storage request.
        ensure!(
            !file_metadata.bsps_volunteered.contains(&who),
            Error::<T>::BspAlreadyConfirmed
        );

        // TODO: Check that the BSP XOR is lower than the threshold

        // Add BSP to storage request metadata.
        expect_or_err!(
            file_metadata.bsps_volunteered.try_push(who.clone()).ok(),
            "BSP volunteer failed",
            Error::<T>::BspVolunteerFailed
        );

        <StorageRequests<T>>::set(&location, Some(file_metadata.clone()));

        Ok(())
    }

    /// Revoke a storage request.
    pub(crate) fn do_revoke_storage_request(
        who: T::AccountId,
        location: FileLocation<T>,
    ) -> DispatchResult {
        // Check that the storage request exists.
        ensure!(
            <StorageRequests<T>>::contains_key(&location),
            Error::<T>::StorageRequestNotRegistered
        );

        // Get storage request metadata.
        let file_metadata = expect_or_err!(
            <StorageRequests<T>>::get(&location),
            "Storage request should exist",
            Error::<T>::StorageRequestNotRegistered
        );

        // Check that the sender is the same as the one who requested the storage.
        ensure!(
            file_metadata.requested_by == who,
            Error::<T>::StorageRequestNotAuthorized
        );

        // Remove storage request.
        <StorageRequests<T>>::remove(&location);

        // TODO: initiate deletion request for SPs.

        Ok(())
    }

    /// Get the block number at which the storage request will expire.
    ///
    /// This will also update the [`CurrentExpirationBlock`] if the current expiration block pointer is lower then the [`crate::Config::StorageRequestTtl`].
    pub(crate) fn next_expiration_insertion_block_number() -> BlockNumberFor<T>
    where
        T: pallet::Config,
    {
        let current_block_number = <frame_system::Pallet<T>>::block_number();
        let min_expiration_block = current_block_number + T::StorageRequestTtl::get().into();

        // Reset the current expiration block pointer if it is lower then the minimum storage request TTL.
        if <NextAvailableExpirationInsertionBlock<T>>::get() < min_expiration_block {
            <NextAvailableExpirationInsertionBlock<T>>::set(min_expiration_block);
        }

        let block_to_insert_expiration = max(
            min_expiration_block,
            <NextAvailableExpirationInsertionBlock<T>>::get(),
        );
        block_to_insert_expiration
    }
}
