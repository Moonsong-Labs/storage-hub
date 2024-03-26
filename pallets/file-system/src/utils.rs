use core::cmp::max;

use codec::{Decode, Encode};
use frame_support::{ensure, pallet_prelude::DispatchResult, traits::Get};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::{
    traits::{CheckedAdd, One, Zero},
    BoundedVec,
};
use sp_runtime::{SaturatedConversion, Saturating};
use sp_std::vec;

use crate::types::{FileKey, TargetBspsRequired};
use crate::{
    pallet,
    types::{
        FileLocation, Fingerprint, MaxBspsPerStorageRequest, MultiAddresses, Proof,
        StorageProviderId, StorageRequestBspsMetadata, StorageRequestMetadata, StorageUnit,
    },
    Error, NextAvailableExpirationInsertionBlock, Pallet, StorageRequestBsps,
    StorageRequestExpirations, StorageRequests,
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
    /// Request storage for a file.
    ///
    /// In the event that a storage request is created without any user multiaddresses (checkout `do_bsp_stop_storing`),
    /// it is expected that storage providers that do have this file in storage already, will be able to send a
    /// transaction to the chain to add themselves as a data server for the storage request.
    pub(crate) fn do_request_storage(
        owner: StorageProviderId<T>,
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

        let bsps_required = bsps_required.unwrap_or(TargetBspsRequired::<T>::get());

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
        who: StorageProviderId<T>,
        location: FileLocation<T>,
        fingerprint: Fingerprint<T>,
    ) -> DispatchResult {
        // TODO: Verify BSP has enough storage capacity to store the file
        // TODO: Check that sender is a registered storage provider

        // Check that the storage request exists.
        let file_metadata = expect_or_err!(
            <StorageRequests<T>>::get(&location),
            "Storage request should exist",
            Error::<T>::StorageRequestNotFound
        );

        expect_or_err!(
            file_metadata.bsps_confirmed < file_metadata.bsps_required,
            "Storage request should never have confirmed bsps equal to or greater than required bsps, since they are deleted when it is reached.",
            Error::<T>::StorageRequestBspsRequiredFulfilled,
            bool
        );

        // Check if the BSP is already volunteered for this storage request.
        ensure!(
            !<StorageRequestBsps<T>>::contains_key(&location, &who),
            Error::<T>::BspAlreadyVolunteered
        );

        // Check that the threshold value is high enough to qualify as BSP for the storage request.
        let threshold = Self::calculate_xor(
            fingerprint
                .as_ref()
                .try_into()
                .map_err(|_| Error::<T>::FailedToEncodeFingerprint)?,
            &who.encode()
                .try_into()
                .map_err(|_| Error::<T>::FailedToEncodeBsp)?,
        );

        let bsp_threshold = T::AssignmentThreshold::decode(&mut &threshold[..])
            .map_err(|_| Error::<T>::FailedToDecodeThreshold)?;

        let blocks_since_requested = <frame_system::Pallet<T>>::block_number()
            .saturating_sub(file_metadata.requested_at)
            .saturated_into::<u32>();

        let rate_increase = blocks_since_requested
            .saturating_mul(T::AssignmentThresholdMultiplier::get())
            .saturated_into::<T::AssignmentThreshold>();

        let threshold = rate_increase.saturating_add(T::MinBspsAssignmentThreshold::get());

        ensure!(bsp_threshold <= threshold, Error::<T>::ThresholdTooLow);

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

    pub(crate) fn do_bsp_confirm_storing(
        who: StorageProviderId<T>,
        location: FileLocation<T>,
        root: FileKey<T>,
        proof: Proof<T>,
    ) -> DispatchResult {
        let bsp = match <T::Providers as storage_hub_traits::ProvidersInterface>::get_provider(
            who.clone(),
        ) {
            Some(bsp) => bsp,
            None => return Err(Error::<T>::NotABsp.into()),
        };

        // Check that the storage request exists.
        let file_metadata = expect_or_err!(
            <StorageRequests<T>>::get(&location),
            "Storage request should exist",
            Error::<T>::StorageRequestNotFound
        );

        expect_or_err!(
                    file_metadata.bsps_confirmed < file_metadata.bsps_required,
                    "Storage request should never have confirmed bsps equal to or greater than required bsps, since they are deleted when it is reached.",
                    Error::<T>::StorageRequestBspsRequiredFulfilled,
                    bool
                );

        // Check that the sender is a registered storage provider.
        ensure!(
            <StorageRequestBsps<T>>::contains_key(&location, &who),
            Error::<T>::BspNotVolunteered
        );

        // Check that the storage provider has not already confirmed storing the file.
        ensure!(
            !<StorageRequestBsps<T>>::get(&location, &who)
                .expect("BSP should exist since we checked it above")
                .confirmed,
            Error::<T>::BspAlreadyConfirmed
        );

        // Check that the proof is valid.
        ensure!(
            <T::ProofDealer as storage_hub_traits::ProofsDealerInterface>::verify_proof(
                &bsp, &root, &proof,
            )
            .is_ok(),
            Error::<T>::InvalidProof
        );

        // Increment the number of confirmed storage providers.
        <StorageRequests<T>>::try_mutate(&location, |file_metadata| -> DispatchResult {
            let file_metadata = file_metadata
                .as_mut()
                .ok_or(Error::<T>::StorageRequestNotFound)?;

            match file_metadata
                .bsps_confirmed
                .checked_add(&T::StorageRequestBspsRequiredType::one())
            {
                Some(bsps_confirmed) => {
                    file_metadata.bsps_confirmed = bsps_confirmed;
                }
                None => {
                    return Err(Error::<T>::StorageRequestBspsRequiredFulfilled.into());
                }
            }

            Ok(())
        })?;

        Ok(())
    }

    /// Revoke a storage request.
    pub(crate) fn do_revoke_storage_request(
        who: StorageProviderId<T>,
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
        owner: StorageProviderId<T>,
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

        // TODO: loose couple this with a trait
        // Challenge BSP to force update its storage root to uninclude the file.
        ensure!(
            <T::ProofDealer as storage_hub_traits::ProofsDealerInterface>::challenge_with_priority(
                &file_key
            )
            .is_ok(),
            Error::<T>::InvalidProof
        );

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

    /// Calculate the XOR of the fingerprint and the BSP.
    fn calculate_xor(fingerprint: &[u8; 32], bsp: &[u8; 32]) -> Vec<u8> {
        let mut xor_result = Vec::with_capacity(32);
        for i in 0..32 {
            xor_result.push(fingerprint[i] ^ bsp[i]);
        }

        xor_result
    }
}
