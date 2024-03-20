use core::cmp::max;

use frame_support::{ensure, pallet_prelude::DispatchResult, traits::Get};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::Saturating;
use sp_runtime::{
    traits::{CheckedAdd, Zero},
    BoundedVec,
};
use sp_std::vec;

use crate::types::{DefaultBspsRequired, FileKey};
use crate::{
    pallet,
    types::{
        FileLocation, Fingerprint, MaxBspsPerStorageRequest, MultiAddresses, StorageProviderId,
        StorageRequestBspsMetadata, StorageRequestMetadata, StorageUnit,
    },
    Error, NextAvailableExpirationInsertionBlock, Pallet, StorageRequestBsps,
    StorageRequestExpirations, StorageRequests,
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
    /// Request storage for a file.
    ///
    /// In the event that a storage request is created without any user multiaddresses (checkout `do_bsp_stop_storing`),
    /// it is expected that storage providers that do have this file in storage already, will be able to send a
    /// transaction to the chain to add themselves as a data server for the storage request.
    pub(crate) fn do_request_storage(
        owner: T::AccountId,
        location: FileLocation<T>,
        fingerprint: Fingerprint<T>,
        size: StorageUnit<T>,
        bsps_required: Option<T::StorageRequestBspsRequiredType>,
        user_multiaddresses: Option<MultiAddresses<T>>,
        data_server_sps: BoundedVec<StorageProviderId<T>, MaxBspsPerStorageRequest<T>>,
    ) -> DispatchResult {
        // TODO: Check user funds and lock them for the storage request.
        // TODO: Check storage capacity of chosen MSP (when we support MSPs)
        // TODO: Return error if the file is already stored and overwrite is false.

        let bsps_required = bsps_required.unwrap_or(DefaultBspsRequired::<T>::get());

        if bsps_required.is_zero() {
            return Err(Error::<T>::BspsRequiredCannotBeZero)?;
        }

        if bsps_required > MaxBspsPerStorageRequest::<T>::get().into() {
            return Err(Error::<T>::BspsRequiredExceedsMax)?;
        }

        let file_metadata = StorageRequestMetadata::<T> {
            requested_at: <frame_system::Pallet<T>>::block_number(),
            owner,
            fingerprint,
            size,
            user_multiaddresses: user_multiaddresses.unwrap_or_default(),
            data_server_sps,
            bsps_required,
            bsps_confirmed: T::StorageRequestBspsRequiredType::zero(),
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
        let storage_request_expirations =
            <StorageRequestExpirations<T>>::get(block_to_insert_expiration);

        // Check size of storage request expirations vec.
        if storage_request_expirations.len() >= T::MaxExpiredStorageRequests::get() as usize {
            block_to_insert_expiration = match block_to_insert_expiration.checked_add(&1u8.into()) {
                Some(block) => block,
                None => {
                    return Err(Error::<T>::MaxBlockNumberReached.into());
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
        let file_metadata = expect_or_err!(
            <StorageRequests<T>>::get(&location),
            "Storage request should exist",
            Error::<T>::StorageRequestNotFound
        );

        // Check that the storage request did not reach the required bsps.
        ensure!(
            file_metadata.bsps_confirmed < file_metadata.bsps_required,
            Error::<T>::StorageRequestBspsRequiredFullfilled
        );

        // Check if the BSP is already volunteered for this storage request.
        ensure!(
            !<StorageRequestBsps<T>>::contains_key(&location, &who),
            Error::<T>::BspAlreadyVolunteered
        );

        // TODO: Check that the BSP XOR is lower than the threshold

        // Add BSP to storage request metadata.
        <StorageRequestBsps<T>>::insert(
            &location,
            &who,
            StorageRequestBspsMetadata::<T> {
                confirmed: false,
                _phantom: Default::default(),
            },
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
            Error::<T>::StorageRequestNotFound
        );

        // Get storage request metadata.
        let file_metadata = expect_or_err!(
            <StorageRequests<T>>::get(&location),
            "Storage request should exist",
            Error::<T>::StorageRequestNotFound
        );

        // Check that the sender is the same as the one who requested the storage.
        ensure!(
            file_metadata.owner == who,
            Error::<T>::StorageRequestNotAuthorized
        );

        // Remove storage request.
        <StorageRequests<T>>::remove(&location);

        // TODO: initiate deletion request for SPs.

        Ok(())
    }

    /// BSP stops storing a file.
    ///
    /// `can_serve` is a flag that indicates if the BSP can serve the file. This is useful for
    /// the case where the BSP lost the file somehow and cannot send it to other BSPs. When it is `true`,
    /// the multiaddresses of the BSP are fetched from the `pallet-storage-providers` and added to the storage request.
    /// When it is `false`, the storage request will be created without any multiaddresses and `do_request_storage`
    /// will handle triggering the appropriate event and pending storage request.
    pub(crate) fn do_bsp_stop_storing(
        who: StorageProviderId<T>,
        file_key: FileKey<T>,
        location: FileLocation<T>,
        owner: T::AccountId,
        fingerprint: Fingerprint<T>,
        size: StorageUnit<T>,
        can_serve: bool,
    ) -> DispatchResult {
        // Check that the storage request exists.
        ensure!(
            <StorageRequests<T>>::contains_key(&location),
            Error::<T>::StorageRequestNotFound
        );

        // TODO: Check that the hash of all the metadata is equal to the `file_key` hash.

        // If the storage request exists, then we should reduce the number of bsps confirmed and
        match <StorageRequests<T>>::get(&location) {
            Some(mut metadata) => {
                // Remove BSP from storage request and challenge if has confirmed having stored this file.
                if let Some(bsp) = <StorageRequestBsps<T>>::get(&location, &who) {
                    // TODO: challenge the BSP to force update its storage

                    if bsp.confirmed {
                        metadata.bsps_confirmed =
                            metadata.bsps_confirmed.saturating_sub(1u32.into());

                        <StorageRequests<T>>::set(&location, Some(metadata));
                    }

                    <StorageRequestBsps<T>>::remove(&location, &who);
                }
            }
            None => {
                Self::do_request_storage(
                    owner,
                    location.clone(),
                    fingerprint,
                    size,
                    Some(1u32.into()),
                    None,
                    if can_serve {
                        BoundedVec::try_from(vec![who.clone()]).unwrap()
                    } else {
                        BoundedVec::default()
                    },
                )?;
            }
        };

        // Challenge BSP to force update its storage root to uninclude the file.
        pallet_proofs_dealer::Pallet::<T>::do_challenge(&who, &file_key)?;

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
