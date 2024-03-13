use frame_support::{ensure, pallet_prelude::DispatchResult, sp_runtime::BoundedVec, traits::Get};

use crate::{
    pallet,
    types::{FileLocation, Fingerprint, StorageRequestMetadata},
    Error, Pallet, StorageRequests,
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
    ) -> DispatchResult {
        // TODO: Perform various checks of users funds, storage capacity, etc.
        // TODO: Not relevant for PoC.

        let file_metadata = StorageRequestMetadata::<T> {
            requested_at: <frame_system::Pallet<T>>::block_number(),
            fingerprint,
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

        Ok(())
    }

    pub fn do_bsp_volunteer(
        who: T::AccountId,
        location: FileLocation<T>,
        _fingerprint: Fingerprint<T>,
    ) -> DispatchResult {
        // TODO: Perform various checks of BSP staking, total capacity, etc.

        // TODO add identiy pallet to config
        // Check that sender is a registered storage provider.
        // ensure!(<T as Config>::BspsRegistry::get_user(who.clone()).is_some(), Error::<T>::NotBsp);

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

        // TODO Check that the threshold value is high enough to qualify as BSP for the storage request.

        // Add BSP to storage request metadata.
        file_metadata
            .bsps_volunteered
            .try_push(who.clone())
            .map_err(|_| Error::<T>::BspVolunteerFailed)?;
        <StorageRequests<T>>::set(&location, Some(file_metadata.clone()));

        // Check if maximum number of BSPs has been reached.
        if file_metadata.bsps_volunteered.len() == T::MaxBsps::get() as usize {
            // Clear storage request from StorageRequests.
            <StorageRequests<T>>::remove(&location);
        }

        Ok(())
    }
}
