use core::cmp::max;

use frame_support::{ensure, pallet_prelude::DispatchResult, sp_runtime::BoundedVec, traits::Get};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::traits::Saturating;

use crate::{
    pallet,
    types::{FileLocation, Fingerprint, MultiAddress, StorageRequestMetadata, StorageUnit},
    CurrentExpirationBlock, Error, Pallet, StorageRequestExpirations, StorageRequests,
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

        let block_to_insert_expiration = Self::next_expiration_block_number();

        // TODO: Maybe worth it to loop a few times until we find a free slot. (this should never fail and so a loop is required)
        <StorageRequestExpirations<T>>::try_append(block_to_insert_expiration, location)
            .map_err(|_| Error::<T>::StorageRequestExpiredNoSlotAvailable)?;

        // Increment the current expiration block pointer if the current storage request expirations reached max capacity at the block which was inserted into.
        if <StorageRequestExpirations<T>>::get(block_to_insert_expiration)
            .expect("Storage request expiration at block should exist")
            .len()
            == T::MaxExpiredStorageRequests::get() as usize
        {
            <CurrentExpirationBlock<T>>::set(block_to_insert_expiration.saturating_add(1u8.into()));
        }

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

        // TODO: Check that the BSP XOR is higher then the threshold

        // Add BSP to storage request metadata.
        expect_or_err!(
            file_metadata.bsps_volunteered.try_push(who.clone()).ok(),
            "BSP volunteer failed",
            Error::<T>::BspVolunteerFailed
        );

        <StorageRequests<T>>::set(&location, Some(file_metadata.clone()));

        Ok(())
    }

    /// Get the block number at which the storage request will expire.
    ///
    /// This will also update the [`CurrentExpirationBlock`] if the current expiration block pointer is lower then the [`crate::Config::StorageRequestTtl`].
    pub(crate) fn next_expiration_block_number() -> BlockNumberFor<T>
    where
        T: pallet::Config,
    {
        let current_block_number = <frame_system::Pallet<T>>::block_number();
        let min_expiration_block = current_block_number + T::StorageRequestTtl::get().into();

        // Reset the current expiration block pointer if it is lower then the minimum storage request TTL.
        if <CurrentExpirationBlock<T>>::get() < min_expiration_block {
            <CurrentExpirationBlock<T>>::set(min_expiration_block);
        }

        let block_to_insert_expiration = max(
            current_block_number + T::StorageRequestTtl::get().into(),
            <CurrentExpirationBlock<T>>::get(),
        );
        block_to_insert_expiration
    }
}
