use codec::Encode;
use frame_support::{
    ensure,
    pallet_prelude::DispatchResult,
    traits::{
        fungible::{InspectHold, Mutate, MutateHold},
        nonfungibles_v2::{Create, Destroy},
        tokens::{Precision, Preservation},
        Get,
    },
};
use num_bigint::BigUint;
use sp_runtime::{
    traits::{
        Bounded, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Convert, ConvertBack, Hash, One,
        Saturating, Zero,
    },
    ArithmeticError, BoundedBTreeSet, BoundedVec, DispatchError,
};
use sp_std::{collections::btree_set::BTreeSet, vec::Vec};

use pallet_file_system_runtime_api::{
    IsStorageRequestOpenToVolunteersError, QueryBspConfirmChunksToProveForFileError,
    QueryConfirmChunksToProveForFileError, QueryFileEarliestVolunteerTickError,
    QueryMspConfirmChunksToProveForFileError,
};
use pallet_nfts::{CollectionConfig, CollectionSettings, ItemSettings, MintSettings, MintType};
use shp_file_metadata::ChunkId;
use shp_traits::{
    CommitRevealRandomnessInterface, MutateBucketsInterface, MutateStorageProvidersInterface,
    PaymentStreamsInterface, ReadBucketsInterface, ReadProvidersInterface,
    ReadStorageProvidersInterface, ReadUserSolvencyInterface, TrieAddMutation, TrieRemoveMutation,
};

use crate::{
    pallet,
    types::{
        BucketIdFor, BucketMoveRequestResponse, BucketNameFor, CollectionConfigFor,
        CollectionIdFor, EitherAccountIdOrMspId, ExpirationItem, FileDeletionRequestExpirationItem,
        FileKeyHasher, FileKeyWithProof, FileLocation, Fingerprint, ForestProof, MerkleHash,
        MoveBucketRequestMetadata, MultiAddresses, PeerIds, PendingFileDeletionRequest,
        PendingStopStoringRequest, ProviderIdFor, RejectedStorageRequest, ReplicationTargetType,
        StorageData, StorageRequestBspsMetadata, StorageRequestMetadata,
        StorageRequestMspAcceptedFileKeys, StorageRequestMspBucketResponse,
        StorageRequestMspResponse, TickNumber, ValuePropId,
    },
    BucketsWithStorageRequests, Error, Event, HoldReason, MaxReplicationTarget, Pallet,
    PendingBucketsToMove, PendingFileDeletionRequests, PendingMoveBucketRequests,
    PendingStopStoringRequests, StorageRequestBsps, StorageRequestExpirations, StorageRequests,
    TickRangeToMaximumThreshold,
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
        // Get the tick number at which the storage request was created.
        let (storage_request_tick, fingerprint) = match <StorageRequests<T>>::get(&file_key) {
            Some(storage_request) => (storage_request.requested_at, storage_request.fingerprint),
            None => {
                return Err(QueryFileEarliestVolunteerTickError::StorageRequestNotFound);
            }
        };

        // Get the threshold needed for the BSP to be able to volunteer for the storage request.
        let bsp_threshold = Self::get_threshold_for_bsp_request(&bsp_id, &fingerprint);

        // Compute the tick number at which the BSP should send the volunteer request.
        Self::compute_volunteer_tick_number(bsp_id, bsp_threshold, storage_request_tick)
            .map_err(|_| QueryFileEarliestVolunteerTickError::ThresholdArithmeticError)
    }

    fn compute_volunteer_tick_number(
        bsp_id: ProviderIdFor<T>,
        bsp_threshold: T::ThresholdType,
        storage_request_tick: TickNumber<T>,
    ) -> Result<TickNumber<T>, DispatchError>
    where
        T: frame_system::Config,
    {
        // Compute the threshold to succeed and the slope of the bsp.
        let (to_succeed, slope) =
            Self::compute_threshold_to_succeed(&bsp_id, storage_request_tick)?;

        let threshold_diff = match bsp_threshold.checked_sub(&to_succeed) {
            Some(diff) => diff,
            None => {
                // The BSP's threshold is less than the current threshold.
                let current_tick =
                    <T::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                return Ok(current_tick);
            }
        };

        // Calculate the number of ticks required to be below the threshold.
        let ticks_to_wait = match threshold_diff.checked_div(&slope) {
            Some(ticks) => ticks,
            None => {
                return Err(Error::<T>::ThresholdArithmeticError.into());
            }
        };

        // Compute the tick number at which the BSP should send the volunteer request.
        let volunteer_tick_number = storage_request_tick
            .saturating_add(T::ThresholdTypeToTickNumber::convert(ticks_to_wait));

        Ok(volunteer_tick_number)
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
        );

        let chunks = storage_request_metadata.to_file_metadata().chunks_count();

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
    ) -> Vec<<<T as pallet::Config>::Providers as ReadProvidersInterface>::MerkleHash> {
        let file_metadata = storage_request_metadata.clone().to_file_metadata();
        let chunks_to_check = file_metadata.chunks_to_check();

        let mut challenges =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::generate_challenges_from_seed(
                T::MerkleHashToRandomnessOutput::convert(file_key),
                &sp_id,
                chunks_to_check.saturating_sub(One::one()),
            );

        let last_chunk_id = file_metadata.last_chunk_id();

        challenges.push(T::ChunkIdToMerkleHash::convert(last_chunk_id));

        challenges
    }

    /// Create a bucket for an owner (user) under a given MSP account.
    pub(crate) fn do_create_bucket(
        sender: T::AccountId,
        msp_id: ProviderIdFor<T>,
        name: BucketNameFor<T>,
        private: bool,
        value_prop_id: Option<ValuePropId<T>>,
    ) -> Result<(BucketIdFor<T>, Option<CollectionIdFor<T>>), DispatchError> {
        // Check if the MSP is indeed an MSP.
        ensure!(
            <T::Providers as ReadStorageProvidersInterface>::is_msp(&msp_id),
            Error::<T>::NotAMsp
        );

        // Check if MSP is insolvent
        ensure!(
            !<T::Providers as ReadProvidersInterface>::is_provider_insolvent(msp_id),
            Error::<T>::OperationNotAllowedForInsolventProvider
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
    ) -> Result<(), DispatchError> {
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

        // Check if the newly selected MSP is not insolvent
        ensure!(
            !<T::Providers as ReadProvidersInterface>::is_provider_insolvent(new_msp_id),
            Error::<T>::OperationNotAllowedForInsolventProvider
        );

        // Check if the bucket is already stored by the new MSP.
        ensure!(
            !<T::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(
                &new_msp_id,
                &bucket_id
            ),
            Error::<T>::MspAlreadyStoringBucket
        );

        if <PendingBucketsToMove<T>>::contains_key(&bucket_id) {
            return Err(Error::<T>::BucketIsBeingMoved.into());
        }

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
            &new_msp_id,
            bucket_id,
            MoveBucketRequestMetadata {
                requester: sender.clone(),
            },
        );
        <PendingBucketsToMove<T>>::insert(&bucket_id, ());

        let expiration_item = ExpirationItem::MoveBucketRequest((new_msp_id, bucket_id));
        Self::enqueue_expiration_item(expiration_item)?;

        Ok(())
    }

    pub(crate) fn do_msp_respond_move_bucket_request(
        sender: T::AccountId,
        bucket_id: BucketIdFor<T>,
        response: BucketMoveRequestResponse,
    ) -> Result<ProviderIdFor<T>, DispatchError> {
        let msp_id = <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(sender)
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

        // Check if the move bucket request exists for the MSP and bucket.
        let move_bucket_requester = <PendingMoveBucketRequests<T>>::take(&msp_id, bucket_id);
        ensure!(
            move_bucket_requester.is_some(),
            Error::<T>::MoveBucketRequestNotFound
        );

        if response == BucketMoveRequestResponse::Accepted {
            let bucket_size = <T::Providers as ReadBucketsInterface>::get_bucket_size(&bucket_id)?;

            let previous_msp_id =
                <T::Providers as ReadBucketsInterface>::get_msp_bucket(&bucket_id)?;

            // Update the previous MSP's capacity used.
            if let Some(msp_id) = previous_msp_id {
                // Decrease the used capacity of the previous MSP.
                <T::Providers as MutateStorageProvidersInterface>::decrease_capacity_used(
                    &msp_id,
                    bucket_size,
                )?;
            }

            // Check if MSP has enough available capacity to store the bucket.
            ensure!(
                <T::Providers as ReadStorageProvidersInterface>::available_capacity(&msp_id)
                    >= bucket_size,
                Error::<T>::InsufficientAvailableCapacity
            );

            // Change the MSP that stores the bucket.
            <T::Providers as MutateBucketsInterface>::assign_msp_to_bucket(&bucket_id, &msp_id)?;

            // Increase the used capacity of the new MSP.
            <T::Providers as MutateStorageProvidersInterface>::increase_capacity_used(
                &msp_id,
                bucket_size,
            )?;
        }

        <PendingBucketsToMove<T>>::remove(&bucket_id);

        Ok(msp_id)
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
        <T::Providers as MutateBucketsInterface>::remove_root_bucket(bucket_id)?;

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
        size: StorageData<T>,
        msp_id: Option<ProviderIdFor<T>>,
        replication_target: Option<ReplicationTargetType<T>>,
        user_peer_ids: Option<PeerIds<T>>,
    ) -> Result<MerkleHash<T>, DispatchError> {
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
            !<PendingBucketsToMove<T>>::contains_key(&bucket_id),
            Error::<T>::BucketIsBeingMoved
        );

        // Check if we can hold the storage request creation deposit from the user
        let deposit = T::StorageRequestCreationDeposit::get();
        ensure!(
            T::Currency::can_hold(
                &HoldReason::StorageRequestCreationHold.into(),
                &sender,
                deposit
            ),
            Error::<T>::CannotHoldDeposit
        );

        // If a specific MSP ID is provided, check that it is a valid MSP and that it has enough available capacity to store the file.
        let msp = if let Some(ref msp_id) = msp_id {
            // Check that the received Provider ID corresponds to a valid MSP.
            ensure!(
                <T::Providers as ReadStorageProvidersInterface>::is_msp(msp_id),
                Error::<T>::NotAMsp
            );

            // Check if the MSP is insolvent
            ensure!(
                !<T::Providers as ReadProvidersInterface>::is_provider_insolvent(*msp_id),
                Error::<T>::OperationNotAllowedForInsolventProvider
            );

            // Check that the MSP received is the one storing the bucket.
            ensure!(
                <T::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(msp_id, &bucket_id),
                Error::<T>::MspNotStoringBucket
            );

            Some((*msp_id, false))
        } else {
            None
        };

        let replication_target = replication_target.unwrap_or(T::DefaultReplicationTarget::get());

        if replication_target.is_zero() {
            return Err(Error::<T>::ReplicationTargetCannotBeZero)?;
        }

        if replication_target > MaxReplicationTarget::<T>::get().into() {
            return Err(Error::<T>::ReplicationTargetExceedsMaximum)?;
        }

        let current_tick =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();

        // Compute the file key used throughout this file's lifespan.
        let file_key = Self::compute_file_key(
            sender.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

        // Check a storage request does not already exist for this file key.
        ensure!(
            !<StorageRequests<T>>::contains_key(&file_key),
            Error::<T>::StorageRequestAlreadyRegistered
        );

        // Enqueue an expiration item for the storage request to clean it up if it expires without being fulfilled or cancelled.
        let expiration_item = ExpirationItem::StorageRequest(file_key);
        let expiration_tick = Self::enqueue_expiration_item(expiration_item)?;

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
        };

        // Hold the deposit from the user
        T::Currency::hold(
            &HoldReason::StorageRequestCreationHold.into(),
            &sender,
            deposit,
        )?;

        // Register storage request.
        <StorageRequests<T>>::insert(&file_key, storage_request_metadata);

        <BucketsWithStorageRequests<T>>::insert(&bucket_id, &file_key, ());

        // BSPs listen to this event and volunteer to store the file
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
    ///
    /// This is using a best-effort strategy to process as many file keys as possible, returning
    /// the ones that were accepted, rejected, or failed to be processed.
    ///
    /// File keys that are not part of the bucket they belong to will be skipped (failed).
    ///
    /// All file keys will be processed (unless there are duplicates, they are simply skipped) and any errors
    /// while processing them will be marked as a failed key and continue processing the rest. It is up to the
    /// caller to verify the final result and apply only the file keys that have been successfully accepted.
    pub(crate) fn do_msp_respond_storage_request(
        sender: T::AccountId,
        storage_request_msp_response: StorageRequestMspResponse<T>,
    ) -> Result<(), DispatchError> {
        // Check that the sender is a Storage Provider and get its MSP ID
        let msp_id =
            <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(sender.clone())
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
        let msp_id =
            <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(sender.clone())
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

    /// Accept as many storage requests as possible (best-effort) belonging to the same bucket.
    ///
    /// There should be a single non-inclusion forest proof for all file keys, and finally there should
    /// be a list of file key(s) with a key proof for each of them.
    ///
    /// The implementation follows this sequence:
    /// 1. Verify the non-inclusion proof.
    /// 2. For each file key: Verify and process the acceptance. If any operation fails during the processing of a file key,
    /// the entire function will fail and no changes will be applied.
    /// 3. If all file keys are successfully processed, apply the delta with all the accepted keys to the root of the bucket.
    /// 4. If any step fails, the function will return an error and no changes will be made to the storage state.
    fn do_msp_accept_storage_request(
        msp_id: ProviderIdFor<T>,
        bucket_id: BucketIdFor<T>,
        accepted_file_keys: StorageRequestMspAcceptedFileKeys<T>,
    ) -> Result<MerkleHash<T>, DispatchError> {
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
                &accepted_file_keys.non_inclusion_forest_proof,
            )?;

        let mut accepted_files_metadata = Vec::new();

        for file_key_with_proof in accepted_file_keys.file_keys_and_proofs.iter() {
            let mut storage_request_metadata =
                <StorageRequests<T>>::get(&file_key_with_proof.file_key)
                    .ok_or(Error::<T>::StorageRequestNotFound)?;

            // Ensure that the file key IS NOT part of the bucket's forest.
            if proven_keys.contains(&file_key_with_proof.file_key) {
                return Err(Error::<T>::ExpectedNonInclusionProof.into());
            }

            // Check that the storage request bucket ID matches the provided bucket ID.
            if storage_request_metadata.bucket_id != bucket_id {
                return Err(Error::<T>::InvalidBucketIdFileKeyPair.into());
            }

            // Check that the MSP is the one storing the bucket.
            if !<T::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(
                &msp_id,
                &storage_request_metadata.bucket_id,
            ) {
                return Err(Error::<T>::MspNotStoringBucket.into());
            }

            // Check that the storage request has a MSP.
            if storage_request_metadata.msp.is_none() {
                return Err(Error::<T>::RequestWithoutMsp.into());
            }

            let (request_msp_id, confirm_status) = storage_request_metadata.msp.unwrap();

            // Check that the sender corresponds to the MSP in the storage request and that it hasn't yet confirmed storing the file.
            if request_msp_id != msp_id {
                return Err(Error::<T>::NotSelectedMsp.into());
            }

            if confirm_status {
                return Err(Error::<T>::MspAlreadyConfirmed.into());
            }

            // Check that the MSP still has enough available capacity to store the file.
            if <T::Providers as ReadStorageProvidersInterface>::available_capacity(&msp_id)
                < storage_request_metadata.size
            {
                return Err(Error::<T>::InsufficientAvailableCapacity.into());
            }

            // Get the file metadata to insert into the bucket under the file key.
            let file_metadata = storage_request_metadata.clone().to_file_metadata();

            accepted_files_metadata.push(file_metadata);

            let chunk_challenges = Self::generate_chunk_challenges_on_sp_confirm(
                msp_id,
                file_key_with_proof.file_key,
                &storage_request_metadata,
            );

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

            // Notify that the storage request has been accepted by an MSP.
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

                // Make sure that the expected number of bsps were removed.
                expect_or_err!(
                    storage_request_metadata.bsps_volunteered == removed.into(),
                    "Number of volunteered bsps for storage request should have been removed",
                    Error::<T>::UnexpectedNumberOfRemovedVolunteeredBsps,
                    bool
                );

                // Return the storage request creation deposit to the user
                T::Currency::release(
                    &HoldReason::StorageRequestCreationHold.into(),
                    &storage_request_metadata.owner,
                    T::StorageRequestCreationDeposit::get(),
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
                    .zip(accepted_file_keys.file_keys_and_proofs)
                    .map(|(file_metadata, file_key_with_proof)| {
                        (
                            file_key_with_proof.file_key,
                            TrieAddMutation::new(file_metadata.encode()).into(),
                        )
                    })
                    .collect::<Vec<_>>()
                    .as_slice(),
                &accepted_file_keys.non_inclusion_forest_proof,
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
        let bsp_id =
            <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(sender.clone())
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

        expect_or_err!(
            storage_request_metadata.bsps_confirmed < storage_request_metadata.bsps_required,
            "Storage request should never have confirmed bsps equal to or greater than required bsps, since they are deleted when it is reached.",
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

        // Increment the number of bsps volunteered.
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
        let bsp_id =
            <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(sender.clone())
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
            );

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

            // Check if a payment stream between the user and provider already exists.
            // If it does not, create it. If it does, update it.
            match <T::PaymentStreams as PaymentStreamsInterface>::get_dynamic_rate_payment_stream_amount_provided(&bsp_id, &storage_request_metadata.owner) {
				Some(previous_amount_provided) => {
					// Update the payment stream.
                    let new_amount_provided = &previous_amount_provided.checked_add(&storage_request_metadata.size).ok_or(ArithmeticError::Overflow)?;
					<T::PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
						&bsp_id,
						&storage_request_metadata.owner,
						new_amount_provided,
					)?;
				},
				None => {
					// Create the payment stream.
					<T::PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
						&bsp_id,
						&storage_request_metadata.owner,
						&storage_request_metadata.size,
					)?;
				}
			}

            // Get the file metadata to insert into the Provider's trie under the file key.
            let file_metadata = storage_request_metadata.clone().to_file_metadata();
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
                    "Number of volunteered bsps for storage request should have been removed",
                    Error::<T>::UnexpectedNumberOfRemovedVolunteeredBsps,
                    bool
                );

                // Return the storage request creation deposit to the user
                T::Currency::release(
                    &HoldReason::StorageRequestCreationHold.into(),
                    &storage_request_metadata.owner,
                    T::StorageRequestCreationDeposit::get(),
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
    fn cleanup_storage_request(
        revoker: EitherAccountIdOrMspId<T>,
        file_key: MerkleHash<T>,
        storage_request_metadata: &StorageRequestMetadata<T>,
    ) -> DispatchResult {
        // Check if there are already BSPs who have confirmed to store the file.
        if storage_request_metadata.bsps_confirmed >= ReplicationTargetType::<T>::one() {
            // Apply Remove mutation of the file key to the BSPs that have confirmed storing the file (proofs of inclusion).
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::challenge_with_priority(
                &file_key, true,
            )?;

            // Emit event.
            Self::deposit_event(Event::PriorityChallengeForFileDeletionQueued {
                issuer: revoker,
                file_key,
            });
        }

        // Remove storage request bsps
        let removed = <StorageRequestBsps<T>>::drain_prefix(&file_key)
            .fold(0, |acc, _| acc.saturating_add(One::one()));

        // Make sure that the expected number of bsps were removed.
        expect_or_err!(
            storage_request_metadata.bsps_volunteered == removed.into(),
            "Number of volunteered bsps for storage request should have been removed",
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
            T::StorageRequestCreationDeposit::get(),
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
        size: StorageData<T>,
        can_serve: bool,
        inclusion_forest_proof: ForestProof<T>,
    ) -> Result<ProviderIdFor<T>, DispatchError> {
        let bsp_id =
            <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(sender.clone())
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
        );

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
                    // We need to decrement the number of bsps confirmed and volunteered, remove the BSP as a data server and from the storage request.
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
                    // We need to increment the number of bsps required.
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
                    Some(ReplicationTargetType::<T>::one()),
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
                tick_when_requested: frame_system::Pallet::<T>::block_number(),
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
        let bsp_id =
            <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(sender.clone())
                .ok_or(Error::<T>::NotASp)?;

        // Ensure the ID belongs to a BSP, not a MSP
        ensure!(
            <T::Providers as ReadStorageProvidersInterface>::is_bsp(&bsp_id),
            Error::<T>::NotABsp
        );

        // Get the block when the pending stop storing request of the BSP for the file key was opened.
        let PendingStopStoringRequest {
            tick_when_requested: block_when_opened,
            file_size,
            file_owner,
        } = <PendingStopStoringRequests<T>>::get(&bsp_id, &file_key)
            .ok_or(Error::<T>::PendingStopStoringRequestNotFound)?;

        // Check that enough time has passed since the pending stop storing request was opened.
        ensure!(
            frame_system::Pallet::<T>::block_number()
                >= block_when_opened.saturating_add(T::MinWaitForStopStoring::get()),
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

        // Update the payment stream between the user and the BSP. If the new amount provided is zero, delete it instead.
        let new_amount_provided = <T::PaymentStreams as PaymentStreamsInterface>::get_dynamic_rate_payment_stream_amount_provided(&bsp_id, &file_owner)
			.ok_or(Error::<T>::DynamicRatePaymentStreamNotFound)?
			.saturating_sub(file_size);
        if new_amount_provided == Zero::zero() {
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

        // If the new capacity used for this BSP is 0, stop its randomness cycle.
        if <T::Providers as ReadStorageProvidersInterface>::get_used_capacity(&bsp_id)
            == Zero::zero()
        {
            <T::CrRandomness as CommitRevealRandomnessInterface>::stop_randomness_cycle(&bsp_id)?;
        }

        // Remove the pending stop storing request from storage.
        <PendingStopStoringRequests<T>>::remove(&bsp_id, &file_key);

        if new_root == <T::Providers as shp_traits::ReadProvidersInterface>::get_default_root() {
            let used_capacity =
                <T::Providers as ReadStorageProvidersInterface>::get_used_capacity(&bsp_id);
            if used_capacity != Zero::zero() {
                // Emit event if we have inconsistency. We can later monitor for those.
                Self::deposit_event(Event::UsedCapacityShouldBeZero {
                    actual_used_capacity: used_capacity,
                });
            }

            // Stop the BSP's challenge and randomness cycles.
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::stop_challenge_cycle(&bsp_id)?;
            <T::CrRandomness as CommitRevealRandomnessInterface>::stop_randomness_cycle(&bsp_id)?;
        };

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
        size: StorageData<T>,
        inclusion_forest_proof: ForestProof<T>,
    ) -> Result<(ProviderIdFor<T>, MerkleHash<T>), DispatchError> {
        // Get the SP ID
        let sp_id =
            <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(sender.clone())
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
        );

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

    pub(crate) fn do_delete_file(
        sender: T::AccountId,
        bucket_id: BucketIdFor<T>,
        file_key: MerkleHash<T>,
        location: FileLocation<T>,
        fingerprint: Fingerprint<T>,
        size: StorageData<T>,
        maybe_inclusion_forest_proof: Option<ForestProof<T>>,
    ) -> Result<(bool, Option<ProviderIdFor<T>>), DispatchError> {
        // Compute the file key hash.
        let computed_file_key = Self::compute_file_key(
            sender.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

        // Check that the metadata corresponds to the expected file key.
        ensure!(
            file_key == computed_file_key,
            Error::<T>::InvalidFileKeyMetadata
        );

        // Check if sender is the owner of the bucket.
        ensure!(
            <T::Providers as ReadBucketsInterface>::is_bucket_owner(&sender, &bucket_id)?,
            Error::<T>::NotBucketOwner
        );

        let msp_id = <T::Providers as ReadBucketsInterface>::get_msp_of_bucket(&bucket_id)?;

        ensure!(
            msp_id.is_some(),
            Error::<T>::OperationNotAllowedWhileBucketIsNotStoredByMsp
        );

        let file_key_included = match maybe_inclusion_forest_proof {
            // If the user did not supply a proof of inclusion, queue a pending deletion file request.
            // This will leave a window of time for the MSP to provide the proof of (non-)inclusion. Until the MSP provides the proof, it is
            // removed from the privileged providers' list, which means it is not allowed to charge users.
            // If the proof is not provided within the TTL, the hook will queue a priority challenge to remove the file key from all the providers.
            None => {
                let pending_file_deletion_requests = <PendingFileDeletionRequests<T>>::get(&sender);

                // Check if the file key is already in the pending deletion requests.
                let pending_file_deletion_request = PendingFileDeletionRequest {
                    user: sender.clone(),
                    file_key,
                    bucket_id,
                    file_size: size,
                };
                ensure!(
                    !pending_file_deletion_requests.contains(&pending_file_deletion_request),
                    Error::<T>::FileKeyAlreadyPendingDeletion
                );

                // Add the file key to the pending deletion requests.
                PendingFileDeletionRequests::<T>::try_append(
                    &sender,
                    pending_file_deletion_request,
                )
                .map_err(|_| Error::<T>::MaxUserPendingDeletionRequestsReached)?;

                // If the bucket is stored by a MSP, remove the MSP from the privileged providers list.
                if let Some(msp_id) = msp_id {
                    <<T as crate::Config>::PaymentStreams as PaymentStreamsInterface>::remove_privileged_provider(&msp_id);
                }

                // Queue the expiration item.
                let expiration_item = ExpirationItem::PendingFileDeletionRequests(
                    FileDeletionRequestExpirationItem::<T> {
                        user: sender.clone(),
                        file_key,
                        bucket_id,
                        file_size: size,
                    },
                );
                Self::enqueue_expiration_item(expiration_item)?;

                false
            }
            // If the user supplied a proof of inclusion, verify the proof and queue a priority challenge to remove the file key from all the providers.
            Some(inclusion_forest_proof) => {
                // Get the root of the bucket.
                let bucket_root =
                    <T::Providers as shp_traits::ReadBucketsInterface>::get_root_bucket(&bucket_id)
                        .ok_or(Error::<T>::BucketNotFound)?;

                // Verify the proof of inclusion.
                let proven_keys =
                    <T::ProofDealer as shp_traits::ProofsDealerInterface>::verify_generic_forest_proof(
                        &bucket_root,
                        &[file_key],
                        &inclusion_forest_proof,
                    )?;

                // Ensure that the file key IS part of the owner's forest.
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
                    )?;

                // Update root of the Bucket.
                <T::Providers as shp_traits::MutateBucketsInterface>::change_root_bucket(
                    bucket_id, new_root,
                )?;

                // Decrease size of the bucket.
                <T::Providers as MutateBucketsInterface>::decrease_bucket_size(&bucket_id, size)?;

                // If the bucket has a MSP, decrease its used capacity.
                if let Some(msp_id) = msp_id {
                    <T::Providers as MutateStorageProvidersInterface>::decrease_capacity_used(
                        &msp_id, size,
                    )?;
                }

                // Initiate the priority challenge to remove the file key from all the providers.
                <T::ProofDealer as shp_traits::ProofsDealerInterface>::challenge_with_priority(
                    &file_key, true,
                )?;

                // Emit event.
                Self::deposit_event(Event::PriorityChallengeForFileDeletionQueued {
                    issuer: EitherAccountIdOrMspId::<T>::AccountId(sender.clone()),
                    file_key,
                });

                true
            }
        };

        Ok((file_key_included, msp_id))
    }

    pub(crate) fn do_pending_file_deletion_request_submit_proof(
        sender: T::AccountId,
        user: T::AccountId,
        file_key: MerkleHash<T>,
        file_size: StorageData<T>,
        bucket_id: BucketIdFor<T>,
        forest_proof: ForestProof<T>,
    ) -> Result<(bool, ProviderIdFor<T>), DispatchError> {
        let msp_id =
            <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(sender.clone())
                .ok_or(Error::<T>::NotAMsp)?;

        // Check that the provider is indeed an MSP.
        ensure!(
            <T::Providers as ReadStorageProvidersInterface>::is_msp(&msp_id),
            Error::<T>::NotAMsp
        );

        ensure!(
            <T::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(&msp_id, &bucket_id),
            Error::<T>::MspNotStoringBucket
        );

        let pending_file_deletion_requests = <PendingFileDeletionRequests<T>>::get(&user);

        // Check if the file key is in the pending deletion requests.
        let pending_file_deletion_request_to_prove = PendingFileDeletionRequest {
            user: user.clone(),
            file_key,
            bucket_id,
            file_size,
        };

        ensure!(
            pending_file_deletion_requests.contains(&pending_file_deletion_request_to_prove),
            Error::<T>::FileKeyNotPendingDeletion
        );

        // Get the root of the bucket.
        let bucket_root =
            <T::Providers as shp_traits::ReadBucketsInterface>::get_root_bucket(&bucket_id)
                .ok_or(Error::<T>::BucketNotFound)?;

        // Verify the proof of inclusion.
        let proven_keys =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::verify_generic_forest_proof(
                &bucket_root,
                &[file_key],
                &forest_proof,
            )?;

        let file_key_included = proven_keys.contains(&file_key);

        // If the file key was part of the forest, remove it from the forest and update the root of the bucket.
        if file_key_included {
            // Compute new root after removing file key from forest partial trie.
            let new_root =
                <T::ProofDealer as shp_traits::ProofsDealerInterface>::generic_apply_delta(
                    &bucket_root,
                    &[(file_key, TrieRemoveMutation::default().into())],
                    &forest_proof,
                )?;

            // Update root of the Bucket.
            <T::Providers as shp_traits::MutateBucketsInterface>::change_root_bucket(
                bucket_id, new_root,
            )?;

            // Decrease size of the bucket.
            <T::Providers as MutateBucketsInterface>::decrease_bucket_size(&bucket_id, file_size)?;

            // Decrease the used capacity of the MSP.
            <T::Providers as MutateStorageProvidersInterface>::decrease_capacity_used(
                &msp_id, file_size,
            )?;

            // Initiate the priority challenge to remove the file key from all the providers.
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::challenge_with_priority(
                &file_key, true,
            )?;

            // Emit event.
            Self::deposit_event(Event::PriorityChallengeForFileDeletionQueued {
                issuer: EitherAccountIdOrMspId::<T>::MspId(msp_id),
                file_key,
            });
        }

        // Delete the pending deletion request.
        <PendingFileDeletionRequests<T>>::mutate(&user, |requests| {
            requests.retain(|pending_file_deletion_request| {
                &pending_file_deletion_request.file_key != &file_key
            });
        });

        // Add back the MSP to the privileged providers, to allow it to charge users again.
        <<T as crate::Config>::PaymentStreams as PaymentStreamsInterface>::add_privileged_provider(
            &msp_id,
        );

        Ok((file_key_included, msp_id))
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
        size: StorageData<T>,
        fingerprint: Fingerprint<T>,
    ) -> MerkleHash<T> {
        shp_file_metadata::FileMetadata::<
            { shp_constants::H_LENGTH },
            { shp_constants::FILE_CHUNK_SIZE },
            { shp_constants::FILE_SIZE_TO_CHALLENGES },
        > {
            owner: owner.encode(),
            bucket_id: bucket_id.as_ref().to_vec(),
            location: location.clone().to_vec(),
            file_size: size.into(),
            fingerprint: fingerprint.as_ref().into(),
        }
        .file_key::<FileKeyHasher<T>>()
    }

    pub fn get_threshold_for_bsp_request(
        bsp_id: &ProviderIdFor<T>,
        fingerprint: &Fingerprint<T>,
    ) -> T::ThresholdType {
        // Concatenate the BSP ID and the fingerprint and hash them to get the volunteering hash.
        let concatenated = sp_std::vec![bsp_id.encode(), fingerprint.encode()].concat();
        let volunteering_hash =
            <<T as frame_system::Config>::Hashing as Hash>::hash(concatenated.as_ref());

        // Return the threshold needed for the BSP to be able to volunteer for the storage request.
        T::HashToThresholdType::convert(volunteering_hash)
    }

    /// Compute the threshold for a BSP to succeed.
    ///
    /// Succeeding this threshold is required for the BSP to be eligible to volunteer for a storage request.
    /// The threshold is computed based on the global reputation weight and the BSP's reputation weight, giving
    /// an advantage to BSPs with higher reputation weights.
    ///
    /// The formalized formulas are documented in the [README](https://github.com/Moonsong-Labs/storage-hub/blob/main/pallets/file-system/README.md#volunteering-succeeding-threshold-checks).
    pub fn compute_threshold_to_succeed(
        bsp_id: &ProviderIdFor<T>,
        requested_at: TickNumber<T>,
    ) -> Result<(T::ThresholdType, T::ThresholdType), DispatchError> {
        let maximum_threshold = T::ThresholdType::max_value();

        let global_weight =
            T::ThresholdType::from(T::Providers::get_global_bsps_reputation_weight());

        if global_weight == T::ThresholdType::zero() {
            return Err(Error::<T>::NoGlobalReputationWeightSet.into());
        }

        // Global threshold starting point from which all BSPs begin their threshold slope. All BSPs start at this point
        // with the starting reputation weight.
        //
        // The calculation is designed to achieve the following:
        //
        // 1. In a regular scenario, `maximum_threshold` would be very large, and you'd start bringing it down with
        //    `global_weight`, also a large number. That way, when you multiply it by the replication target,
        //    you should still be within the numerical domain.
        //
        // 2. If `global_weight` is low still (in the early days of the network), when multiplying with
        //    replication target, you'll get at most `u32::MAX` and then the threshold would be
        //    u32::MAX / 2 (still pretty high).
        //
        // 3. If maximum_threshold is very low (like sometimes set in tests), the division would saturate to 1,
        //    and then the threshold would be replication target / 2 (still very low).
        let threshold_global_starting_point = maximum_threshold
            .checked_div(&global_weight)
            .unwrap_or(T::ThresholdType::one())
            .checked_mul(&MaxReplicationTarget::<T>::get().into()).unwrap_or({
                log::warn!("Global starting point is beyond MaximumThreshold. Setting it to half of the MaximumThreshold.");
                maximum_threshold
            })
            .checked_div(&T::ThresholdType::from(2u32))
            .unwrap_or(T::ThresholdType::one());

        // Get the BSP's reputation weight.
        let bsp_weight = T::ThresholdType::from(T::Providers::get_bsp_reputation_weight(&bsp_id)?);

        // Actual BSP's threshold starting point, taking into account their reputation weight.
        let threshold_weighted_starting_point =
            bsp_weight.saturating_mul(threshold_global_starting_point);

        // Rate of increase from the weighted threshold starting point up to the maximum threshold within a tick range.
        let base_slope = maximum_threshold
            .saturating_sub(threshold_global_starting_point)
            .checked_div(&T::ThresholdTypeToTickNumber::convert_back(
                TickRangeToMaximumThreshold::<T>::get(),
            ))
            .unwrap_or(T::ThresholdType::one());

        let threshold_slope = base_slope
            .checked_mul(&bsp_weight)
            .unwrap_or(maximum_threshold);

        // Since checked_div only returns None on a result of zero, there is the case when the result is between 0 and 1 and rounds down to 0.
        let threshold_slope = if threshold_slope.is_zero() {
            T::ThresholdType::one()
        } else {
            threshold_slope
        };

        let current_tick_number =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();

        // Get number of ticks since the storage request was issued.
        let ticks_since_requested = current_tick_number.saturating_sub(requested_at);
        let ticks_since_requested =
            T::ThresholdTypeToTickNumber::convert_back(ticks_since_requested);

        let to_succeed = threshold_weighted_starting_point
            .saturating_add(threshold_slope.saturating_mul(ticks_since_requested));

        Ok((to_succeed, threshold_slope))
    }
}

mod hooks {
    use crate::{
        pallet,
        types::{MerkleHash, RejectedStorageRequestReason, ReplicationTargetType, TickNumber},
        utils::{
            BucketIdFor, EitherAccountIdOrMspId, FileDeletionRequestExpirationItem, ProviderIdFor,
        },
        weights::WeightInfo,
        BucketsWithStorageRequests, Event, FileDeletionRequestExpirations, HoldReason,
        MaxReplicationTarget, MoveBucketRequestExpirations, NextStartingTickToCleanUp, Pallet,
        PendingBucketsToMove, PendingFileDeletionRequests, PendingMoveBucketRequests,
        StorageRequestBsps, StorageRequestExpirations, StorageRequests,
    };
    use frame_support::traits::{fungible::MutateHold, tokens::Precision};
    use sp_runtime::{
        traits::{Get, One},
        Saturating,
    };
    use sp_weights::{RuntimeDbWeight, WeightMeter};

    impl<T: pallet::Config> Pallet<T> {
        pub(crate) fn do_on_poll(weight: &mut WeightMeter) {
            let current_data_price_per_giga_unit =
                <T::PaymentStreams as shp_traits::MutatePricePerGigaUnitPerTickInterface>::get_price_per_giga_unit_per_tick();

            let new_data_price_per_giga_unit =
                <T::UpdateStoragePrice as shp_traits::UpdateStoragePrice>::update_storage_price(
                    current_data_price_per_giga_unit,
                    <T::Providers as shp_traits::SystemMetricsInterface>::get_total_used_capacity(),
                    <T::Providers as shp_traits::SystemMetricsInterface>::get_total_capacity(),
                );

            if new_data_price_per_giga_unit != current_data_price_per_giga_unit {
                <T::PaymentStreams as shp_traits::MutatePricePerGigaUnitPerTickInterface>::set_price_per_giga_unit_per_tick(
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
            // and get the storage request expirations for the current tick, continue.
            if meter.can_consume(db_weight.reads_writes(2, 1)) {
                // Get the maximum amount of BSPs required for a storage request.
                // As of right now, the upper bound limit to the number of BSPs required to fulfill a storage request is set by `MaxReplicationTarget`.
                // We could increase this potential weight to account for potentially more volunteers.
                let max_bsp_required: u64 = MaxReplicationTarget::<T>::get().into();
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

            // Expired file deletion requests clean up section:
            // Note: this is unchanged since it has been deprecated and will be removed in another PR.
            if !meter.can_consume(db_weight.reads_writes(1, 1)) {
                return true;
            }

            let mut expired_file_deletion_requests =
                FileDeletionRequestExpirations::<T>::take(&tick);
            meter.consume(db_weight.reads_writes(1, 1));

            while let Some(expired_file_deletion_request) = expired_file_deletion_requests.pop() {
                Self::process_expired_pending_file_deletion(expired_file_deletion_request, meter);
            }

            if !expired_file_deletion_requests.is_empty() {
                FileDeletionRequestExpirations::<T>::insert(&tick, expired_file_deletion_requests);
                meter.consume(db_weight.writes(1));
            }

            // Expired move bucket requests clean up section:
            // If there's enough weight to get from storage the expired move bucket requests, continue.
            if meter.can_consume(db_weight.reads_writes(1, 1)) {
                // Get the expired move bucket requests for the current tick.
                let mut expired_move_bucket_requests =
                    MoveBucketRequestExpirations::<T>::take(&tick);
                meter.consume(db_weight.reads_writes(1, 1));

                // Get the required weight to process one expired move bucket request.
                let required_weight_processing_expired_move_bucket_request =
                    T::WeightInfo::process_expired_move_bucket_request();

                // While there's enough weight to process an expired move bucket request AND re-insert the remaining move bucket requests to storage, continue.
                while let Some((msp_id, bucket_id)) = expired_move_bucket_requests.pop() {
                    if meter.can_consume(
                        required_weight_processing_expired_move_bucket_request
                            .saturating_add(db_weight.writes(1)),
                    ) {
                        // Process an expired move bucket request. This internally consumes the used weight from the meter.
                        Self::process_expired_move_bucket_request(msp_id, bucket_id, meter);
                    } else {
                        // Push back the expired move bucket request into the move bucket requests queue to be able to re-insert it.
                        // This should never fail since this element was just taken from the bounded vector, so there must be space for it.
                        let _ = expired_move_bucket_requests.try_push((msp_id, bucket_id));
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
            // Remove storage request and all BSPs that volunteered for it.
            let storage_request_metadata = StorageRequests::<T>::take(&file_key);
            let amount_of_deleted_bsps = StorageRequestBsps::<T>::drain_prefix(&file_key)
                .fold(0u32, |acc, _| acc.saturating_add(One::one()));

            match storage_request_metadata {
                Some(storage_request_metadata) => match storage_request_metadata.msp {
                    None | Some((_, true)) => {
                        // If the request was originated by a request to stop storing from a BSP for a file that had no
                        // storage request open, or if the MSP has already accepted storing the file, treat the storage request
                        // as fulfilled with whatever amount of BSPs got to volunteer and confirm the file. For that:
                        // Return the storage request creation deposit to the user, emitting an error event if it fails
                        // but continuing execution.
                        let storage_request_creation_deposit =
                            T::StorageRequestCreationDeposit::get();
                        let _ = T::Currency::release(
                            &HoldReason::StorageRequestCreationHold.into(),
                            &storage_request_metadata.owner,
                            storage_request_creation_deposit,
                            Precision::BestEffort,
                        )
                        .map_err(|e| {
                            Self::deposit_event(
                                Event::FailedToReleaseStorageRequestCreationDeposit {
                                    file_key,
                                    owner: storage_request_metadata.owner.clone(),
                                    amount_to_return: storage_request_creation_deposit,
                                    error: e,
                                },
                            );
                        });

                        // Remove the storage request from the active storage requests for the bucket
                        <BucketsWithStorageRequests<T>>::remove(
                            &storage_request_metadata.bucket_id,
                            &file_key,
                        );

                        // Emit the StorageRequestExpired event
                        Self::deposit_event(Event::StorageRequestExpired { file_key });

                        // Consume the weight used.
                        meter.consume(
                            T::WeightInfo::process_expired_storage_request_msp_accepted_or_no_msp(
                                amount_of_deleted_bsps,
                            ),
                        );
                    }
                    Some((msp_id, false)) => {
                        // If the MSP did not accept the file in time, treat the storage request as rejected. For that:
                        // Check if there are already BSPs who have confirmed to store the file.
                        if storage_request_metadata.bsps_confirmed
                            >= ReplicationTargetType::<T>::one()
                        {
                            // If there are, queue up a priority challenge for the file key with a remove mutation, forcing all BSPs to delete the file.
                            // This can error if the priority challenge queue is full. In that case, we emit an error event and continue. The
                            // priority challenge then should be enqueued manually at a later time.
                            let _ = <T::ProofDealer as shp_traits::ProofsDealerInterface>::challenge_with_priority(
									&file_key,
									true,
								).map_err(|e| {
									Self::deposit_event(Event::FailedToQueuePriorityChallenge {
										file_key,
										error: e,
									});
								});

                            // Emit the event of the priority challenge being queued.
                            Self::deposit_event(Event::PriorityChallengeForFileDeletionQueued {
                                issuer: EitherAccountIdOrMspId::MspId(msp_id),
                                file_key,
                            });
                        }

                        // Return the storage request creation deposit to the user, emitting an error event if it fails
                        // but continuing execution.
                        let storage_request_creation_deposit =
                            T::StorageRequestCreationDeposit::get();
                        let _ = T::Currency::release(
                            &HoldReason::StorageRequestCreationHold.into(),
                            &storage_request_metadata.owner,
                            storage_request_creation_deposit,
                            Precision::BestEffort,
                        )
                        .map_err(|e| {
                            Self::deposit_event(
                                Event::FailedToReleaseStorageRequestCreationDeposit {
                                    file_key,
                                    owner: storage_request_metadata.owner.clone(),
                                    amount_to_return: storage_request_creation_deposit,
                                    error: e,
                                },
                            );
                        });

                        // Remove the storage request from the active storage requests for the bucket
                        <BucketsWithStorageRequests<T>>::remove(
                            &storage_request_metadata.bucket_id,
                            &file_key,
                        );

                        // Emit the StorageRequestRejected event
                        Self::deposit_event(Event::StorageRequestRejected {
                            file_key,
                            reason: RejectedStorageRequestReason::RequestExpired,
                        });

                        // Consume the weight used.
                        meter.consume(T::WeightInfo::process_expired_storage_request_msp_rejected(
                            amount_of_deleted_bsps,
                        ));
                    }
                },
                None => {
                    // This should never happen, since it would mean the storage request was deleted on
                    // its own but the expiration item wasn't removed from the queue. Do nothing since
                    // the storage request is already gone.
                }
            }
        }

        fn process_expired_pending_file_deletion(
            expired_file_deletion_request: FileDeletionRequestExpirationItem<T>,
            meter: &mut WeightMeter,
        ) {
            let db_weight = T::DbWeight::get();
            let potential_weight = db_weight.reads_writes(2, 3);

            if !meter.can_consume(potential_weight) {
                return;
            }

            let requests =
                PendingFileDeletionRequests::<T>::get(&expired_file_deletion_request.user);

            // Check if the file key is still a pending deletion requests.
            let expired_item_index =
                match requests.iter().position(|pending_file_deletion_request| {
                    &pending_file_deletion_request.file_key
                        == &expired_file_deletion_request.file_key
                }) {
                    Some(i) => i,
                    None => return,
                };

            // Remove the file key from the pending deletion requests.
            PendingFileDeletionRequests::<T>::mutate(
                &expired_file_deletion_request.user,
                |requests| {
                    requests.remove(expired_item_index);
                },
            );

            let user = expired_file_deletion_request.user.clone();

            // Attempt to decrease the bucket size while also reducing the fixed rate payment stream between the user and the MSP
            if let Err(e) =
                <T::Providers as shp_traits::MutateBucketsInterface>::decrease_bucket_size(
                    &expired_file_deletion_request.bucket_id,
                    expired_file_deletion_request.file_size,
                )
            {
                Self::deposit_event(Event::FailedToDecreaseBucketSize {
                    user: user.clone(),
                    bucket_id: expired_file_deletion_request.bucket_id,
                    file_key: expired_file_deletion_request.file_key,
                    file_size: expired_file_deletion_request.file_size,
                    error: e,
                });

                if !<T::Providers as shp_traits::ReadBucketsInterface>::bucket_exists(
                    &expired_file_deletion_request.bucket_id,
                ) {
                    // Skip expired file deletion request if the bucket does not exist.
                    return;
                }
            }

            // Decrease the used capacity of the MSP.
            if let Ok(Some(msp_id)) =
                <T::Providers as shp_traits::ReadBucketsInterface>::get_msp_of_bucket(
                    &expired_file_deletion_request.bucket_id,
                )
                .map_err(|e| {
                    Self::deposit_event(Event::FailedToGetMspOfBucket {
                        bucket_id: expired_file_deletion_request.bucket_id,
                        error: e,
                    });
                })
            {
                let _ = <T::Providers as shp_traits::MutateStorageProvidersInterface>::decrease_capacity_used(
					&msp_id,
					expired_file_deletion_request.file_size,
				)
				.map_err(|e| {
					Self::deposit_event(Event::FailedToDecreaseMspUsedCapacity {
						user: user.clone(),
						msp_id: msp_id.clone(),
						file_key: expired_file_deletion_request.file_key,
						file_size: expired_file_deletion_request.file_size,
						error: e,
					});
				});
            }

            // Queue a priority challenge to remove the file key from all the providers.
            let _ = <T::ProofDealer as shp_traits::ProofsDealerInterface>::challenge_with_priority(
                &expired_file_deletion_request.file_key,
                true,
            )
            .map_err(|e| {
                Self::deposit_event(Event::FailedToQueuePriorityChallenge {
                    file_key: expired_file_deletion_request.file_key,
                    error: e,
                });
            });

            Self::deposit_event(Event::PriorityChallengeForFileDeletionQueued {
                issuer: EitherAccountIdOrMspId::<T>::AccountId(user.clone()),
                file_key: expired_file_deletion_request.file_key,
            });

            meter.consume(potential_weight);
        }

        pub(crate) fn process_expired_move_bucket_request(
            msp_id: ProviderIdFor<T>,
            bucket_id: BucketIdFor<T>,
            meter: &mut WeightMeter,
        ) {
            // Remove from storage the pending move bucket request.
            PendingMoveBucketRequests::<T>::remove(&msp_id, &bucket_id);
            PendingBucketsToMove::<T>::remove(&bucket_id);

            // Deposit the event of the expired move bucket request.
            Self::deposit_event(Event::MoveBucketRequestExpired { msp_id, bucket_id });

            // Consume the weight used by this function.
            meter.consume(T::WeightInfo::process_expired_move_bucket_request());
        }
    }
}
