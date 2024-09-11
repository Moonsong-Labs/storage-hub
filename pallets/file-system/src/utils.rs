use codec::Encode;
use frame_support::{
    ensure,
    pallet_prelude::DispatchResult,
    traits::{nonfungibles_v2::Create, Get},
};
use frame_system::pallet_prelude::BlockNumberFor;
use num_bigint::BigUint;
use sp_runtime::{
    traits::{
        Bounded, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Convert, ConvertBack, Hash, One,
        Saturating, Zero,
    },
    ArithmeticError, BoundedVec, DispatchError,
};
use sp_std::{collections::btree_set::BTreeSet, vec, vec::Vec};

use pallet_file_system_runtime_api::{
    QueryBspConfirmChunksToProveForFileError, QueryFileEarliestVolunteerBlockError,
};
use pallet_nfts::{CollectionConfig, CollectionSettings, ItemSettings, MintSettings, MintType};
use shp_file_metadata::ChunkId;
use shp_traits::{
    MutateBucketsInterface, MutateStorageProvidersInterface, ReadBucketsInterface,
    ReadProvidersInterface, ReadStorageProvidersInterface, ReadUserSolvencyInterface,
    TrieAddMutation, TrieRemoveMutation,
};

use crate::{
    pallet,
    types::{
        BucketIdFor, BucketNameFor, CollectionConfigFor, CollectionIdFor, ExpirationItem,
        FileKeyHasher, FileLocation, Fingerprint, ForestProof, KeyProof, MaxBspsPerStorageRequest,
        MerkleHash, MultiAddresses, PeerIds, ProviderIdFor, ReplicationTargetType, StorageData,
        StorageRequestBspsMetadata, StorageRequestMetadata,
    },
    BlockRangeToMaximumThreshold, Error, Event, Pallet, PendingFileDeletionRequests,
    PendingMoveBucketRequests, PendingStopStoringRequests, ReplicationTarget, StorageRequestBsps,
    StorageRequests,
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
    /// Compute the block number at which the BSP is eligible to volunteer for a storage request.
    pub fn query_earliest_file_volunteer_block(
        bsp_id: ProviderIdFor<T>,
        file_key: MerkleHash<T>,
    ) -> Result<BlockNumberFor<T>, QueryFileEarliestVolunteerBlockError>
    where
        T: frame_system::Config,
    {
        // Get the block number at which the storage request was created.
        let (storage_request_block, fingerprint) = match <StorageRequests<T>>::get(&file_key) {
            Some(storage_request) => (storage_request.requested_at, storage_request.fingerprint),
            None => {
                return Err(QueryFileEarliestVolunteerBlockError::StorageRequestNotFound);
            }
        };

        // Get the threshold needed for the BSP to be able to volunteer for the storage request.
        let bsp_threshold = Self::get_threshold_for_bsp_request(&bsp_id, &fingerprint);

        // Compute the block number at which the BSP should send the volunteer request.
        Self::compute_volunteer_block_number(bsp_id, bsp_threshold, storage_request_block)
            .map_err(|_| QueryFileEarliestVolunteerBlockError::ThresholdArithmeticError)
    }

    fn compute_volunteer_block_number(
        bsp_id: ProviderIdFor<T>,
        bsp_threshold: T::ThresholdType,
        storage_request_block: BlockNumberFor<T>,
    ) -> Result<BlockNumberFor<T>, DispatchError>
    where
        T: frame_system::Config,
    {
        // Compute the threshold to succeed and the slope of the bsp.
        let (to_succeed, slope) =
            Self::compute_threshold_to_succeed(&bsp_id, storage_request_block)?;

        let threshold_diff = match bsp_threshold.checked_sub(&to_succeed) {
            Some(diff) => diff,
            None => {
                // The BSP's threshold is less than the current threshold.
                return Ok(<frame_system::Pallet<T>>::block_number());
            }
        };

        // Calculate the number of blocks required to be below the threshold.
        let blocks_to_wait = match threshold_diff.checked_div(&slope) {
            Some(blocks) => blocks,
            None => {
                return Err(Error::<T>::ThresholdArithmeticError.into());
            }
        };

        // Compute the block number at which the BSP should send the volunteer request.
        let volunteer_block_number = storage_request_block
            .saturating_add(T::ThresholdTypeToBlockNumber::convert(blocks_to_wait));

        Ok(volunteer_block_number)
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

        // Generate the list of chunks to prove.
        let challenges = Self::generate_chunk_challenges_on_sp_confirm(
            bsp_id,
            file_key,
            &storage_request_metadata,
        );

        let chunks = storage_request_metadata.to_file_metadata().chunks_count();

        let chunks_to_prove = challenges
            .iter()
            .map(|challenge| {
                let challenged_chunk = BigUint::from_bytes_be(challenge.as_ref()) % chunks;
                let challenged_chunk: ChunkId = ChunkId::new(
                    challenged_chunk
                        .try_into()
                        .map_err(|_| QueryBspConfirmChunksToProveForFileError::InternalError)?,
                );

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
                chunks_to_check - 1,
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
    ) -> Result<(BucketIdFor<T>, Option<CollectionIdFor<T>>), DispatchError> {
        // TODO: Hold user funds for the bucket creation.

        // Check if the MSP is indeed an MSP.
        ensure!(
            <T::Providers as ReadStorageProvidersInterface>::is_msp(&msp_id),
            Error::<T>::NotAMsp
        );

        // Create collection only if bucket is private
        let maybe_collection_id = if private {
            // The `owner` of the collection is also the admin of the collection since most operations require the sender to be the admin.
            Some(Self::create_collection(sender.clone())?)
        } else {
            None
        };

        let bucket_id = <T as crate::Config>::Providers::derive_bucket_id(&msp_id, &sender, name);

        <T::Providers as MutateBucketsInterface>::add_bucket(
            msp_id,
            sender,
            bucket_id,
            private,
            maybe_collection_id.clone(),
        )?;

        Ok((bucket_id, maybe_collection_id))
    }

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

        // Check if the bucket is already stored by the new MSP.
        ensure!(
            <T::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(
                &new_msp_id,
                &bucket_id
            ),
            Error::<T>::MspAlreadyStoringBucket
        );

        // Register the move bucket request.
        <PendingMoveBucketRequests<T>>::insert(&new_msp_id, bucket_id, sender);

        let expiration_item = ExpirationItem::MoveBucketRequest((new_msp_id, bucket_id));
        Self::enqueue_expiration_item(expiration_item)?;

        Ok(())
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
            // Handle case where the bucket has an existing collection.
            (_, Some(current_collection_id))
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
    /// *Callable only by the owner of the bucket. The bucket must be private.*
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
        bsps_required: Option<ReplicationTargetType<T>>,
        user_peer_ids: Option<PeerIds<T>>,
        data_server_sps: BoundedVec<ProviderIdFor<T>, MaxBspsPerStorageRequest<T>>,
    ) -> Result<MerkleHash<T>, DispatchError> {
        // TODO: Check user funds and lock them for the storage request.
        // TODO: Return error if the file is already stored and overwrite is false.

        // Check that the file size is greater than zero.
        ensure!(size > Zero::zero(), Error::<T>::FileSizeCannotBeZero);

        // Check that a bucket under the received ID exists and that the sender is the owner of the bucket.
        ensure!(
            <T::Providers as ReadBucketsInterface>::is_bucket_owner(&sender, &bucket_id)?,
            Error::<T>::NotBucketOwner
        );

        // If a specific MSP ID is provided, check that it is a valid MSP and that it has enough available capacity to store the file.
        let msp = if let Some(ref msp_id) = msp_id {
            // Check that the received Provider ID corresponds to a valid MSP.
            ensure!(
                <T::Providers as ReadStorageProvidersInterface>::is_msp(msp_id),
                Error::<T>::NotAMsp
            );

            // Check that the MSP has enough available capacity to store the file.
            ensure!(
                <T::Providers as ReadStorageProvidersInterface>::available_capacity(msp_id) >= size,
                Error::<T>::InsufficientAvailableCapacity
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

        let bsps_required = bsps_required.unwrap_or(ReplicationTarget::<T>::get());

        if bsps_required.is_zero() {
            return Err(Error::<T>::ReplicationTargetCannotBeZero)?;
        }

        if bsps_required > MaxBspsPerStorageRequest::<T>::get().into() {
            return Err(Error::<T>::BspsRequiredExceedsMax)?;
        }

        let storage_request_metadata = StorageRequestMetadata::<T> {
            requested_at: <frame_system::Pallet<T>>::block_number(),
            owner: sender.clone(),
            bucket_id,
            location: location.clone(),
            fingerprint,
            size,
            msp,
            user_peer_ids: user_peer_ids.unwrap_or_default(),
            data_server_sps,
            bsps_required,
            bsps_confirmed: ReplicationTargetType::<T>::zero(),
            bsps_volunteered: ReplicationTargetType::<T>::zero(),
        };

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

        // Register storage request.
        <StorageRequests<T>>::insert(&file_key, storage_request_metadata);

        let expiration_item = ExpirationItem::StorageRequest(file_key);
        Self::enqueue_expiration_item(expiration_item)?;

        Ok(file_key)
    }

    pub(crate) fn do_msp_accept_storage_request(
        sender: T::AccountId,
        file_key: MerkleHash<T>,
        file_proof: KeyProof<T>,
        non_inclusion_forest_proof: ForestProof<T>,
    ) -> Result<(ProviderIdFor<T>, MerkleHash<T>, StorageRequestMetadata<T>), DispatchError> {
        // Check that the storage request exists for the file key.
        let mut storage_request_metadata =
            <StorageRequests<T>>::get(&file_key).ok_or(Error::<T>::StorageRequestNotFound)?;

        // Check that the sender is a Storage Provider and get its SP ID
        let sp_id =
            <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(sender.clone())
                .ok_or(Error::<T>::NotASp)?;

        // Check that the sender is a MSP.
        ensure!(
            <T::Providers as ReadStorageProvidersInterface>::is_msp(&sp_id),
            Error::<T>::NotAMsp
        );

        // Check that the sender corresponds to the MSP in the storage request and that it hasn't yet confirmed storing the file.
        let (request_msp_id, confirm_status) = storage_request_metadata
            .msp
            .clone()
            .ok_or(Error::<T>::RequestWithoutMsp)?;
        ensure!(request_msp_id == sp_id, Error::<T>::NotSelectedMsp);
        ensure!(confirm_status == false, Error::<T>::MspAlreadyConfirmed);

        // Get the bucket ID from the storage request metadata
        let bucket_id = storage_request_metadata.bucket_id;

        // Check that the MSP is the one storing the bucket.
        ensure!(
            <T::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(&sp_id, &bucket_id),
            Error::<T>::MspNotStoringBucket
        );

        // Check that the MSP still has enough available capacity to store the file.
        ensure!(
            <T::Providers as ReadStorageProvidersInterface>::available_capacity(&sp_id)
                >= storage_request_metadata.size,
            Error::<T>::InsufficientAvailableCapacity
        );

        // Get the current root of the bucket where the file will be stored.
        let bucket_root = expect_or_err!(
            <T::Providers as shp_traits::ReadBucketsInterface>::get_root_bucket(&bucket_id),
            "Failed to get root for bucket, when it was already checked to exist",
            Error::<T>::BucketNotFound
        );

        // Verify the proof of non-inclusion.
        let proven_keys: BTreeSet<MerkleHash<T>> =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::verify_generic_forest_proof(
                &bucket_root,
                &[file_key],
                &non_inclusion_forest_proof,
            )?;

        // Ensure that the file key IS NOT part of the bucket's forest.
        ensure!(
            !proven_keys.contains(&file_key),
            Error::<T>::ExpectedNonInclusionProof
        );

        // Generate the challenges to verify that the file proof is valid.
        let chunk_challenges = Self::generate_chunk_challenges_on_sp_confirm(
            sp_id,
            file_key,
            &storage_request_metadata,
        );

        // Check that the key proof is valid.
        <T::ProofDealer as shp_traits::ProofsDealerInterface>::verify_key_proof(
            &file_key,
            &chunk_challenges,
            &file_proof,
        )?;

        // Compute the new bucket root after inserting new file key in its forest partial trie.
        let new_bucket_root =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::generic_apply_delta(
                &bucket_root,
                &[(file_key, TrieAddMutation::default().into())],
                &non_inclusion_forest_proof,
            )?;

        // Update root of the bucket.
        <T::Providers as shp_traits::MutateBucketsInterface>::change_root_bucket(
            bucket_id,
            new_bucket_root,
        )?;

        // Increase the used capacity of the MSP
        <T::Providers as MutateStorageProvidersInterface>::increase_capacity_used(
            &sp_id,
            storage_request_metadata.size,
        )?;

        // Set as confirmed the MSP in the storage request metadata.
        storage_request_metadata.msp = Some((request_msp_id, true));

        // Update storage request metadata.
        <StorageRequests<T>>::set(&file_key, Some(storage_request_metadata.clone()));

        Ok((sp_id, new_bucket_root, storage_request_metadata))
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

        // Get the threshold needed for the BSP to be able to volunteer for the storage request.
        let bsp_threshold =
            Self::get_threshold_for_bsp_request(&bsp_id, &storage_request_metadata.fingerprint);

        // Compute threshold for BSP to succeed.
        let (to_succeed, _slope) =
            Self::compute_threshold_to_succeed(&bsp_id, storage_request_metadata.requested_at)?;

        // Check that the BSP's threshold is under the threshold required to volunteer for the storage request.
        ensure!(bsp_threshold <= to_succeed, Error::<T>::AboveThreshold);

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
        file_keys_and_proofs: BoundedVec<
            (MerkleHash<T>, KeyProof<T>),
            T::MaxBatchConfirmStorageRequests,
        >,
    ) -> DispatchResult {
        let bsp_id =
            <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(sender.clone())
                .ok_or(Error::<T>::NotABsp)?;

        // Check that the provider is indeed a BSP.
        ensure!(
            <T::Providers as ReadStorageProvidersInterface>::is_bsp(&bsp_id),
            Error::<T>::NotABsp
        );

        let file_keys = file_keys_and_proofs
            .iter()
            .map(|(fk, _)| *fk)
            .collect::<Vec<_>>();

        // Verify the proof of non-inclusion.
        let proven_keys: BTreeSet<MerkleHash<T>> =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::verify_forest_proof(
                &bsp_id,
                file_keys.as_slice(),
                &non_inclusion_forest_proof,
            )?;

        let mut seen_keys = BTreeSet::new();
        for file_key in file_keys_and_proofs.iter() {
            // Skip any duplicates.
            if !seen_keys.insert(file_key.0) {
                continue;
            }

            // Check that the storage request exists.
            let mut storage_request_metadata =
                <StorageRequests<T>>::get(&file_key.0).ok_or(Error::<T>::StorageRequestNotFound)?;

            expect_or_err!(
                storage_request_metadata.bsps_confirmed < storage_request_metadata.bsps_required,
                "Storage request should never have confirmed bsps equal to or greater than required bsps, since they are deleted when it is reached.",
                Error::<T>::StorageRequestBspsRequiredFulfilled,
                bool
            );

            // Check that the BSP has volunteered for the storage request.
            ensure!(
                <StorageRequestBsps<T>>::contains_key(&file_key.0, &bsp_id),
                Error::<T>::BspNotVolunteered
            );

            let requests = expect_or_err!(
                <StorageRequestBsps<T>>::get(&file_key.0, &bsp_id),
                "BSP should exist since we checked it above",
                Error::<T>::ImpossibleFailedToGetValue
            );

            // Check that the storage provider has not already confirmed storing the file.
            ensure!(!requests.confirmed, Error::<T>::BspAlreadyConfirmed);

            // Check that the number of confirmed bsps is less than the required bsps and increment it.
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

            // Increment the number of bsps confirmed.
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
                !proven_keys.contains(&file_key.0),
                Error::<T>::ExpectedNonInclusionProof
            );

            let chunk_challenges = Self::generate_chunk_challenges_on_sp_confirm(
                bsp_id,
                file_key.0,
                &storage_request_metadata,
            );

            // Check that the key proof is valid.
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::verify_key_proof(
                &file_key.0,
                &chunk_challenges,
                &file_key.1,
            )?;

            // Add data to storage provider.
            <T::Providers as MutateStorageProvidersInterface>::increase_capacity_used(
                &bsp_id,
                storage_request_metadata.size,
            )?;

            // Remove storage request if we reached the required number of bsps.
            if storage_request_metadata.bsps_confirmed == storage_request_metadata.bsps_required {
                // TODO: we should only delete if the MSP also confirmed to store the file (this is not implemented yet).
                // Remove storage request metadata.
                <StorageRequests<T>>::remove(&file_key.0);

                // Remove storage request bsps
                let removed =
                    <StorageRequestBsps<T>>::drain_prefix(&file_key.0).fold(0, |acc, _| acc + 1);

                // Make sure that the expected number of bsps were removed.
                expect_or_err!(
                    storage_request_metadata.bsps_volunteered == removed.into(),
                    "Number of volunteered bsps for storage request should have been removed",
                    Error::<T>::UnexpectedNumberOfRemovedVolunteeredBsps,
                    bool
                );

                // Notify that the storage request has been fulfilled.
                Self::deposit_event(Event::StorageRequestFulfilled {
                    file_key: file_key.0,
                });
            } else {
                // Update storage request metadata.
                <StorageRequests<T>>::set(&file_key.0, Some(storage_request_metadata.clone()));

                // Update bsp for storage request.
                <StorageRequestBsps<T>>::mutate(&file_key.0, &bsp_id, |bsp| {
                    if let Some(bsp) = bsp {
                        bsp.confirmed = true;
                    }
                });
            }
        }

        // Check if this is the first file added to the BSP's Forest. If so, initialise last block proven by this BSP.
        let old_root = expect_or_err!(
            <T::Providers as shp_traits::ReadProvidersInterface>::get_root(bsp_id),
            "Failed to get root for BSP, when it was already checked to be a BSP",
            Error::<T>::NotABsp
        );

        if old_root == <T::Providers as shp_traits::ReadProvidersInterface>::get_default_root() {
            // This means that this is the first file added to the BSP's Forest.
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::initialise_challenge_cycle(
                &bsp_id,
            )?;

            // Emit the corresponding event.
            Self::deposit_event(Event::<T>::BspChallengeCycleInitialised {
                who: sender.clone(),
                bsp_id,
            });
        }

        // Compute new root after inserting new file keys in forest partial trie.
        let new_root = <T::ProofDealer as shp_traits::ProofsDealerInterface>::apply_delta(
            &bsp_id,
            file_keys
                .iter()
                .map(|fk| (*fk, TrieAddMutation::default().into()))
                .collect::<Vec<_>>()
                .as_slice(),
            &non_inclusion_forest_proof,
        )?;

        // Update root of BSP.
        <T::Providers as shp_traits::MutateProvidersInterface>::update_root(bsp_id, new_root)?;

        // Emit event.
        Self::deposit_event(Event::BspConfirmedStoring {
            who: sender,
            bsp_id,
            file_keys: file_keys.to_vec().try_into().unwrap(),
            new_root,
        });

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

        // Check that the sender is the same as the one who requested the storage.
        ensure!(
            storage_request_metadata.owner == sender,
            Error::<T>::StorageRequestNotAuthorized
        );

        // Check if there are already BSPs who have confirmed to store the file.
        if storage_request_metadata.bsps_confirmed >= ReplicationTargetType::<T>::one() {
            // Apply Remove mutation of the file key to the BSPs that have confirmed storing the file (proofs of inclusion).
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::challenge_with_priority(
                &file_key,
                Some(TrieRemoveMutation),
            )?;

            // Emit event.
            Self::deposit_event(Event::PriorityChallengeForFileDeletionQueued {
                user: sender,
                file_key,
            });
        }

        // Remove storage request bsps
        let removed = <StorageRequestBsps<T>>::drain_prefix(&file_key).fold(0, |acc, _| acc + 1);

        // Make sure that the expected number of bsps were removed.
        expect_or_err!(
            storage_request_metadata.bsps_volunteered == removed.into(),
            "Number of volunteered bsps for storage request should have been removed",
            Error::<T>::UnexpectedNumberOfRemovedVolunteeredBsps,
            bool
        );

        // Remove storage request.
        <StorageRequests<T>>::remove(&file_key);

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

        // TODO: charge SP for this action.

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
                    // We need to decrement the number of bsps confirmed and volunteered and remove the BSP from the storage request.
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
            // We need to create a new storage request with a single bsp required.
            None => {
                Self::do_request_storage(
                    owner,
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    None,
                    Some(ReplicationTargetType::<T>::one()),
                    None,
                    if can_serve {
                        BoundedVec::try_from(vec![bsp_id]).unwrap()
                    } else {
                        BoundedVec::default()
                    },
                )?;
            }
        };

        // Add the pending stop storing request to storage.
        <PendingStopStoringRequests<T>>::insert(
            &bsp_id,
            &file_key,
            (frame_system::Pallet::<T>::block_number(), size),
        );

        Ok(bsp_id)
    }

    pub(crate) fn do_bsp_confirm_stop_storing(
        sender: T::AccountId,
        file_key: MerkleHash<T>,
        inclusion_forest_proof: ForestProof<T>,
    ) -> Result<(ProviderIdFor<T>, MerkleHash<T>), DispatchError> {
        // Get the BSP ID of the sender
        let bsp_id =
            <T::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(sender.clone())
                .ok_or(Error::<T>::NotABsp)?;

        // Get the block when the pending stop storing request of the BSP for the file key was opened.
        let (block_when_opened, file_size) =
            <PendingStopStoringRequests<T>>::get(&bsp_id, &file_key)
                .ok_or(Error::<T>::PendingStopStoringRequestNotFound)?;

        // Check that enough time has passed since the pending stop storing request was opened.
        ensure!(
            frame_system::Pallet::<T>::block_number()
                >= block_when_opened + T::MinWaitForStopStoring::get(),
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

        // Remove the pending stop storing request from storage.
        <PendingStopStoringRequests<T>>::remove(&bsp_id, &file_key);

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

        // Check that the owner of the file has been flagged as insolvent
        ensure!(
            <T::UserSolvency as ReadUserSolvencyInterface>::is_user_insolvent(&owner),
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
        let proven_keys = if <T::Providers as ReadStorageProvidersInterface>::is_bsp(&sp_id) {
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::verify_forest_proof(
                &sp_id,
                &[file_key],
                &inclusion_forest_proof,
            )?
        } else {
            // If the Provider is a MSP, the proof is verified against the Bucket's root.

            // Check that the Bucket is stored by the MSP
            ensure!(
                <T::Providers as shp_traits::ReadBucketsInterface>::is_bucket_stored_by_msp(
                    &sp_id, &bucket_id
                ),
                Error::<T>::MspNotStoringBucket
            );

            // Get the Bucket's root
            let bucket_root =
                <T::Providers as shp_traits::ReadBucketsInterface>::get_root_bucket(&bucket_id)
                    .ok_or(Error::<T>::BucketNotFound)?;

            <T::ProofDealer as shp_traits::ProofsDealerInterface>::verify_generic_forest_proof(
                &bucket_root,
                &[file_key],
                &inclusion_forest_proof,
            )?
        };

        // Ensure that the file key IS part of the SP's forest.
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

        // Update root of SP.
        <T::Providers as shp_traits::MutateProvidersInterface>::update_root(sp_id, new_root)?;

        // Decrease data used by the SP.
        <T::Providers as MutateStorageProvidersInterface>::decrease_capacity_used(&sp_id, size)?;

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
    ) -> Result<(bool, ProviderIdFor<T>), DispatchError> {
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

        let msp_id = <T::Providers as ReadBucketsInterface>::get_msp_of_bucket(&bucket_id)
            .ok_or(Error::<T>::BucketNotFound)?;

        let file_key_included = match maybe_inclusion_forest_proof {
            // If the user did not supply a proof of inclusion, queue a pending deletion file request.
            // This will leave a window of time for the MSP to provide the proof of (non-)inclusion.
            // If the proof is not provided within the TTL, the hook will queue a priority challenge to remove the file key from all the providers.
            None => {
                let pending_file_deletion_requests = <PendingFileDeletionRequests<T>>::get(&sender);

                // Check if the file key is already in the pending deletion requests.
                ensure!(
                    !pending_file_deletion_requests.contains(&(file_key, bucket_id)),
                    Error::<T>::FileKeyAlreadyPendingDeletion
                );

                // Add the file key to the pending deletion requests.
                PendingFileDeletionRequests::<T>::try_append(&sender, (file_key, bucket_id))
                    .map_err(|_| Error::<T>::MaxUserPendingDeletionRequestsReached)?;

                // Queue the expiration item.
                let expiration_item =
                    ExpirationItem::PendingFileDeletionRequests((sender, file_key));
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

                // Initiate the priority challenge to remove the file key from all the providers.
                <T::ProofDealer as shp_traits::ProofsDealerInterface>::challenge_with_priority(
                    &file_key,
                    Some(TrieRemoveMutation),
                )?;

                // Emit event.
                Self::deposit_event(Event::PriorityChallengeForFileDeletionQueued {
                    user: sender,
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
        ensure!(
            pending_file_deletion_requests.contains(&(file_key, bucket_id)),
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

        if file_key_included {
            // Initiate the priority challenge to remove the file key from all the providers.
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::challenge_with_priority(
                &file_key,
                Some(TrieRemoveMutation),
            )?;

            // Emit event.
            Self::deposit_event(Event::PriorityChallengeForFileDeletionQueued {
                user: user.clone(),
                file_key,
            });
        }

        // Delete the pending deletion request.
        <PendingFileDeletionRequests<T>>::mutate(&user, |requests| {
            requests.retain(|(key, _)| key != &file_key);
        });

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

    /// Compute the next block number to insert an expiring item, and insert it in the corresponding expiration queue.
    ///
    /// This function attempts to insert a the expiration item at the next available block starting from
    /// the current next available block.
    pub(crate) fn enqueue_expiration_item(
        expiration_item: ExpirationItem<T>,
    ) -> Result<BlockNumberFor<T>, DispatchError> {
        let expiration_block = expiration_item.get_next_expiration_block();
        let new_expiration_block = expiration_item.try_append(expiration_block)?;
        expiration_item.set_next_expiration_block(new_expiration_block);

        Ok(new_expiration_block)
    }

    pub fn compute_file_key(
        owner: T::AccountId,
        bucket_id: BucketIdFor<T>,
        location: FileLocation<T>,
        size: StorageData<T>,
        fingerprint: Fingerprint<T>,
    ) -> MerkleHash<T> {
        let size: u32 = size.into();

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
        requested_at: BlockNumberFor<T>,
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
            .checked_mul(&ReplicationTarget::<T>::get().into()).unwrap_or({
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

        // Rate of increase from the weighted threshold starting point up to the maximum threshold within a block range.
        let threshold_slope = maximum_threshold
            .saturating_sub(threshold_weighted_starting_point)
            .checked_div(&T::ThresholdTypeToBlockNumber::convert_back(
                BlockRangeToMaximumThreshold::<T>::get(),
            ))
            .unwrap_or(T::ThresholdType::one());

        // Since checked_div only returns None on a result of zero, there is the case when the result is between 0 and 1 and rounds down to 0.
        let threshold_slope = if threshold_slope.is_zero() {
            T::ThresholdType::one()
        } else {
            threshold_slope
        };

        let current_block_number = <frame_system::Pallet<T>>::block_number();

        // Get number of blocks since the storage request was issued.
        let blocks_since_requested = current_block_number.saturating_sub(requested_at);
        let blocks_since_requested =
            T::ThresholdTypeToBlockNumber::convert_back(blocks_since_requested);

        let to_succeed = threshold_weighted_starting_point
            .saturating_add(threshold_slope.saturating_mul(blocks_since_requested));

        Ok((to_succeed, threshold_slope))
    }
}

mod hooks {
    use crate::MoveBucketRequestExpirations;
    use crate::{
        pallet,
        types::MerkleHash,
        utils::{BucketIdFor, ProviderIdFor},
        Event, FileDeletionRequestExpirations, NextStartingBlockToCleanUp, Pallet,
        PendingFileDeletionRequests, PendingMoveBucketRequests, ReplicationTarget,
        StorageRequestBsps, StorageRequestExpirations, StorageRequests,
    };
    use frame_support::weights::Weight;
    use frame_system::pallet_prelude::BlockNumberFor;
    use shp_traits::TrieRemoveMutation;
    use sp_runtime::traits::{Get, One, Zero};
    use sp_runtime::Saturating;

    impl<T: pallet::Config> Pallet<T> {
        pub(crate) fn do_on_idle(
            current_block: BlockNumberFor<T>,
            mut remaining_weight: &mut Weight,
        ) -> &mut Weight {
            let db_weight = T::DbWeight::get();
            let mut block_to_clean = NextStartingBlockToCleanUp::<T>::get();

            while block_to_clean <= current_block && !remaining_weight.is_zero() {
                Self::process_block_expired_items(block_to_clean, &mut remaining_weight);

                if remaining_weight.is_zero() {
                    break;
                }

                block_to_clean.saturating_accrue(BlockNumberFor::<T>::one());
            }

            // Update the next starting block for cleanup
            if block_to_clean > NextStartingBlockToCleanUp::<T>::get() {
                NextStartingBlockToCleanUp::<T>::put(block_to_clean);
                remaining_weight.saturating_reduce(db_weight.writes(1));
            }

            remaining_weight
        }

        fn process_block_expired_items(block: BlockNumberFor<T>, remaining_weight: &mut Weight) {
            let db_weight = T::DbWeight::get();
            let minimum_required_weight = db_weight.reads_writes(1, 1);

            if !remaining_weight.all_gte(minimum_required_weight) {
                return;
            }

            // Remove expired storage requests if any existed and process them.
            let mut expired_storage_requests = StorageRequestExpirations::<T>::take(&block);
            remaining_weight.saturating_reduce(minimum_required_weight);

            // TODO: After benchmarking, we should check before this loop that there is enough remaining weight to
            // TODO: process all the expired storage requests. If not, we should return early.
            while let Some(file_key) = expired_storage_requests.pop() {
                Self::process_expired_storage_request(file_key, remaining_weight)
            }

            // If there are remaining items which were not processed, put them back in storage
            if !expired_storage_requests.is_empty() {
                StorageRequestExpirations::<T>::insert(&block, expired_storage_requests);
                remaining_weight.saturating_reduce(db_weight.writes(1));
            }

            // Remove expired file deletion requests if any existed and process them.
            let mut expired_file_deletion_requests =
                FileDeletionRequestExpirations::<T>::take(&block);
            remaining_weight.saturating_reduce(minimum_required_weight);

            // TODO: After benchmarking, we should check before this loop that there is enough remaining weight to
            // TODO: process all the expired file deletion requests. If not, we should return early.
            while let Some((user, file_key)) = expired_file_deletion_requests.pop() {
                Self::process_expired_pending_file_deletion(user, file_key, remaining_weight)
            }

            // If there are remaining items which were not processed, put them back in storage
            if !expired_file_deletion_requests.is_empty() {
                FileDeletionRequestExpirations::<T>::insert(&block, expired_file_deletion_requests);
                remaining_weight.saturating_reduce(db_weight.writes(1));
            }

            // Remove expired move bucket requests if any existed and process them.
            let mut expired_move_bucket_requests = MoveBucketRequestExpirations::<T>::take(&block);
            remaining_weight.saturating_reduce(minimum_required_weight);

            while let Some((msp_id, bucket_id)) = expired_move_bucket_requests.pop() {
                Self::process_expired_move_bucket_request(msp_id, bucket_id, remaining_weight);
            }
        }

        fn process_expired_storage_request(file_key: MerkleHash<T>, remaining_weight: &mut Weight) {
            let db_weight = T::DbWeight::get();

            // As of right now, the upper bound limit to the number of BSPs required to fulfill a storage request is set by `ReplicationTarget`.
            // We could increase this potential weight to account for potentially more volunteers.
            let potential_weight =
                db_weight.writes(ReplicationTarget::<T>::get().saturating_plus_one().into());

            if !remaining_weight.all_gte(potential_weight) {
                return;
            }

            // Remove storage request and all bsps that volunteered for it.
            StorageRequests::<T>::remove(&file_key);
            let removed =
                StorageRequestBsps::<T>::drain_prefix(&file_key).fold(0, |acc, _| acc + 1u32);

            remaining_weight.saturating_reduce(db_weight.writes(1.saturating_add(removed.into())));

            Self::deposit_event(Event::StorageRequestExpired { file_key });
        }

        fn process_expired_pending_file_deletion(
            user: T::AccountId,
            file_key: MerkleHash<T>,
            remaining_weight: &mut Weight,
        ) {
            let db_weight = T::DbWeight::get();
            let potential_weight = db_weight.reads_writes(2, 3);

            if !remaining_weight.all_gte(potential_weight) {
                return;
            }

            let requests = PendingFileDeletionRequests::<T>::get(&user);

            // Check if the file key is still a pending deletion requests.
            let expired_item_index = match requests.iter().position(|(key, _)| key == &file_key) {
                Some(i) => i,
                None => return,
            };

            // Remove the file key from the pending deletion requests.
            PendingFileDeletionRequests::<T>::mutate(&user, |requests| {
                requests.remove(expired_item_index);
            });

            // Queue a priority challenge to remove the file key from all the providers.
            let _ = <T::ProofDealer as shp_traits::ProofsDealerInterface>::challenge_with_priority(
                &file_key,
                Some(TrieRemoveMutation),
            )
            .map_err(|_| {
                Self::deposit_event(Event::FailedToQueuePriorityChallenge {
                    user: user.clone(),
                    file_key,
                });
            });

            // Emit event.
            Self::deposit_event(Event::PriorityChallengeForFileDeletionQueued {
                user: user.clone(),
                file_key,
            });

            remaining_weight.saturating_reduce(potential_weight);
        }

        fn process_expired_move_bucket_request(
            msp_id: ProviderIdFor<T>,
            bucket_id: BucketIdFor<T>,
            remaining_weight: &mut Weight,
        ) {
            let db_weight = T::DbWeight::get();
            let potential_weight = db_weight.reads_writes(2, 3);

            if !remaining_weight.all_gte(potential_weight) {
                return;
            }

            PendingMoveBucketRequests::<T>::remove(&msp_id, &bucket_id);

            remaining_weight.saturating_reduce(potential_weight);

            Self::deposit_event(Event::MoveBucketRequestExpired { msp_id, bucket_id });
        }
    }
}
