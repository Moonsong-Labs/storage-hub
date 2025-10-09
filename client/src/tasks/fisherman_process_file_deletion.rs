use anyhow::anyhow;
use codec::Encode;
use futures::future::{join_all, BoxFuture};
use hex;
use sc_tracing::tracing::*;
use shc_actors_framework::{actor::ActorHandle, event_bus::EventHandler};
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface, types::SendExtrinsicOptions,
};
use shc_common::{
    traits::StorageEnableRuntime,
    types::{
        FileDeletionRequest, Fingerprint, ForestProof as CommonForestProof, OffchainSignature,
        StorageProofsMerkleTrieLayout, StorageProviderId,
    },
};
use shc_fisherman_service::{
    commands::FishermanServiceCommandInterface,
    events::{FileDeletionTarget, ProcessFileDeletionRequest, ProcessIncompleteStorageRequest},
    {FileKeyOperation, FishermanService},
};
use shc_forest_manager::{in_memory::InMemoryForestStorage, traits::ForestStorage};
use shc_indexer_db::models::BspFile;
use sp_core::H256;
use sp_runtime::traits::SaturatedConversion;
use std::time::Duration;

use crate::{
    handler::StorageHubHandler,
    types::{FishermanForestStorageHandlerT, ShNodeType},
};

// TODO: Refactor task to support batch file deletions

/// Data structure holding common file deletion information retrieved from indexer database.
///
/// Contains all the metadata required to process file deletion operations across
/// both [`ProcessFileDeletionRequest`] and [`ProcessIncompleteStorageRequest`] events.
struct FileDeletionData<Runtime: StorageEnableRuntime> {
    /// Metadata of the file to be deleted
    file_metadata: shc_common::types::FileMetadata,
    /// List of BSP IDs that are storing this file
    bsp_ids: Vec<shc_indexer_db::OnchainBspId>,
    /// Target bucket for file deletion operations
    bucket_target: FileDeletionTarget<Runtime>,
}

/// Fetches common file deletion data from the indexer database.
///
/// This function queries the indexer database to retrieve file metadata, bucket information,
/// and all BSP IDs storing the specified file. This data is used by both deletion task types.
///
/// # Arguments
/// * `storage_hub_handler` - Handler providing access to indexer database
/// * `file_key` - Key of the file to fetch deletion data for
///
/// # Returns
/// [`FileDeletionData`] containing file metadata, BSP IDs, and bucket target
async fn fetch_file_deletion_data<NT, Runtime>(
    storage_hub_handler: &StorageHubHandler<NT, Runtime>,
    file_key: &shp_types::Hash,
) -> anyhow::Result<FileDeletionData<Runtime>>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    // Get indexer connection
    let indexer_db_pool = storage_hub_handler
        .indexer_db_pool
        .as_ref()
        .ok_or_else(|| anyhow!("Indexer is disabled but a file deletion event was received"))?;

    let mut conn = indexer_db_pool
        .get()
        .await
        .map_err(|e| anyhow!("Failed to get indexer connection: {:?}", e))?;

    // Get file and bucket info
    let file = shc_indexer_db::models::File::get_by_file_key(&mut conn, &file_key)
        .await
        .map_err(|e| anyhow!("Failed to get file from indexer: {:?}", e))?;

    let file_metadata = file
        .to_file_metadata(file.onchain_bucket_id.clone())
        .map_err(|e| anyhow!("Failed to convert file to metadata: {:?}", e))?;

    // Query for BSPs storing this file
    let bsp_ids = BspFile::get_bsps_for_file_key(&mut conn, file_key.as_ref())
        .await
        .map_err(|e| anyhow!("Failed to query BSPs for file: {:?}", e))?;

    drop(conn);

    let bucket_id_array: [u8; 32] = file
        .onchain_bucket_id
        .clone()
        .try_into()
        .map_err(|_| anyhow!("Invalid bucket ID length"))?;

    Ok(FileDeletionData {
        file_metadata,
        bsp_ids,
        bucket_target: FileDeletionTarget::BucketId(H256::from(bucket_id_array)),
    })
}

const LOG_TARGET: &str = "fisherman-process-file-deletion-task";

/// Task handler for processing signed file deletion requests from fisherman service.
///
/// This task processes [`ProcessFileDeletionRequest`] events which contain user-signed
/// file deletion intentions. It validates signatures, constructs forest proofs from
/// the current blockchain state, and submits delete_file extrinsics for both the
/// target bucket (MSP) and all associated BSPs storing the file.
///
/// The deletion process runs in parallel for all targets to optimize performance.
pub struct FishermanProcessFileDeletionTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    /// Handler providing access to blockchain, indexer database, and forest storage
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
    /// Actor handle for communicating with the fisherman service
    fisherman_service: ActorHandle<FishermanService<Runtime>>,
}

impl<NT, Runtime> Clone for FishermanProcessFileDeletionTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> FishermanProcessFileDeletionTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            fisherman_service: self.fisherman_service.clone(),
        }
    }
}

impl<NT, Runtime> FishermanProcessFileDeletionTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    /// Creates a new [`FishermanProcessFileDeletionTask`].
    ///
    /// # Arguments
    /// * `storage_hub_handler` - Handler providing access to required services
    ///
    /// # Panics
    /// Panics if the fisherman service handle is not available in the storage hub handler
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        let fisherman_service = storage_hub_handler
            .fisherman
            .clone()
            .expect("FishermanProcessFileDeletionTask requires fisherman service handle");

        Self {
            storage_hub_handler,
            fisherman_service,
        }
    }
}

/// Task handler for processing incomplete storage file deletions from fisherman service.
///
/// This task processes [`ProcessIncompleteStorageRequest`] events which are triggered
/// when files need to be cleaned up due to incomplete storage operations. Unlike signed
/// deletions, these do not require user signatures and are initiated by the system.
///
/// Currently, this task prepares deletion parameters but does not submit extrinsics,
/// as indicated by the TODO comment in the implementation.
pub struct FishermanProcessIncompleteStorageTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    /// Handler providing access to blockchain, indexer database, and forest storage
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
    /// Actor handle for communicating with the fisherman service
    fisherman_service: ActorHandle<FishermanService<Runtime>>,
}

impl<NT, Runtime> Clone for FishermanProcessIncompleteStorageTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> FishermanProcessIncompleteStorageTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            fisherman_service: self.fisherman_service.clone(),
        }
    }
}

impl<NT, Runtime> FishermanProcessIncompleteStorageTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    /// Creates a new [`FishermanProcessIncompleteStorageTask`].
    ///
    /// # Arguments
    /// * `storage_hub_handler` - Handler providing access to required services
    ///
    /// # Panics
    /// Panics if the fisherman service handle is not available in the storage hub handler
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        let fisherman_service = storage_hub_handler
            .fisherman
            .clone()
            .expect("FishermanProcessIncompleteStorageTask requires fisherman service handle");

        Self {
            storage_hub_handler,
            fisherman_service,
        }
    }
}

impl<NT, Runtime> EventHandler<ProcessFileDeletionRequest<Runtime>>
    for FishermanProcessFileDeletionTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: ProcessFileDeletionRequest<Runtime>,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing file deletion request for signed intention file key: {:?}",
            event.signed_file_operation_intention.file_key
        );

        let file_key = &event.signed_file_operation_intention.file_key;
        let signature = &event.signature;

        // Fetch common file deletion data
        let deletion_data = fetch_file_deletion_data(&self.storage_hub_handler, file_key).await?;

        // Create a vector of futures for parallel processing
        let mut deletion_futures: Vec<BoxFuture<'_, anyhow::Result<()>>> = Vec::new();

        let event_ref = &event;
        let file_metadata_ref = &deletion_data.file_metadata;

        // Clone self before moving into async blocks
        let self_clone = self.clone();

        deletion_futures.push(Box::pin(async move {
            self_clone
                .process_deletion_for_target(
                    event_ref,
                    file_key,
                    signature,
                    deletion_data.bucket_target,
                    file_metadata_ref,
                )
                .await
        }));

        // Process BSP targets in parallel
        for onchain_bsp_id in deletion_data.bsp_ids {
            // Convert OnchainBspId to H256 for the target
            let bsp_target = FileDeletionTarget::BspId(onchain_bsp_id.into());

            let self_clone = self.clone();

            deletion_futures.push(Box::pin(async move {
                self_clone
                    .process_deletion_for_target(
                        event_ref,
                        file_key,
                        signature,
                        bsp_target,
                        file_metadata_ref,
                    )
                    .await
            }));
        }

        // Execute all deletions in parallel and collect results
        let results = join_all(deletion_futures).await;

        // Check for any failures
        for result in results {
            result?;
        }

        Ok(())
    }
}

impl<NT, Runtime> EventHandler<ProcessIncompleteStorageRequest>
    for FishermanProcessIncompleteStorageTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: ProcessIncompleteStorageRequest) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing incomplete storage request for file key: {:?}",
            event.file_key
        );

        let file_key = &event.file_key;

        // Query the incomplete storage request metadata
        let incomplete_metadata = self
            .fisherman_service
            .query_incomplete_storage_request(*file_key)
            .await
            .map_err(|e| anyhow!("Failed to query incomplete storage request: {:?}", e))?;

        // Create FileMetadata directly from the runtime API response
        let file_metadata = shc_common::types::FileMetadata::new(
            incomplete_metadata.owner.encode(),
            incomplete_metadata.bucket_id.as_ref().to_vec(),
            incomplete_metadata.location.clone(),
            incomplete_metadata.file_size.saturated_into::<u64>(),
            shc_common::types::Fingerprint::from(incomplete_metadata.fingerprint.to_fixed_bytes()),
        )
        .map_err(|e| anyhow!("Failed to create file metadata: {:?}", e))?;

        // Create a vector of futures for parallel processing
        let mut deletion_futures: Vec<BoxFuture<'_, anyhow::Result<()>>> = Vec::new();

        let file_metadata_ref = &file_metadata;

        // Process bucket deletion only if pending_bucket_removal is true
        if incomplete_metadata.pending_bucket_removal {
            let bucket_id_array: [u8; 32] = incomplete_metadata
                .bucket_id
                .as_ref()
                .try_into()
                .map_err(|_| anyhow!("Invalid bucket ID length"))?;

            let bucket_target = FileDeletionTarget::BucketId(H256::from(bucket_id_array));
            let self_clone = self.clone();

            deletion_futures.push(Box::pin(async move {
                self_clone
                    .process_deletion_for_target_incomplete(
                        file_key,
                        bucket_target,
                        file_metadata_ref,
                    )
                    .await
            }));
        }

        // Process BSP targets in parallel - use pending_bsp_removals directly as source of truth
        for bsp_id in incomplete_metadata.pending_bsp_removals {
            let bsp_target = FileDeletionTarget::BspId(bsp_id);
            let self_clone = self.clone();

            deletion_futures.push(Box::pin(async move {
                self_clone
                    .process_deletion_for_target_incomplete(file_key, bsp_target, file_metadata_ref)
                    .await
            }));
        }

        // Execute all deletions in parallel and collect results
        let results = join_all(deletion_futures).await;

        // Check for any failures
        for result in results {
            result?;
        }

        trace!("Completed processing incomplete storage requests");

        Ok(())
    }
}

impl<NT, Runtime> FishermanProcessFileDeletionTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    /// Processes file deletion for a specific target (BSP id or Bucket id) with user signature validation (which is required for the runtime to validate
    /// that the user does indeed want to delete the file).
    ///
    /// This method constructs and submits a delete_file extrinsic for the given target,
    /// using the signed file operation intention and signature from the user.
    ///
    /// # Arguments
    /// * `event` - The deletion request event containing signed intention
    /// * `file_key` - Key of the file to delete
    /// * `signature` - User's signature for the deletion request
    /// * `deletion_target` - Target (BSP or bucket) for the deletion
    /// * `file_metadata` - Metadata of the file being deleted
    async fn process_deletion_for_target(
        &self,
        event: &ProcessFileDeletionRequest<Runtime>,
        file_key: &shp_types::Hash,
        signature: &OffchainSignature<Runtime>,
        deletion_target: FileDeletionTarget<Runtime>,
        file_metadata: &shc_common::types::FileMetadata,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing deletion for file key {:?} and target {:?}",
            file_key,
            deletion_target
        );

        // Use the common logic function to get parameters
        let (file_owner, bucket_id, location, size, fingerprint, provider_id, forest_proof) =
            process_deletion_common(
                &self.storage_hub_handler,
                &self.fisherman_service,
                file_key,
                deletion_target,
                file_metadata,
            )
            .await?;

        // Build the delete_files_for_incomplete_storage_request extrinsic call
        // Pass None for bucket deletion (MSP), Some(id) for BSP deletion
        let maybe_bsp_id = match provider_id {
            Some(StorageProviderId::BackupStorageProvider(id)) => Some(id),
            Some(StorageProviderId::MainStorageProvider(_)) | None => None,
        };

        // Log the signed intention file key for signed deletions
        trace!(
            target: LOG_TARGET,
            "File key from signed intention: 0x{}",
            hex::encode(event.signed_file_operation_intention.file_key.as_ref() as &[u8])
        );

        trace!(
            target: LOG_TARGET,
            "Submitting delete_file extrinsic (batched with single file)"
        );

        // Build the file deletion request
        let file_deletion = FileDeletionRequest {
            file_owner: file_owner.clone(),
            signed_intention: event.signed_file_operation_intention.clone(),
            signature: signature.clone(),
            bucket_id,
            location: location
                .try_into()
                .map_err(|_| anyhow!("Location too long"))?,
            size,
            fingerprint: H256::from_slice(fingerprint.as_ref()),
        };

        // TODO: Wrap in BoundedVec (single file for now)
        let file_deletions = vec![file_deletion]
            .try_into()
            .expect("Single file fits in MaxFileDeletionsPerExtrinsic");

        // Build the delete_file extrinsic call
        let call = pallet_file_system::Call::<Runtime>::delete_files {
            file_deletions,
            bsp_id: maybe_bsp_id,
            forest_proof: forest_proof.proof,
        };

        // Submit the extrinsic
        self.storage_hub_handler
            .blockchain
            .send_extrinsic(
                call.into(),
                SendExtrinsicOptions::new(Duration::from_secs(60)),
            )
            .await
            .map_err(|e| {
                error!(
                    target: LOG_TARGET,
                    "Failed to submit delete_file extrinsic: {:?}", e
                );
                anyhow!("Failed to submit delete_file extrinsic: {:?}", e)
            })?;

        info!(
            target: LOG_TARGET,
            "Successfully submitted delete_file extrinsic for file key {:?}",
            file_key
        );

        Ok(())
    }
}

impl<NT, Runtime> FishermanProcessIncompleteStorageTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    /// Processes file deletion for incomplete storage scenarios (see. [`monitor_block`](shc_fisherman_service::handler::FishermanService::monitor_block)
    /// for on chain events which trigger this task to process `ProcessIncompleteStorageRequest` events)
    ///
    /// This method prepares deletion parameters using the common logic but does not
    /// submit extrinsics, as these deletions don't require user signatures.
    ///
    /// # Arguments
    /// * `file_key` - Key of the file to delete
    /// * `deletion_target` - Target (BSP or bucket) for the deletion
    /// * `file_metadata` - Metadata of the file being deleted
    async fn process_deletion_for_target_incomplete(
        &self,
        file_key: &shp_types::Hash,
        deletion_target: FileDeletionTarget<Runtime>,
        file_metadata: &shc_common::types::FileMetadata,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing deletion for file key {:?} and target {:?} (incomplete storage)",
            file_key,
            deletion_target
        );

        // Use the common logic function to get parameters
        let (_file_owner, _bucket_id, _location, _size, _fingerprint, provider_id, forest_proof) =
            process_deletion_common(
                &self.storage_hub_handler,
                &self.fisherman_service,
                file_key,
                deletion_target,
                file_metadata,
            )
            .await?;

        trace!(
            target: LOG_TARGET,
            "Submitting delete_files_for_incomplete_storage_request extrinsic"
        );

        // Build the delete_files_for_incomplete_storage_request extrinsic call
        // Pass None for bucket deletion (MSP), Some(id) for BSP deletion
        let maybe_bsp_id = match provider_id {
            Some(StorageProviderId::BackupStorageProvider(id)) => Some(id),
            Some(StorageProviderId::MainStorageProvider(_)) | None => None,
        };

        // Wrap file_key in BoundedVec (single file for now)
        let file_keys = vec![(*file_key).into()]
            .try_into()
            .expect("Single file fits in MaxFileDeletionsPerExtrinsic");

        let call =
            pallet_file_system::Call::<Runtime>::delete_files_for_incomplete_storage_request {
                file_keys,
                bsp_id: maybe_bsp_id,
                forest_proof: forest_proof.proof,
            };

        // Submit the extrinsic
        self.storage_hub_handler
            .blockchain
            .send_extrinsic(
                call.into(),
                SendExtrinsicOptions::new(Duration::from_secs(60)),
            )
            .await
            .map_err(|e| {
                error!(
                    target: LOG_TARGET,
                    "Failed to submit delete_files_for_incomplete_storage_request extrinsic: {:?}", e
                );
                anyhow!(
                    "Failed to submit delete_files_for_incomplete_storage_request extrinsic: {:?}",
                    e
                )
            })?;

        info!(
            target: LOG_TARGET,
            "Successfully submitted delete_files_for_incomplete_storage_request extrinsic for file key {:?}",
            file_key
        );

        Ok(())
    }
}

/// Common deletion processing logic shared between both task implementations.
///
/// This function implements the core logic for preparing file deletion parameters,
/// including forest proof generation using a two-phase approach:
/// 1. Build ephemeral trie from indexer database data at last processed block
/// 2. Apply catch-up changes from last indexed block to current best block
///
/// The forest proof ensures the deletion extrinsic has the correct Merkle proof
/// for the current blockchain state by using the indexer's last processed block
/// as the baseline and applying catch-up changes to reach the current state. That
/// way, the runtime can validate that the file key belongs to the BSP or Bucket Merkle
/// Forest so it may delete the file from the proof on chain.
///
/// # Arguments
/// * `storage_hub_handler` - Handler providing access to blockchain and indexer
/// * `fisherman_service` - Service for querying file key changes
/// * `file_key` - Key of the file being deleted
/// * `deletion_target` - Target (BSP or bucket) for the deletion
/// * `file_metadata` - Metadata of the file being deleted
///
/// # Returns
/// Tuple containing all parameters needed for delete_file extrinsic:
/// - File owner account ID
/// - Bucket ID
/// - File location
/// - File size
/// - File fingerprint
/// - Provider ID (BSP or MSP)
/// - Forest proof
async fn process_deletion_common<NT, Runtime>(
    storage_hub_handler: &StorageHubHandler<NT, Runtime>,
    fisherman_service: &ActorHandle<FishermanService<Runtime>>,
    file_key: &shp_types::Hash,
    deletion_target: FileDeletionTarget<Runtime>,
    file_metadata: &shc_common::types::FileMetadata,
) -> anyhow::Result<(
    <Runtime as frame_system::Config>::AccountId,
    H256,
    Vec<u8>,
    <Runtime as pallet_storage_providers::Config>::StorageDataUnit,
    Fingerprint,
    Option<StorageProviderId<Runtime>>,
    CommonForestProof<StorageProofsMerkleTrieLayout>,
)>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    // Extract file details
    let bucket_id = H256::from_slice(file_metadata.bucket_id());
    let location = file_metadata.location().to_vec();
    let size = file_metadata.file_size().saturated_into();
    let fingerprint = file_metadata.fingerprint();

    // Determine provider ID from deletion target
    let provider_id = match &deletion_target {
        FileDeletionTarget::BspId(bsp_id) => {
            Some(StorageProviderId::BackupStorageProvider(*bsp_id))
        }
        FileDeletionTarget::BucketId(target_bucket_id) => {
            let maybe_msp_id = storage_hub_handler
                .blockchain
                .query_msp_id_of_bucket_id(*target_bucket_id)
                .await
                .map_err(|e| anyhow!("Failed to query MSP ID for bucket: {:?}", e))?;
            if let Some(msp_id) = maybe_msp_id {
                Some(StorageProviderId::MainStorageProvider(msp_id))
            } else {
                None
            }
        }
    };

    // Get indexer database connection
    let indexer_db_pool = storage_hub_handler
        .indexer_db_pool
        .as_ref()
        .ok_or_else(|| anyhow!("Indexer is disabled but a file deletion event was received"))?;

    let mut conn = indexer_db_pool
        .get()
        .await
        .map_err(|e| anyhow!("Failed to get indexer connection: {:?}", e))?;

    // Generate forest proof using two-phase ephemeral trie construction
    let forest_proof = {
        // Get the last processed block from indexer database
        let service_state = shc_indexer_db::models::ServiceState::get(&mut conn)
            .await
            .map_err(|e| anyhow!("Failed to get service state from indexer: {:?}", e))?;
        let last_indexed_finalized_block =
            (service_state.last_indexed_finalized_block as u64).saturated_into();

        trace!(
            target: LOG_TARGET,
            "Building ephemeral trie from indexer data at last processed block {}",
            last_indexed_finalized_block
        );

        // Fetch all file keys for the deletion target from finalized data
        let all_file_keys = match &deletion_target {
            FileDeletionTarget::BspId(bsp_id) => {
                // Convert H256 to OnchainBspId for database query
                let onchain_bsp_id = shc_indexer_db::OnchainBspId::from(*bsp_id);
                BspFile::get_all_file_keys_for_bsp(&mut conn, onchain_bsp_id)
                    .await
                    .map_err(|e| anyhow!("Failed to get all file keys for BSP: {:?}", e))?
            }
            FileDeletionTarget::BucketId(bucket_id) => {
                shc_indexer_db::models::file::File::get_all_file_keys_for_bucket(
                    &mut conn,
                    bucket_id.as_ref(),
                )
                .await
                .map_err(|e| anyhow!("Failed to get all file keys for bucket: {:?}", e))?
            }
        };

        // Fetch all files and convert to FileMetadata
        let mut file_metadatas = Vec::new();
        for key in &all_file_keys {
            let file = shc_indexer_db::models::File::get_by_file_key(&mut conn, key)
                .await
                .map_err(|e| anyhow!("Failed to get file: {:?}", e))?;

            let metadata = file
                .to_file_metadata(file.onchain_bucket_id.clone())
                .map_err(|e| anyhow!("Failed to convert file to metadata: {:?}", e))?;

            file_metadatas.push(metadata);
        }

        drop(conn);

        trace!(
            target: LOG_TARGET,
            "Building ephemeral trie with {} file keys from finalized data",
            all_file_keys.len(),
        );

        // Create ephemeral in-memory forest storage
        let mut ephemeral_forest = InMemoryForestStorage::<StorageProofsMerkleTrieLayout>::new();

        // TODO: Forests are constructed on the fly and fisherman tasks are run in parallel.
        // TODO: It is entirely possible that there me be more than 1 file deletion for the same bucket that
        // TODO: is submitted in the same block as another task. This means that only a single task will have successfully
        // TODO: deleted a file while the other tasks will have a invalid forest root.
        // TODO: We could adopt the same strategy as the InMemoryForestStorage which tracks per bucket forests and have a lock on it.
        // Insert all file keys from finalized data
        <InMemoryForestStorage<StorageProofsMerkleTrieLayout> as ForestStorage<
            StorageProofsMerkleTrieLayout,
            Runtime,
        >>::insert_files_metadata(&mut ephemeral_forest, &file_metadatas)
        .map_err(|e| anyhow!("Failed to insert file keys into ephemeral trie: {:?}", e))?;

        trace!(
            target: LOG_TARGET,
            "Ephemeral trie built with root: {:?}",
            <InMemoryForestStorage<StorageProofsMerkleTrieLayout> as ForestStorage<StorageProofsMerkleTrieLayout, Runtime>>::root(&ephemeral_forest)
        );

        // TODO: Check if the root matches the one on chain

        trace!(
            target: LOG_TARGET,
            "Applying catch-up from block {} to best block",
            last_indexed_finalized_block
        );

        // Get file key changes since finalized block using the generated interface method
        let file_key_changes = fisherman_service
            .get_file_key_changes_since_block(last_indexed_finalized_block, deletion_target.clone())
            .await
            .map_err(|e| anyhow!("Failed to get file key changes: {:?}", e))?;

        trace!(
            target: LOG_TARGET,
            "Applying {} file key changes from catch-up",
            file_key_changes.len()
        );

        // Apply changes to the ephemeral trie
        for change in file_key_changes {
            match change.operation {
                FileKeyOperation::Add(metadata) => {
                    <InMemoryForestStorage<StorageProofsMerkleTrieLayout> as ForestStorage<
                        StorageProofsMerkleTrieLayout,
                        Runtime,
                    >>::insert_files_metadata(
                        &mut ephemeral_forest, &[metadata]
                    )
                    .map_err(|e| anyhow!("Failed to insert file key during catch-up: {:?}", e))?;
                }
                FileKeyOperation::Remove => {
                    // Remove the file key from the trie
                    <InMemoryForestStorage<StorageProofsMerkleTrieLayout> as ForestStorage<
                        StorageProofsMerkleTrieLayout,
                        Runtime,
                    >>::delete_file_key(
                        &mut ephemeral_forest, &change.file_key.into()
                    )
                    .map_err(|e| anyhow!("Failed to remove file key during catch-up: {:?}", e))?;
                }
            }
        }

        trace!(
            target: LOG_TARGET,
            "Updated ephemeral trie root: {:?}",
            <InMemoryForestStorage<StorageProofsMerkleTrieLayout> as ForestStorage<StorageProofsMerkleTrieLayout, Runtime>>::root(&ephemeral_forest)
        );

        // Generate proof for the specific file key being deleted
        let forest_proof_result =
            <InMemoryForestStorage<StorageProofsMerkleTrieLayout> as ForestStorage<
                StorageProofsMerkleTrieLayout,
                Runtime,
            >>::generate_proof(&ephemeral_forest, vec![(*file_key).into()])
            .map_err(|e| anyhow!("Failed to generate forest proof: {:?}", e))?;

        forest_proof_result
    };

    let owner_account = file_metadata.owner();
    let file_owner =
        <Runtime as frame_system::Config>::AccountId::try_from(owner_account.as_slice())
            .map_err(|_| anyhow!("Failed to convert file account to AccountId"))?;

    // Log all parameters
    trace!(
        target: LOG_TARGET,
        "File deletion parameters prepared:
        - File owner: {:?}
        - File key: 0x{}
        - Bucket ID: {:?}
        - Location: 0x{}
        - Size: {} bytes
        - Fingerprint: {:?}
        - Provider ID: {:?}
        - Forest proof keys: {} items",
        file_owner,
        hex::encode(file_key.as_ref()),
        bucket_id,
        hex::encode(&location),
        size,
        fingerprint,
        provider_id,
        forest_proof.proof.encoded_nodes.len()
    );

    Ok((
        file_owner,
        bucket_id,
        location,
        size,
        fingerprint.clone(),
        provider_id,
        forest_proof,
    ))
}
