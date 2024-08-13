use codec::Encode;
use frame_support::{
    ensure, pallet_prelude::DispatchResult, traits::nonfungibles_v2::Create, traits::Get,
};
use frame_system::pallet_prelude::BlockNumberFor;
use num_bigint::BigUint;
use sp_runtime::{
    traits::{CheckedAdd, CheckedDiv, CheckedSub, Convert, Hash, Saturating, Zero},
    ArithmeticError, BoundedVec, DispatchError,
};
use sp_std::{collections::btree_set::BTreeSet, vec, vec::Vec};

use pallet_file_system_runtime_api::{
    QueryBspConfirmChunksToProveForFileError, QueryFileEarliestVolunteerBlockError,
};
use pallet_nfts::{CollectionConfig, CollectionSettings, ItemSettings, MintSettings, MintType};
use shp_file_metadata::ChunkId;
use shp_traits::{
    MutateProvidersInterface, ProvidersInterface, ReadProvidersInterface, TrieAddMutation,
    TrieRemoveMutation,
};
use sp_runtime::traits::{ConvertBack, One};

use crate::types::{
    BlockRangeToMaximumThreshold, BucketNameFor, ExpiredItems, MaximumThreshold,
    ReplicationTargetType,
};
use crate::{
    pallet,
    types::{
        BucketIdFor, CollectionConfigFor, CollectionIdFor, FileKeyHasher, FileLocation,
        Fingerprint, ForestProof, KeyProof, MaxBspsPerStorageRequest, MerkleHash, MultiAddresses,
        PeerIds, ProviderIdFor, ReplicationTarget, StorageData, StorageRequestBspsMetadata,
        StorageRequestMetadata,
    },
    Error, Event, ItemExpirations, NextAvailableExpirationInsertionBlock, Pallet,
    PendingFileDeletionRequests, StorageRequestBsps, StorageRequests,
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
        // Calculate the difference between BSP's XOR and the starting threshold value.
        let (to_succeed, slope) = Self::get_threshold_to_succeed(&bsp_id, storage_request_block)?;
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
        let challenges = Self::generate_chunk_challenges_on_bsp_confirm(
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

    fn generate_chunk_challenges_on_bsp_confirm(
        bsp_id: ProviderIdFor<T>,
        file_key: MerkleHash<T>,
        storage_request_metadata: &StorageRequestMetadata<T>,
    ) -> Vec<<<T as pallet::Config>::Providers as ProvidersInterface>::MerkleHash> {
        let file_metadata = storage_request_metadata.clone().to_file_metadata();
        let chunks_to_check = file_metadata.chunks_to_check() as u32;

        let mut challenges =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::generate_challenges_from_seed(
                T::MerkleHashToRandomnessOutput::convert(file_key),
                &bsp_id,
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
            <T::Providers as ReadProvidersInterface>::is_msp(&msp_id),
            Error::<T>::NotAMsp
        );

        // Create collection only if bucket is private
        let maybe_collection_id = if private {
            // The `owner` of the collection is also the admin of the collection since most operations require the sender to be the admin.
            Some(Self::create_collection(sender.clone())?)
        } else {
            None
        };

        let bucket_id = <T as crate::Config>::Providers::derive_bucket_id(&sender, name);

        <T::Providers as MutateProvidersInterface>::add_bucket(
            msp_id,
            sender,
            bucket_id,
            private,
            maybe_collection_id.clone(),
        )?;

        Ok((bucket_id, maybe_collection_id))
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
            <T::Providers as ReadProvidersInterface>::is_bucket_owner(&sender, &bucket_id)?,
            Error::<T>::NotBucketOwner
        );

        let collection_id = Self::create_collection(sender)?;

        <T::Providers as MutateProvidersInterface>::update_bucket_read_access_group_id(
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
        msp: Option<ProviderIdFor<T>>,
        bsps_required: Option<ReplicationTargetType<T>>,
        user_peer_ids: Option<PeerIds<T>>,
        data_server_sps: BoundedVec<ProviderIdFor<T>, MaxBspsPerStorageRequest<T>>,
    ) -> Result<MerkleHash<T>, DispatchError> {
        // TODO: Check user funds and lock them for the storage request.
        // TODO: Check storage capacity of chosen MSP (when we support MSPs)
        // TODO: Return error if the file is already stored and overwrite is false.

        // Check that the file size is greater than zero.
        ensure!(size > Zero::zero(), Error::<T>::FileSizeCannotBeZero);

        if let Some(ref msp) = msp {
            ensure!(
                <T::Providers as ReadProvidersInterface>::is_msp(msp),
                Error::<T>::NotAMsp
            );
        }

        // Check that bucket exists and that the sender is the owner of the bucket.
        ensure!(
            <T::Providers as ReadProvidersInterface>::is_bucket_owner(&sender, &bucket_id)?,
            Error::<T>::NotBucketOwner
        );

        let bsps_required = bsps_required.unwrap_or(ReplicationTarget::<T>::get());

        if bsps_required.is_zero() {
            return Err(Error::<T>::BspsRequiredCannotBeZero)?;
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

        Self::queue_expiration_item(
            T::StorageRequestTtl::get().into(),
            ExpiredItems::StorageRequest(file_key),
        )?;

        Ok(file_key)
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
            <T::Providers as shp_traits::ProvidersInterface>::get_provider_id(sender.clone())
                .ok_or(Error::<T>::NotABsp)?;
        // Check that the provider is indeed a BSP.
        ensure!(
            <T::Providers as ReadProvidersInterface>::is_bsp(&bsp_id),
            Error::<T>::NotABsp
        );

        // TODO: Verify BSP has enough storage capacity to store the file

        // Check that the storage request exists.
        let mut storage_request_metadata =
            <StorageRequests<T>>::get(&file_key).ok_or(Error::<T>::StorageRequestNotFound)?;

        expect_or_err!(
            storage_request_metadata.bsps_confirmed < storage_request_metadata.bsps_required,
            "Storage request should never have confirmed bsps equal to or greater than required bsps, since they are deleted when it is reached.",
            Error::<T>::StorageRequestBspsRequiredFulfilled,
            bool
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
            Self::get_threshold_to_succeed(&bsp_id, storage_request_metadata.requested_at)?;

        // Check that the BSP's threshold is under the threshold required to volunteer for the storage request.
        ensure!(bsp_threshold <= (to_succeed), Error::<T>::AboveThreshold);

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
            <T::Providers as shp_traits::ProvidersInterface>::get_provider_id(sender.clone())
                .ok_or(Error::<T>::NotABsp)?;

        // Check that the provider is indeed a BSP.
        ensure!(
            <T::Providers as ReadProvidersInterface>::is_bsp(&bsp_id),
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

            let chunk_challenges = Self::generate_chunk_challenges_on_bsp_confirm(
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
            <T::Providers as MutateProvidersInterface>::increase_data_used(
                &bsp_id,
                storage_request_metadata.size,
            )?;

            // Remove storage request if we reached the required number of bsps.
            if storage_request_metadata.bsps_confirmed == storage_request_metadata.bsps_required {
                // TODO: we should only delete if the MSP also confirmed to store the file (this is not implemented yet).
                // Remove storage request metadata.
                <StorageRequests<T>>::remove(&file_key.0);

                // There should only be the number of bsps volunteered under the storage request prefix.
                let remove_limit: u32 = storage_request_metadata
                    .bsps_volunteered
                    .try_into()
                    .map_err(|_| Error::<T>::FailedTypeConversion)?;

                // Remove storage request bsps
                let removed =
                    <StorageRequestBsps<T>>::drain_prefix(&file_key.0).fold(0, |acc, _| acc + 1);

                // Make sure that the expected number of bsps were removed.
                expect_or_err!(
                    removed == remove_limit,
                    "Number of volunteered bsps for storage request should have been removed",
                    Error::<T>::UnexpectedNumberOfRemovedVolunteeredBsps,
                    bool
                );
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
            <T::Providers as shp_traits::ProvidersInterface>::get_root(bsp_id),
            "Failed to get root for BSP, when it was already checked to be a BSP",
            Error::<T>::NotABsp
        );

        if old_root == <T::Providers as shp_traits::ProvidersInterface>::get_default_root() {
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
        <T::Providers as shp_traits::ProvidersInterface>::update_root(bsp_id, new_root)?;

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
        }

        // There should only be the number of bsps volunteered under the storage request prefix.
        let remove_limit: u32 = storage_request_metadata
            .bsps_volunteered
            .try_into()
            .map_err(|_| Error::<T>::FailedTypeConversion)?;

        // Remove storage request bsps
        let removed = <StorageRequestBsps<T>>::drain_prefix(&file_key).fold(0, |acc, _| acc + 1);

        // Make sure that the expected number of bsps were removed.
        expect_or_err!(
            removed == remove_limit,
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
    pub(crate) fn do_bsp_stop_storing(
        sender: T::AccountId,
        file_key: MerkleHash<T>,
        bucket_id: BucketIdFor<T>,
        location: FileLocation<T>,
        owner: T::AccountId,
        fingerprint: Fingerprint<T>,
        size: StorageData<T>,
        can_serve: bool,
        inclusion_forest_proof: ForestProof<T>,
    ) -> Result<(ProviderIdFor<T>, MerkleHash<T>), DispatchError> {
        let bsp_id =
            <T::Providers as shp_traits::ProvidersInterface>::get_provider_id(sender.clone())
                .ok_or(Error::<T>::NotABsp)?;

        // Check that the provider is indeed a BSP.
        ensure!(
            <T::Providers as ReadProvidersInterface>::is_bsp(&bsp_id),
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
        <T::Providers as shp_traits::ProvidersInterface>::update_root(bsp_id, new_root)?;

        // Decrease data used by the BSP.
        <T::Providers as MutateProvidersInterface>::decrease_data_used(&bsp_id, size)?;

        Ok((bsp_id, new_root))
    }

    pub(crate) fn do_delete_file(
        sender: T::AccountId,
        bucket_id: ProviderIdFor<T>,
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
            <T::Providers as ReadProvidersInterface>::is_bucket_owner(&sender, &bucket_id)?,
            Error::<T>::NotBucketOwner
        );

        let msp_id = <T::Providers as ReadProvidersInterface>::get_msp_of_bucket(&bucket_id)
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
                Self::queue_expiration_item(
                    T::PendingFileDeletionRequestTtl::get().into(),
                    ExpiredItems::PendingFileDeletionRequests((sender, file_key)),
                )?;

                false
            }
            // If the user supplied a proof of inclusion, verify the proof and queue a priority challenge to remove the file key from all the providers.
            Some(inclusion_forest_proof) => {
                // Verify the proof of inclusion.
                let proven_keys =
                    <T::ProofDealer as shp_traits::ProofsDealerInterface>::verify_forest_proof(
                        &bucket_id,
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
            <T::Providers as shp_traits::ProvidersInterface>::get_provider_id(sender.clone())
                .ok_or(Error::<T>::NotAMsp)?;

        // Check that the provider is indeed an MSP.
        ensure!(
            <T::Providers as ReadProvidersInterface>::is_msp(&msp_id),
            Error::<T>::NotAMsp
        );

        ensure!(
            <T::Providers as ReadProvidersInterface>::is_bucket_stored_by_msp(&msp_id, &bucket_id),
            Error::<T>::MspNotStoringBucket
        );

        let pending_file_deletion_requests = <PendingFileDeletionRequests<T>>::get(&user);

        // Check if the file key is in the pending deletion requests.
        ensure!(
            pending_file_deletion_requests.contains(&(file_key, bucket_id)),
            Error::<T>::FileKeyNotPendingDeletion
        );

        // Verify the proof of inclusion.let proven_keys =
        let proven_keys =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::verify_forest_proof(
                &bucket_id,
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

    /// Compute the next block number to insert an expiration after a given TTL.
    ///
    /// This function attempts to insert an expiration item at the next available block after the given TTL starting from
    /// the current [`NextAvailableExpirationInsertionBlock`].
    pub(crate) fn queue_expiration_item(
        expiration_item_ttl: BlockNumberFor<T>,
        expiration_item: ExpiredItems<T>,
    ) -> Result<BlockNumberFor<T>, DispatchError> {
        let mut expiration_block = NextAvailableExpirationInsertionBlock::<T>::get()
            .checked_add(&expiration_item_ttl)
            .ok_or(Error::<T>::MaxBlockNumberReached)?;

        while let Err(_) =
            <ItemExpirations<T>>::try_append(expiration_block, expiration_item.clone())
        {
            expiration_block = expiration_block
                .checked_add(&1u8.into())
                .ok_or(Error::<T>::MaxBlockNumberReached)?;
        }

        <NextAvailableExpirationInsertionBlock<T>>::set(expiration_block);

        Ok(expiration_block)
    }

    pub(crate) fn compute_file_key(
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

    pub fn get_threshold_to_succeed(
        bsp_id: &ProviderIdFor<T>,
        requested_at: BlockNumberFor<T>,
    ) -> Result<(T::ThresholdType, T::ThresholdType), DispatchError> {
        let maximum_threshold: u32 = MaximumThreshold::<T>::get().into();
        let global_weight = T::Providers::get_global_bsps_reputation_weight();

        if global_weight
            == <T::Providers as shp_traits::ReadProvidersInterface>::ReputationWeight::zero()
        {
            return Err(Error::<T>::NoGlobalReputationWeightSet.into());
        }

        // Global threshold starting point from which all BSPs begin their threshold slope. All BSPs start at this point
        // with the starting reputation weight.
        let threshold_global_starting_point = maximum_threshold.checked_mul(ReplicationTarget::<T>::get().into()
            / global_weight.into()
            / 2).unwrap_or_else(|| {
                log::warn!("Global starting point is beyond MaximumThreshold. Setting it to half of the MaximumThreshold.");
                maximum_threshold/ 2
            });

        // Get the BSP's reputation weight.
        let bsp_weight: u32 = T::Providers::get_bsp_reputation_weight(&bsp_id)?.into();

        // Actual BSP's threshold starting point, taking into account their reputation weight.
        let threshold_weighted_starting_point: T::ThresholdType = bsp_weight
            .saturating_mul(threshold_global_starting_point)
            .into();

        // Rate of increase from the global threshold starting point up to the maximum threshold within a block range.
        let threshold_slope = <u32 as Into<T::ThresholdType>>::into(
            maximum_threshold - threshold_global_starting_point,
        ) / T::ThresholdTypeToBlockNumber::convert_back(
            BlockRangeToMaximumThreshold::<T>::get(),
        );

        let current_block_number = <frame_system::Pallet<T>>::block_number();

        // Get number of blocks since the storage request was issued.
        let blocks_since_requested = current_block_number.saturating_sub(requested_at);
        let blocks_since_requested =
            T::ThresholdTypeToBlockNumber::convert_back(blocks_since_requested);

        Ok((
            threshold_weighted_starting_point
                .saturating_add(threshold_slope.saturating_mul(blocks_since_requested)),
            threshold_slope,
        ))
    }
}

impl<T: crate::Config> shp_traits::SubscribeProvidersInterface for Pallet<T> {
    type ProviderId = ProviderIdFor<T>;

    fn subscribe_bsp_sign_off(_who: &Self::ProviderId) -> DispatchResult {
        todo!("remove this")
    }

    fn subscribe_bsp_sign_up(_who: &Self::ProviderId) -> DispatchResult {
        todo!("remove this")
    }
}

mod hooks {
    use crate::types::{ExpiredItems, MerkleHash};
    use crate::{
        pallet, Event, ItemExpirations, NextStartingBlockToCleanUp, Pallet,
        PendingFileDeletionRequests, StorageRequestBsps, StorageRequests,
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

            // Remove expired items if any existed and process them.
            let mut expired_items = ItemExpirations::<T>::take(&block);
            remaining_weight.saturating_reduce(minimum_required_weight);

            while let Some(expired) = expired_items.pop() {
                match expired {
                    ExpiredItems::StorageRequest(file_key) => {
                        Self::process_expired_storage_request(file_key, remaining_weight)
                    }
                    ExpiredItems::PendingFileDeletionRequests((user, file_key)) => {
                        Self::process_expired_pending_file_deletion(
                            user,
                            file_key,
                            remaining_weight,
                        )
                    }
                };
            }

            // If there are remaining items which were not processed, put them back in storage
            if !expired_items.is_empty() {
                ItemExpirations::<T>::insert(&block, expired_items);
                remaining_weight.saturating_reduce(db_weight.writes(1));
            }
        }

        fn process_expired_storage_request(file_key: MerkleHash<T>, remaining_weight: &mut Weight) {
            let db_weight = T::DbWeight::get();

            // As of right now, the upper bound limit to the number of BSPs required to fulfill a storage request is set by `ReplicationTarget`.
            // We could increase this potential weight to account for potentially more volunteers.
            let potential_weight = db_weight.writes(
                Into::<u32>::into(T::ReplicationTarget::get())
                    .saturating_add(1)
                    .into(),
            );

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

            remaining_weight.saturating_reduce(potential_weight);
        }
    }
}
