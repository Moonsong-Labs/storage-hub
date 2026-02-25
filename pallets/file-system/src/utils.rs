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
use sp_std::{
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    vec::Vec,
};

use pallet_file_system_runtime_api::{
    GenericApplyDeltaEventInfoError, IsStorageRequestOpenToVolunteersError,
    QueryBspConfirmChunksToProveForFileError, QueryBspsVolunteeredForFileError,
    QueryConfirmChunksToProveForFileError, QueryFileEarliestVolunteerTickError,
    QueryIncompleteStorageRequestMetadataError, QueryMspConfirmChunksToProveForFileError,
};
use pallet_nfts::{CollectionConfig, CollectionSettings, ItemSettings, MintSettings, MintType};
use shp_constants::GIGAUNIT;
use shp_file_metadata::ChunkId;
use shp_traits::{
    CommitRevealRandomnessInterface, MessageAdapter, MutateBucketsInterface,
    MutateProvidersInterface, MutateStorageProvidersInterface, PaymentStreamsInterface,
    PricePerGigaUnitPerTickInterface, ProofsDealerInterface, ReadBucketsInterface,
    ReadProvidersInterface, ReadStorageProvidersInterface, ReadUserSolvencyInterface,
    TrieAddMutation, TrieRemoveMutation,
};

use crate::{
    pallet,
    types::{
        BucketIdFor, BucketMoveRequestResponse, BucketNameFor, CollectionConfigFor,
        CollectionIdFor, ExpirationItem, FileDeletionRequest, FileKeyHasher, FileKeyWithProof,
        FileLocation, FileMetadata, FileOperation, FileOperationIntention, Fingerprint,
        ForestProof, IncompleteStorageRequestMetadata, MerkleHash, MoveBucketRequestMetadata,
        MspStorageRequestStatus, MultiAddresses, PeerIds, PendingStopStoringRequest, ProviderIdFor,
        RejectedStorageRequest, ReplicationTarget, ReplicationTargetType, StorageDataUnit,
        StorageRequestBspsMetadata, StorageRequestMetadata, StorageRequestMspAcceptedFileKeys,
        StorageRequestMspBucketResponse, StorageRequestMspResponse, TickNumber,
        UserOperationPauseFlags, ValuePropId,
    },
    weights::WeightInfo,
    BucketsWithStorageRequests, Error, Event, HoldReason, IncompleteStorageRequests, Pallet,
    PendingMoveBucketRequests, PendingStopStoringRequests, StorageRequestBsps,
    StorageRequestExpirations, StorageRequests, UserOperationPauseFlagsStorage,
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

        // Get the current tick number
        let current_tick =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();

        // Get the threshold for the BSP to be able to volunteer for the storage request.
        // The current eligibility value of this storage request for this BSP has to be greater than or equal to
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
                Some(diff) if !diff.is_zero() => diff,
                _ => {
                    // The BSP's threshold is less than or equal to the current eligibility value,
                    // which means the BSP is already eligible to volunteer.
                    return Ok(current_tick);
                }
            };

        // If the BSP can't volunteer yet, calculate the number of ticks it has to wait for before it can.
        // We use ceiling division to ensure the BSP waits long enough for the threshold to be met.
        // Formula: ceil(a / b) = floor(a / b) + (1 if a % b != 0 else 0)
        let min_ticks_to_wait_to_volunteer = match eligibility_diff
            .checked_div(&bsp_eligibility_slope)
        {
            Some(quotient) => {
                // Check if there's a remainder by verifying if quotient * slope == eligibility_diff
                // If not equal, there's a remainder and we need to round up
                let has_remainder = match quotient.checked_mul(&bsp_eligibility_slope) {
                    Some(product) => product != eligibility_diff,
                    None => {
                        return Err(QueryFileEarliestVolunteerTickError::ThresholdArithmeticError);
                    }
                };

                if !has_remainder {
                    // Exact division, no rounding needed
                    max(quotient, T::ThresholdType::one())
                } else {
                    // Round up by adding 1 to the quotient
                    match quotient.checked_add(&T::ThresholdType::one()) {
                        Some(result) => max(result, T::ThresholdType::one()),
                        None => {
                            return Err(
                                QueryFileEarliestVolunteerTickError::ThresholdArithmeticError,
                            );
                        }
                    }
                }
            }
            None => {
                return Err(QueryFileEarliestVolunteerTickError::ThresholdArithmeticError);
            }
        };

        // Compute the earliest tick number at which the BSP can send the volunteer request.
        let earliest_volunteer_tick = current_tick.saturating_add(
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

    pub fn query_bsps_volunteered_for_file(
        file_key: MerkleHash<T>,
    ) -> Result<Vec<ProviderIdFor<T>>, QueryBspsVolunteeredForFileError> {
        // Check that the storage request exists.
        if !<StorageRequests<T>>::contains_key(&file_key) {
            return Err(QueryBspsVolunteeredForFileError::StorageRequestNotFound);
        }

        let bsps_volunteered =
            <StorageRequestBsps<T>>::iter_prefix(&file_key).map(|(bsp_id, _)| bsp_id);

        Ok(bsps_volunteered.collect())
    }

    pub fn decode_generic_apply_delta_event_info(
        encoded_event_info: Vec<u8>,
    ) -> Result<BucketIdFor<T>, GenericApplyDeltaEventInfoError> {
        Self::do_decode_generic_apply_delta_event_info(encoded_event_info.as_ref())
            .map_err(|_| GenericApplyDeltaEventInfoError::DecodeError)
    }

    pub fn query_incomplete_storage_request_metadata(
        file_key: MerkleHash<T>,
    ) -> Result<
        pallet_file_system_runtime_api::IncompleteStorageRequestMetadataResponse<
            T::AccountId,
            BucketIdFor<T>,
            StorageDataUnit<T>,
            Fingerprint<T>,
            ProviderIdFor<T>,
        >,
        QueryIncompleteStorageRequestMetadataError,
    > {
        let metadata = IncompleteStorageRequests::<T>::get(&file_key)
            .ok_or(QueryIncompleteStorageRequestMetadataError::StorageNotFound)?;

        // Convert to response type
        let pending_bsp_removals: Vec<ProviderIdFor<T>> =
            metadata.pending_bsp_removals.into_iter().collect();

        Ok(
            pallet_file_system_runtime_api::IncompleteStorageRequestMetadataResponse {
                owner: metadata.owner,
                bucket_id: metadata.bucket_id,
                location: metadata.location.to_vec(),
                file_size: metadata.file_size,
                fingerprint: metadata.fingerprint,
                pending_bsp_removals,
                pending_bucket_removal: metadata.pending_bucket_removal,
            },
        )
    }

    pub fn list_incomplete_storage_request_keys(
        start_after: Option<MerkleHash<T>>,
        limit: u32,
    ) -> Vec<MerkleHash<T>> {
        /// Maximum number of incomplete storage request keys that can be returned in a single call.
        const MAX_INCOMPLETE_REQUEST_KEYS: u32 = 10_000;

        let limit = limit.min(MAX_INCOMPLETE_REQUEST_KEYS);

        if limit == 0 {
            return Vec::new();
        }

        let mut keys = Vec::new();

        let iter = match start_after {
            Some(start_key) => {
                // iter_from_key excludes the starting key
                IncompleteStorageRequests::<T>::iter_from_key(start_key)
            }
            None => IncompleteStorageRequests::<T>::iter(),
        };

        for (key, _) in iter.take(limit as usize) {
            keys.push(key);
        }

        keys
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
        // Check that creating buckets is not currently paused.
        let pause_flags = UserOperationPauseFlagsStorage::<T>::get();
        ensure!(
            !pause_flags.is_all_set(UserOperationPauseFlags::FLAG_CREATE_BUCKET),
            Error::<T>::UserOperationPaused
        );

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
        // Check that requesting to move buckets is not currently paused.
        let pause_flags = UserOperationPauseFlagsStorage::<T>::get();
        ensure!(
            !pause_flags.is_all_set(UserOperationPauseFlags::FLAG_REQUEST_MOVE_BUCKET),
            Error::<T>::UserOperationPaused
        );

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
        // Check that updating bucket privacy and related collection operations are not paused.
        let pause_flags = UserOperationPauseFlagsStorage::<T>::get();
        ensure!(
            !pause_flags
                .is_all_set(UserOperationPauseFlags::FLAG_UPDATE_BUCKET_PRIVACY_AND_COLLECTION),
            Error::<T>::UserOperationPaused
        );

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
        // Check that updating bucket privacy and related collection operations are not paused.
        let pause_flags = UserOperationPauseFlagsStorage::<T>::get();
        ensure!(
            !pause_flags
                .is_all_set(UserOperationPauseFlags::FLAG_UPDATE_BUCKET_PRIVACY_AND_COLLECTION),
            Error::<T>::UserOperationPaused
        );

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
        // Check that deleting buckets is not currently paused.
        let pause_flags = UserOperationPauseFlagsStorage::<T>::get();
        ensure!(
            !pause_flags.is_all_set(UserOperationPauseFlags::FLAG_DELETE_BUCKET),
            Error::<T>::UserOperationPaused
        );

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

        // Check if there are any open storage requests for the bucket.
        // Do not allow a bucket to be deleted if there are any open storage requests for it.
        // Storage requests must be revoked or fulfilled before a bucket can be deleted.
        ensure!(
            !<BucketsWithStorageRequests<T>>::iter_prefix(bucket_id)
                .next()
                .is_some(),
            Error::<T>::StorageRequestExists
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
        // Check that issuing storage requests is not currently paused.
        let pause_flags = UserOperationPauseFlagsStorage::<T>::get();
        ensure!(
            !pause_flags.is_all_set(UserOperationPauseFlags::FLAG_ISSUE_STORAGE_REQUEST),
            Error::<T>::UserOperationPaused
        );

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
        let bucket_msp_result = expect_or_err!(
            <T::Providers as ReadBucketsInterface>::get_msp_of_bucket(&bucket_id),
            "Bucket was checked to exist previously. qed",
            Error::<T>::BucketNotFound,
            result
        );
        let msp_id_storing_bucket = expect_or_err!(
            bucket_msp_result,
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
        let msp_status = if let Some(ref msp_id) = msp_id {
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

            MspStorageRequestStatus::Pending(*msp_id)
        } else {
            MspStorageRequestStatus::None
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

        // Check that an `IncompleteStorageRequest` does not exist for this file key.
        // If it has, the user must wait until the fisherman nodes delete the file from the BSPs and/or Bucket.
        ensure!(
            !<IncompleteStorageRequests<T>>::contains_key(&file_key),
            Error::<T>::FileHasIncompleteStorageRequest
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
            msp_status,
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

        Self::deposit_event(Event::NewStorageRequest {
            who: sender,
            file_key,
            bucket_id,
            location,
            fingerprint,
            size,
            peer_ids: user_peer_ids.unwrap_or_default(),
            expires_at: expiration_tick,
            bsps_required: replication_target,
            msp_id,
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

                // We check if there are any BSP that has already confirmed the storage request
                if !storage_request_metadata.bsps_confirmed.is_zero() {
                    // We create the incomplete storage request metadata and insert it into the incomplete storage requests
                    let incomplete_storage_request_metadata: IncompleteStorageRequestMetadata<T> =
                        (&storage_request_metadata, &file_key).into();

                    Self::add_incomplete_storage_request(
                        file_key,
                        incomplete_storage_request_metadata,
                    );
                }

                // We cleanup the storage request
                Self::cleanup_storage_request(&file_key, &storage_request_metadata);

                Self::deposit_event(Event::StorageRequestRejected {
                    file_key,
                    msp_id,
                    bucket_id: storage_request_metadata.bucket_id,
                    reason,
                });
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
        // Check that requesting file deletions is not currently paused.
        let pause_flags = UserOperationPauseFlagsStorage::<T>::get();
        ensure!(
            !pause_flags.is_all_set(UserOperationPauseFlags::FLAG_REQUEST_DELETE_FILE),
            Error::<T>::UserOperationPaused
        );

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

        // Check that the file does not have an active storage request.
        // If it has, the user should use the `revoke_storage_request` extrinsic to revoke it first.
        ensure!(
            !<StorageRequests<T>>::contains_key(&signed_intention.file_key),
            Error::<T>::FileHasActiveStorageRequest
        );

        // Check that the file does not have an `IncompleteStorageRequest` associated with it.
        // If it has, the user must wait until the fisherman nodes delete the file from the BSPs and/or Bucket.
        ensure!(
            !<IncompleteStorageRequests<T>>::contains_key(&signed_intention.file_key),
            Error::<T>::FileHasIncompleteStorageRequest
        );

        // Encode the intention for signature verification
        let signed_intention_encoded = signed_intention.encode();
        // Adapt the bytes to verify depending on the runtime configuration
        let to_verify = <T as crate::pallet::Config>::IntentionMsgAdapter::bytes_to_verify(
            &signed_intention_encoded,
        );

        let is_valid = signature.verify(&to_verify[..], &who);
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
    ///
    /// Passing `None` for `bsp_id` will treat this as a Bucket Forest deletion and a BSP Forest deletion otherwise.
    pub(crate) fn do_delete_file(
        file_deletions: BoundedVec<FileDeletionRequest<T>, T::MaxFileDeletionsPerExtrinsic>,
        bsp_id: Option<ProviderIdFor<T>>,
        forest_proof: ForestProof<T>,
    ) -> DispatchResult {
        // Check that executing file deletions is not currently paused.
        let pause_flags = UserOperationPauseFlagsStorage::<T>::get();
        ensure!(
            !pause_flags.is_all_set(UserOperationPauseFlags::FLAG_DELETE_FILES),
            Error::<T>::UserOperationPaused
        );

        // Ensure we have at least one file to delete
        ensure!(!file_deletions.is_empty(), Error::<T>::NoFileKeysToDelete);

        // Collect validated file deletion data
        let mut validated_deletions = Vec::new();

        // Process each file deletion request
        for deletion_request in file_deletions.iter() {
            // Check that the file owner is not currently insolvent
            ensure!(
                !<T::UserSolvency as ReadUserSolvencyInterface>::is_user_insolvent(
                    &deletion_request.file_owner
                ),
                Error::<T>::OperationNotAllowedWithInsolventUser
            );

            // Verify that the operation is Delete
            ensure!(
                deletion_request.signed_intention.operation == FileOperation::Delete,
                Error::<T>::InvalidSignedOperation
            );

            // Encode the intention for signature verification
            let signed_intention_encoded = deletion_request.signed_intention.encode();
            // Adapt the bytes to verify depending on the runtime configuration
            let to_verify = <T as crate::pallet::Config>::IntentionMsgAdapter::bytes_to_verify(
                &signed_intention_encoded,
            );

            let is_valid = deletion_request
                .signature
                .verify(&to_verify[..], &deletion_request.file_owner);
            ensure!(is_valid, Error::<T>::InvalidSignature);

            // Compute file key from the provided metadata
            let computed_file_key = Self::compute_file_key(
                deletion_request.file_owner.clone(),
                deletion_request.bucket_id,
                deletion_request.location.clone(),
                deletion_request.size,
                deletion_request.fingerprint,
            )
            .map_err(|_| Error::<T>::FailedToComputeFileKey)?;

            // Verify that the file_key in the signed intention matches the computed one
            ensure!(
                deletion_request.signed_intention.file_key == computed_file_key,
                Error::<T>::InvalidFileKeyMetadata
            );

            validated_deletions.push((
                deletion_request.file_owner.clone(),
                computed_file_key,
                deletion_request.size,
                deletion_request.bucket_id,
            ));
        }

        // Forest proof verification and file deletion
        if let Some(bsp_id) = bsp_id {
            // Ensure the provided ID is actually a BSP
            ensure!(
                <T::Providers as ReadStorageProvidersInterface>::is_bsp(&bsp_id),
                Error::<T>::InvalidProviderID
            );

            Self::delete_files_from_bsp(validated_deletions.as_slice(), bsp_id, forest_proof)?;
        } else {
            Self::delete_files_from_bucket(validated_deletions.as_slice(), forest_proof)?;
        }

        // TODO: Reward the caller
        Ok(())
    }

    /// Delete files associated to incomplete storage requests from a Bucket or BSP.
    ///
    /// Passing `None` for `bsp_id` will treat this as a Bucket Forest deletion and a BSP Forest deletion otherwise.
    /// Multiple files can be deleted in a single call using one forest proof.
    pub(crate) fn do_delete_files_for_incomplete_storage_request(
        file_keys: BoundedVec<MerkleHash<T>, T::MaxFileDeletionsPerExtrinsic>,
        bsp_id: Option<ProviderIdFor<T>>,
        forest_proof: ForestProof<T>,
    ) -> DispatchResult {
        // Ensure we have at least one file to delete
        ensure!(!file_keys.is_empty(), Error::<T>::NoFileKeysToConfirm);

        // Collect validated file deletion data
        let mut validated_deletions = Vec::new();

        // Process each file key
        for file_key in file_keys.iter() {
            let incomplete_storage_request_metadata = IncompleteStorageRequests::<T>::get(file_key)
                .ok_or(Error::<T>::IncompleteStorageRequestNotFound)?;

            // Verify file key integrity
            let computed_file_key = Self::compute_file_key(
                incomplete_storage_request_metadata.owner.clone(),
                incomplete_storage_request_metadata.bucket_id,
                incomplete_storage_request_metadata.location.clone(),
                incomplete_storage_request_metadata.file_size,
                incomplete_storage_request_metadata.fingerprint,
            )
            .map_err(|_| Error::<T>::FailedToComputeFileKey)?;

            ensure!(
                computed_file_key == *file_key,
                Error::<T>::InvalidFileKeyMetadata
            );

            // Verify the provider is in the pending removals
            if let Some(bsp_id) = bsp_id {
                let is_bsp = incomplete_storage_request_metadata
                    .pending_bsp_removals
                    .contains(&bsp_id);

                // Ensure the BSP is actually in the pending removals list
                ensure!(is_bsp, Error::<T>::ProviderNotStoringFile);
            } else {
                ensure!(
                    incomplete_storage_request_metadata.pending_bucket_removal,
                    Error::<T>::ProviderNotStoringFile
                );
            }

            validated_deletions.push((
                incomplete_storage_request_metadata.owner.clone(),
                *file_key,
                incomplete_storage_request_metadata.file_size,
                incomplete_storage_request_metadata.bucket_id,
            ));
        }

        // Delete all files from the provider's forest
        // The deletion functions will automatically handle the cleanup of incomplete storage requests
        if let Some(bsp_id) = bsp_id {
            Self::delete_files_from_bsp(validated_deletions.as_slice(), bsp_id, forest_proof)?;
        } else {
            Self::delete_files_from_bucket(validated_deletions.as_slice(), forest_proof)?;
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

            // Check that the storage request has a pending MSP.
            let request_msp_id = match &storage_request_metadata.msp_status {
                MspStorageRequestStatus::Pending(id) => *id,
                MspStorageRequestStatus::None => return Err(Error::<T>::RequestWithoutMsp.into()),
                MspStorageRequestStatus::AcceptedNewFile(_)
                | MspStorageRequestStatus::AcceptedExistingFile(_) => {
                    return Err(Error::<T>::MspAlreadyConfirmed.into())
                }
            };

            // Check that the sender corresponds to the MSP in the storage request.
            ensure!(request_msp_id == msp_id, Error::<T>::NotSelectedMsp);

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
            // Track whether this is an inclusion proof
            let is_inclusion_proof = proven_keys.contains(&file_key_with_proof.file_key);

            // This can happen if the storage request was issued again by the user and the MSP has already stored the file.
            if !is_inclusion_proof {
                accepted_files_metadata.push((file_metadata.clone(), file_key_with_proof));

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
                file_metadata: file_metadata.clone(),
            });

            // Check if all BSPs have confirmed storing the file.
            if storage_request_metadata.bsps_confirmed == storage_request_metadata.bsps_required {
                // Clean up all storage request related data
                Self::cleanup_storage_request(
                    &file_key_with_proof.file_key,
                    &storage_request_metadata,
                );

                // Notify that the storage request has been fulfilled.
                Self::deposit_event(Event::StorageRequestFulfilled {
                    file_key: file_key_with_proof.file_key,
                });
            } else {
                // Set the MSP acceptance status in the storage request metadata.
                // The status depends on whether the MSP accepted with an inclusion or non-inclusion forestproof.
                storage_request_metadata.msp_status = if is_inclusion_proof {
                    MspStorageRequestStatus::AcceptedExistingFile(msp_id)
                } else {
                    MspStorageRequestStatus::AcceptedNewFile(msp_id)
                };

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
            (MerkleHash<T>, Vec<u8>, FileMetadata),
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
                file_keys_and_metadata.try_push((file_key, encoded_trie_value, file_metadata)),
                "Failed to push file key and metadata",
                Error::<T>::FileMetadataProcessingQueueFull,
                result
            );

            // Remove storage request if we reached the required number of BSPs and the MSP has accepted to store the file.
            // If there's no MSP assigned to it, we treat it as accepted since BSP-only requests don't need MSP acceptance.
            if storage_request_metadata.bsps_confirmed == storage_request_metadata.bsps_required
                && (storage_request_metadata.msp_status.is_accepted()
                    || !storage_request_metadata.msp_status.has_msp())
            {
                // Cleanup all storage request related data.
                Self::cleanup_storage_request(&file_key, &storage_request_metadata);

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
        file_keys_and_metadata.retain(|(fk, _, _)| !skipped_file_keys.contains(fk));

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
            .map(|(fk, encoded_metadata, _)| {
                (*fk, TrieAddMutation::new(encoded_metadata.clone()).into())
            })
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

        let confirmed_file_keys: BoundedVec<
            (MerkleHash<T>, FileMetadata),
            T::MaxBatchConfirmStorageRequests,
        > = expect_or_err!(
            file_keys_and_metadata
                .into_iter()
                .map(|(fk, _, fm)| (fk, fm))
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

        // We check if there are any BSP or MSP that have already confirmed or accepted the storage request.
        // This means, either the confirmed BSPs count is not zero, or the MSP has accepted.
        if !storage_request_metadata.bsps_confirmed.is_zero()
            || storage_request_metadata.msp_status.is_accepted()
        {
            // We create the incomplete storage request metadata and insert it into the incomplete storage requests
            let incomplete_storage_request_metadata: IncompleteStorageRequestMetadata<T> =
                (&storage_request_metadata, &file_key).into();
            Self::add_incomplete_storage_request(file_key, incomplete_storage_request_metadata);
        }

        // We cleanup the storage request
        Self::cleanup_storage_request(&file_key, &storage_request_metadata);

        Ok(())
    }

    /// Utility function to clean up expired storage request data including:
    /// - Releasing the storage request creation deposit to the owner
    /// - Removing the storage request from bucket associations
    /// - Removing BSPs that volunteered for the storage request
    /// - Removing the storage request itself
    pub(crate) fn cleanup_storage_request(
        file_key: &MerkleHash<T>,
        storage_request_metadata: &StorageRequestMetadata<T>,
    ) {
        // Remove the storage request from the expiration queue.
        // This is safe to run even if the storage request is not in the expiration queue. (Case of processing expired storage requests)
        let expiration_tick = storage_request_metadata.expires_at;
        <StorageRequestExpirations<T>>::mutate(expiration_tick, |expiration_items| {
            expiration_items.retain(|item| item != file_key);
        });

        // We always return the storage request creation deposit to the user.
        // Emitting an error event if it fails but continuing execution.
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

    /// Opens a pending request for a BSP to stop storing a file.
    ///
    /// This is the first step of the two-phase stop storing process. The BSP must later call
    /// [`do_bsp_confirm_stop_storing`] after a minimum waiting period to complete the process.
    ///
    /// ## Important
    ///
    /// **This function does NOT modify the BSP's forest root.** The file remains in the BSP's
    /// forest until [`do_bsp_confirm_stop_storing`] is called. The inclusion proof is only used
    /// to verify that the BSP currently has the file in their forest.
    ///
    /// ## Scenarios
    ///
    /// This function handles three different scenarios based on the current state:
    ///
    /// 1. **Storage request exists and BSP has confirmed storing it**: The BSP is removed from the
    ///    storage request's confirmed and volunteered lists. The `bsps_confirmed` and `bsps_volunteered`
    ///    counts are decremented, and the BSP is removed from `StorageRequestBsps`.
    ///
    /// 2. **Storage request exists but BSP is not in the volunteer list**: This can happen if the
    ///    storage request was created by another BSP (e.g., one that lost the file) and the current
    ///    BSP is already storing the file from a previous fulfilled request. In this case, we increment
    ///    `bsps_required` by 1 to ensure another BSP picks up the file.
    ///
    /// 3. **No storage request exists**: A new storage request is created with `bsps_required = 1`
    ///    so another BSP can volunteer and maintain the file's replication. If `can_serve` is true,
    ///    the requesting BSP is added as a confirmed data server to help the new volunteer download
    ///    the file before the requesting BSP completes the stop storing process.
    ///
    /// ## Parameters
    ///
    /// * `can_serve` - Whether the BSP can still serve the file to other BSPs. If true and no storage
    ///   request exists, the BSP is added as a data server for the newly created storage request.
    ///
    /// ## Restrictions
    ///
    /// This function will fail with [`FileHasIncompleteStorageRequest`] if an `IncompleteStorageRequest`
    /// exists for the file key. This enforces the invariant that a `StorageRequest` and
    /// `IncompleteStorageRequest` cannot coexist for the same file key (since scenario 3 can create
    /// a new storage request). The BSP must wait until fisherman nodes clean up the incomplete request.
    ///
    /// ## Fees
    ///
    /// The BSP is charged a penalty fee ([`BspStopStoringFilePenalty`]) which is transferred to the treasury.
    ///
    /// ## Payment Stream
    ///
    /// The payment stream with the file owner is **updated immediately** in this function, not in
    /// [`do_bsp_confirm_stop_storing`]. This removes any financial incentive for the BSP to delay
    /// or skip the confirmation, as they stop getting paid as soon as they announce their intent to stop storing.
    ///
    /// Note: This creates a temporary inconsistency where the payment stream's `amount_provided` is
    /// less than the actual data being stored by the BSP for the file owner. This is acceptable because
    /// the BSP is incentivized to correct this by confirming the stop storing request.
    ///
    /// ## Returns
    ///
    /// The provider ID of the BSP on success.
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

        // Check that an `IncompleteStorageRequest` does not exist for this file key.
        // This is enforced to maintain the invariant that a `StorageRequest` and `IncompleteStorageRequest`
        // cannot coexist for the same file key.
        // The BSP must wait until fisherman nodes clean up the incomplete request.
        ensure!(
            !<IncompleteStorageRequests<T>>::contains_key(&file_key),
            Error::<T>::FileHasIncompleteStorageRequest
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

        // Update the payment stream between the user and the BSP.
        // This is done in the request phase (not confirm) to remove any financial incentive
        // for the BSP to delay or skip the confirmation.
        // This is safe to do as the BSP must have a payment stream with the user and its amount
        // provided must be equal or greater than the file size, otherwise it would mean the BSP
        // does not have the file and as such shouldn't be able to provide an inclusion proof.
        let new_amount_provided = <T::PaymentStreams as PaymentStreamsInterface>::get_dynamic_rate_payment_stream_amount_provided(&bsp_id, &owner)
            .ok_or(Error::<T>::DynamicRatePaymentStreamNotFound)?
            .saturating_sub(size);
        if new_amount_provided.is_zero() {
            <T::PaymentStreams as PaymentStreamsInterface>::delete_dynamic_rate_payment_stream(
                &bsp_id, &owner,
            )?;
        } else {
            <T::PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                &bsp_id,
                &owner,
                &new_amount_provided,
            )?;
        }

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

    /// Confirms a BSP's request to stop storing a file and removes it from their forest.
    ///
    /// This is the second step of the two-phase stop storing process. The BSP must have previously
    /// called [`do_bsp_request_stop_storing`] to open a pending stop storing request.
    ///
    /// ## Minimum Wait Time
    ///
    /// A minimum waiting period ([`MinWaitForStopStoring`]) must pass between the request and this
    /// confirmation. This prevents BSPs from immediately dropping a file when challenged for it,
    /// ensuring they can't avoid slashing by quickly calling stop storing upon receiving a challenge.
    ///
    /// ## What this function does
    ///
    /// 1. Retrieves and removes the [`PendingStopStoringRequest`] for this BSP and file
    /// 2. Verifies the minimum wait time has passed since the request was opened
    /// 3. Verifies the file is still in the BSP's forest via the inclusion proof
    /// 4. **Removes the file from the BSP's forest and computes the new root**
    /// 5. Updates the BSP's root in storage
    /// 6. Decreases the BSP's used capacity by the file size
    /// 7. Stops the BSP's challenge and randomness cycles if their forest is now empty
    ///
    /// Note: The payment stream was already updated in [`do_bsp_request_stop_storing`].
    ///
    /// ## Returns
    ///
    /// A tuple of (bsp_id, new_root) on success, where `new_root` is the BSP's updated forest root
    /// after removing the file.
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

        // Note: Payment stream was already updated in do_bsp_request_stop_storing.
        // Here we only need to manage the BSP cycles if the root becomes default.
        Self::stop_bsp_cycles_if_root_is_default(bsp_id, new_root)?;

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
        } else {
            // If the provider is an MSP (so this was a bucket file deletion), check if the bucket doesn't have
            // any more files and clean it up.
            // A bucket is empty if its root is the default root.
            if new_root == <T::Providers as shp_traits::ReadProvidersInterface>::get_default_root()
            {
                // Check the bucket size, to log an error if it's not zero. We can later monitor for those
                // to detect inconsistencies.
                let bucket_size =
                    <T::Providers as ReadBucketsInterface>::get_bucket_size(&bucket_id)?;
                if bucket_size != StorageDataUnit::<T>::zero() {
                    Self::deposit_event(Event::UsedCapacityShouldBeZero {
                        actual_used_capacity: bucket_size,
                    });
                }

                // Delete the bucket.
                <T::Providers as MutateBucketsInterface>::delete_bucket(bucket_id)?;
            }
        }

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

    pub fn storage_requests_by_msp(
        msp_id: ProviderIdFor<T>,
    ) -> BTreeMap<MerkleHash<T>, StorageRequestMetadata<T>> {
        StorageRequests::<T>::iter()
            .filter(|(_, metadata)| metadata.msp_status.msp_id() == Some(msp_id))
            .collect()
    }

    pub fn pending_storage_requests_by_msp(
        msp_id: ProviderIdFor<T>,
    ) -> BTreeMap<MerkleHash<T>, StorageRequestMetadata<T>> {
        // Get the storage requests for a specific MSP that are still pending response
        StorageRequests::<T>::iter()
            .filter(|(_, metadata)| {
                metadata.msp_status.msp_id() == Some(msp_id) && metadata.msp_status.is_pending()
            })
            .collect()
    }

    /// Filter the given file keys to return only those that the BSP still needs to confirm storing after
    /// volunteering for them.
    ///
    /// This function queries `StorageRequestBsps` for each provided file key and BSP ID,
    /// filtering out file keys where:
    /// - The BSP has already confirmed storing (confirmed = true)
    /// - The BSP is not a volunteer for the storage request (no entry exists)
    /// - The storage request doesn't exist
    ///
    /// Returns a Vec of file keys that the BSP has volunteered for but not yet confirmed.
    pub fn query_pending_bsp_confirm_storage_requests(
        bsp_id: ProviderIdFor<T>,
        file_keys: Vec<MerkleHash<T>>,
    ) -> Vec<MerkleHash<T>> {
        file_keys
            .into_iter()
            .filter(|file_key| {
                // Check if BSP has volunteered but not yet confirmed for this file key
                match StorageRequestBsps::<T>::get(file_key, &bsp_id) {
                    Some(metadata) => !metadata.confirmed,
                    None => false,
                }
            })
            .collect()
    }

    pub fn get_max_batch_confirm_storage_requests() -> u32 {
        T::MaxBatchConfirmStorageRequests::get()
    }

    /// Removes multiple file keys from the bucket's forest in a single operation, updating the bucket's root.
    ///
    /// Does not enforce the presence of an MSP storing the bucket. If no MSP is found to be
    /// storing the bucket, no payment stream is updated.
    ///
    /// This is to support the case where an MSP stops storing a bucket while there still exists
    /// incomplete storage requests for that bucket.
    ///
    /// # Arguments
    /// * `file_deletions` - Slice of tuples containing (owner, file_key, size, bucket_id) for each file
    /// * `forest_proof` - Proof that all files exist in the bucket's forest
    fn delete_files_from_bucket(
        file_deletions: &[(
            T::AccountId,
            MerkleHash<T>,
            StorageDataUnit<T>,
            BucketIdFor<T>,
        )],
        forest_proof: ForestProof<T>,
    ) -> DispatchResult {
        // All files must be in the same bucket - validate and get bucket_id
        let bucket_id = file_deletions[0].3;

        // Single pass collection: gather file keys, mutations, total size, and validate bucket consistency
        let mut file_keys = BoundedVec::<MerkleHash<T>, T::MaxFileDeletionsPerExtrinsic>::default();
        let mut mutations = Vec::with_capacity(file_deletions.len());
        let mut total_size = StorageDataUnit::<T>::zero();
        let mut seen_keys = BTreeSet::new();

        for (_, file_key, size, bid) in file_deletions {
            // Ensure all files are in the same bucket
            ensure!(
                *bid == bucket_id,
                Error::<T>::BatchFileDeletionMustContainSingleBucket
            );

            // Detect duplicate file keys in the batch
            ensure!(
                seen_keys.insert(*file_key),
                Error::<T>::DuplicateFileKeyInBatchFileDeletion
            );

            expect_or_err!(
                file_keys.try_push(*file_key),
                "file_deletions is already bounded by MaxFileDeletionsPerExtrinsic",
                Error::<T>::FailedToPushFileKeyToBucketDeletionVector,
                result
            );
            mutations.push((*file_key, TrieRemoveMutation::default().into()));
            total_size = total_size.saturating_add(*size);

            // Check if there's an open storage request for this file key
            if let Some(storage_request) = StorageRequests::<T>::get(&file_key) {
                // If there is, remove it and issue a `IncompleteStorageRequest` with the confirmed providers
                // This is to avoid having a situation where:
                // - The MSP accepted the existing storage request, but it's not fulfilled yet.
                // - A deletion requests exists for the same file key, so the file is deleted from that MSP's bucket here.
                // - A BSP confirms the storage request, and it gets fulfilled.
                // This would result in the file being stored by the BSP, but not by the MSP.
                Self::add_incomplete_storage_request(
                    *file_key,
                    IncompleteStorageRequestMetadata::from((&storage_request, file_key)),
                );
                Self::cleanup_storage_request(&file_key, &storage_request);
            }

            // Remove bucket from incomplete storage request if it exists
            Self::remove_provider_from_incomplete_storage_request(*file_key, None);
        }

        // Drop seen_keys to free memory - no longer needed after validation
        drop(seen_keys);

        // Get current bucket root
        let old_bucket_root = <T::Providers as ReadBucketsInterface>::get_root_bucket(&bucket_id)
            .ok_or(Error::<T>::BucketNotFound)?;

        // Verify all file keys are part of the bucket's forest
        let proven_keys = <T::ProofDealer as ProofsDealerInterface>::verify_generic_forest_proof(
            &old_bucket_root,
            file_keys.as_slice(),
            &forest_proof,
        )?;

        // Ensure that all file keys are proven
        for file_key in &file_keys {
            ensure!(
                proven_keys.contains(file_key),
                Error::<T>::ExpectedInclusionProof
            );
        }

        // Compute new root after removing all file keys from forest
        let new_root = <T::ProofDealer as ProofsDealerInterface>::generic_apply_delta(
            &old_bucket_root,
            &mutations,
            &forest_proof,
            Some(bucket_id.encode()),
        )?;

        // Drop mutations to free memory - no longer needed after apply_delta
        drop(mutations);

        // Check if there's an MSP storing the bucket
        let maybe_msp_id = <T::Providers as ReadBucketsInterface>::get_bucket_msp(&bucket_id)?;

        if let Some(msp_id) = maybe_msp_id {
            <T::Providers as MutateBucketsInterface>::change_root_bucket(bucket_id, new_root)?;
            // Decrease capacity used of the MSP
            <T::Providers as MutateStorageProvidersInterface>::decrease_capacity_used(
                &msp_id, total_size,
            )?;

            // Decrease bucket size
            // This function also updates the fixed rate payment stream between the user and the MSP.
            // via apply_delta_fixed_rate_payment_stream function in providers pallet.
            <T::Providers as MutateBucketsInterface>::decrease_bucket_size(&bucket_id, total_size)?;
        } else {
            <T::Providers as MutateBucketsInterface>::change_root_bucket_without_msp(
                bucket_id, new_root,
            )?;
            // Decrease bucket size
            <T::Providers as MutateBucketsInterface>::decrease_bucket_size_without_msp(
                &bucket_id, total_size,
            )?;
        }

        // Emit single event for the batch
        Self::deposit_event(Event::BucketFileDeletionsCompleted {
            // We validate that all the deleted files belong to the same bucket and therefore a single user owns them all
            user: file_deletions[0].0.clone(),
            file_keys,
            bucket_id,
            msp_id: maybe_msp_id,
            old_root: old_bucket_root,
            new_root,
        });

        Ok(())
    }

    /// Removes multiple file keys from the BSP's forest in a single operation, updating the BSP's root.
    ///
    /// # Arguments
    /// * `file_deletions` - Slice of tuples containing (owner, file_key, size, bucket_id) for each file
    /// * `bsp_id` - The BSP from which to delete the files
    /// * `forest_proof` - Proof that all files exist in the BSP's forest
    fn delete_files_from_bsp(
        file_deletions: &[(
            T::AccountId,
            MerkleHash<T>,
            StorageDataUnit<T>,
            BucketIdFor<T>,
        )],
        bsp_id: ProviderIdFor<T>,
        forest_proof: ForestProof<T>,
    ) -> DispatchResult {
        // Get current BSP root
        let old_root = <T::Providers as ReadProvidersInterface>::get_root(bsp_id)
            .ok_or(Error::<T>::NotABsp)?;

        // Single pass collection: gather users, file keys, mutations, total size, and owner sizes
        let mut users = BoundedVec::<T::AccountId, T::MaxFileDeletionsPerExtrinsic>::default();
        let mut file_keys = BoundedVec::<MerkleHash<T>, T::MaxFileDeletionsPerExtrinsic>::default();
        let mut mutations = Vec::with_capacity(file_deletions.len());
        let mut total_size = StorageDataUnit::<T>::zero();
        let mut owner_sizes: BTreeMap<T::AccountId, StorageDataUnit<T>> = BTreeMap::new();
        let mut seen_keys = BTreeSet::new();

        for (owner, file_key, size, _) in file_deletions {
            // Detect duplicate file keys in the batch
            ensure!(
                seen_keys.insert(*file_key),
                Error::<T>::DuplicateFileKeyInBatchFileDeletion
            );

            expect_or_err!(
                users.try_push(owner.clone()),
                "file_deletions is already bounded by MaxFileDeletionsPerExtrinsic",
                Error::<T>::FailedToPushUserToBspDeletionVector,
                result
            );
            expect_or_err!(
                file_keys.try_push(*file_key),
                "file_deletions is already bounded by MaxFileDeletionsPerExtrinsic",
                Error::<T>::FailedToPushFileKeyToBspDeletionVector,
                result
            );
            mutations.push((*file_key, TrieRemoveMutation::default().into()));
            total_size.saturating_accrue(*size);

            // Aggregate sizes per owner for payment stream updates.
            owner_sizes
                .entry(owner.clone())
                .and_modify(|s| s.saturating_accrue(*size))
                .or_insert(*size);

            // Check if there's an open storage request for this file key
            if let Some(storage_request) = StorageRequests::<T>::get(&file_key) {
                // If there is, remove it and issue a `IncompleteStorageRequest` with the confirmed providers
                // This is to avoid having a situation where:
                // - The MSP accepted the existing storage request, but it's not fulfilled yet.
                // - A deletion requests exists for the same file key, so the file is deleted from that MSP's bucket here.
                // - A BSP confirms the storage request, and it gets fulfilled.
                // This would result in the file being stored by the BSP, but not by the MSP.
                Self::add_incomplete_storage_request(
                    *file_key,
                    IncompleteStorageRequestMetadata::from((&storage_request, file_key)),
                );
                Self::cleanup_storage_request(&file_key, &storage_request);
            }

            // Remove BSP from incomplete storage request if it exists
            Self::remove_provider_from_incomplete_storage_request(*file_key, Some(bsp_id));
        }

        // Drop seen_keys to free memory - no longer needed after validation
        drop(seen_keys);

        // Verify all file keys are part of the BSP's forest
        let proven_keys = <T::ProofDealer as ProofsDealerInterface>::verify_forest_proof(
            &bsp_id,
            file_keys.as_slice(),
            &forest_proof,
        )?;

        // Ensure that all file keys are proven
        for file_key in &file_keys {
            ensure!(
                proven_keys.contains(file_key),
                Error::<T>::ExpectedInclusionProof
            );
        }

        // Compute new root after removing all file keys from forest
        let new_root = <T::ProofDealer as ProofsDealerInterface>::apply_delta(
            &bsp_id,
            &mutations,
            &forest_proof,
        )?;

        // Drop mutations to free memory - no longer needed after apply_delta
        drop(mutations);

        // Update root of BSP
        <T::Providers as MutateProvidersInterface>::update_root(bsp_id, new_root)?;

        // Decrease capacity used by the BSP
        <T::Providers as MutateStorageProvidersInterface>::decrease_capacity_used(
            &bsp_id, total_size,
        )?;

        // Update payment streams for each file owner
        for (owner, size) in owner_sizes {
            Self::update_bsp_payment_and_cycles_after_file_removal(bsp_id, &owner, size, new_root)?;
        }

        // Emit single event for the batch
        Self::deposit_event(Event::BspFileDeletionsCompleted {
            users,
            file_keys,
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

    /// Stops a BSP's challenge and randomness cycles if their forest root is the default (empty) root.
    ///
    /// Also performs a sanity check that used_capacity is zero when root is default, and emits
    /// an event if there's an inconsistency.
    fn stop_bsp_cycles_if_root_is_default(
        bsp_id: ProviderIdFor<T>,
        new_root: MerkleHash<T>,
    ) -> DispatchResult {
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

    /// Remove a provider (BSP or bucket) from an incomplete storage request.
    /// If no more providers are pending removal, the incomplete storage request is deleted
    /// and an `IncompleteStorageRequestCleanedUp` event is emitted.
    ///
    /// # Arguments
    /// * `file_key` - The file key for the incomplete storage request
    /// * `provider_id` - `Some(bsp_id)` to remove a BSP, or `None` to remove the bucket
    fn remove_provider_from_incomplete_storage_request(
        file_key: MerkleHash<T>,
        provider_id: Option<ProviderIdFor<T>>,
    ) {
        IncompleteStorageRequests::<T>::mutate(file_key, |incomplete_storage_request| {
            if let Some(ref mut metadata) = incomplete_storage_request {
                // Remove the provider from the pending removals
                metadata.remove_provider(provider_id);

                // Check if we have removed the file from all pending providers
                if metadata.is_fully_cleaned() {
                    // No more providers pending removal, so remove the incomplete storage request
                    *incomplete_storage_request = None;

                    // Emit the `IncompleteStorageRequestCleanedUp` event
                    Self::deposit_event(Event::IncompleteStorageRequestCleanedUp { file_key });
                }
            }
        });
    }

    /// Add storage request to [`IncompleteStorageRequests`] storage and emit `IncompleteStorageRequest` event
    ///
    /// We check first if there's any provider pending deletion before inserting it, as it could be the case that
    /// this storage request was not confirmed by any BSP and the MSP already had the file from a previous storage request,
    /// which means nothing has to be deleted in the runtime and, as such, there's no need to track the incomplete storage request.
    ///
    /// If there are no providers to clean, we emit `IncompleteStorageRequestCleanedUp` to notify
    /// that the incomplete storage request has been fully cleaned up.
    fn add_incomplete_storage_request(
        file_key: MerkleHash<T>,
        metadata: IncompleteStorageRequestMetadata<T>,
    ) {
        if metadata.pending_bucket_removal || !metadata.pending_bsp_removals.is_empty() {
            IncompleteStorageRequests::<T>::insert(&file_key, metadata);
            Self::deposit_event(Event::IncompleteStorageRequest { file_key });
        } else {
            // No providers to clean, emit cleanup event immediately
            Self::deposit_event(Event::IncompleteStorageRequestCleanedUp { file_key });
        }
    }
}

mod hooks {
    use crate::{
        pallet,
        types::{
            IncompleteStorageRequestMetadata, MerkleHash, MspStorageRequestStatus,
            RejectedStorageRequestReason, TickNumber,
        },
        utils::BucketIdFor,
        weights::WeightInfo,
        Event, MoveBucketRequestExpirations, NextStartingTickToCleanUp, Pallet,
        PendingMoveBucketRequests, StorageRequestBsps, StorageRequestExpirations, StorageRequests,
    };
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
                Some(storage_request_metadata) => match &storage_request_metadata.msp_status {
                    MspStorageRequestStatus::None
                    | MspStorageRequestStatus::AcceptedNewFile(_)
                    | MspStorageRequestStatus::AcceptedExistingFile(_) => {
                        // If the request was originated by a request to stop storing from a BSP for a file that had no
                        // storage request open, or if the MSP has already accepted storing the file (and the bucket and
                        // payment stream with the user still exists), treat the storage request as fulfilled with whatever
                        // amount of BSPs got to volunteer and confirm the file. For that:
                        // Clean up storage request data
                        Self::cleanup_storage_request(&file_key, &storage_request_metadata);

                        // Emit the StorageRequestExpired event
                        Self::deposit_event(Event::StorageRequestExpired { file_key });

                        // Consume the weight used.
                        meter.consume(
                            T::WeightInfo::process_expired_storage_request_msp_accepted_or_no_msp(
                                amount_of_volunteered_bsps,
                            ),
                        );
                    }
                    MspStorageRequestStatus::Pending(msp_id) => {
                        // If the MSP did not accept the file in time, treat the storage request as rejected.
                        if !storage_request_metadata.bsps_confirmed.is_zero() {
                            // There are BSPs that have confirmed storing the file, so we need to create an incomplete storage request metadata
                            // This will allow the fisherman node to delete the file from the confirmed BSPs.
                            let incomplete_storage_request_metadata: IncompleteStorageRequestMetadata<T> =
                                (&storage_request_metadata, &file_key).into();

                            Self::add_incomplete_storage_request(
                                file_key,
                                incomplete_storage_request_metadata,
                            );
                        }
                        // Clean up all storage request related data
                        Self::cleanup_storage_request(&file_key, &storage_request_metadata);
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
                            msp_id: *msp_id,
                            bucket_id: storage_request_metadata.bucket_id,
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
