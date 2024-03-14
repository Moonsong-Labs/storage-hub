use frame_support::{ensure, pallet_prelude::DispatchResult, sp_runtime::BoundedVec, traits::Get};

use crate::{
    pallet,
    types::{FileLocation, Fingerprint, MultiAddress, StorageRequestMetadata, StorageUnit},
    Error, Pallet, StorageRequestExpirations, StorageRequests,
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
    pub fn do_request_storage(
        location: FileLocation<T>,
        fingerprint: Fingerprint<T>,
        size: StorageUnit<T>,
        user_multiaddr: MultiAddress<T>,
    ) -> DispatchResult {
        // TODO: Check user funds and lock them for the storage request.
        // TODO: Check storage capacity of chosen MSP (when we support MSPs)
        // TODO: Return error if the file is already stored and overwrite is false.
        let current_block_number = <frame_system::Pallet<T>>::block_number();
        let file_metadata = StorageRequestMetadata::<T> {
            requested_at: current_block_number,
            fingerprint,
            size,
            user_multiaddr,
            bsps_volunteered: BoundedVec::default(),
            bsps_confirmed: BoundedVec::default(),
        };

        // Check that storage request is not already registered.
        ensure!(
            !<StorageRequests<T>>::contains_key(&location),
            Error::<T>::StorageRequestAlreadyRegistered
        );

        // Register storage request.
        <StorageRequests<T>>::insert(&location, file_metadata);

        // TODO: Maybe worth it to loop a few times until we find a free slot. (this should never fail and so a loop is required)
        <StorageRequestExpirations<T>>::try_append(
            current_block_number + T::StorageRequestTtl::get().into(),
            &location,
        )
        .map_err(|_| Error::<T>::StorageRequestExpiredNoSlotAvailable)?;

        Ok(())
    }

    pub fn do_bsp_volunteer(
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

        // TODO: Check that the threshold value is high enough to qualify as BSP for the storage request.

        // Add BSP to storage request metadata.
        file_metadata
            .bsps_volunteered
            .try_push(who.clone())
            .map_err(|_| Error::<T>::BspVolunteerFailed)?;
        <StorageRequests<T>>::set(&location, Some(file_metadata.clone()));

        // Check if maximum number of BSPs has been reached.
        if file_metadata.bsps_volunteered.len() == T::MaxBspsPerStorageRequest::get() as usize {
            // Clear storage request from StorageRequests.
            <StorageRequests<T>>::remove(&location);
        }

        Ok(())
    }
}
