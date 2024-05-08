use core::cmp::max;

use codec::{Decode, Encode};
use frame_support::{ensure, pallet_prelude::DispatchResult, traits::Get};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::{
    traits::{CheckedAdd, CheckedDiv, CheckedMul, EnsureFrom, Hash, One, Saturating, Zero},
    ArithmeticError, BoundedVec, DispatchError,
};
use sp_std::{vec, vec::Vec};
use storage_hub_traits::ReadProvidersInterface;

use crate::{
    pallet,
    types::{
        FileKeyHasher, FileLocation, Fingerprint, ForestProof, KeyProof, MaxBspsPerStorageRequest,
        MultiAddresses, PeerIds, StorageData, StorageRequestBspsMetadata, StorageRequestMetadata,
    },
    Error, NextAvailableExpirationInsertionBlock, Pallet, StorageRequestBsps,
    StorageRequestExpirations, StorageRequests,
};
use crate::{
    types::{FileKey, TargetBspsRequired},
    BspsAssignmentThreshold,
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
        owner: T::AccountId,
        location: FileLocation<T>,
        fingerprint: Fingerprint<T>,
        size: StorageData<T>,
        bsps_required: Option<T::StorageRequestBspsRequiredType>,
        user_peer_ids: Option<PeerIds<T>>,
        data_server_sps: BoundedVec<T::AccountId, MaxBspsPerStorageRequest<T>>,
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
            user_peer_ids: user_peer_ids.unwrap_or_default(),
            data_server_sps,
            bsps_required,
            bsps_confirmed: T::StorageRequestBspsRequiredType::zero(),
            bsps_volunteered: T::StorageRequestBspsRequiredType::zero(),
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

        // Add storage request expiration at next available block.
        expect_or_err!(
            // TODO: Verify that try_append gets an empty BoundedVec when appending a first element.
            <StorageRequestExpirations<T>>::try_append(block_to_insert_expiration, location).ok(),
            "Storage request expiration should have enough slots available since it was just checked.",
            Error::<T>::StorageRequestExpiredNoSlotAvailable
        );

        Ok(())
    }

    /// Volunteer to store a file.
    ///
    /// *Callable only by BSP accounts*
    ///
    /// A BSP can only volunteer for a storage request if it is eligible based on the XOR of the `fingerprint` and the BSP's account ID and if it evaluates to a value
    /// less than the [globally computed threshold](BspsAssignmentThreshold). As the number of BSPs signed up increases, the threshold decreases, meaning there is a
    /// lower chance of a BSP being eligible to volunteer for a storage request.
    ///
    /// Though, as the storage request remains open, the threshold increases over time based on the number of blocks since the storage request was issued. This is to
    /// ensure that the storage request is fulfilled by opening up the opportunity for more BSPs to volunteer.
    pub(crate) fn do_bsp_volunteer(
        who: T::AccountId,
        location: FileLocation<T>,
        fingerprint: Fingerprint<T>,
    ) -> Result<(MultiAddresses<T>, StorageData<T>, T::AccountId), DispatchError> {
        let bsp =
            <T::Providers as storage_hub_traits::ProvidersInterface>::get_provider(who.clone())
                .ok_or(Error::<T>::NotABsp)?;

        // Check that the provider is indeed a BSP.
        ensure!(
            <T::Providers as storage_hub_traits::ReadProvidersInterface>::is_bsp(&bsp),
            Error::<T>::NotABsp
        );

        // TODO: Verify BSP has enough storage capacity to store the file

        // Check that the storage request exists.
        let mut file_metadata =
            <StorageRequests<T>>::get(&location).ok_or(Error::<T>::StorageRequestNotFound)?;

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

        // Compute BSP's threshold
        let bsp_threshold: T::ThresholdType = Self::compute_bsp_xor(
            fingerprint
                .as_ref()
                .try_into()
                .map_err(|_| Error::<T>::FailedToEncodeFingerprint)?,
            &who.encode()
                .try_into()
                .map_err(|_| Error::<T>::FailedToEncodeBsp)?,
        )?;

        // Get number of blocks since the storage request was issued.
        let blocks_since_requested: u128 = <frame_system::Pallet<T>>::block_number()
            .saturating_sub(file_metadata.requested_at)
            .try_into()
            .map_err(|_| Error::<T>::FailedToConvertBlockNumber)?;

        // Note. This should never fail since the storage request expiration would never reach such a high number.
        // Storage requests are cleared after reaching `StorageRequestTtl` blocks which is defined in the pallet Config.
        let blocks_since_requested: T::ThresholdType =
            T::ThresholdType::ensure_from(blocks_since_requested)?;

        // Compute the threshold increasing rate.
        let rate_increase =
            blocks_since_requested.saturating_mul(T::AssignmentThresholdMultiplier::get());

        // Compute current threshold needed to volunteer.
        let threshold = rate_increase.saturating_add(BspsAssignmentThreshold::<T>::get());

        // Check that the BSP's threshold is under the threshold required to qualify as BSP for the storage request.
        ensure!(bsp_threshold <= (threshold), Error::<T>::AboveThreshold);

        // Add BSP to storage request metadata.
        <StorageRequestBsps<T>>::insert(
            &location,
            &who,
            StorageRequestBspsMetadata::<T> {
                confirmed: false,
                _phantom: Default::default(),
            },
        );

        // Increment the number of bsps volunteered.
        match file_metadata
            .bsps_volunteered
            .checked_add(&T::StorageRequestBspsRequiredType::one())
        {
            Some(inc_bsps_volunteered) => {
                file_metadata.bsps_volunteered = inc_bsps_volunteered;
            }
            None => {
                return Err(ArithmeticError::Overflow.into());
            }
        }

        <StorageRequests<T>>::set(&location, Some(file_metadata.clone()));

        let multiaddresses = T::Providers::get_bsp_multiaddresses(&bsp)?;
        let size = file_metadata.size;
        let owner = file_metadata.owner;

        Ok((multiaddresses, size, owner))
    }

    /// Confirm storing a file.
    ///
    /// *Callable only by BSP accounts*
    ///
    /// This function can only be called after a BSP has volunteered for the storage request. The BSP must provide a merkle proof of the file
    /// and a proof of inclusion of the `file_key` in their merkle patricia trie.
    ///
    /// If the proof is valid, the root of the BSP is updated to reflect the new root of the merkle patricia trie and the number of `bsps_confirmed` is
    /// incremented. If the number of `bsps_confirmed` reaches the number of `bsps_required`, the storage request is deleted. Finally the BSP's data
    /// used is incremented by the size of the file.
    pub(crate) fn do_bsp_confirm_storing(
        who: T::AccountId,
        location: FileLocation<T>,
        root: FileKey<T>,
        forest_proof: ForestProof<T>,
        key_proof: KeyProof<T>,
    ) -> DispatchResult {
        let bsp =
            <T::Providers as storage_hub_traits::ProvidersInterface>::get_provider(who.clone())
                .ok_or(Error::<T>::NotABsp)?;

        // Check that the provider is indeed a BSP.
        ensure!(
            <T::Providers as storage_hub_traits::ReadProvidersInterface>::is_bsp(&bsp),
            Error::<T>::NotABsp
        );

        // Check that the storage request exists.
        let mut file_metadata =
            <StorageRequests<T>>::get(&location).ok_or(Error::<T>::StorageRequestNotFound)?;

        expect_or_err!(
                    file_metadata.bsps_confirmed < file_metadata.bsps_required,
                    "Storage request should never have confirmed bsps equal to or greater than required bsps, since they are deleted when it is reached.",
                    Error::<T>::StorageRequestBspsRequiredFulfilled,
                    bool
                );

        // Check that the BSP has volunteered for the storage request.
        ensure!(
            <StorageRequestBsps<T>>::contains_key(&location, &who),
            Error::<T>::BspNotVolunteered
        );

        let requests = expect_or_err!(
            <StorageRequestBsps<T>>::get(&location, &who),
            "BSP should exist since we checked it above",
            Error::<T>::ImpossibleFailedToGetValue
        );

        // Check that the storage provider has not already confirmed storing the file.
        ensure!(!requests.confirmed, Error::<T>::BspAlreadyConfirmed);

        // Check that the number of confirmed bsps is less than the required bsps and increment it.
        expect_or_err!(
            file_metadata.bsps_confirmed < file_metadata.bsps_required,
            "Storage request should never have confirmed bsps equal to or greater than required bsps, since they are deleted when it is reached.",
            Error::<T>::StorageRequestBspsRequiredFulfilled,
            bool
        );

        // Increment the number of bsps confirmed.
        match file_metadata
            .bsps_confirmed
            .checked_add(&T::StorageRequestBspsRequiredType::one())
        {
            Some(inc_bsps_confirmed) => {
                file_metadata.bsps_confirmed = inc_bsps_confirmed;
            }
            None => {
                return Err(ArithmeticError::Overflow.into());
            }
        }

        // TODO: Initialise challenges properly constructing the key for this particular file.
        let file_key = FileKeyHasher::<T>::hash(&location.encode());
        let challenges = vec![file_key];

        // Check that the forest proof is valid.
        <T::ProofDealer as storage_hub_traits::ProofsDealerInterface>::verify_forest_proof(
            &bsp,
            challenges.as_slice(),
            &forest_proof,
        )?;

        // TODO: Generate challenges for the key proof properly.
        let challenges = vec![];

        // Check that the key proof is valid.
        <T::ProofDealer as storage_hub_traits::ProofsDealerInterface>::verify_key_proof(
            &file_key,
            &challenges,
            &key_proof,
        )?;

        // Remove storage request if we reached the required number of bsps.
        if file_metadata.bsps_confirmed == file_metadata.bsps_required {
            // Remove storage request metadata.
            <StorageRequests<T>>::remove(&location);

            // There should only be the number of bsps volunteered under the storage request prefix.
            let remove_limit: u32 = file_metadata
                .bsps_volunteered
                .try_into()
                .map_err(|_| Error::<T>::FailedTypeConversion)?;

            // Remove storage request bsps
            let removed = <StorageRequestBsps<T>>::clear_prefix(&location, remove_limit, None);

            // Make sure that the expected number of bsps were removed.
            expect_or_err!(
                removed.backend == remove_limit,
                "Number of volunteered bsps for storage request should have been removed",
                Error::<T>::UnexpectedNumberOfRemovedVolunteeredBsps,
                bool
            );
        } else {
            // Update storage request metadata.
            <StorageRequests<T>>::set(&location, Some(file_metadata.clone()));

            // Update bsp for storage request.
            <StorageRequestBsps<T>>::mutate(&location, &who, |bsp| {
                if let Some(bsp) = bsp {
                    bsp.confirmed = true;
                }
            });
        }

        // Update root of bsp.
        <T::Providers as storage_hub_traits::MutateProvidersInterface>::change_root_bsp(bsp, root)?;

        // Add data to storage provider.
        <T::Providers as storage_hub_traits::MutateProvidersInterface>::increase_data_used(
            &who,
            file_metadata.size,
        )?;

        Ok(())
    }

    /// Revoke a storage request.
    ///
    /// *Callable by the owner of the storage request. Users, BSPs and MSPs can be the owners.*
    ///
    /// When the owner revokes a storage request which has already been confirmed by some BSPs, a challenge (with priority) is
    /// issued to force the BSPs to update their storage root to uninclude the file from their storage.
    ///
    /// All BSPs that have volunteered to store the file are removed from the storage request and the storage request is deleted.
    pub(crate) fn do_revoke_storage_request(
        who: T::AccountId,
        location: FileLocation<T>,
        file_key: FileKey<T>,
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

        // Check if there are already BSPs who have confirmed to store the file.
        if file_metadata.bsps_confirmed >= T::StorageRequestBspsRequiredType::zero() {
            // Issue a challenge to force the BSPs to update their storage root.
            <T::ProofDealer as storage_hub_traits::ProofsDealerInterface>::challenge_with_priority(
                &file_key,
            )?;
        }

        // There should only be the number of bsps volunteered under the storage request prefix.
        let remove_limit: u32 = file_metadata
            .bsps_volunteered
            .try_into()
            .map_err(|_| Error::<T>::FailedTypeConversion)?;

        // Remove storage request bsps
        let removed = <StorageRequestBsps<T>>::clear_prefix(&location, remove_limit, None);

        // Make sure that the expected number of bsps were removed.
        expect_or_err!(
            removed.backend == remove_limit,
            "Number of volunteered bsps for storage request should have been removed",
            Error::<T>::UnexpectedNumberOfRemovedVolunteeredBsps,
            bool
        );

        // Remove storage request.
        <StorageRequests<T>>::remove(&location);

        Ok(())
    }

    /// BSP stops storing a file.
    ///
    /// *Callable only by BSP accounts*
    ///
    /// This function covers a few scenarios in which a BSP invokes this function to stop storing a file:
    ///
    /// 1. The BSP has volunteered and confirmed storing the file and wants to stop storing it while the storage request is still open.
    ///
    /// > In this case, the BSP has volunteered and confirmed storing the file for an existing storage request.
    ///     Therefore, we decrement the `bsps_confirmed` by 1.  
    ///
    /// 2. The BSP stops storing a file that has an opened storage request but is not a volunteer.
    ///
    /// > In this case, the storage request was probably created by another BSP for some reason (e.g. that BSP lost the file)
    ///     and the current BSP is not a volunteer for this since it is already storing it. But since they to have stopped storing it,
    ///     we increment the `bsps_requred` by 1.
    ///
    /// 3. The BSP stops storing a file that no longer has an opened storage request.
    ///
    /// > In this case, there is no storage request opened for the file they no longer are storing. Therefore, we
    ///     create a storage request with `bsps_required` set to 1.
    ///
    /// *This function does not give BSPs the possibility to remove themselves from being a __volunteer__ of a storage request.*
    ///
    /// A proof of storing the file is required to record the new root of the BSPs merkle patricia trie. First we validate the proof
    /// and ensure the `file_key` is indeed part of the merkle patricia trie. Then finally we re-compute the new merkle patricia trie root
    /// without the `file_key` and update the root of the BSP.
    ///
    /// `can_serve`: A flag that indicates if the BSP can serve the file to other BSPs. If the BSP can serve the file, then
    /// they are added to the storage request as a data server.
    pub(crate) fn do_bsp_stop_storing(
        who: T::AccountId,
        _file_key: FileKey<T>,
        location: FileLocation<T>,
        owner: T::AccountId,
        fingerprint: Fingerprint<T>,
        size: StorageData<T>,
        can_serve: bool,
    ) -> DispatchResult {
        // TODO: charge SP for this action.
        // TODO: Require & verify proof that the file key is indeed stored by the BSP.
        // TODO: Check that the hash of all the metadata is equal to the `file_key` hash.
        match <StorageRequests<T>>::get(&location) {
            Some(mut metadata) => {
                match <StorageRequestBsps<T>>::get(&location, &who) {
                    // We hit scenario 1. The BSP is a volunteer and has confirmed storing the file.
                    // We need to decrement the number of bsps confirmed and volunteered and remove the BSP from the storage request.
                    Some(bsp) => {
                        expect_or_err!(
                            bsp.confirmed,
                            "BSP should have confirmed storing the file since we verify the proof and their root matches the one in storage",
                            Error::<T>::BspNotConfirmed,
                            bool
                        );

                        metadata.bsps_confirmed =
                            metadata.bsps_confirmed.saturating_sub(1u32.into());

                        metadata.bsps_volunteered =
                            metadata.bsps_volunteered.saturating_sub(1u32.into());

                        <StorageRequestBsps<T>>::remove(&location, &who);
                    }
                    // We hit scenario 2. There is an open storage request but the BSP is not a volunteer.
                    // We need to increment the number of bsps required.
                    None => {
                        metadata.bsps_required = metadata.bsps_required.saturating_add(1u32.into())
                    }
                }

                // Update storage request metadata.
                <StorageRequests<T>>::set(&location, Some(metadata));
            }
            // We hit scenario 3. There is no storage request opened for the file.
            // We need to create a new storage request with a single bsp required.
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

        // TODO: compute new root from proof and update the storage root of bsp.

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

    /// Compute the asymptotic threshold point for the given number of total BSPs.
    ///
    /// This function calculates the threshold at which the decay factor stabilizes,
    /// representing an horizontal asymptote.
    pub(crate) fn compute_asymptotic_threshold_point(
        total_bsps: u32,
    ) -> Result<T::ThresholdType, Error<T>> {
        let asymptotic_decay_factor = T::AssignmentThresholdDecayFactor::get().saturating_pow(
            total_bsps
                .try_into()
                .map_err(|_| Error::<T>::FailedToConvertBlockNumber)?,
        );

        Ok(T::AssignmentThresholdAsymptote::get().saturating_add(asymptotic_decay_factor))
    }

    /// Calculate the XOR of the fingerprint and the BSP.
    fn compute_bsp_xor(
        fingerprint: &[u8; 32],
        bsp: &[u8; 32],
    ) -> Result<T::ThresholdType, Error<T>> {
        let xor_result = fingerprint
            .iter()
            .zip(bsp.iter())
            .map(|(&x1, &x2)| x1 ^ x2)
            .collect::<Vec<_>>();

        T::ThresholdType::decode(&mut &xor_result[..])
            .map_err(|_| Error::<T>::FailedToDecodeThreshold)
    }
}

impl<T: crate::Config> storage_hub_traits::SubscribeProvidersInterface for Pallet<T> {
    type Provider = T::AccountId;

    fn subscribe_bsp_sign_up(_who: &Self::Provider) -> DispatchResult {
        // Adjust bsp assignment threshold by applying the decay function after removing the asymptote
        let mut bsp_assignment_threshold = BspsAssignmentThreshold::<T>::get();
        let base_threshold =
            bsp_assignment_threshold.saturating_sub(T::AssignmentThresholdAsymptote::get());

        bsp_assignment_threshold = base_threshold
            .checked_mul(&T::AssignmentThresholdDecayFactor::get())
            .ok_or(Error::<T>::ThresholdArithmeticError)?
            .saturating_add(T::AssignmentThresholdAsymptote::get());

        BspsAssignmentThreshold::<T>::put(bsp_assignment_threshold);

        Ok(())
    }

    fn subscribe_bsp_sign_off(_who: &Self::Provider) -> DispatchResult {
        // Adjust bsp assignment threshold by applying the inverse of the decay function after removing the asymptote
        let mut bsp_assignment_threshold = BspsAssignmentThreshold::<T>::get();
        let base_threshold =
            bsp_assignment_threshold.saturating_sub(T::AssignmentThresholdAsymptote::get());

        bsp_assignment_threshold = base_threshold
            .checked_div(&T::AssignmentThresholdDecayFactor::get())
            .ok_or(Error::<T>::ThresholdArithmeticError)?
            .saturating_add(T::AssignmentThresholdAsymptote::get());

        BspsAssignmentThreshold::<T>::put(bsp_assignment_threshold);

        Ok(())
    }
}
