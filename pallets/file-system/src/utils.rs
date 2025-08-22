use codec::{Decode, Encode};
use core::cmp::max;
use frame_support::{
    ensure,
    pallet_prelude::DispatchResult,
    traits::{
        fungible::{InspectHold, Mutate, MutateHold},
        nonfungibles_v2::{Create, Destroy},
        tokens::{Fortitude, Precision, Preservation, Restriction},
        Get,
    },
};
use num_bigint::BigUint;
use sp_runtime::{
    traits::{
        Bounded, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Convert, ConvertBack, Hash, One,
        SaturatedConversion, Saturating, Verify, Zero,
    },
    ArithmeticError, BoundedBTreeSet, BoundedVec, DispatchError,
};
use sp_std::{collections::btree_set::BTreeSet, vec::Vec};

use pallet_file_system_runtime_api::{
    GenericApplyDeltaEventInfoError, IsStorageRequestOpenToVolunteersError,
    QueryBspConfirmChunksToProveForFileError, QueryConfirmChunksToProveForFileError,
    QueryFileEarliestVolunteerTickError, QueryMspConfirmChunksToProveForFileError,
};
use pallet_nfts::{CollectionConfig, CollectionSettings, ItemSettings, MintSettings, MintType};
use shp_constants::GIGAUNIT;
use shp_file_metadata::ChunkId;
use shp_traits::{
    CommitRevealRandomnessInterface, MutateBucketsInterface, MutateProvidersInterface,
    MutateStorageProvidersInterface, PaymentStreamsInterface, PricePerGigaUnitPerTickInterface,
    ProofsDealerInterface, ReadBucketsInterface, ReadProvidersInterface,
    ReadStorageProvidersInterface, ReadUserSolvencyInterface, TrieAddMutation, TrieRemoveMutation,
};
use sp_std::collections::btree_map::BTreeMap;

use crate::{
    pallet,
    types::{
        BucketIdFor, BucketMoveRequestResponse, BucketNameFor, CollectionConfigFor,
        CollectionIdFor, EitherAccountIdOrMspId, ExpirationItem, FileKeyHasher, FileKeyWithProof,
        FileLocation, FileOperation, FileOperationIntention, Fingerprint, ForestProof,
        IncompleteStorageRequestMetadata, MerkleHash, MoveBucketRequestMetadata, MultiAddresses,
        PeerIds, PendingStopStoringRequest, ProviderIdFor, RejectedStorageRequest,
        ReplicationTarget, ReplicationTargetType, StorageDataUnit, StorageRequestBspsMetadata,
        StorageRequestMetadata, StorageRequestMspAcceptedFileKeys, StorageRequestMspBucketResponse,
        StorageRequestMspResponse, TickNumber, ValuePropId,
    },
    weights::WeightInfo,
    BucketsWithStorageRequests, Error, Event, HoldReason, IncompleteStorageRequests, Pallet,
    PendingMoveBucketRequests, PendingStopStoringRequests, StorageRequestBsps,
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
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        if !$condition {
            #[cfg(test)]
            unreachable!($error_msg);

            #[allow(unreachable_code)]
            {
                Err($error_type)?
            }
        }
    }};
    // Handle Result type
    ($result:expr, $error_msg:expr, $error_type:path, result) => {{
        match $result {
            Ok(value) => value,
            Err(_) => {
                #[cfg(test)]
                unreachable!($error_msg);

                #[allow(unreachable_code)]
                {
                    Err($error_type)?
                }
            }
        }
    }};
}

impl<T> Pallet<T>
where
    T: pallet::Config,
{
    /// This function is used primarily for the runtime API exposed for BSPs to call before they attempt to volunteer for a storage request.
    pub fn is_storage_request_open_to_volunteers(
        file_key: MerkleHash<T>,
    ) -> Result<bool, IsStorageRequestOpenToVolunteersError>
    where
        T: frame_system::Config,
    {
        // Check if the storage request exists.
        let storage_request = match <StorageRequests<T>>::get(&file_key) {
            Some(storage_request) => storage_request,
            None => {
                return Err(IsStorageRequestOpenToVolunteersError::StorageRequestNotFound);
            }
        };

        // This should always be true since the storage request will be deleted from storage if the `bsps_confirmed` is equal to `bsps_required`.
        Ok(storage_request.bsps_confirmed < storage_request.bsps_required)
    }

    /// Compute the tick number at which the BSP is eligible to volunteer for a storage request.
    pub fn query_earliest_file_volunteer_tick(
        bsp_id: ProviderIdFor<T>,
        file_key: MerkleHash<T>,
    ) -> Result<TickNumber<T>, QueryFileEarliestVolunteerTickError>
    where
        T: frame_system::Config,
    {
        // Get the tick number at which the storage request was created and
        // the amount of BSPs required to confirm storing the file for the storage request
        // to be considered fulfilled (replication target).
        let (storage_request_tick, replication_target) = match <StorageRequests<T>>::get(&file_key)
        {
            Some(storage_request) => (storage_request.requested_at, storage_request.bsps_required),
            None => {
                return Err(QueryFileEarliestVolunteerTickError::StorageRequestNotFound);
            }
        };

        // Get the threshold for the BSP to be able to volunteer for the storage request.
        // The current eligibility value of this storage request for this BSP has to be greater than
        // this for the BSP to be able to volunteer.
        let bsp_volunteering_threshold = Self::get_volunteer_threshold_of_bsp(&bsp_id, &file_key);

        // Get the current eligibility value of this storage request and the slope with which it
        // increments every tick (this is weighted considering the BSP reputation so it depends on the BSP).
        let (bsp_current_eligibility_value, bsp_eligibility_slope) =
            Self::compute_request_eligibility_criteria(
                &bsp_id,
                storage_request_tick,
                replication_target,
            )
            .map_err(|_| QueryFileEarliestVolunteerTickError::FailedToComputeEligibilityCriteria)?;

        // Calculate the difference between the BSP's threshold and the current eligibility value.
        let eligibility_diff =
            match bsp_volunteering_threshold.checked_sub(&bsp_current_eligibility_value) {
                Some(diff) => diff,
                None => {
                    // The BSP's threshold is less than the eligibility current value, which means the BSP is already eligible to volunteer.
                    let current_tick =
                        <T::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                    return Ok(current_tick);
                }
            };

        // If the BSP can't volunteer yet, calculate the number of ticks it has to wait for before it can.
        let min_ticks_to_wait_to_volunteer =
            match eligibility_diff.checked_div(&bsp_eligibility_slope) {
                Some(ticks) => max(ticks, T::ThresholdType::one()),
                None => {
                    return Err(QueryFileEarliestVolunteerTickError::ThresholdArithmeticError);
                }
            };

        // Compute the earliest tick number at which the BSP can send the volunteer request.
        let earliest_volunteer_tick = storage_request_tick.saturating_add(
            T::ThresholdTypeToTickNumber::convert(min_ticks_to_wait_to_volunteer),
        );

        Ok(earliest_volunteer_tick)
    }

    /// Compute the eligibility value threshold for a BSP to be able to volunteer for a storage request
    /// for a file which has the specified file key.
    ///
    /// The threshold is computed by concatenating the encoded BSP ID and file key,
    /// then hashing the result to get the volunteering hash. The volunteering hash is then
    /// converted to the threshold type.
    ///
    /// We use the file key for additional entropy to ensure that the threshold is unique.
    pub fn get_volunteer_threshold_of_bsp(
        bsp_id: &ProviderIdFor<T>,
        file_key: &MerkleHash<T>,
    ) -> T::ThresholdType {
        // Concatenate the encoded BSP ID and file key and hash them to get the volunteering hash.
        let concatenated = sp_std::vec![bsp_id.encode(), file_key.encode()].concat();
        let volunteering_hash =
            <<T as frame_system::Config>::Hashing as Hash>::hash(concatenated.as_ref());

        // Return the threshold needed for the BSP to be able to volunteer for the storage request.
        T::HashToThresholdType::convert(volunteering_hash)
    }

    /// Compute the eligibility value of a storage request issued at `requested_at` with `bsps_required` as
    /// its replication target, for a specific BSP identified by `bsp_id`, and the slope with which the
    /// eligibility value increases every tick.
    ///
    /// The eligibility initial value depends on the global BSP reputation weight and the storage
    /// request's replication target, and increases linearly over time to achieve its maximum value after
    /// [`crate::Config::TickRangeToMaximumThreshold`] ticks, a constant defined in this pallet's configuration.
    /// Both these values get weighted with the BSP's reputation weight to give an advantage to BSPs with
    /// higher reputation weights, since both the initial weighted eligibility value and slope will be
    /// higher for them.
    ///
    /// A BSP is eligible to volunteer for a storage request when the returned eligibility value
    /// is greater than that BSP's volunteer threshold value.
    ///
    /// The formalized formulas are documented in the [README](https://github.com/Moonsong-Labs/storage-hub/blob/main/pallets/file-system/README.md#volunteering-succeeding-threshold-checks).
    pub fn compute_request_eligibility_criteria(
        bsp_id: &ProviderIdFor<T>,
        requested_at: TickNumber<T>,
        replication_target: ReplicationTargetType<T>,
    ) -> Result<(T::ThresholdType, T::ThresholdType), DispatchError> {
        // Get the maximum eligibility value, which would allow all BSP's to volunteer for the storage request.
        let max_eligibility_value = T::ThresholdType::max_value();

        // Get the global reputation weight of all BSPs.
        let global_reputation_weight =
            T::ThresholdType::from(T::Providers::get_global_bsps_reputation_weight());

        // If the global reputation weight is zero, there are no BSPs in the network, so no storage requests can be fulfilled.
        if global_reputation_weight == T::ThresholdType::zero() {
            return Err(Error::<T>::NoGlobalReputationWeightSet.into());
        }

        // Calculate the starting eligibility value for all BSPs, regardless of their reputation weight.
        // This is set to maximize the probability that exactly `replication_target` BSPs with the starting
        // reputation weight will be able to volunteer in the initial tick, which minimizes the probability
        // that a malicious user controlling a large number of BSPs will be able to control all the
        // BSPs that volunteer in the initial tick.
        //
        // The calculation is designed to achieve the following:
        //
        // 1. Initially, the global reputation weight will be low since there won't be many BSPs in the network,
        // so the starting eligibility value will be high, allowing more BSPs to volunteer in the initial tick
        // thus decreasing the time it takes for a storage request to be fulfilled even in the early days of the network.
        //
        // 2. As the global reputation weight of all BSPs increases (i.e., more BSPs join the network or the ones
        // already participating gain reputation), the starting eligibility value for all BSPs will decrease, allowing
        // less BSPs to volunteer in the initial tick and prioritizing higher reputation BSPs.
        //
        // 3. Multiplying the starting eligibility value by the replication target ensures that the number of BSPs with
        // the starting reputation weight that can volunteer in the initial tick is probabilistically equal to the
        // replication target chosen by the user, thus decreasing the time it takes for a storage request to be fulfilled
        // (in comparison to not multiplying by the replication target) while still preserving the security of the network
        // against malicious users controlling a large number of BSPs (in comparison to having a really high
        // starting eligibility value).
        let global_eligibility_starting_value = max_eligibility_value
            .checked_div(&global_reputation_weight)
            .unwrap_or(T::ThresholdType::one())
            .saturating_mul(replication_target.into());

        // Calculate the rate of increase per tick of the global eligibility value.
        // This is such that, after `TickRangeToMaximumThreshold` ticks, the global eligibility value will be
        // equal to `max_eligibility_value`, thus allowing all BSPs to volunteer.
        // The base slope for the storage request should be at the very least 1, so that all BSPs can volunteer eventually.
        let base_slope = max_eligibility_value
            .saturating_sub(global_eligibility_starting_value)
            .checked_div(&T::ThresholdTypeToTickNumber::convert_back(
                T::TickRangeToMaximumThreshold::get(),
            ))
            .unwrap_or(T::ThresholdType::one());
        let base_slope = max(base_slope, T::ThresholdType::one());

        // Get the BSP's reputation weight.
        let bsp_reputation_weight =
            T::ThresholdType::from(T::Providers::get_bsp_reputation_weight(&bsp_id)?);

        // If the BSP's reputation weight is zero, the BSP is not allowed to volunteer for any storage request.
        if bsp_reputation_weight == T::ThresholdType::zero() {
            return Err(Error::<T>::NoBspReputationWeightSet.into());
        }

        // The eligibility starting value for this BSP is the global one weighted by the BSP's reputation weight.
        let bsp_eligibility_starting_value =
            global_eligibility_starting_value.saturating_mul(bsp_reputation_weight);

        // The rate of increase for this BSP is the global one weighted by the BSP's reputation weight.
        let bsp_eligibility_slope = base_slope
            .checked_mul(&bsp_reputation_weight)
            .unwrap_or(max_eligibility_value);

        // Get the current tick.
        let current_tick_number =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();

        // Get the amount of elapsed ticks since the storage request was issued and convert the number to the threshold type.
        let elapsed_ticks = current_tick_number.saturating_sub(requested_at);
        let elapsed_ticks = T::ThresholdTypeToTickNumber::convert_back(elapsed_ticks);

        // Finally, calculate the current eligibility value for this BSP.
        let current_eligibility_value_for_bsp = bsp_eligibility_starting_value
            .saturating_add(bsp_eligibility_slope.saturating_mul(elapsed_ticks));

        Ok((current_eligibility_value_for_bsp, bsp_eligibility_slope))
    }

    pub fn query_bsp_confirm_chunks_to_prove_for_file(
        bsp_id: ProviderIdFor<T>,
        file_key: MerkleHash<T>,
    ) -> Result<Vec<ChunkId>, QueryBspConfirmChunksToProveForFileError> {
        // Get the storage request metadata.
        let storage_request_metadata = match <StorageRequests<T>>::get(&file_key) {
            Some(storage_request) => storage_request,
            None => {
                return Err(QueryBspConfirmChunksToProveForFileError::StorageRequestNotFound);
            }
        };

        Self::query_confirm_chunks_to_prove_for_file(bsp_id, storage_request_metadata, file_key)
            .map_err(|e| QueryBspConfirmChunksToProveForFileError::ConfirmChunks(e))
    }

    pub fn query_msp_confirm_chunks_to_prove_for_file(
        msp_id: ProviderIdFor<T>,
        file_key: MerkleHash<T>,
    ) -> Result<Vec<ChunkId>, QueryMspConfirmChunksToProveForFileError> {
        // Get the storage request metadata.
        let storage_request_metadata = match <StorageRequests<T>>::get(&file_key) {
            Some(storage_request) => storage_request,
            None => {
                return Err(QueryMspConfirmChunksToProveForFileError::StorageRequestNotFound);
            }
        };

        Self::query_confirm_chunks_to_prove_for_file(msp_id, storage_request_metadata, file_key)
            .map_err(|e| QueryMspConfirmChunksToProveForFileError::ConfirmChunks(e))
    }

    pub fn decode_generic_apply_delta_event_info(
        encoded_event_info: Vec<u8>,
    ) -> Result<BucketIdFor<T>, GenericApplyDeltaEventInfoError> {
        Self::do_decode_generic_apply_delta_event_info(encoded_event_info.as_ref())
            .map_err(|_| GenericApplyDeltaEventInfoError::DecodeError)
    }

    fn query_confirm_chunks_to_prove_for_file(
        provider_id: ProviderIdFor<T>,
        storage_request_metadata: StorageRequestMetadata<T>,
        file_key: MerkleHash<T>,
    ) -> Result<Vec<ChunkId>, QueryConfirmChunksToProveForFileError> {
        // Generate the list of chunks to prove.
        let challenges = Self::generate_chunk_challenges_on_sp_confirm(
            provider_id,
            file_key,
            &storage_request_metadata,
        )
        .map_err(|_| QueryConfirmChunksToProveForFileError::FailedToGenerateChunkChallenges)?;

        let chunks = storage_request_metadata
            .to_file_metadata()
            .map_err(|_| QueryConfirmChunksToProveForFileError::FailedToCreateFileMetadata)?
            .chunks_count();

        let chunks_to_prove = challenges
            .iter()
            .map(|challenge| {
                let challenged_chunk = BigUint::from_bytes_be(challenge.as_ref()) % chunks;
                let challenged_chunk: ChunkId =
                    ChunkId::new(challenged_chunk.try_into().map_err(|_| {
                        QueryConfirmChunksToProveForFileError::ChallengedChunkToChunkIdError
                    })?);

                Ok(challenged_chunk)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(chunks_to_prove)
    }

    fn generate_chunk_challenges_on_sp_confirm(
        sp_id: ProviderIdFor<T>,
        file_key: MerkleHash<T>,
        storage_request_metadata: &StorageRequestMetadata<T>,
    ) -> Result<
        Vec<<<T as pallet::Config>::Providers as ReadProvidersInterface>::MerkleHash>,
        DispatchError,
    > {
        let file_metadata = storage_request_metadata.clone().to_file_metadata()?;
        let chunks_to_check = file_metadata.chunks_to_check();

        let mut challenges =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::generate_challenges_from_seed(
                T::MerkleHashToRandomnessOutput::convert(file_key),
                &sp_id,
                chunks_to_check.saturating_sub(One::one()),
            );

        let last_chunk_id = file_metadata.last_chunk_id();

        challenges.push(T::ChunkIdToMerkleHash::convert(last_chunk_id));

        Ok(challenges)
    }

    /// Create a bucket for an owner (user) under a given MSP account.
    pub(crate) fn do_create_bucket(
        sender: T::AccountId,
        msp_id: ProviderIdFor<T>,
        name: BucketNameFor<T>,
        private: bool,
        value_prop_id: ValuePropId<T>,
    ) -> Result<(BucketIdFor<T>, Option<CollectionIdFor<T>>), DispatchError> {
        // Check if the MSP is indeed an MSP.
        ensure!(
            <T::Providers as ReadStorageProvidersInterface>::is_msp(&msp_id),
            Error::<T>::NotAMsp
        );

        // Check that the selected value proposition is currently available under the MSP.
        ensure!(
            <T::Providers as ReadStorageProvidersInterface>::is_value_prop_available(
                &msp_id,
                &value_prop_id
            ),
            Error::<T>::ValuePropositionNotAvailable
        );

        // Check if MSP is insolvent
        ensure!(
            !<T::Providers as ReadProvidersInterface>::is_provider_insolvent(msp_id),
            Error::<T>::OperationNotAllowedForInsolventProvider
        );

        // Check that the user is not currently insolvent.
        ensure!(
            !<T::UserSolvency as ReadUserSolvencyInterface>::is_user_insolvent(&sender),
            Error::<T>::OperationNotAllowedWithInsolventUser
        );

        // Create collection only if bucket is private
        let maybe_collection_id = if private {
            // The `owner` of the collection is also the admin of the collection since most operations require the sender to be the admin.
            Some(Self::create_collection(sender.clone())?)
        } else {
            None
        };

        let bucket_id = <T as crate::Config>::Providers::derive_bucket_id(&sender, name);

        <T::Providers as MutateBucketsInterface>::add_bucket(
            msp_id,
            sender.clone(),
            bucket_id,
            private,
            maybe_collection_id.clone(),
            value_prop_id,
        )?;

        Ok((bucket_id, maybe_collection_id))
    }

    /// This does not guarantee that the MSP will have enough storage capacity to store the entire bucket. Therefore,
    /// between the creation of the request and its expiration, the MSP can increase its capacity before accepting the request.
    ///
    /// Forcing the MSP to have enough capacity before the request is created would not enable MSPs to automatically scale based on demand.
    pub(crate) fn do_request_move_bucket(
        sender: T::AccountId,
        bucket_id: BucketIdFor<T>,
        new_msp_id: ProviderIdFor<T>,
        new_value_prop_id: ValuePropId<T>,
    ) -> Result<(), DispatchError> {
        // Check that the user is not currently insolvent.
        ensure!(
            !<T::UserSolvency as ReadUserSolvencyInterface>::is_user_insolvent(&sender),
            Error::<T>::OperationNotAllowedWithInsolventUser
        );

        // Check if the sender is the owner of the bucket.
        ensure!(
            <T::Providers as ReadBucketsInterface>::is_bucket_owner(&sender, &bucket_id)?,
            Error::<T>::NotBucketOwner
        );

        // Check if the new MSP is indeed an MSP.
        ensure!(
            <T::Providers as ReadStorageProvidersInterface>::is_msp(&new_msp_id),
            Error::<T>::NotAMsp
        );

        // Check if the new value proposition exists under the new MSP and is currently available.
        ensure!(
            <T::Providers as ReadStorageProvidersInterface>::is_value_prop_available(
                &new_msp_id,
                &new_value_prop_id
            ),
            Error::<T>::ValuePropositionNotAvailable
        );

        // Check if the newly selected MSP is not insolvent
        ensure!(
            !<T::Providers as ReadProvidersInterface>::is_provider_insolvent(new_msp_id),
            Error::<T>::OperationNotAllowedForInsolventProvider
        );

        // Get the current MSP that is storing the bucket, if any.
        let maybe_previous_msp_id =
            <T::Providers as ReadBucketsInterface>::get_msp_of_bucket(&bucket_id)?;

        // Check if the bucket is already stored by the new MSP.
        if let Some(previous_msp_id) = maybe_previous_msp_id {
            ensure!(
                previous_msp_id != new_msp_id,
                Error::<T>::MspAlreadyStoringBucket
            );
        }

        // Check that the bucket is not being moved.
        ensure!(
            !<PendingMoveBucketRequests<T>>::contains_key(&bucket_id),
            Error::<T>::BucketIsBeingMoved
        );

        // Check if there are any open storage requests for the bucket.
        // Do not allow any storage requests and move bucket requests to coexist for the same bucket.
        ensure!(
            !<BucketsWithStorageRequests<T>>::iter_prefix(bucket_id)
                .next()
                .is_some(),
            Error::<T>::StorageRequestExists
        );

        // Register the move bucket request.
        <PendingMoveBucketRequests<T>>::insert(
            bucket_id,
            MoveBucketRequestMetadata {
                requester: sender.clone(),
                new_msp_id,
                new_value_prop_id,
            },
        );

        let expiration_item = ExpirationItem::MoveBucketRequest(bucket_id);
        Self::enqueue_expiration_item(expiration_item)?;

        Ok(())
    }

    pub(crate) fn do_msp_respond_move_bucket_request(
        sender: T::AccountId,
        bucket_id: BucketIdFor<T>,
        response: BucketMoveRequestResponse,
    ) -> Result<(Option<ProviderIdFor<T>>, ProviderIdFor<T>, ValuePropId<T>), DispatchError> {
        let msp_id = <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(&sender)
            .ok_or(Error::<T>::NotAMsp)?;

        // Check if MSP is insolvent.
        ensure!(
            !<T::Providers as ReadProvidersInterface>::is_provider_insolvent(msp_id),
            Error::<T>::OperationNotAllowedForInsolventProvider
        );

        // Check if the sender is a MSP.
        ensure!(
            <T::Providers as ReadStorageProvidersInterface>::is_msp(&msp_id),
            Error::<T>::NotAMsp
        );

        // Check if the move bucket request exists for the MSP and bucket, removing it from storage if it does.
        let move_bucket_request_metadata = expect_or_err!(
            <PendingMoveBucketRequests<T>>::take(bucket_id),
            "Move bucket request should exist",
            Error::<T>::MoveBucketRequestNotFound
        );

        // Ensure the new MSP that the user selected to store the bucket matches the one responding the request.
        ensure!(
            msp_id == move_bucket_request_metadata.new_msp_id,
            Error::<T>::NotSelectedMsp
        );

        // Get the previous MSP that was storing the bucket, if any.
        let maybe_previous_msp_id =
            <T::Providers as ReadBucketsInterface>::get_msp_of_bucket(&bucket_id)?;

        // If the new MSP accepted storing the bucket...
        if response == BucketMoveRequestResponse::Accepted {
            // Get the current size of the bucket.
            let bucket_size = <T::Providers as ReadBucketsInterface>::get_bucket_size(&bucket_id)?;

            // If another MSP was previously storing the bucket, update its used capacity to reflect the removal of the bucket.
            if let Some(previous_msp_id) = maybe_previous_msp_id {
                <T::Providers as MutateStorageProvidersInterface>::decrease_capacity_used(
                    &previous_msp_id,
                    bucket_size,
                )?;
            }

            // Check if the new MSP has enough available capacity to store the bucket.
            ensure!(
                <T::Providers as ReadStorageProvidersInterface>::available_capacity(&msp_id)
                    >= bucket_size,
                Error::<T>::InsufficientAvailableCapacity
            );

            // Change the MSP that stores the bucket.
            <T::Providers as MutateBucketsInterface>::assign_msp_to_bucket(
                &bucket_id,
                &msp_id,
                &move_bucket_request_metadata.new_value_prop_id,
            )?;

            // Increase the used capacity of the new MSP.
            <T::Providers as MutateStorageProvidersInterface>::increase_capacity_used(
                &msp_id,
                bucket_size,
            )?;
        }

        Ok((
            maybe_previous_msp_id,
            move_bucket_request_metadata.new_msp_id,
            move_bucket_request_metadata.new_value_prop_id,
        ))
    }

    /// Update the privacy of a bucket.
    ///
    /// This function allows the owner of a bucket to update its privacy setting.
    /// If the bucket is set to private and no collection exists,
    /// a new collection will be created. If the bucket is set to public and
    /// an associated collection exists, the collection remains but the privacy setting is updated to public.
    /// If the bucket has an associated collection, and it does not exist in storage, a new collection will be created.
    pub(crate) fn do_update_bucket_privacy(
        sender: T::AccountId,
        bucket_id: BucketIdFor<T>,
        private: bool,
    ) -> Result<Option<CollectionIdFor<T>>, DispatchError> {
        // Check that the user is not currently insolvent.
        ensure!(
            !<T::UserSolvency as ReadUserSolvencyInterface>::is_user_insolvent(&sender),
            Error::<T>::OperationNotAllowedWithInsolventUser
        );

        // Ensure the sender is the owner of the bucket.
        ensure!(
            T::Providers::is_bucket_owner(&sender, &bucket_id)?,
            Error::<T>::NotBucketOwner
        );

        // Retrieve the collection ID associated with the bucket, if any.
        let maybe_collection_id = T::Providers::get_read_access_group_id_of_bucket(&bucket_id)?;

        // Determine the appropriate collection ID based on the new privacy setting.
        let collection_id = match (private, maybe_collection_id) {
            // Create a new collection if the bucket will be private and no collection exists.
            (true, None) => {
                Some(Self::do_create_and_associate_collection_with_bucket(sender.clone(), bucket_id)?)
            }
            // Handle case where the bucket has an existing collection, but the collection is not in storage.
            (true, Some(current_collection_id))
            if !<T::CollectionInspector as shp_traits::InspectCollections>::collection_exists(&current_collection_id) =>
                {
                    Some(Self::do_create_and_associate_collection_with_bucket(sender.clone(), bucket_id)?)
                }
            // Use the existing collection ID if it exists.
            (_, Some(current_collection_id)) => Some(current_collection_id),
            // No collection needed if the bucket is public and no collection exists.
            (false, None) => None,
        };

        // Update the privacy setting of the bucket.
        T::Providers::update_bucket_privacy(bucket_id, private)?;

        Ok(collection_id)
    }

    /// Create and associate collection with a bucket.
    ///
    /// *Callable only by the owner of the bucket.*
    ///
    /// It is possible to have a bucket that is private but does not have a collection associated with it. This can happen if
    /// a user destroys the collection associated with the bucket by calling the NFTs pallet directly.
    ///
    /// In any case, we will set a new collection the bucket even if there is an existing one associated with it.
    pub(crate) fn do_create_and_associate_collection_with_bucket(
        sender: T::AccountId,
        bucket_id: BucketIdFor<T>,
    ) -> Result<CollectionIdFor<T>, DispatchError> {
        // Check that the user is not currently insolvent.
        ensure!(
            !<T::UserSolvency as ReadUserSolvencyInterface>::is_user_insolvent(&sender),
            Error::<T>::OperationNotAllowedWithInsolventUser
        );

        // Check if sender is the owner of the bucket.
        ensure!(
            <T::Providers as ReadBucketsInterface>::is_bucket_owner(&sender, &bucket_id)?,
            Error::<T>::NotBucketOwner
        );

        let collection_id = Self::create_collection(sender)?;

        <T::Providers as MutateBucketsInterface>::update_bucket_read_access_group_id(
            bucket_id,
            Some(collection_id.clone()),
        )?;

        Ok(collection_id)
    }

    /// Delete an empty bucket.
    ///
    /// *Callable only by the User owner of the bucket.*
    ///
    /// This function will delete the bucket and the associated collection if the bucket is empty.
    /// If the bucket is not empty, the function will return an error.
    /// The bucket deposit paid by the User when initially creating the bucket will be returned to the User.
    pub(crate) fn do_delete_bucket(
        sender: T::AccountId,
        bucket_id: BucketIdFor<T>,
    ) -> Result<Option<CollectionIdFor<T>>, DispatchError> {
        // Check that the bucket with the received ID exists.
        ensure!(
            <T::Providers as ReadBucketsInterface>::bucket_exists(&bucket_id),
            Error::<T>::BucketNotFound
        );

        // Check if the sender is the owner of the bucket.
        ensure!(
            <T::Providers as ReadBucketsInterface>::is_bucket_owner(&sender, &bucket_id)?,
            Error::<T>::NotBucketOwner
        );

        // Check if the bucket is empty, both by checking its size and that its root is the default one
        // (the root of an empty trie).
        ensure!(
            <T::Providers as ReadBucketsInterface>::get_bucket_size(&bucket_id)? == Zero::zero(),
            Error::<T>::BucketNotEmpty
        );
        let bucket_root = expect_or_err!(
            <T::Providers as ReadBucketsInterface>::get_root_bucket(&bucket_id),
            "Bucket exists so it should have a root",
            Error::<T>::BucketNotFound
        );
        ensure!(
            bucket_root == <T::Providers as shp_traits::ReadProvidersInterface>::get_default_root(),
            Error::<T>::BucketNotEmpty
        );

        // Retrieve the collection ID associated with the bucket, if any.
        let maybe_collection_id: Option<CollectionIdFor<T>> =
            <T::Providers as ReadBucketsInterface>::get_read_access_group_id_of_bucket(&bucket_id)?;

        // Delete the bucket.
        <T::Providers as MutateBucketsInterface>::delete_bucket(bucket_id)?;

        // Delete the collection associated with the bucket if it existed.
        if let Some(collection_id) = maybe_collection_id.clone() {
            let destroy_witness = expect_or_err!(
                T::Nfts::get_destroy_witness(&collection_id),
                "Failed to get destroy witness for collection, when it was already checked to exist",
                Error::<T>::CollectionNotFound
            );
            T::Nfts::destroy(collection_id, destroy_witness, Some(sender))?;
        }

        // Return the collection ID associated with the bucket, if any.
        Ok(maybe_collection_id)
    }

    /// Request storage for a file.
    ///
    /// In the event that a storage request is created without any user multiaddresses (checkout `do_bsp_stop_storing`),
    /// it is expected that storage providers that do have this file in storage already, will be able to send a
    /// transaction to the chain to add themselves as a data server for the storage request.
    pub(crate) fn do_request_storage(
        sender: T::AccountId,
        bucket_id: BucketIdFor<T>,
        location: FileLocation<T>,
        fingerprint: Fingerprint<T>,
        size: StorageDataUnit<T>,
        msp_id: Option<ProviderIdFor<T>>,
        replication_target: ReplicationTarget<T>,
        user_peer_ids: Option<PeerIds<T>>,
    ) -> Result<MerkleHash<T>, DispatchError> {
        // Check that the user is not currently insolvent.
        ensure!(
            !<T::UserSolvency as ReadUserSolvencyInterface>::is_user_insolvent(&sender),
            Error::<T>::OperationNotAllowedWithInsolventUser
        );

        // Check that the file size is greater than zero.
        ensure!(size > Zero::zero(), Error::<T>::FileSizeCannotBeZero);

        // Check that a bucket under the received ID exists and that the sender is the owner of the bucket.
        ensure!(
            <T::Providers as ReadBucketsInterface>::is_bucket_owner(&sender, &bucket_id)?,
            Error::<T>::NotBucketOwner
        );

        // Check that the bucket is not being moved.
        // Do not allow any storage requests and move bucket requests to coexist for the same bucket.
        ensure!(
            !<PendingMoveBucketRequests<T>>::contains_key(&bucket_id),
            Error::<T>::BucketIsBeingMoved
        );

        // Get the MSP that's currently storing the bucket. It should exist since the bucket is not currently being moved.
        let msp_id_storing_bucket = expect_or_err!(
            <T::Providers as ReadBucketsInterface>::get_msp_of_bucket(&bucket_id)
                .expect("Bucket was checked to exist previously. qed"),
            "MSP should exist for bucket",
            Error::<T>::MspNotStoringBucket
        );

        // Ensure a payment stream between the MSP and the user exists. This is to avoid storage requests
        // being issued for buckets that belonged to an insolvent user (that's no longer insolvent) and
        // the MSP did not delete.
        ensure!(
            <T::PaymentStreams as PaymentStreamsInterface>::has_active_payment_stream_with_user(
                &msp_id_storing_bucket,
                &sender
            ),
            Error::<T>::FixedRatePaymentStreamNotFound
        );

        // Check if we can hold the storage request creation deposit from the user.
        // The storage request creation deposit should be enough to cover the weight of the `bsp_volunteer`
        // extrinsic for ALL BSPs of the network.
        let number_of_bsps = <T::Providers as ReadStorageProvidersInterface>::get_number_of_bsps();
        let number_of_bsps_balance_typed =
            <T as crate::Config>::ReplicationTargetToBalance::convert(number_of_bsps);
        let deposit = <T::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
            &T::WeightInfo::bsp_volunteer(),
        )
        .saturating_mul(number_of_bsps_balance_typed)
        .saturating_add(T::BaseStorageRequestCreationDeposit::get());
        ensure!(
            T::Currency::can_hold(
                &HoldReason::StorageRequestCreationHold.into(),
                &sender,
                deposit
            ),
            Error::<T>::CannotHoldDeposit
        );

        // Get the chosen replication target.
        let replication_target = match replication_target {
            ReplicationTarget::Basic => T::BasicReplicationTarget::get(),
            ReplicationTarget::Standard => T::StandardReplicationTarget::get(),
            ReplicationTarget::HighSecurity => T::HighSecurityReplicationTarget::get(),
            ReplicationTarget::SuperHighSecurity => T::SuperHighSecurityReplicationTarget::get(),
            ReplicationTarget::UltraHighSecurity => T::UltraHighSecurityReplicationTarget::get(),
            ReplicationTarget::Custom(replication_target) => replication_target,
        };

        // Ensure that the chosen replication target is not zero.
        ensure!(
            !replication_target.is_zero(),
            Error::<T>::ReplicationTargetCannotBeZero
        );

        // Ensure that the chosen replication target is not greater than the maximum allowed replication target.
        ensure!(
            replication_target <= T::MaxReplicationTarget::get().into(),
            Error::<T>::ReplicationTargetExceedsMaximum
        );

        // If a MSP ID is provided, this storage request came from a user.
        let msp = if let Some(ref msp_id) = msp_id {
            // Check that the received Provider ID corresponds to a valid MSP.
            ensure!(
                <T::Providers as ReadStorageProvidersInterface>::is_msp(msp_id),
                Error::<T>::NotAMsp
            );

            // Check that it matches the one storing the bucket.
            ensure!(
                msp_id == &msp_id_storing_bucket,
                Error::<T>::MspNotStoringBucket
            );

            // Check if the MSP is insolvent
            ensure!(
                !<T::Providers as ReadProvidersInterface>::is_provider_insolvent(*msp_id),
                Error::<T>::OperationNotAllowedForInsolventProvider
            );

            // Since this request came from a user, it has to pay an amount upfront to cover the effects that
            // file retrieval will have on the network's availability.
            // This amount is paid to the treasury. Governance can then decide what to do with the accumulated
            // funds (such as splitting them among the BSPs).
            let amount_to_pay_upfront = <T::PaymentStreams as PricePerGigaUnitPerTickInterface>::get_price_per_giga_unit_per_tick()
				.saturating_mul(T::TickNumberToBalance::convert(T::UpfrontTicksToPay::get()))
				.saturating_mul(T::ReplicationTargetToBalance::convert(replication_target))
				.saturating_mul(T::StorageDataUnitToBalance::convert(size))
				.checked_div(&GIGAUNIT.into())
				.unwrap_or_default();
            T::Currency::transfer(
                &sender,
                &T::TreasuryAccount::get(),
                amount_to_pay_upfront,
                Preservation::Preserve,
            )?;

            Some((*msp_id, false))
        } else {
            None
        };

        // Compute the file key used throughout this file's lifespan.
        let file_key = Self::compute_file_key(
            sender.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        )
        .map_err(|_| Error::<T>::FailedToComputeFileKey)?;

        // Check a storage request does not already exist for this file key.
        ensure!(
            !<StorageRequests<T>>::contains_key(&file_key),
            Error::<T>::StorageRequestAlreadyRegistered
        );

        // Enqueue an expiration item for the storage request to clean it up if it expires without being fulfilled or cancelled.
        let expiration_item = ExpirationItem::StorageRequest(file_key);
        let expiration_tick = Self::enqueue_expiration_item(expiration_item)?;

        // Get the current tick.
        let current_tick =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();

        // Create the storage request's metadata.
        let zero = ReplicationTargetType::<T>::zero();
        let storage_request_metadata = StorageRequestMetadata::<T> {
            requested_at: current_tick,
            owner: sender.clone(),
            bucket_id,
            location: location.clone(),
            fingerprint,
            size,
            msp,
            user_peer_ids: user_peer_ids.clone().unwrap_or_default(),
            bsps_required: replication_target,
            bsps_confirmed: zero,
            bsps_volunteered: zero,
            expires_at: expiration_tick,
            deposit_paid: deposit,
        };

        // Hold the required deposit from the user.
        T::Currency::hold(
            &HoldReason::StorageRequestCreationHold.into(),
            &sender,
            deposit,
        )?;

        // Register the storage request and add it to the bucket's storage requests.
        <StorageRequests<T>>::insert(&file_key, storage_request_metadata);
        <BucketsWithStorageRequests<T>>::insert(&bucket_id, &file_key, ());

        // Emit the `NewStorageRequest` event.
        Self::deposit_event(Event::NewStorageRequest {
            who: sender,
            file_key,
            bucket_id,
            location,
            fingerprint,
            size,
            peer_ids: user_peer_ids.unwrap_or_default(),
            expires_at: expiration_tick,
        });

        Ok(file_key)
    }

    /// Accepts or rejects batches of storage requests assumed to be grouped by bucket.
    pub(crate) fn do_msp_respond_storage_request(
        sender: T::AccountId,
        storage_request_msp_response: StorageRequestMspResponse<T>,
    ) -> Result<(), DispatchError> {
        // Check that the sender is a Storage Provider and get its MSP ID
        let msp_id = <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(&sender)
            .ok_or(Error::<T>::NotASp)?;

        // Check that the sender is an MSP
        ensure!(
            <T::Providers as ReadStorageProvidersInterface>::is_msp(&msp_id),
            Error::<T>::NotAMsp
        );

        // Preliminary check to ensure that the MSP is the one storing each bucket in the responses
        for StorageRequestMspBucketResponse { bucket_id, .. } in storage_request_msp_response.iter()
        {
            ensure!(
                <T::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(
                    &msp_id, &bucket_id
                ),
                Error::<T>::MspNotStoringBucket
            );
        }

        // Process each bucket's responses
        for StorageRequestMspBucketResponse {
            bucket_id,
            accept,
            reject,
        } in storage_request_msp_response.into_iter()
        {
            if let Some(accepted_file_keys) = accept {
                Self::do_msp_accept_storage_request(msp_id, bucket_id, accepted_file_keys)?;
            }

            for RejectedStorageRequest { file_key, reason } in reject {
                let storage_request_metadata = <StorageRequests<T>>::get(file_key)
                    .ok_or(Error::<T>::StorageRequestNotFound)?;

                Self::cleanup_storage_request(
                    EitherAccountIdOrMspId::MspId(msp_id),
                    file_key,
                    &storage_request_metadata,
                )?;

                Self::deposit_event(Event::StorageRequestRejected { file_key, reason });
            }
        }

        Ok(())
    }

    pub(crate) fn do_msp_stop_storing_bucket(
        sender: T::AccountId,
        bucket_id: BucketIdFor<T>,
    ) -> Result<(ProviderIdFor<T>, T::AccountId), DispatchError> {
        // Check if the sender is a Provider.
        let msp_id = <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(&sender)
            .ok_or(Error::<T>::NotAMsp)?;

        // Check if the MSP is indeed an MSP.
        ensure!(
            <T::Providers as ReadStorageProvidersInterface>::is_msp(&msp_id),
            Error::<T>::NotAMsp
        );

        // Check if the MSP is storing the bucket.
        ensure!(
            <T::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(&msp_id, &bucket_id),
            Error::<T>::MspNotStoringBucket
        );

        let bucket_owner = <T::Providers as ReadBucketsInterface>::get_bucket_owner(&bucket_id)?;

        // Decrease the used capacity of the MSP.
        let bucket_size = <T::Providers as ReadBucketsInterface>::get_bucket_size(&bucket_id)?;
        <T::Providers as MutateStorageProvidersInterface>::decrease_capacity_used(
            &msp_id,
            bucket_size,
        )?;

        // Remove the MSP from the bucket.
        <T::Providers as MutateBucketsInterface>::unassign_msp_from_bucket(&bucket_id)?;

        Ok((msp_id, bucket_owner))
    }

    /// Deletes a bucket from a user marked as insolvent and all its associated data.
    /// This can be used by MSPs that detect that they are storing a bucket for an insolvent user.
    /// This way, the MSP can remove the bucket and stop storing it, receiving from the user's deposit
    /// the amount it's owed and deleting the payment stream between them in the process.
    pub(crate) fn do_msp_stop_storing_bucket_for_insolvent_user(
        sender: T::AccountId,
        bucket_id: BucketIdFor<T>,
    ) -> Result<(ProviderIdFor<T>, T::AccountId), DispatchError> {
        // Ensure the sender is a registered MSP.
        let msp_id = <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(&sender)
            .ok_or(Error::<T>::NotAMsp)?;
        ensure!(
            <T::Providers as shp_traits::ReadStorageProvidersInterface>::is_msp(&msp_id),
            Error::<T>::NotAMsp
        );

        // Ensure the bucket exists.
        ensure!(
            <T::Providers as shp_traits::ReadBucketsInterface>::bucket_exists(&bucket_id),
            Error::<T>::BucketNotFound
        );

        // Ensure the bucket is stored by the MSP.
        ensure!(
            <T::Providers as shp_traits::ReadBucketsInterface>::is_bucket_stored_by_msp(
                &msp_id, &bucket_id
            ),
            Error::<T>::MspNotStoringBucket
        );

        // Get the owner of the bucket.
        let bucket_owner =
            <T::Providers as shp_traits::ReadBucketsInterface>::get_bucket_owner(&bucket_id)?;

        // Get the size of the bucket.
        let bucket_size =
            <T::Providers as shp_traits::ReadBucketsInterface>::get_bucket_size(&bucket_id)?;

        // To allow the MSP to completely delete the bucket, either the user account is currently insolvent
        // or no payment stream exists between the user and the MSP.
        let is_user_insolvent =
            <T::UserSolvency as shp_traits::ReadUserSolvencyInterface>::is_user_insolvent(
                &bucket_owner,
            );
        let payment_stream_exists =
            <T::PaymentStreams as shp_traits::PaymentStreamsInterface>::has_active_payment_stream_with_user(
                &msp_id,
                &bucket_owner,
            );
        ensure!(
            is_user_insolvent || !payment_stream_exists,
            Error::<T>::UserNotInsolvent
        );

        // Retrieve the collection ID associated with the bucket, if any.
        let maybe_collection_id: Option<CollectionIdFor<T>> =
            <T::Providers as ReadBucketsInterface>::get_read_access_group_id_of_bucket(&bucket_id)?;

        // Delete the collection associated with the bucket if it existed.
        if let Some(collection_id) = maybe_collection_id.clone() {
            let destroy_witness = expect_or_err!(
                T::Nfts::get_destroy_witness(&collection_id),
                "Failed to get destroy witness for collection, when it was already checked to exist",
                Error::<T>::CollectionNotFound
            );
            T::Nfts::destroy(collection_id, destroy_witness, Some(bucket_owner.clone()))?;
        }

        // Delete the bucket from the system.
        <T::Providers as MutateBucketsInterface>::force_delete_bucket(&msp_id, &bucket_id)?;

        // Decrease the used capacity of the MSP.
        <T::Providers as MutateStorageProvidersInterface>::decrease_capacity_used(
            &msp_id,
            bucket_size,
        )?;

        Ok((msp_id, bucket_owner))
    }

    /// Processes a file deletion request.
    ///
    /// This function validates a signed file deletion request by:
    /// 1. Checking that the requester is not insolvent
    /// 2. Verifying the requester owns the bucket containing the file
    /// 3. Validating the signature against the encoded intention
    /// 4. Computing the file key from provided metadata and verifying it matches the signed intention
    /// 5. Ensuring the operation type is Delete
    ///
    /// Note: This function only validates the deletion request but does not perform the actual
    /// file deletion. It serves as a preliminary step before the deletion process can proceed.
    pub(crate) fn do_request_delete_file(
        who: T::AccountId,
        signed_intention: FileOperationIntention<T>,
        signature: T::OffchainSignature,
        bucket_id: BucketIdFor<T>,
        location: FileLocation<T>,
        size: StorageDataUnit<T>,
        fingerprint: Fingerprint<T>,
    ) -> DispatchResult {
        // Check that the user that's sending the deletion request is not currently insolvent.
        // Insolvent users can't interact with the system and should wait for all MSPs and BSPs
        // to delete their files and buckets using the available extrinsics or resolve their
        // insolvency manually.
        ensure!(
            !<T::UserSolvency as ReadUserSolvencyInterface>::is_user_insolvent(&who),
            Error::<T>::OperationNotAllowedWithInsolventUser
        );

        // Check if sender is the owner of the bucket.
        ensure!(
            <T::Providers as ReadBucketsInterface>::is_bucket_owner(&who, &bucket_id)?,
            Error::<T>::NotBucketOwner
        );

        // Verify that the operation is Delete
        ensure!(
            signed_intention.operation == FileOperation::Delete,
            Error::<T>::InvalidSignedOperation
        );

        // Encode the intention for signature verification
        let signed_intention_encoded = signed_intention.encode();

        let is_valid = signature.verify(&signed_intention_encoded[..], &who);
        ensure!(is_valid, Error::<T>::InvalidSignature);

        // Compute file key from the provided metadata
        let computed_file_key =
            Self::compute_file_key(who.clone(), bucket_id, location.clone(), size, fingerprint)
                .map_err(|_| Error::<T>::FailedToComputeFileKey)?;

        // Verify that the file_key in the signed intention matches the computed one
        ensure!(
            signed_intention.file_key == computed_file_key,
            Error::<T>::InvalidFileKeyMetadata
        );

        Ok(())
    }

    /// Executes actual file deletion. Any entity that has the owner's signed intention can delete the file on their behalf,
    /// If they present a valid forest proof showing that the file exists in the provider's forest.
    ///
    /// This function validates a signed file deletion request and performs the actual deletion by:
    /// 1. Checking that the file owner is not insolvent
    /// 2. Verifying the intent signer is the owner of the bucket containing the file
    /// 3. Ensuring the operation type is Delete
    /// 4. Validating the signature against the encoded intention
    /// 5. Computing the file key from provided metadata and verifying it matches the signed intention
    /// 6. Verifying the forest proof and updating the provider's root
    pub(crate) fn do_delete_file(
        file_owner: T::AccountId,
        signed_intention: FileOperationIntention<T>,
        signature: T::OffchainSignature,
        bucket_id: BucketIdFor<T>,
        location: FileLocation<T>,
        size: StorageDataUnit<T>,
        fingerprint: Fingerprint<T>,
        provider_id: ProviderIdFor<T>,
        forest_proof: ForestProof<T>,
    ) -> DispatchResult {
        // Check that the file owner is not currently insolvent.
        ensure!(
            !<T::UserSolvency as ReadUserSolvencyInterface>::is_user_insolvent(&file_owner),
            Error::<T>::OperationNotAllowedWithInsolventUser
        );

        // Verify that the operation is Delete
        ensure!(
            signed_intention.operation == FileOperation::Delete,
            Error::<T>::InvalidSignedOperation
        );

        // Encode the intention for signature verification
        let signed_intention_encoded = signed_intention.encode();

        let is_valid = signature.verify(&signed_intention_encoded[..], &file_owner);
        ensure!(is_valid, Error::<T>::InvalidSignature);

        // Compute file key from the provided metadata
        let computed_file_key = Self::compute_file_key(
            file_owner.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        )
        .map_err(|_| Error::<T>::FailedToComputeFileKey)?;

        // Verify that the file_key in the signed intention matches the computed one
        ensure!(
            signed_intention.file_key == computed_file_key,
            Error::<T>::InvalidFileKeyMetadata
        );

        // Forest proof verification and file deletion
        if <T::Providers as ReadStorageProvidersInterface>::is_msp(&provider_id) {
            Self::delete_file_from_msp(
                file_owner,
                computed_file_key,
                size,
                bucket_id,
                provider_id,
                forest_proof,
            )?;
        } else if <T::Providers as ReadStorageProvidersInterface>::is_bsp(&provider_id) {
            Self::delete_file_from_bsp(
                file_owner,
                computed_file_key,
                size,
                provider_id,
                forest_proof,
            )?;
        } else {
            // Entity provided an incorrect provider ID
            return Err(Error::<T>::InvalidProviderID.into());
        }

        // TODO: Reward the caller
        Ok(())
    }

    pub(crate) fn do_delete_file_for_incomplete_storage_request(
        file_key: MerkleHash<T>,
        provider_id: ProviderIdFor<T>,
        forest_proof: ForestProof<T>,
    ) -> DispatchResult {
        // Fetch incomplete storage request metadata
        // If there is no entry for the file key, return an error.
        let mut incomplete_storage_request_metadata =
            IncompleteStorageRequests::<T>::get(&file_key)
                .ok_or(Error::<T>::IncompleteStorageRequestNotFound)?;

        // Verify file key integrity
        let computed_file_key = Self::compute_file_key(
            incomplete_storage_request_metadata.owner.clone(),
            incomplete_storage_request_metadata.bucket_id,
            incomplete_storage_request_metadata.location.clone(),
            incomplete_storage_request_metadata.size,
            incomplete_storage_request_metadata.fingerprint,
        )
        .map_err(|_| Error::<T>::FailedToComputeFileKey)?;

        ensure!(computed_file_key == file_key, Error::<T>::FileKeyMismatch);

        // Perform deletion based on provider type
        if <T::Providers as ReadStorageProvidersInterface>::is_msp(&provider_id) {
            // Check that the provider_id is the msp that is storing the file in the incomplete storage request metadata
            ensure!(
                incomplete_storage_request_metadata.pending_msp_removal == Some(provider_id),
                Error::<T>::ProviderNotStoringFile
            );

            Self::delete_file_from_msp(
                incomplete_storage_request_metadata.owner.clone(),
                file_key,
                incomplete_storage_request_metadata.size,
                incomplete_storage_request_metadata.bucket_id,
                provider_id,
                forest_proof,
            )?;
        } else if <T::Providers as ReadStorageProvidersInterface>::is_bsp(&provider_id) {
            // Check that the provider_id is in the pending removal lists
            ensure!(
                incomplete_storage_request_metadata
                    .pending_bsp_removals
                    .contains(&provider_id),
                Error::<T>::ProviderNotStoringFile
            );

            Self::delete_file_from_bsp(
                incomplete_storage_request_metadata.owner.clone(),
                file_key,
                incomplete_storage_request_metadata.size,
                provider_id,
                forest_proof,
            )?;
        } else {
            return Err(Error::<T>::InvalidProviderID.into());
        }

        // Remove provider from pending lists
        incomplete_storage_request_metadata.remove_provider(provider_id);

        // Check if all providers have removed their files
        if incomplete_storage_request_metadata.is_fully_cleaned() {
            IncompleteStorageRequests::<T>::remove(&file_key);
        } else {
            IncompleteStorageRequests::<T>::insert(&file_key, incomplete_storage_request_metadata);
        }

        Ok(())
    }

    /// Accept as many storage requests as possible (best-effort) belonging to the same bucket.
    ///
    /// There should be a single non-inclusion forest proof for all file keys, and finally there should
    /// be a list of file key(s) with a key proof for each of them.
    ///
    /// The implementation follows this sequence:
    /// 1. Verify the non-inclusion proof.
    /// 2. For each file key: Verify and process the acceptance. If any operation fails during the processing of a file key,
    /// the entire function will fail and no changes will be applied.
    /// 3. If all file keys are successfully processed, apply the delta with all the accepted keys to the root of the bucket which are part of the set of
    /// non-inclusion file keys (since it is possible that the file key was already stored by the MSP).
    /// 4. If any step fails, the function will return an error and no changes will be made to the storage state.
    fn do_msp_accept_storage_request(
        msp_id: ProviderIdFor<T>,
        bucket_id: BucketIdFor<T>,
        accepted_file_keys: StorageRequestMspAcceptedFileKeys<T>,
    ) -> Result<MerkleHash<T>, DispatchError> {
        // Get the user owner of the bucket.
        let bucket_owner =
            <T::Providers as shp_traits::ReadBucketsInterface>::get_bucket_owner(&bucket_id)?;

        // Check that the bucket owner is not currently insolvent. This is done to error out early, since otherwise
        // it will go through with all the verification logic and then fail when trying to update the payment stream
        // between the MSP and the user.
        ensure!(
            !<T::UserSolvency as ReadUserSolvencyInterface>::is_user_insolvent(&bucket_owner),
            Error::<T>::OperationNotAllowedWithInsolventUser
        );

        // Check if MSP is insolvent.
        ensure!(
            !<T::Providers as ReadProvidersInterface>::is_provider_insolvent(msp_id),
            Error::<T>::OperationNotAllowedForInsolventProvider
        );

        let file_keys = accepted_file_keys
            .file_keys_and_proofs
            .iter()
            .map(|file_key_with_proof| file_key_with_proof.file_key)
            .collect::<Vec<_>>();

        // Get the Bucket's root
        let bucket_root =
            <T::Providers as shp_traits::ReadBucketsInterface>::get_root_bucket(&bucket_id)
                .ok_or(Error::<T>::BucketNotFound)?;

        // Verify the proof of non-inclusion.
        let proven_keys: BTreeSet<MerkleHash<T>> =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::verify_generic_forest_proof(
                &bucket_root,
                file_keys.as_slice(),
                &accepted_file_keys.forest_proof,
            )?;

        let mut accepted_files_metadata = Vec::new();

        for file_key_with_proof in accepted_file_keys.file_keys_and_proofs.iter() {
            let mut storage_request_metadata =
                <StorageRequests<T>>::get(&file_key_with_proof.file_key)
                    .ok_or(Error::<T>::StorageRequestNotFound)?;

            // Check that the storage request bucket ID matches the provided bucket ID.
            ensure!(
                storage_request_metadata.bucket_id == bucket_id,
                Error::<T>::InvalidBucketIdFileKeyPair
            );

            // Check that the MSP is the one storing the bucket.
            ensure!(
                <T::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(
                    &msp_id,
                    &storage_request_metadata.bucket_id
                ),
                Error::<T>::MspNotStoringBucket
            );

            // Check that the storage request has a MSP.
            ensure!(
                storage_request_metadata.msp.is_some(),
                Error::<T>::RequestWithoutMsp
            );

            let (request_msp_id, confirm_status) = storage_request_metadata.msp.unwrap();

            // Check that the sender corresponds to the MSP in the storage request and that it hasn't yet confirmed storing the file.
            ensure!(request_msp_id == msp_id, Error::<T>::NotSelectedMsp);

            // Check that the MSP hasn't already confirmed storing the file.
            ensure!(!confirm_status, Error::<T>::MspAlreadyConfirmed);

            // Check that the MSP still has enough available capacity to store the file.
            ensure!(
                <T::Providers as ReadStorageProvidersInterface>::available_capacity(&msp_id)
                    >= storage_request_metadata.size,
                Error::<T>::InsufficientAvailableCapacity
            );

            // Get the file metadata to insert into the bucket under the file key.
            let file_metadata = storage_request_metadata
                .clone()
                .to_file_metadata()
                .map_err(|_| Error::<T>::FileMetadataProcessingQueueFull)?;

            let chunk_challenges = Self::generate_chunk_challenges_on_sp_confirm(
                msp_id,
                file_key_with_proof.file_key,
                &storage_request_metadata,
            )?;

            // Only check the key proof, increase the bucket size and capacity used if the file key is not in the forest proof, and
            // add the file metadata to the `accepted_files_metadata` since all keys in this array will be added to the bucket forest via an apply delta.
            // This can happen if the storage request was issued again by the user and the MSP has already stored the file.
            if !proven_keys.contains(&file_key_with_proof.file_key) {
                accepted_files_metadata.push((file_metadata, file_key_with_proof));

                // Check that the key proof is valid.
                <T::ProofDealer as shp_traits::ProofsDealerInterface>::verify_key_proof(
                    &file_key_with_proof.file_key,
                    &chunk_challenges,
                    &file_key_with_proof.proof,
                )?;

                // Increase size of the bucket.
                <T::Providers as MutateBucketsInterface>::increase_bucket_size(
                    &storage_request_metadata.bucket_id,
                    storage_request_metadata.size,
                )?;

                // Increase the used capacity of the MSP
                // This should not fail since we checked that the MSP has enough available capacity to store the file.
                expect_or_err!(
                    <T::Providers as MutateStorageProvidersInterface>::increase_capacity_used(
                        &msp_id,
                        storage_request_metadata.size,
                    ),
                    "Failed to increase capacity used for MSP",
                    Error::<T>::TooManyStorageRequestResponses,
                    result
                );
            }

            // Notify that the storage request has been accepted by the MSP.
            Self::deposit_event(Event::MspAcceptedStorageRequest {
                file_key: file_key_with_proof.file_key,
            });

            // Check if all BSPs have confirmed storing the file.
            if storage_request_metadata.bsps_confirmed == storage_request_metadata.bsps_required {
                // Remove the storage request from the expiration queue.
                let expiration_tick = storage_request_metadata.expires_at;
                <StorageRequestExpirations<T>>::mutate(expiration_tick, |expiration_items| {
                    expiration_items.retain(|item| item != &file_key_with_proof.file_key);
                });

                // Remove storage request metadata.
                <StorageRequests<T>>::remove(&file_key_with_proof.file_key);
                <BucketsWithStorageRequests<T>>::remove(
                    &storage_request_metadata.bucket_id,
                    &file_key_with_proof.file_key,
                );

                // Remove storage request bsps
                let removed = <StorageRequestBsps<T>>::drain_prefix(&file_key_with_proof.file_key)
                    .fold(0, |acc, _| acc.saturating_add(One::one()));

                // Make sure that the expected number of BSPs were removed.
                expect_or_err!(
                    storage_request_metadata.bsps_volunteered == removed.into(),
                    "Number of volunteered BSPs for storage request should have been removed",
                    Error::<T>::UnexpectedNumberOfRemovedVolunteeredBsps,
                    bool
                );

                // Return the storage request creation deposit to the user
                T::Currency::release(
                    &HoldReason::StorageRequestCreationHold.into(),
                    &storage_request_metadata.owner,
                    storage_request_metadata.deposit_paid,
                    Precision::BestEffort,
                )?;

                // Notify that the storage request has been fulfilled.
                Self::deposit_event(Event::StorageRequestFulfilled {
                    file_key: file_key_with_proof.file_key,
                });
            } else {
                // Set as confirmed the MSP in the storage request metadata.
                storage_request_metadata.msp = Some((msp_id, true));

                // Update storage request metadata.
                <StorageRequests<T>>::set(
                    &file_key_with_proof.file_key,
                    Some(storage_request_metadata.clone()),
                );
            }
        }

        // If there are no mutations to apply, return the current root of the bucket.
        if accepted_files_metadata.is_empty() {
            return Ok(bucket_root);
        }

        // Get the current root of the bucket where the file will be stored.
        let bucket_root = expect_or_err!(
            <T::Providers as shp_traits::ReadBucketsInterface>::get_root_bucket(&bucket_id),
            "Failed to get root for bucket, when it was already checked to exist",
            Error::<T>::BucketNotFound
        );

        // Compute the new bucket root after inserting new file key in its forest partial trie.
        let new_bucket_root =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::generic_apply_delta(
                &bucket_root,
                accepted_files_metadata
                    .iter()
                    .map(|(file_metadata, file_key_with_proof)| {
                        (
                            file_key_with_proof.file_key,
                            TrieAddMutation::new(file_metadata.encode()).into(),
                        )
                    })
                    .collect::<Vec<_>>()
                    .as_slice(),
                &accepted_file_keys.forest_proof,
                Some(bucket_id.encode()),
            )?;

        // Update root of the bucket.
        <T::Providers as shp_traits::MutateBucketsInterface>::change_root_bucket(
            bucket_id,
            new_bucket_root,
        )?;

        Ok(new_bucket_root)
    }

    /// Volunteer to store a file.
    ///
    /// *Callable only by BSP accounts*
    ///
    /// A BSP can only volunteer for a storage request if it is eligible based on the XOR of the `fingerprint` and the BSP's account ID and if it evaluates to a value
    /// less than the [globally computed threshold](BspsAssignmentThreshold). As the number of BSPs signed up increases, the threshold decreases, meaning there is a
    /// lower chance of a BSP being eligible to volunteer for a storage request.
    ///
    /// Though, as the storage request remains open, the threshold increases over time based on the number of ticks since the storage request was issued. This is to
    /// ensure that the storage request is fulfilled by opening up the opportunity for more BSPs to volunteer.
    ///
    /// For more information on what "ticks" are, see the [Proofs Dealer pallet](https://github.com/Moonsong-Labs/storage-hub/blob/main/pallets/proofs-dealer/README.md).
    pub(crate) fn do_bsp_volunteer(
        sender: T::AccountId,
        file_key: MerkleHash<T>,
    ) -> Result<
        (
            ProviderIdFor<T>,
            MultiAddresses<T>,
            StorageRequestMetadata<T>,
        ),
        DispatchError,
    > {
        let bsp_id = <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(&sender)
            .ok_or(Error::<T>::NotABsp)?;

        // Check if BSP is insolvent.
        ensure!(
            !<T::Providers as ReadProvidersInterface>::is_provider_insolvent(bsp_id),
            Error::<T>::OperationNotAllowedForInsolventProvider
        );

        // Check that the provider is indeed a BSP.
        ensure!(
            <T::Providers as ReadStorageProvidersInterface>::is_bsp(&bsp_id),
            Error::<T>::NotABsp
        );

        // Check that the storage request exists.
        let mut storage_request_metadata =
            <StorageRequests<T>>::get(&file_key).ok_or(Error::<T>::StorageRequestNotFound)?;

        // Check that the user that issued the storage request is not currently insolvent. This is done
        // to avoid the BSP the trouble of volunteering, then fetching the file from the user, generating
        // the proof and then not being able to confirm storing the file because the user is insolvent.
        ensure!(
            !<T::UserSolvency as ReadUserSolvencyInterface>::is_user_insolvent(
                &storage_request_metadata.owner
            ),
            Error::<T>::OperationNotAllowedWithInsolventUser
        );

        expect_or_err!(
            storage_request_metadata.bsps_confirmed < storage_request_metadata.bsps_required,
            "Storage request should never have confirmed BSPs equal to or greater than required bsps, since they are deleted when it is reached.",
            Error::<T>::StorageRequestBspsRequiredFulfilled,
            bool
        );

        let available_capacity =
            <T::Providers as ReadStorageProvidersInterface>::available_capacity(&bsp_id);

        // Check if the BSP has enough capacity to store the file.
        ensure!(
            available_capacity > storage_request_metadata.size,
            Error::<T>::InsufficientAvailableCapacity
        );

        // Check if the BSP is already volunteered for this storage request.
        ensure!(
            !<StorageRequestBsps<T>>::contains_key(&file_key, &bsp_id),
            Error::<T>::BspAlreadyVolunteered
        );

        let earliest_volunteer_tick = Self::query_earliest_file_volunteer_tick(bsp_id, file_key)
            .map_err({
                |e| {
                    log::error!("Failed to query earliest file volunteer tick: {:?}", e);
                    Error::<T>::FailedToQueryEarliestFileVolunteerTick
                }
            })?;

        // Check if the BSP is eligible to volunteer for the storage request.
        let current_tick_number =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();

        ensure!(
            current_tick_number >= earliest_volunteer_tick,
            Error::<T>::BspNotEligibleToVolunteer
        );

        // Add BSP to storage request metadata.
        <StorageRequestBsps<T>>::insert(
            &file_key,
            &bsp_id,
            StorageRequestBspsMetadata::<T> {
                confirmed: false,
                _phantom: Default::default(),
            },
        );

        // Increment the number of BSPs volunteered.
        match storage_request_metadata
            .bsps_volunteered
            .checked_add(&ReplicationTargetType::<T>::one())
        {
            Some(inc_bsps_volunteered) => {
                storage_request_metadata.bsps_volunteered = inc_bsps_volunteered;
            }
            None => {
                return Err(ArithmeticError::Overflow.into());
            }
        }

        // Get the payment account of the BSP.
        let bsp_payment_account =
            <T::Providers as shp_traits::ReadProvidersInterface>::get_payment_account(bsp_id)
                .ok_or(Error::<T>::FailedToGetPaymentAccount)?;

        // Calculate how much the BSP should be reimbursed for this extrinsic from the user's deposit.
        let amount_to_pay = <T::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
            &T::WeightInfo::bsp_volunteer(),
        );

        // Transfer the funds to the BSP.
        let amount_transferred = T::Currency::transfer_on_hold(
            &HoldReason::StorageRequestCreationHold.into(),
            &storage_request_metadata.owner,
            &bsp_payment_account,
            amount_to_pay,
            Precision::BestEffort,
            Restriction::Free,
            Fortitude::Force,
        )?;

        // If the transfer was successful, substract the amount from the deposit paid by the user.
        storage_request_metadata.deposit_paid = storage_request_metadata
            .deposit_paid
            .saturating_sub(amount_transferred);

        // Update storage request metadata.
        <StorageRequests<T>>::set(&file_key, Some(storage_request_metadata.clone()));

        let multiaddresses = T::Providers::get_bsp_multiaddresses(&bsp_id)?;

        Ok((bsp_id, multiaddresses, storage_request_metadata))
    }

    /// Confirm storing a file.
    ///
    /// *Callable only by BSP accounts*
    ///
    /// This function can only be called after a BSP has volunteered for the storage request. The BSP must provide a merkle proof of the file
    /// and a proof of inclusion of the `file_key` in their merkle patricia trie.
    ///
    /// If the proof is valid, the root of the BSP is updated to reflect the new root of the merkle patricia trie and the number of `bsps_confirmed` is
    /// incremented. If the number of `bsps_confirmed` reaches the number of `bsps_required`, the storage request is deleted. Finally, the BSP's data
    /// used is incremented by the size of the file.
    pub(crate) fn do_bsp_confirm_storing(
        sender: T::AccountId,
        non_inclusion_forest_proof: ForestProof<T>,
        file_keys_and_proofs: BoundedVec<FileKeyWithProof<T>, T::MaxBatchConfirmStorageRequests>,
    ) -> DispatchResult {
        // Get the Provider ID of the sender.
        let bsp_id = <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(&sender)
            .ok_or(Error::<T>::NotABsp)?;

        // Check if the Provider is insolvent.
        ensure!(
            !<T::Providers as ReadProvidersInterface>::is_provider_insolvent(bsp_id),
            Error::<T>::OperationNotAllowedForInsolventProvider
        );

        // Check that the Provider is indeed a BSP.
        ensure!(
            <T::Providers as ReadStorageProvidersInterface>::is_bsp(&bsp_id),
            Error::<T>::NotABsp
        );

        // Verify the proof of non-inclusion.
        let proven_keys: BTreeSet<MerkleHash<T>> =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::verify_forest_proof(
                &bsp_id,
                file_keys_and_proofs
                    .iter()
                    .map(|file_key_with_proof| file_key_with_proof.file_key)
                    .collect::<Vec<_>>()
                    .as_slice(),
                &non_inclusion_forest_proof,
            )?;

        // Create a queue to store the file keys and metadata to be processed.
        let mut file_keys_and_metadata: BoundedVec<
            (MerkleHash<T>, Vec<u8>),
            T::MaxBatchConfirmStorageRequests,
        > = BoundedVec::new();

        // Create a set to store the keys that were already processed.
        let mut seen_keys = BTreeSet::new();

        // Create a set to store the keys that were skipped.
        let mut skipped_file_keys: BoundedBTreeSet<
            MerkleHash<T>,
            T::MaxBatchConfirmStorageRequests,
        > = BoundedBTreeSet::new();

        // For each file key and proof, process the confirm storing request.
        for file_key_with_proof in file_keys_and_proofs.iter() {
            // Get the file key and the proof.
            let file_key = file_key_with_proof.file_key;

            // Skip any duplicates.
            if !seen_keys.insert(file_key) {
                continue;
            }

            // Get the storage request metadata for this file key.
            let mut storage_request_metadata = match <StorageRequests<T>>::get(&file_key) {
                Some(metadata) if metadata.bsps_confirmed < metadata.bsps_required => metadata,
                // Since BSPs need to race one another to confirm storage requests, it is entirely possible that a BSP confirms a storage request
                // after the storage request has been fulfilled or the replication target has been reached (bsps_required == bsps_confirmed).
                Some(_) | None => {
                    expect_or_err!(
                        skipped_file_keys.try_insert(file_key),
                        "Failed to push file key to skipped_file_keys",
                        Error::<T>::TooManyStorageRequestResponses,
                        result
                    );
                    continue;
                }
            };

            // Check that the user that issued the storage request is not currently insolvent. This is done to continue the loop early,
            // since the file key would still be skipped after failing to update the payment stream between the user and the BSP.
            if <T::UserSolvency as ReadUserSolvencyInterface>::is_user_insolvent(
                &storage_request_metadata.owner,
            ) {
                // Skip file key if the owner of the file related to the storage request is currently insolvent.
                expect_or_err!(
                    skipped_file_keys.try_insert(file_key),
                    "Failed to push file key to skipped_file_keys",
                    Error::<T>::TooManyStorageRequestResponses,
                    result
                );
                continue;
            }

            // Check that the bucket of the file key still exists. This is done since if the user was previously insolvent the bucket
            // of the storage request might have been deleted by the MSP.
            ensure!(
                <T::Providers as ReadBucketsInterface>::bucket_exists(
                    &storage_request_metadata.bucket_id
                ),
                Error::<T>::BucketNotFound
            );

            // Check that the BSP has volunteered for the storage request.
            ensure!(
                <StorageRequestBsps<T>>::contains_key(&file_key, &bsp_id),
                Error::<T>::BspNotVolunteered
            );

            let requests = expect_or_err!(
                <StorageRequestBsps<T>>::get(&file_key, &bsp_id),
                "BSP should exist since we checked it above",
                Error::<T>::ImpossibleFailedToGetValue
            );

            // Check that the storage provider has not already confirmed storing the file.
            ensure!(!requests.confirmed, Error::<T>::BspAlreadyConfirmed);

            let available_capacity =
                <T::Providers as ReadStorageProvidersInterface>::available_capacity(&bsp_id);

            // Check if the BSP has enough capacity to store the file.
            ensure!(
                available_capacity > storage_request_metadata.size,
                Error::<T>::InsufficientAvailableCapacity
            );

            // All errors from the payment stream operations (create/update) are ignored, and the file key is added to the `skipped_file_keys` set instead of erroring out.
            // This is done to avoid a malicious user, owner of one of the files from the batch of confirmations, being able to prevent the BSP from confirming any files by making itself insolvent so payment stream operations fail.
            // This operation must be executed first, before updating any storage elements, to prevent potential cases
            // where a storage element is updated but should not be.
            match <T::PaymentStreams as PaymentStreamsInterface>::get_dynamic_rate_payment_stream_amount_provided(&bsp_id, &storage_request_metadata.owner) {
				Some(previous_amount_provided) => {
					// Update the payment stream.
                    let new_amount_provided = &previous_amount_provided.checked_add(&storage_request_metadata.size).ok_or(ArithmeticError::Overflow)?;
					if let Err(_) = <T::PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
						&bsp_id,
						&storage_request_metadata.owner,
						new_amount_provided,
					) {
                        // Skip file key if we could not successfully update the payment stream
                        expect_or_err!(
                            skipped_file_keys.try_insert(file_key),
                            "Failed to push file key to skipped_file_keys",
                            Error::<T>::TooManyStorageRequestResponses,
                            result
                        );
                        continue;
                    }
				},
				None => {
					// Create the payment stream.
					if let Err(_) = <T::PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
						&bsp_id,
						&storage_request_metadata.owner,
						&storage_request_metadata.size,
					) {
                        // Skip file key if we could not successfully create the payment stream
                        expect_or_err!(
                            skipped_file_keys.try_insert(file_key),
                            "Failed to push file key to skipped_file_keys",
                            Error::<T>::TooManyStorageRequestResponses,
                            result
                        );
                        continue;
                    }
				}
			}

            // Increment the number of BSPs confirmed.
            match storage_request_metadata
                .bsps_confirmed
                .checked_add(&ReplicationTargetType::<T>::one())
            {
                Some(inc_bsps_confirmed) => {
                    storage_request_metadata.bsps_confirmed = inc_bsps_confirmed;
                }
                None => {
                    return Err(ArithmeticError::Overflow.into());
                }
            }

            // Ensure that the file key IS NOT part of the BSP's forest.
            // Note: The runtime is responsible for adding and removing keys, computing the new root and updating the BSP's root.
            ensure!(
                !proven_keys.contains(&file_key),
                Error::<T>::ExpectedNonInclusionProof
            );

            let chunk_challenges = Self::generate_chunk_challenges_on_sp_confirm(
                bsp_id,
                file_key,
                &storage_request_metadata,
            )?;

            // Check that the key proof is valid.
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::verify_key_proof(
                &file_key,
                &chunk_challenges,
                &file_key_with_proof.proof,
            )?;

            // Increase this Provider's capacity used.
            <T::Providers as MutateStorageProvidersInterface>::increase_capacity_used(
                &bsp_id,
                storage_request_metadata.size,
            )?;

            // Get the file metadata to insert into the Provider's trie under the file key.
            let file_metadata = storage_request_metadata
                .clone()
                .to_file_metadata()
                .map_err(|_| Error::<T>::FileMetadataProcessingQueueFull)?;
            let encoded_trie_value = file_metadata.encode();

            expect_or_err!(
                file_keys_and_metadata.try_push((file_key, encoded_trie_value)),
                "Failed to push file key and metadata",
                Error::<T>::FileMetadataProcessingQueueFull,
                result
            );

            // Remove storage request if we reached the required number of BSPs and the MSP has accepted to store the file.
            if storage_request_metadata.bsps_confirmed == storage_request_metadata.bsps_required
                && storage_request_metadata
                    .msp
                    .map(|(_, confirmed)| confirmed)
                    .unwrap_or(true)
            {
                // Remove the storage request from the expiration queue.
                let expiration_tick = storage_request_metadata.expires_at;
                <StorageRequestExpirations<T>>::mutate(expiration_tick, |expiration_items| {
                    expiration_items.retain(|item| item != &file_key);
                });

                // Remove storage request metadata.
                <StorageRequests<T>>::remove(&file_key);
                <BucketsWithStorageRequests<T>>::remove(
                    &storage_request_metadata.bucket_id,
                    &file_key,
                );

                // Remove storage request bsps
                let removed = <StorageRequestBsps<T>>::drain_prefix(&file_key)
                    .fold(0, |acc, _| acc.saturating_add(One::one()));

                // Make sure that the expected number of BSPs were removed.
                expect_or_err!(
                    storage_request_metadata.bsps_volunteered == removed.into(),
                    "Number of volunteered BSPs for storage request should have been removed",
                    Error::<T>::UnexpectedNumberOfRemovedVolunteeredBsps,
                    bool
                );

                // Return the storage request creation deposit to the user
                T::Currency::release(
                    &HoldReason::StorageRequestCreationHold.into(),
                    &storage_request_metadata.owner,
                    storage_request_metadata.deposit_paid,
                    Precision::BestEffort,
                )?;

                // Notify that the storage request has been fulfilled.
                Self::deposit_event(Event::StorageRequestFulfilled { file_key });
            } else {
                // Update storage request metadata.
                <StorageRequests<T>>::set(&file_key, Some(storage_request_metadata.clone()));

                // Update bsp for storage request.
                <StorageRequestBsps<T>>::mutate(&file_key, &bsp_id, |bsp| {
                    if let Some(bsp) = bsp {
                        bsp.confirmed = true;
                    }
                });
            }
        }

        // Remove all the skipped file keys from file_keys_and_metadata
        file_keys_and_metadata.retain(|(fk, _)| !skipped_file_keys.contains(fk));

        ensure!(
            !file_keys_and_metadata.is_empty(),
            Error::<T>::NoFileKeysToConfirm
        );

        // Check if this is the first file added to the BSP's Forest. If so, initialise last tick proven by this BSP.
        let old_root = expect_or_err!(
            <T::Providers as shp_traits::ReadProvidersInterface>::get_root(bsp_id),
            "Failed to get root for BSP, when it was already checked to be a BSP",
            Error::<T>::NotABsp
        );

        if old_root == <T::Providers as shp_traits::ReadProvidersInterface>::get_default_root() {
            // This means the BSP just started storing files, so its challenge cycle and
            // randomness cycle should be initialised.
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::initialise_challenge_cycle(
                &bsp_id,
            )?;

            <T::CrRandomness as shp_traits::CommitRevealRandomnessInterface>::initialise_randomness_cycle(&bsp_id)?;

            // Emit the corresponding event.
            Self::deposit_event(Event::BspChallengeCycleInitialised {
                who: sender.clone(),
                bsp_id,
            });
        }

        let mutations = file_keys_and_metadata
            .iter()
            .map(|(fk, metadata)| (*fk, TrieAddMutation::new(metadata.clone()).into()))
            .collect::<Vec<_>>();

        // Compute new root after inserting new file keys in forest partial trie.
        let new_root = <T::ProofDealer as shp_traits::ProofsDealerInterface>::apply_delta(
            &bsp_id,
            mutations.as_slice(),
            &non_inclusion_forest_proof,
        )?;

        // Root should have changed.
        ensure!(old_root != new_root, Error::<T>::RootNotUpdated);

        // Update root of BSP.
        <T::Providers as shp_traits::MutateProvidersInterface>::update_root(bsp_id, new_root)?;

        // This should not fail since `skipped_file_keys` purposely share the same bound as `file_keys_and_metadata`.
        let skipped_file_keys: BoundedVec<MerkleHash<T>, T::MaxBatchConfirmStorageRequests> = expect_or_err!(
            skipped_file_keys.into_iter().collect::<Vec<_>>().try_into(),
            "Failed to convert skipped_file_keys to BoundedVec",
            Error::<T>::TooManyStorageRequestResponses,
            result
        );

        let confirmed_file_keys: BoundedVec<MerkleHash<T>, T::MaxBatchConfirmStorageRequests> = expect_or_err!(
            file_keys_and_metadata
                .into_iter()
                .map(|(fk, _)| fk)
                .collect::<Vec<_>>()
                .try_into(),
            "Failed to convert file_keys_and_metadata to BoundedVec",
            Error::<T>::TooManyStorageRequestResponses,
            result
        );

        Self::deposit_event(Event::BspConfirmedStoring {
            who: sender,
            bsp_id,
            confirmed_file_keys,
            skipped_file_keys,
            new_root,
        });

        Ok(())
    }

    /// Revoke a storage request.
    ///
    /// *Callable by the owner of the storage request*
    pub(crate) fn do_revoke_storage_request(
        sender: T::AccountId,
        file_key: MerkleHash<T>,
    ) -> DispatchResult {
        // Check that the storage request exists.
        ensure!(
            <StorageRequests<T>>::contains_key(&file_key),
            Error::<T>::StorageRequestNotFound
        );

        // Get storage request metadata.
        let storage_request_metadata = expect_or_err!(
            <StorageRequests<T>>::get(&file_key),
            "Storage request should exist",
            Error::<T>::StorageRequestNotFound
        );

        // Check that the sender is the owner of the storage request.
        ensure!(
            storage_request_metadata.owner == sender,
            Error::<T>::StorageRequestNotAuthorized
        );

        Self::cleanup_storage_request(
            EitherAccountIdOrMspId::AccountId(sender),
            file_key,
            &storage_request_metadata,
        )?;

        Ok(())
    }

    /// When a storage request is revoked and has already been confirmed by some BSPs, a challenge (with priority) is
    /// issued to force the BSPs to update their storage root to uninclude the file from their storage.
    ///
    /// All BSPs that have volunteered to store the file are removed from the storage request and the storage request is deleted.
    ///
    /// TODO: We should also clean up the MSP (decreasing its used capacity, the bucket size, etc) if it has already confirmed storing the file,
    /// but we can't apply delta... so we need to think about how to do this.
    fn cleanup_storage_request(
        _revoker: EitherAccountIdOrMspId<T>,
        file_key: MerkleHash<T>,
        storage_request_metadata: &StorageRequestMetadata<T>,
    ) -> DispatchResult {
        // TODO: Call `delete_file` - user signature needs to be added to StorageRequestMetadata to be able to call it

        // Remove storage request bsps
        let removed = <StorageRequestBsps<T>>::drain_prefix(&file_key)
            .fold(0, |acc, _| acc.saturating_add(One::one()));

        // Make sure that the expected number of BSPs were removed.
        expect_or_err!(
            storage_request_metadata.bsps_volunteered == removed.into(),
            "Number of volunteered BSPs for storage request should have been removed",
            Error::<T>::UnexpectedNumberOfRemovedVolunteeredBsps,
            bool
        );

        // Remove the storage request from the expiration queue.
        let expiration_tick = storage_request_metadata.expires_at;
        <StorageRequestExpirations<T>>::mutate(expiration_tick, |expiration_items| {
            expiration_items.retain(|item| item != &file_key);
        });

        // Remove storage request.
        <StorageRequests<T>>::remove(&file_key);

        // Return the storage request creation deposit to the user
        T::Currency::release(
            &HoldReason::StorageRequestCreationHold.into(),
            &storage_request_metadata.owner,
            storage_request_metadata.deposit_paid,
            Precision::BestEffort,
        )?;

        // A revoked storage request is not considered active anymore.
        <BucketsWithStorageRequests<T>>::remove(&storage_request_metadata.bucket_id, &file_key);

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
    ///     Therefore, we decrement the `bsps_confirmed` by 1 and remove the BSP as a data server for the file.
    ///
    /// 2. The BSP stops storing a file that has an opened storage request but is not a volunteer.
    ///
    /// > In this case, the storage request was probably created by another BSP for some reason (e.g. that BSP lost the file)
    ///     and the current BSP is not a volunteer for this since it is already storing it. But since they to have stopped storing it,
    ///     we increment the `bsps_required` by 1.
    ///
    /// 3. The BSP stops storing a file that no longer has an opened storage request.
    ///
    /// > In this case, there is no storage request opened for the file they no longer are storing. Therefore, we
    ///     create a storage request with `bsps_required` set to 1.
    ///
    /// *This function does not give BSPs the possibility to remove themselves from being a __volunteer__ of a storage request.*
    ///
    /// A proof of inclusion is required to record the new root of the BSPs merkle patricia trie. First we validate the proof
    /// and ensure the `file_key` is indeed part of the merkle patricia trie. Then finally compute the new merkle patricia trie root
    /// by removing the `file_key` and update the root of the BSP.
    ///
    /// `can_serve`: A flag that indicates if the BSP can serve the file to other BSPs. If the BSP can serve the file, then
    /// they are added to the storage request as a data server.
    pub(crate) fn do_bsp_request_stop_storing(
        sender: T::AccountId,
        file_key: MerkleHash<T>,
        bucket_id: BucketIdFor<T>,
        location: FileLocation<T>,
        owner: T::AccountId,
        fingerprint: Fingerprint<T>,
        size: StorageDataUnit<T>,
        can_serve: bool,
        inclusion_forest_proof: ForestProof<T>,
    ) -> Result<ProviderIdFor<T>, DispatchError> {
        // Check that the user that owns the file is not currently insolvent. The BSP should
        // call `sp_stop_storing_for_insolvent_user` instead if the user is insolvent.
        ensure!(
            !<T::UserSolvency as ReadUserSolvencyInterface>::is_user_insolvent(&owner),
            Error::<T>::OperationNotAllowedWithInsolventUser
        );

        let bsp_id = <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(&sender)
            .ok_or(Error::<T>::NotABsp)?;

        // Check that the provider is indeed a BSP.
        ensure!(
            <T::Providers as ReadStorageProvidersInterface>::is_bsp(&bsp_id),
            Error::<T>::NotABsp
        );

        let bsp_account_id = expect_or_err!(
            <T::Providers as shp_traits::ReadProvidersInterface>::get_owner_account(bsp_id),
            "Failed to get owner account for BSP",
            Error::<T>::FailedToGetOwnerAccount
        );

        // Penalise the BSP for stopping storing the file and send the funds to the treasury.
        T::Currency::transfer(
            &bsp_account_id,
            &T::TreasuryAccount::get(),
            T::BspStopStoringFilePenalty::get(),
            Preservation::Preserve,
        )?;

        // Compute the file key hash.
        let computed_file_key = Self::compute_file_key(
            owner.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        )
        .map_err(|_| Error::<T>::FailedToComputeFileKey)?;

        // Check that the metadata corresponds to the expected file key.
        ensure!(
            file_key == computed_file_key,
            Error::<T>::InvalidFileKeyMetadata
        );

        // Check that a pending stop storing request for that BSP and file does not exist yet.
        ensure!(
            !<PendingStopStoringRequests<T>>::contains_key(&bsp_id, &file_key),
            Error::<T>::PendingStopStoringRequestAlreadyExists
        );

        // Verify the proof of inclusion.
        let proven_keys =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::verify_forest_proof(
                &bsp_id,
                &[file_key],
                &inclusion_forest_proof,
            )?;

        // Ensure that the file key IS part of the BSP's forest.
        ensure!(
            proven_keys.contains(&file_key),
            Error::<T>::ExpectedInclusionProof
        );

        match <StorageRequests<T>>::get(&file_key) {
            Some(mut storage_request_metadata) => {
                match <StorageRequestBsps<T>>::get(&file_key, &bsp_id) {
                    // We hit scenario 1. The BSP is a volunteer and has confirmed storing the file.
                    // We need to decrement the number of BSPs confirmed and volunteered, remove the BSP as a data server and from the storage request.
                    Some(bsp) => {
                        expect_or_err!(
                            bsp.confirmed,
                            "BSP should have confirmed storing the file since we verify the proof and their root matches the one in storage",
                            Error::<T>::BspNotConfirmed,
                            bool
                        );

                        storage_request_metadata.bsps_confirmed = storage_request_metadata
                            .bsps_confirmed
                            .saturating_sub(ReplicationTargetType::<T>::one());

                        storage_request_metadata.bsps_volunteered = storage_request_metadata
                            .bsps_volunteered
                            .saturating_sub(ReplicationTargetType::<T>::one());

                        <StorageRequestBsps<T>>::remove(&file_key, &bsp_id);
                    }
                    // We hit scenario 2. There is an open storage request but the BSP is not a volunteer.
                    // We need to increment the number of BSPs required.
                    None => {
                        storage_request_metadata.bsps_required = storage_request_metadata
                            .bsps_required
                            .saturating_add(ReplicationTargetType::<T>::one());
                    }
                }

                // Update storage request metadata.
                <StorageRequests<T>>::set(&file_key, Some(storage_request_metadata));
            }
            // We hit scenario 3. There is no storage request opened for the file.
            // We need to create a new storage request with a single bsp required and
            // add this BSP as a data server if they can serve the file.
            None => {
                Self::do_request_storage(
                    owner.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    None,
                    ReplicationTarget::Custom(ReplicationTargetType::<T>::one()),
                    None,
                )?;

                if can_serve {
                    // Add the BSP as a data server for the file.
                    <StorageRequestBsps<T>>::insert(
                        &file_key,
                        &bsp_id,
                        StorageRequestBspsMetadata::<T> {
                            confirmed: true,
                            _phantom: Default::default(),
                        },
                    );
                }
            }
        };

        // Add the pending stop storing request to storage.
        <PendingStopStoringRequests<T>>::insert(
            &bsp_id,
            &file_key,
            PendingStopStoringRequest {
                tick_when_requested: <T::ProofDealer as ProofsDealerInterface>::get_current_tick(),
                file_owner: owner,
                file_size: size,
            },
        );

        Ok(bsp_id)
    }

    pub(crate) fn do_bsp_confirm_stop_storing(
        sender: T::AccountId,
        file_key: MerkleHash<T>,
        inclusion_forest_proof: ForestProof<T>,
    ) -> Result<(ProviderIdFor<T>, MerkleHash<T>), DispatchError> {
        // Get the SP ID of the sender
        let bsp_id = <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(&sender)
            .ok_or(Error::<T>::NotASp)?;

        // Ensure the ID belongs to a BSP, not a MSP
        ensure!(
            <T::Providers as ReadStorageProvidersInterface>::is_bsp(&bsp_id),
            Error::<T>::NotABsp
        );

        // Get the block when the pending stop storing request of the BSP for the file key was opened,
        // the file size to stop storing and the file owner.
        let PendingStopStoringRequest {
            tick_when_requested,
            file_size,
            file_owner,
        } = <PendingStopStoringRequests<T>>::take(&bsp_id, &file_key)
            .ok_or(Error::<T>::PendingStopStoringRequestNotFound)?;

        // Check that the user that owns the file is not currently insolvent. The BSP should
        // call `sp_stop_storing_for_insolvent_user` instead if the user is insolvent.
        // This is done to error out early since this extrinsic would eventually fail when trying
        // to update the payment stream between the user and the BSP.
        ensure!(
            !<T::UserSolvency as ReadUserSolvencyInterface>::is_user_insolvent(&file_owner),
            Error::<T>::OperationNotAllowedWithInsolventUser
        );

        // Check that enough time has passed since the pending stop storing request was opened.
        ensure!(
            <<T as crate::Config>::ProofDealer as ProofsDealerInterface>::get_current_tick()
                >= tick_when_requested.saturating_add(T::MinWaitForStopStoring::get()),
            Error::<T>::MinWaitForStopStoringNotReached
        );

        // Verify the proof of inclusion.
        let proven_keys =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::verify_forest_proof(
                &bsp_id,
                &[file_key],
                &inclusion_forest_proof,
            )?;

        // Ensure that the file key IS part of the BSP's forest.
        // The runtime is responsible for adding and removing keys, computing the new root and updating the BSP's root.
        ensure!(
            proven_keys.contains(&file_key),
            Error::<T>::ExpectedInclusionProof
        );

        // Compute new root after removing file key from forest partial trie.
        let new_root = <T::ProofDealer as shp_traits::ProofsDealerInterface>::apply_delta(
            &bsp_id,
            &[(file_key, TrieRemoveMutation::default().into())],
            &inclusion_forest_proof,
        )?;

        // Update root of BSP.
        <T::Providers as shp_traits::MutateProvidersInterface>::update_root(bsp_id, new_root)?;

        // Decrease data used by the BSP.
        <T::Providers as MutateStorageProvidersInterface>::decrease_capacity_used(
            &bsp_id, file_size,
        )?;

        // Update payment stream and manage BSP cycles after file removal
        Self::update_bsp_payment_and_cycles_after_file_removal(
            bsp_id,
            &file_owner,
            file_size,
            new_root,
        )?;

        Ok((bsp_id, new_root))
    }

    /// SP stops storing a file from a User that has been flagged as insolvent.
    ///
    /// *Callable only by SP accounts*
    ///
    /// A proof of inclusion is required to record the new root of the SPs merkle patricia trie. First we validate the proof
    /// and ensure the `file_key` is indeed part of the merkle patricia trie. Then finally compute the new merkle patricia trie root
    /// by removing the `file_key` and update the root of the SP.
    pub(crate) fn do_sp_stop_storing_for_insolvent_user(
        sender: T::AccountId,
        file_key: MerkleHash<T>,
        bucket_id: BucketIdFor<T>,
        location: FileLocation<T>,
        owner: T::AccountId,
        fingerprint: Fingerprint<T>,
        size: StorageDataUnit<T>,
        inclusion_forest_proof: ForestProof<T>,
    ) -> Result<(ProviderIdFor<T>, MerkleHash<T>), DispatchError> {
        // Get the SP ID
        let sp_id = <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(&sender)
            .ok_or(Error::<T>::NotASp)?;

        // Check that the owner of the file has been flagged as insolvent OR that the Provider does not
        // have any active payment streams with the user. The rationale here is that if there is a
        // user who cannot pay, or is just not paying anymore, the SP has the right to stop storing files for them
        // without having to pay any penalty.
        ensure!(
            <T::UserSolvency as ReadUserSolvencyInterface>::is_user_insolvent(&owner)
                || !<T::PaymentStreams as PaymentStreamsInterface>::has_active_payment_stream_with_user(
                    &sp_id, &owner
                ),
            Error::<T>::UserNotInsolvent
        );

        // Compute the file key hash.
        let computed_file_key = Self::compute_file_key(
            owner.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        )
        .map_err(|_| Error::<T>::FailedToComputeFileKey)?;

        // Check that the metadata corresponds to the expected file key.
        ensure!(
            file_key == computed_file_key,
            Error::<T>::InvalidFileKeyMetadata
        );

        // Verify the proof of inclusion.
        // If the Provider is a BSP, the proof is verified against the BSP's forest.
        let new_root = if <T::Providers as ReadStorageProvidersInterface>::is_bsp(&sp_id) {
            let proven_keys =
                <T::ProofDealer as shp_traits::ProofsDealerInterface>::verify_forest_proof(
                    &sp_id,
                    &[file_key],
                    &inclusion_forest_proof,
                )?;

            // Ensure that the file key IS part of the BSP's forest.
            ensure!(
                proven_keys.contains(&file_key),
                Error::<T>::ExpectedInclusionProof
            );

            // Compute new root after removing file key from forest partial trie.
            let new_root = <T::ProofDealer as shp_traits::ProofsDealerInterface>::apply_delta(
                &sp_id,
                &[(file_key, TrieRemoveMutation::default().into())],
                &inclusion_forest_proof,
            )?;

            // In case there was a pending stop storing request that the BSP had initiated before the user
            // became insolvent, remove it.
            <PendingStopStoringRequests<T>>::remove(&sp_id, &file_key);

            // Update root of the BSP.
            <T::Providers as shp_traits::MutateProvidersInterface>::update_root(sp_id, new_root)?;

            // Delete payment stream between this BSP and this user (also charge it for all the owed funds
            // of all files that were stored by this BSP).
            if <T::PaymentStreams as PaymentStreamsInterface>::get_dynamic_rate_payment_stream_info(
                &sp_id, &owner,
            )
            .is_some()
            {
                <T::PaymentStreams as PaymentStreamsInterface>::delete_dynamic_rate_payment_stream(
                    &sp_id, &owner,
                )?;
            }

            new_root
        } else {
            // If the Provider is a MSP, the proof is verified against the Bucket's root.

            // Check that the Bucket is stored by the MSP
            ensure!(
                <T::Providers as shp_traits::ReadBucketsInterface>::is_bucket_stored_by_msp(
                    &sp_id, &bucket_id
                ),
                Error::<T>::MspNotStoringBucket
            );

            // Decrease size of the bucket.
            <T::Providers as MutateBucketsInterface>::decrease_bucket_size(&bucket_id, size)?;

            // Get the Bucket's root
            let bucket_root =
                <T::Providers as shp_traits::ReadBucketsInterface>::get_root_bucket(&bucket_id)
                    .ok_or(Error::<T>::BucketNotFound)?;

            let proven_keys =
                <T::ProofDealer as shp_traits::ProofsDealerInterface>::verify_generic_forest_proof(
                    &bucket_root,
                    &[file_key],
                    &inclusion_forest_proof,
                )?;

            // Ensure that the file key IS part of the Bucket's trie.
            ensure!(
                proven_keys.contains(&file_key),
                Error::<T>::ExpectedInclusionProof
            );

            // Compute new root after removing file key from forest partial trie.
            let new_root =
                <T::ProofDealer as shp_traits::ProofsDealerInterface>::generic_apply_delta(
                    &bucket_root,
                    &[(file_key, TrieRemoveMutation::default().into())],
                    &inclusion_forest_proof,
                    Some(bucket_id.encode()),
                )?;

            // Update root of the Bucket.
            <T::Providers as shp_traits::MutateBucketsInterface>::change_root_bucket(
                bucket_id, new_root,
            )?;

            // Delete payment stream between this MSP and this user (also charge it for all the owed funds
            // of all files that were stored by this MSP).
            if <T::PaymentStreams as PaymentStreamsInterface>::get_fixed_rate_payment_stream_info(
                &sp_id, &owner,
            )
            .is_some()
            {
                <T::PaymentStreams as PaymentStreamsInterface>::delete_fixed_rate_payment_stream(
                    &sp_id, &owner,
                )?;
            }

            new_root
        };

        // Decrease data used by the SP.
        <T::Providers as MutateStorageProvidersInterface>::decrease_capacity_used(&sp_id, size)?;

        if <T::Providers as ReadStorageProvidersInterface>::is_bsp(&sp_id) {
            // If it doesn't store any files we stop the challenge cycle and stop its randomness cycle.
            if new_root == <T::Providers as shp_traits::ReadProvidersInterface>::get_default_root()
            {
                let used_capacity =
                    <T::Providers as ReadStorageProvidersInterface>::get_used_capacity(&sp_id);
                if used_capacity != Zero::zero() {
                    // Emit event if we have inconsistency. We can later monitor for those.
                    Self::deposit_event(Event::UsedCapacityShouldBeZero {
                        actual_used_capacity: used_capacity,
                    });
                }

                <T::ProofDealer as shp_traits::ProofsDealerInterface>::stop_challenge_cycle(
                    &sp_id,
                )?;

                <T::CrRandomness as CommitRevealRandomnessInterface>::stop_randomness_cycle(
                    &sp_id,
                )?;
            }
        };

        Ok((sp_id, new_root))
    }

    /// Create a collection.
    fn create_collection(owner: T::AccountId) -> Result<CollectionIdFor<T>, DispatchError> {
        // TODO: Parametrize the collection settings.
        let config: CollectionConfigFor<T> = CollectionConfig {
            settings: CollectionSettings::all_enabled(),
            max_supply: None,
            mint_settings: MintSettings {
                mint_type: MintType::Issuer,
                price: None,
                start_block: None,
                end_block: None,
                default_item_settings: ItemSettings::all_enabled(),
            },
        };

        T::Nfts::create_collection(&owner, &owner, &config)
    }

    /// Compute the next tick number to insert an expiring item, and insert it in the corresponding expiration queue.
    ///
    /// This function attempts to insert a the expiration item at the next available tick starting from
    /// the current next available tick.
    pub(crate) fn enqueue_expiration_item(
        expiration_item: ExpirationItem<T>,
    ) -> Result<TickNumber<T>, DispatchError> {
        let expiration_tick = expiration_item.get_next_expiration_tick();
        let new_expiration_tick = expiration_item.try_append(expiration_tick)?;
        expiration_item.set_next_expiration_tick(new_expiration_tick);

        Ok(new_expiration_tick)
    }

    pub fn compute_file_key(
        owner: T::AccountId,
        bucket_id: BucketIdFor<T>,
        location: FileLocation<T>,
        size: StorageDataUnit<T>,
        fingerprint: Fingerprint<T>,
    ) -> Result<MerkleHash<T>, DispatchError> {
        match shp_file_metadata::FileMetadata::<
            { shp_constants::H_LENGTH },
            { shp_constants::FILE_CHUNK_SIZE },
            { shp_constants::FILE_SIZE_TO_CHALLENGES },
        >::new(
            owner.encode(),
            bucket_id.as_ref().to_vec(),
            location.clone().to_vec(),
            size.saturated_into(),
            fingerprint.as_ref().into(),
        ) {
            Ok(file_metadata) => Ok(file_metadata.file_key::<FileKeyHasher<T>>()),
            Err(_) => return Err(Error::<T>::FailedToCreateFileMetadata.into()),
        }
    }

    fn do_decode_generic_apply_delta_event_info(
        mut encoded_event_info: &[u8],
    ) -> Result<BucketIdFor<T>, codec::Error> {
        BucketIdFor::<T>::decode(&mut encoded_event_info)
    }

    pub fn pending_storage_requests_by_msp(
        msp_id: ProviderIdFor<T>,
    ) -> BTreeMap<MerkleHash<T>, StorageRequestMetadata<T>> {
        // Get the storage requests for a specific MSP
        StorageRequests::<T>::iter()
            .filter(|(_, metadata)| {
                if let Some(msp) = metadata.msp {
                    msp.0 == msp_id && !msp.1
                } else {
                    false
                }
            })
            .collect()
    }

    /// Removes file key from the bucket's forest, updating the bucket's root.
    pub(crate) fn delete_file_from_msp(
        file_owner: T::AccountId,
        file_key: MerkleHash<T>,
        size: StorageDataUnit<T>,
        bucket_id: BucketIdFor<T>,
        provider_id: ProviderIdFor<T>,
        forest_proof: ForestProof<T>,
    ) -> DispatchResult {
        // Ensure that the provider_id is the owner of the bucket
        ensure!(
            <T::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(
                &provider_id,
                &bucket_id
            ),
            Error::<T>::MspNotStoringBucket
        );

        // Get current bucket root
        let old_bucket_root = <T::Providers as ReadBucketsInterface>::get_root_bucket(&bucket_id)
            .ok_or(Error::<T>::BucketNotFound)?;

        // Verify if the file key is part of the bucket's forest
        let proven_keys = <T::ProofDealer as ProofsDealerInterface>::verify_generic_forest_proof(
            &old_bucket_root,
            &[file_key],
            &forest_proof,
        )?;

        // Ensure that the file key is part of the bucket's forest
        ensure!(
            proven_keys.contains(&file_key),
            Error::<T>::ExpectedInclusionProof
        );

        // Compute new root after removing file key from forest
        let new_root = <T::ProofDealer as ProofsDealerInterface>::generic_apply_delta(
            &old_bucket_root,
            &[(file_key, TrieRemoveMutation::default().into())],
            &forest_proof,
            Some(bucket_id.encode()),
        )?;

        // Update root of the bucket
        <T::Providers as MutateBucketsInterface>::change_root_bucket(bucket_id, new_root)?;

        // Decrease capacity used of the MSP
        <T::Providers as MutateStorageProvidersInterface>::decrease_capacity_used(
            &provider_id,
            size,
        )?;

        // Decrease bucket size of the MSP
        // This function also updates the fixed rate payment stream between the user and the MSP.
        // via apply_delta_fixed_rate_payment_stream function in providers pallet.
        <T::Providers as MutateBucketsInterface>::decrease_bucket_size(&bucket_id, size)?;

        // Emit the MSP file deletion completed event
        Self::deposit_event(Event::MspFileDeletionCompleted {
            user: file_owner,
            file_key,
            file_size: size,
            bucket_id,
            msp_id: provider_id,
            old_root: old_bucket_root,
            new_root,
        });

        Ok(())
    }

    /// Removes file key from the BSP's forest, updating the BSP's root.
    pub(crate) fn delete_file_from_bsp(
        file_owner: T::AccountId,
        file_key: MerkleHash<T>,
        size: StorageDataUnit<T>,
        bsp_id: ProviderIdFor<T>,
        forest_proof: ForestProof<T>,
    ) -> DispatchResult {
        // Get current BSP root
        let old_root = <T::Providers as ReadProvidersInterface>::get_root(bsp_id)
            .ok_or(Error::<T>::NotABsp)?;

        // Verify that the file key is part of the BSP's forest
        let proven_keys = <T::ProofDealer as ProofsDealerInterface>::verify_forest_proof(
            &bsp_id,
            &[file_key],
            &forest_proof,
        )?;

        // Ensure that the file key is part of the BSP's forest
        ensure!(
            proven_keys.contains(&file_key),
            Error::<T>::ExpectedInclusionProof
        );

        // Compute new root after removing file key from forest
        let new_root = <T::ProofDealer as ProofsDealerInterface>::apply_delta(
            &bsp_id,
            &[(file_key, TrieRemoveMutation::default().into())],
            &forest_proof,
        )?;

        // Update root of BSP
        <T::Providers as MutateProvidersInterface>::update_root(bsp_id, new_root)?;

        // Decrease capacity used by the BSP
        <T::Providers as MutateStorageProvidersInterface>::decrease_capacity_used(&bsp_id, size)?;

        // Update payment stream and manage BSP cycles after file removal
        Self::update_bsp_payment_and_cycles_after_file_removal(
            bsp_id,
            &file_owner,
            size,
            new_root,
        )?;

        // Emit the BSP file deletion completed event
        Self::deposit_event(Event::BspFileDeletionCompleted {
            user: file_owner,
            file_key,
            file_size: size,
            bsp_id,
            old_root,
            new_root,
        });

        Ok(())
    }

    /// Updates the BSP payment stream and manages BSP cycles after file removal.
    ///
    /// 1. Updating or deleting the payment stream between a user and BSP based on file size
    /// 2. Stopping BSP challenge and randomness cycles when the BSP root becomes default (no more files stored)
    fn update_bsp_payment_and_cycles_after_file_removal(
        bsp_id: ProviderIdFor<T>,
        file_owner: &T::AccountId,
        file_size: StorageDataUnit<T>,
        new_root: MerkleHash<T>,
    ) -> DispatchResult {
        // Update the payment stream between the user and the BSP. If the new amount provided is zero, delete it instead.
        let new_amount_provided = <T::PaymentStreams as PaymentStreamsInterface>::get_dynamic_rate_payment_stream_amount_provided(&bsp_id, &file_owner)
            .ok_or(Error::<T>::DynamicRatePaymentStreamNotFound)?
            .saturating_sub(file_size);
        if new_amount_provided.is_zero() {
            <T::PaymentStreams as PaymentStreamsInterface>::delete_dynamic_rate_payment_stream(
                &bsp_id,
                &file_owner,
            )?;
        } else {
            <T::PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                &bsp_id,
                &file_owner,
                &new_amount_provided,
            )?;
        }

        // If the root of the BSP is now the default root, stop its cycles.
        if new_root == <T::Providers as shp_traits::ReadProvidersInterface>::get_default_root() {
            // Check the current used capacity of the BSP. Since its root is the default one, it should
            // be zero.
            let used_capacity =
                <T::Providers as ReadStorageProvidersInterface>::get_used_capacity(&bsp_id);
            if !used_capacity.is_zero() {
                // Emit event if we have inconsistency. We can later monitor for those.
                Self::deposit_event(Event::UsedCapacityShouldBeZero {
                    actual_used_capacity: used_capacity,
                });
            }

            // Stop the BSP's challenge and randomness cycles.
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::stop_challenge_cycle(&bsp_id)?;
            <T::CrRandomness as CommitRevealRandomnessInterface>::stop_randomness_cycle(&bsp_id)?;
        }

        Ok(())
    }

    /// Construct IncompleteStorageRequestMetadata from existing storage request data
    pub(crate) fn create_incomplete_storage_request_metadata(
        storage_request: &StorageRequestMetadata<T>,
        file_key: &MerkleHash<T>,
    ) -> IncompleteStorageRequestMetadata<T> {
        // Collect all confirmed BSPs with simple iteration
        let mut confirmed_bsps = sp_std::vec::Vec::new();
        for (bsp_id, metadata) in StorageRequestBsps::<T>::iter_prefix(file_key) {
            if metadata.confirmed {
                confirmed_bsps.push(bsp_id);
            }
        }

        // Check if MSP accepted the file
        let accepted_msp = match storage_request.msp {
            Some((msp_id, true)) => Some(msp_id),
            _ => None,
        };

        // Convert to bounded vec
        let bounded_bsps = BoundedVec::truncate_from(confirmed_bsps);

        IncompleteStorageRequestMetadata {
            owner: storage_request.owner.clone(),
            bucket_id: storage_request.bucket_id,
            location: storage_request.location.clone(),
            size: storage_request.size,
            fingerprint: storage_request.fingerprint,
            pending_bsp_removals: bounded_bsps,
            pending_msp_removal: accepted_msp,
        }
    }
}

mod hooks {
    use crate::{
        pallet,
        types::{MerkleHash, RejectedStorageRequestReason, StorageRequestMetadata, TickNumber},
        utils::BucketIdFor,
        weights::WeightInfo,
        BucketsWithStorageRequests, Event, HoldReason, IncompleteStorageRequests,
        MoveBucketRequestExpirations, NextStartingTickToCleanUp, Pallet, PendingMoveBucketRequests,
        StorageRequestBsps, StorageRequestExpirations, StorageRequests,
    };
    use frame_support::traits::{fungible::MutateHold, tokens::Precision};
    use sp_runtime::{
        traits::{Get, One, Zero},
        Saturating,
    };
    use sp_weights::{RuntimeDbWeight, WeightMeter};

    impl<T: pallet::Config> Pallet<T> {
        pub(crate) fn do_on_poll(weight: &mut WeightMeter) {
            let current_data_price_per_giga_unit =
                <T::PaymentStreams as shp_traits::PricePerGigaUnitPerTickInterface>::get_price_per_giga_unit_per_tick();

            let new_data_price_per_giga_unit =
                <T::UpdateStoragePrice as shp_traits::UpdateStoragePrice>::update_storage_price(
                    current_data_price_per_giga_unit,
                    <T::Providers as shp_traits::SystemMetricsInterface>::get_total_used_capacity(),
                    <T::Providers as shp_traits::SystemMetricsInterface>::get_total_capacity(),
                );

            if new_data_price_per_giga_unit != current_data_price_per_giga_unit {
                <T::PaymentStreams as shp_traits::PricePerGigaUnitPerTickInterface>::set_price_per_giga_unit_per_tick(
                    new_data_price_per_giga_unit,
                );
            }

            // Consume the weight utilised by this hook
            weight.consume(T::WeightInfo::on_poll_hook());
        }

        pub(crate) fn do_on_idle(
            current_tick: TickNumber<T>,
            mut meter: &mut WeightMeter,
        ) -> &mut WeightMeter {
            let db_weight = T::DbWeight::get();

            // If there's enough weight to get from storage the next tick to clean up and possibly update it afterwards, continue
            if meter.can_consume(T::DbWeight::get().reads_writes(1, 1)) {
                // Get the next tick for which to clean up expired items
                let mut tick_to_clean = NextStartingTickToCleanUp::<T>::get();
                let initial_tick_to_clean = tick_to_clean;

                // While the tick to clean up is less than or equal to the current tick, process the expired items for that tick.
                while tick_to_clean <= current_tick {
                    // Process the expired items for the current tick to cleanup.
                    let exited_early =
                        Self::process_tick_expired_items(tick_to_clean, &mut meter, &db_weight);

                    // If processing had to exit early because of weight limitations, stop processing expired items.
                    if exited_early {
                        break;
                    }
                    // Otherwise, increment the tick to clean up and continue processing the next tick.
                    tick_to_clean.saturating_accrue(TickNumber::<T>::one());
                }

                // Update the next starting tick for cleanup
                if tick_to_clean > initial_tick_to_clean {
                    NextStartingTickToCleanUp::<T>::put(tick_to_clean);
                    meter.consume(db_weight.writes(1));
                }
            }

            meter
        }

        // This function cleans up the expired items for the current tick to cleanup.
        // It returns a boolean which indicates if the function had to exit early for this tick because of weight limitations.
        // This is to allow the caller to know if it should continue processing the next tick or stop.
        fn process_tick_expired_items(
            tick: TickNumber<T>,
            meter: &mut WeightMeter,
            db_weight: &RuntimeDbWeight,
        ) -> bool {
            let mut ran_out_of_weight = false;

            // Expired storage requests clean up section:
            // If there's enough weight to get from storage the maximum amount of BSPs required for a storage request
            // and get the storage request expirations for the current tick, and reinsert them if needed, continue.
            if meter.can_consume(db_weight.reads_writes(2, 2)) {
                // Get the maximum amount of BSPs required for a storage request.
                // As of right now, the upper bound limit to the number of BSPs required to fulfill a storage request is set by `MaxReplicationTarget`.
                // We could increase this potential weight to account for potentially more volunteers.
                let max_bsp_required: u64 = T::MaxReplicationTarget::get().into();
                meter.consume(db_weight.reads(1));

                // Get the storage request expirations for the current tick.
                let mut expired_storage_requests = StorageRequestExpirations::<T>::take(&tick);
                meter.consume(db_weight.reads_writes(1, 1));

                // Get the required weight to process an expired storage request in its worst case scenario.
                let maximum_required_weight_expired_storage_request =
                    T::WeightInfo::process_expired_storage_request_msp_accepted_or_no_msp(
                        max_bsp_required as u32,
                    )
                    .max(
                        T::WeightInfo::process_expired_storage_request_msp_rejected(
                            max_bsp_required as u32,
                        ),
                    );

                // While there's enough weight to process an expired storage request in its worst-case scenario AND re-insert the remaining storage requests to storage, continue.
                while let Some(file_key) = expired_storage_requests.pop() {
                    if meter.can_consume(
                        maximum_required_weight_expired_storage_request
                            .saturating_add(db_weight.writes(1)),
                    ) {
                        // Process a expired storage request. This internally consumes the used weight from the meter.
                        Self::process_expired_storage_request(file_key, meter);
                    } else {
                        // Push back the expired storage request into the storage requests queue to be able to re-insert it.
                        // This should never fail since this element was just taken from the bounded vector, so there must be space for it.
                        let _ = expired_storage_requests.try_push(file_key);
                        ran_out_of_weight = true;
                        break;
                    }
                }

                // If the expired storage requests were not fully processed, re-insert them into storage.
                if !expired_storage_requests.is_empty() {
                    StorageRequestExpirations::<T>::insert(&tick, expired_storage_requests);
                    meter.consume(db_weight.writes(1));
                }
            }

            // Expired move bucket requests clean up section:
            // If there's enough weight to get from storage the expired move bucket requests and reinsert them if needed, continue.
            if meter.can_consume(db_weight.reads_writes(1, 2)) {
                // Get the expired move bucket requests for the current tick.
                let mut expired_move_bucket_requests =
                    MoveBucketRequestExpirations::<T>::take(&tick);
                meter.consume(db_weight.reads_writes(1, 1));

                // Get the required weight to process one expired move bucket request.
                let required_weight_processing_expired_move_bucket_request =
                    T::WeightInfo::process_expired_move_bucket_request();

                // While there's enough weight to process an expired move bucket request AND re-insert the remaining move bucket requests to storage, continue.
                while let Some(bucket_id) = expired_move_bucket_requests.pop() {
                    if meter.can_consume(
                        required_weight_processing_expired_move_bucket_request
                            .saturating_add(db_weight.writes(1)),
                    ) {
                        // Process an expired move bucket request. This internally consumes the used weight from the meter.
                        Self::process_expired_move_bucket_request(bucket_id, meter);
                    } else {
                        // Push back the expired move bucket request into the move bucket requests queue to be able to re-insert it.
                        // This should never fail since this element was just taken from the bounded vector, so there must be space for it.
                        let _ = expired_move_bucket_requests.try_push(bucket_id);
                        ran_out_of_weight = true;
                        break;
                    }
                }

                // If the expired move bucket requests were not fully processed, re-insert them into storage.
                if !expired_move_bucket_requests.is_empty() {
                    MoveBucketRequestExpirations::<T>::insert(&tick, expired_move_bucket_requests);
                    meter.consume(db_weight.writes(1));
                }
            }

            ran_out_of_weight
        }

        pub(crate) fn process_expired_storage_request(
            file_key: MerkleHash<T>,
            meter: &mut WeightMeter,
        ) {
            // Get storage request as mutable and count BSPs that volunteered for it.
            // We do not remove the storage request nor BSPs as the runtime needs this information
            // to be able to know if a fisherman node can delete the respective file.
            let storage_request_metadata = StorageRequests::<T>::get(&file_key);
            let amount_of_volunteered_bsps = StorageRequestBsps::<T>::iter_prefix(&file_key)
                .fold(0u32, |acc, _| acc.saturating_add(One::one()));

            match storage_request_metadata {
                Some(storage_request_metadata) => match storage_request_metadata.msp {
                    None | Some((_, true)) => {
                        // If the request was originated by a request to stop storing from a BSP for a file that had no
                        // storage request open, or if the MSP has already accepted storing the file (and the bucket and
                        // payment stream with the user still exists), treat the storage request as fulfilled with whatever
                        // amount of BSPs got to volunteer and confirm the file. For that:
                        // Clean up storage request data
                        Self::cleanup_expired_storage_request(&file_key, &storage_request_metadata);

                        // Emit the StorageRequestExpired event
                        Self::deposit_event(Event::StorageRequestExpired { file_key });

                        // Consume the weight used.
                        meter.consume(
                            T::WeightInfo::process_expired_storage_request_msp_accepted_or_no_msp(
                                amount_of_volunteered_bsps,
                            ),
                        );
                    }
                    Some((_msp_id, false)) => {
                        // If the MSP did not accept the file in time, treat the storage request as rejected.
                        if !storage_request_metadata.bsps_confirmed.is_zero() {
                            // There are BSPs that have confirmed storing the file, so we need to create an incomplete storage request metadata
                            // This will allow the fisherman node to delete the file from the confirmed BSPs.
                            let incomplete_storage_request_metadata =
                                Self::create_incomplete_storage_request_metadata(
                                    &storage_request_metadata,
                                    &file_key,
                                );
                            // Add to storage mapping
                            IncompleteStorageRequests::<T>::insert(
                                &file_key,
                                incomplete_storage_request_metadata,
                            );
                        }
                        // Clean up all storage request related data
                        Self::cleanup_expired_storage_request(&file_key, &storage_request_metadata);
                        // Consume the weight used.
                        meter.consume(T::WeightInfo::process_expired_storage_request_msp_rejected(
                            amount_of_volunteered_bsps,
                        ));
                        // Emit the StorageRequestRejected event
                        // If there are BSPs that have confirmed storing the file,
                        // this event will be used by the fisherman node to delete the file from the confirmed BSPs.
                        // If there are no BSPs the event is just informative.
                        Self::deposit_event(Event::StorageRequestRejected {
                            file_key,
                            reason: RejectedStorageRequestReason::RequestExpired,
                        });
                    }
                },
                None => {
                    // This should never happen, since it would mean the storage request was deleted on
                    // its own but the expiration item wasn't removed from the queue. Do nothing since
                    // the storage request is already gone.
                }
            }
        }

        /// Utility function to clean up expired storage request data including:
        /// - Releasing the storage request creation deposit to the owner
        /// - Removing the storage request from bucket associations
        /// - Removing BSPs that volunteered for the storage request
        /// - Removing the storage request itself
        pub(crate) fn cleanup_expired_storage_request(
            file_key: &MerkleHash<T>,
            storage_request_metadata: &StorageRequestMetadata<T>,
        ) {
            // Return the storage request creation deposit to the user, emitting an error event if it fails
            // but continuing execution.
            let _ = T::Currency::release(
                &HoldReason::StorageRequestCreationHold.into(),
                &storage_request_metadata.owner,
                storage_request_metadata.deposit_paid,
                Precision::BestEffort,
            )
            .map_err(|e| {
                Self::deposit_event(Event::FailedToReleaseStorageRequestCreationDeposit {
                    file_key: *file_key,
                    owner: storage_request_metadata.owner.clone(),
                    amount_to_return: storage_request_metadata.deposit_paid,
                    error: e,
                });
            });

            // Remove the storage request from the active storage requests for the bucket
            <BucketsWithStorageRequests<T>>::remove(&storage_request_metadata.bucket_id, file_key);

            // Remove BSPs that volunteered for the storage request.
            // We consume the iterator so the drain actually happens.
            let _ = <StorageRequestBsps<T>>::drain_prefix(file_key).count();

            // Remove storage request.
            <StorageRequests<T>>::remove(file_key);
        }

        pub(crate) fn process_expired_move_bucket_request(
            bucket_id: BucketIdFor<T>,
            meter: &mut WeightMeter,
        ) {
            // Remove from storage the pending move bucket request.
            PendingMoveBucketRequests::<T>::remove(&bucket_id);

            // Deposit the event of the expired move bucket request.
            Self::deposit_event(Event::MoveBucketRequestExpired { bucket_id });

            // Consume the weight used by this function.
            meter.consume(T::WeightInfo::process_expired_move_bucket_request());
        }
    }
}
