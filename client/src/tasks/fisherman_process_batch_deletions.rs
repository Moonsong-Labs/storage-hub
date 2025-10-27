//! Fisherman batch file deletion task module.
//!
//! This module implements the single-task architecture for processing batched file deletions.
//! The [`FishermanTask`] handles [`BatchFileDeletions`] events emitted every 5 blocks by the
//! fisherman service, processes files grouped by target (BSP/Bucket), and submits batch
//! extrinsics for efficient deletion.
//!
//! ## Architecture
//!
//! - **Event-driven**: Responds to [`BatchFileDeletions`] events with specified deletion type
//! - **Batch processing**: Queries up to 1000 files per cycle from indexer database
//! - **Parallel execution**: Spawns futures for each target (BSP/Bucket) and processes them
//!   concurrently using [`futures::future::join_all`]
//! - **Lock management**: Always releases the global lock after processing, even on errors
//!
//! ## Processing Flow
//!
//! 1. Receive [`BatchFileDeletions`] event with deletion type (User or Incomplete)
//! 2. Query [`get_pending_deletions`] from indexer database
//! 3. Spawn [`FishermanTask::process_target_batch`] futures for each BSP and Bucket group
//! 4. Each target batch:
//!    - Builds ephemeral forest proof for all files in the batch
//!    - Submits appropriate extrinsic for the deletion type
//! 5. Await all futures with [`join_all`](futures::future::join_all)
//! 6. Release global lock via [`FishermanServiceCommandInterface::release_batch_lock`]

use anyhow::anyhow;
use codec::Decode;
use futures::future::join_all;
use sc_tracing::tracing::*;
use shc_actors_framework::{actor::ActorHandle, event_bus::EventHandler};
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface, types::SendExtrinsicOptions,
};
use shc_common::{
    traits::StorageEnableRuntime,
    types::{
        FileDeletionRequest, ForestProof as CommonForestProof, OffchainSignature,
        StorageProofsMerkleTrieLayout, StorageProviderId,
    },
};
use shc_fisherman_service::{
    commands::FishermanServiceCommandInterface, events::BatchFileDeletions,
    events::FileDeletionTarget, FileKeyOperation, FishermanService,
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

const LOG_TARGET: &str = "fisherman-batch-deletions-task";

/// Grouped pending deletions ready for batch processing.
///
/// Files are grouped by their deletion target (BSP or Bucket) to enable efficient
/// parallel processing of deletions. Each target can be processed independently
/// with its own forest proof.
#[derive(Debug, Clone)]
pub struct PendingDeletionsGrouped<Runtime: StorageEnableRuntime> {
    /// Files to delete from BSP forests, grouped by BSP ID
    pub bsp_deletions: std::collections::HashMap<
        shc_common::types::BackupStorageProviderId<Runtime>,
        Vec<BatchFileDeletionData<Runtime>>,
    >,
    /// Files to delete from bucket forests, grouped by bucket ID
    pub bucket_deletions: std::collections::HashMap<
        shc_common::types::BucketId<Runtime>,
        Vec<BatchFileDeletionData<Runtime>>,
    >,
}

/// Contains all metadata required to process file deletion operations.
///
/// Used for batch deletion processing to group files by target (BSP/Bucket) and
/// includes decoded signatures for user deletions or None for incomplete deletions.
#[derive(Debug, Clone)]
pub struct BatchFileDeletionData<Runtime: StorageEnableRuntime> {
    /// The file key (Merkle hash) uniquely identifying the file
    pub file_key: Runtime::Hash,
    /// File metadata (owner, bucket, location, size, fingerprint)
    pub file_metadata: shc_common::types::FileMetadata,
    /// Decoded signature for user deletions, [`None`] for incomplete deletions
    pub signature: Option<OffchainSignature<Runtime>>,
    /// Reconstructed signed file operation intention (only for user deletions)
    pub signed_intention: Option<shc_common::types::FileOperationIntention<Runtime>>,
}

/// Single task that handles [`BatchFileDeletions`] events.
///
/// This task processes batch deletion events emitted by the fisherman service every 5 blocks.
/// It queries pending deletions for the specified type (User or Incomplete), spawns parallel
/// futures for each target (BSP/Bucket), awaits their completion, and releases the global lock.
///
/// The task architecture ensures:
/// - No per-target locks (global lock prevents overlapping batch cycles)
/// - Parallel processing of all targets within a cycle
/// - Error isolation (one target's failure doesn't block others)
/// - Lock is always released, even on error
pub struct FishermanTask<NT, Runtime>
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

impl<NT, Runtime> Clone for FishermanTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> Self {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            fisherman_service: self.fisherman_service.clone(),
        }
    }
}

impl<NT, Runtime> FishermanTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    /// Creates a new [`FishermanTask`].
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
            .expect("FishermanTask requires fisherman service handle");

        Self {
            storage_hub_handler,
            fisherman_service,
        }
    }

    /// Process all files for a single target (BSP or Bucket).
    ///
    /// This method generates a forest proof for all files in the batch and submits the
    /// appropriate extrinsic based on the deletion type.
    ///
    /// # Arguments
    /// * `target` - The deletion target (BSP ID or Bucket ID)
    /// * `files` - Vector of files to delete for this target
    /// * `deletion_type` - Type of deletion (User or Incomplete)
    ///
    /// # Returns
    /// Result indicating success or failure of processing this target
    async fn batch_delete_files_for_target(
        &self,
        target: FileDeletionTarget<Runtime>,
        files: Vec<BatchFileDeletionData<Runtime>>,
        deletion_type: shc_indexer_db::models::FileDeletionType,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "🎣 Processing {} files for target {:?}",
            files.len(),
            target
        );

        // Extract file keys from files
        let file_keys: Vec<_> = files.iter().map(|f| f.file_key).collect();

        // Generate forest proof for all files in this target's batch
        // Returns only the file_keys that actually exist in the forest after catch-up
        let (valid_file_keys, provider_id, forest_proof) = self
            .build_forest_proof_for_deletions(&file_keys, target.clone())
            .await?;

        // Filter files to only include those with valid file keys
        // This ensures we only submit extrinsics for files that can be proven
        let valid_files: Vec<_> = files
            .into_iter()
            .filter(|f| valid_file_keys.contains(&f.file_key))
            .collect();

        debug!(
            target: LOG_TARGET,
            "🎣 Filtered {} files down to {} valid files for target {:?}",
            file_keys.len(),
            valid_files.len(),
            target
        );

        // Submit extrinsic for the deletion type with only valid files
        match deletion_type {
            shc_indexer_db::models::FileDeletionType::User => {
                self.submit_user_deletion_extrinsic(&valid_files, provider_id, forest_proof)
                    .await?;
            }
            shc_indexer_db::models::FileDeletionType::Incomplete => {
                self.submit_incomplete_deletion_extrinsic(
                    &valid_file_keys,
                    provider_id,
                    forest_proof,
                )
                .await?;
            }
        }

        info!(
            target: LOG_TARGET,
            "🎣 Successfully processed {} valid files for target {:?}",
            valid_files.len(),
            target
        );

        Ok(())
    }

    /// Generate forest proof for batch of files.
    ///
    /// This function:
    /// 1. Extracts bucket ID, provider ID from deletion target (all files in batch share same target)
    /// 2. Gets indexer DB connection
    /// 3. Builds ephemeral forest from indexer data at last finalized block
    /// 4. Applies catch-up changes from last finalized block to current best block
    /// 5. Filters file keys to only those that exist in the forest
    /// 6. Generates proof for the valid file keys
    /// 7. Returns filtered file keys, provider ID, and forest proof
    async fn build_forest_proof_for_deletions(
        &self,
        file_keys: &[Runtime::Hash],
        deletion_target: FileDeletionTarget<Runtime>,
    ) -> anyhow::Result<(
        Vec<Runtime::Hash>,
        Option<StorageProviderId<Runtime>>,
        CommonForestProof<StorageProofsMerkleTrieLayout>,
    )> {
        debug!(
            target: LOG_TARGET,
            "🎣 Generating forest proof for batch of {} files",
            file_keys.len()
        );

        // Determine provider ID from deletion target
        let provider_id = match &deletion_target {
            FileDeletionTarget::BspId(bsp_id) => {
                Some(StorageProviderId::BackupStorageProvider(*bsp_id))
            }
            FileDeletionTarget::BucketId(target_bucket_id) => {
                let maybe_msp_id = self
                    .storage_hub_handler
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
        let indexer_db_pool = self
            .storage_hub_handler
            .indexer_db_pool
            .as_ref()
            .ok_or_else(|| anyhow!("Indexer is disabled but batch deletion event was received"))?;

        let mut conn = indexer_db_pool
            .get()
            .await
            .map_err(|e| anyhow!("Failed to get indexer connection: {:?}", e))?;

        // Generate forest proof using two-phase ephemeral trie construction
        // Returns (valid_file_keys, forest_proof) - only keys that exist in the forest
        let (valid_file_keys, forest_proof) = {
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
            let mut all_file_metadatas = Vec::new();
            for key in &all_file_keys {
                let file = shc_indexer_db::models::File::get_by_file_key(&mut conn, key)
                    .await
                    .map_err(|e| anyhow!("Failed to get file: {:?}", e))?;

                let metadata = file
                    .to_file_metadata(file.onchain_bucket_id.clone())
                    .map_err(|e| anyhow!("Failed to convert file to metadata: {:?}", e))?;

                all_file_metadatas.push(metadata);
            }

            drop(conn);

            trace!(
                target: LOG_TARGET,
                "Building ephemeral trie with {} file keys from finalized data",
                all_file_keys.len(),
            );

            // Create ephemeral in-memory forest storage
            let mut ephemeral_forest =
                InMemoryForestStorage::<StorageProofsMerkleTrieLayout>::new();

            // TODO: Forests are constructed on the fly and fisherman tasks are run in parallel.
            // TODO: It is entirely possible that there me be more than 1 file deletion for the same bucket that
            // TODO: is submitted in the same block as another task. This means that only a single task will have successfully
            // TODO: deleted a file while the other tasks will have a invalid forest root.
            // TODO: We could adopt the same strategy as the InMemoryForestStorage which tracks per bucket forests and have a lock on it.

            // Insert all file keys from finalized data
            <InMemoryForestStorage<StorageProofsMerkleTrieLayout> as ForestStorage<
                StorageProofsMerkleTrieLayout,
                Runtime,
            >>::insert_files_metadata(&mut ephemeral_forest, &all_file_metadatas)
            .map_err(|e| anyhow!("Failed to insert file keys into ephemeral trie: {:?}", e))?;

            trace!(
                target: LOG_TARGET,
                "Ephemeral trie built with root: {:?}",
                <InMemoryForestStorage<StorageProofsMerkleTrieLayout> as ForestStorage<StorageProofsMerkleTrieLayout, Runtime>>::root(&ephemeral_forest)
            );

            trace!(
                target: LOG_TARGET,
                "Applying catch-up from block {} to best block",
                last_indexed_finalized_block
            );

            // Get file key changes since finalized block
            let file_key_changes = self
                .fisherman_service
                .get_file_key_changes_since_block(
                    last_indexed_finalized_block,
                    deletion_target.clone(),
                )
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
                        .map_err(|e| {
                            anyhow!("Failed to insert file key during catch-up: {:?}", e)
                        })?;
                    }
                    FileKeyOperation::Remove => {
                        // Remove the file key from the trie
                        <InMemoryForestStorage<StorageProofsMerkleTrieLayout> as ForestStorage<
                            StorageProofsMerkleTrieLayout,
                            Runtime,
                        >>::delete_file_key(
                            &mut ephemeral_forest, &change.file_key.into()
                        )
                        .map_err(|e| {
                            anyhow!("Failed to remove file key during catch-up: {:?}", e)
                        })?;
                    }
                }
            }

            trace!(
                target: LOG_TARGET,
                "Updated ephemeral trie root: {:?}",
                <InMemoryForestStorage<StorageProofsMerkleTrieLayout> as ForestStorage<StorageProofsMerkleTrieLayout, Runtime>>::root(&ephemeral_forest)
            );

            // Filter file keys to only those that exist in the forest
            // This is critical: files may have been deleted during catch-up or may not exist
            let mut valid_file_keys = Vec::new();
            for file_key in file_keys {
                match <InMemoryForestStorage<StorageProofsMerkleTrieLayout> as ForestStorage<
                    StorageProofsMerkleTrieLayout,
                    Runtime,
                >>::contains_file_key(&ephemeral_forest, &(*file_key).into())
                {
                    Ok(true) => {
                        valid_file_keys.push(*file_key);
                    }
                    Ok(false) => {
                        warn!(
                            target: LOG_TARGET,
                            "🎣 File key {:?} not found in forest after catch-up, skipping deletion",
                            file_key
                        );
                    }
                    Err(e) => {
                        error!(
                            target: LOG_TARGET,
                            "🎣 Error checking file key {:?} in forest: {:?}, skipping",
                            file_key,
                            e
                        );
                    }
                }
            }

            // If no valid files remain, return error
            if valid_file_keys.is_empty() {
                return Err(anyhow!(
                    "No valid file keys found in forest after filtering. All {} requested files are missing.",
                    file_keys.len()
                ));
            }

            debug!(
                target: LOG_TARGET,
                "🎣 Filtered {} file keys down to {} valid keys that exist in forest",
                file_keys.len(),
                valid_file_keys.len()
            );

            // Generate proof only for valid file keys
            let file_keys_for_proof: Vec<_> = valid_file_keys.iter().map(|k| (*k).into()).collect();

            debug!(
                target: LOG_TARGET,
                "🎣 Generating forest proof for {} valid file keys",
                file_keys_for_proof.len()
            );

            let forest_proof_result =
                <InMemoryForestStorage<StorageProofsMerkleTrieLayout> as ForestStorage<
                    StorageProofsMerkleTrieLayout,
                    Runtime,
                >>::generate_proof(&ephemeral_forest, file_keys_for_proof)
                .map_err(|e| anyhow!("Failed to generate forest proof: {:?}", e))?;

            (valid_file_keys, forest_proof_result)
        };

        debug!(
            target: LOG_TARGET,
            "🎣 Forest proof generated for {} valid files (out of {} requested), proof size: {} encoded nodes",
            valid_file_keys.len(),
            file_keys.len(),
            forest_proof.proof.encoded_nodes.len()
        );

        Ok((valid_file_keys, provider_id, forest_proof))
    }

    /// Submit user deletion extrinsic.
    ///
    /// This function:
    /// 1. Determines BSP ID from provider_id
    /// 2. Builds Vec<FileDeletionRequest> from files by extracting data from file_metadata
    /// 3. Converts to BoundedVec (respecting MaxFileDeletionsPerExtrinsic limit)
    /// 4. Builds pallet_file_system::Call::delete_files
    /// 5. Submits extrinsic with timeout
    async fn submit_user_deletion_extrinsic(
        &self,
        files: &[BatchFileDeletionData<Runtime>],
        provider_id: Option<StorageProviderId<Runtime>>,
        forest_proof: CommonForestProof<StorageProofsMerkleTrieLayout>,
    ) -> anyhow::Result<()> {
        debug!(
            target: LOG_TARGET,
            "🎣 Submitting user deletion extrinsic for {} files",
            files.len()
        );

        // Determine BSP ID from provider_id
        let maybe_bsp_id = match provider_id {
            Some(StorageProviderId::BackupStorageProvider(id)) => Some(id),
            Some(StorageProviderId::MainStorageProvider(_)) | None => None,
        };

        // Build Vec<FileDeletionRequest> for all files in the batch
        let mut file_deletion_requests = Vec::new();
        for file in files.iter() {
            // Extract signature and signed_intention from BatchFileDeletionData
            let signature = file
                .signature
                .clone()
                .ok_or_else(|| anyhow!("Missing signature for user deletion"))?;
            let signed_intention = file
                .signed_intention
                .clone()
                .ok_or_else(|| anyhow!("Missing signed intention for user deletion"))?;

            // Extract file data from file_metadata
            let file_owner =
                <Runtime as frame_system::Config>::AccountId::try_from(file.file_metadata.owner())
                    .map_err(|_| anyhow!("Failed to convert file account to AccountId"))?;
            let bucket_id = H256::from_slice(file.file_metadata.bucket_id());
            let location = file.file_metadata.location().to_vec();
            let size = file.file_metadata.file_size().saturated_into();
            let fingerprint = file.file_metadata.fingerprint().clone();

            let file_deletion = FileDeletionRequest {
                file_owner,
                signed_intention,
                signature,
                bucket_id,
                location: location
                    .try_into()
                    .map_err(|_| anyhow!("Location too long for file {:?}", file.file_key))?,
                size,
                fingerprint: H256::from_slice(fingerprint.as_ref()),
            };

            file_deletion_requests.push(file_deletion);
        }

        // Convert to BoundedVec
        let file_deletions = file_deletion_requests
            .try_into()
            .map_err(|_| anyhow!("Batch size exceeds MaxFileDeletionsPerExtrinsic limit"))?;

        // Build the delete_files extrinsic call
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
                SendExtrinsicOptions::new(Duration::from_secs(120)), // Longer timeout for batch
            )
            .await
            .map_err(|e| {
                error!(
                    target: LOG_TARGET,
                    "Failed to submit delete_files extrinsic for {} files: {:?}",
                    files.len(),
                    e
                );
                anyhow!("Failed to submit delete_files extrinsic: {:?}", e)
            })?;

        info!(
            target: LOG_TARGET,
            "🎣 Successfully submitted delete_files extrinsic for {} files",
            files.len()
        );

        Ok(())
    }

    /// Submit incomplete deletion extrinsic.
    ///
    /// This function:
    /// 1. Determines BSP ID from provider_id
    /// 2. Converts file keys to BoundedVec
    /// 3. Builds pallet_file_system::Call::delete_files_for_incomplete_storage_request
    /// 4. Submits extrinsic with timeout
    async fn submit_incomplete_deletion_extrinsic(
        &self,
        file_keys: &[Runtime::Hash],
        provider_id: Option<StorageProviderId<Runtime>>,
        forest_proof: CommonForestProof<StorageProofsMerkleTrieLayout>,
    ) -> anyhow::Result<()> {
        debug!(
            target: LOG_TARGET,
            "🎣 Submitting incomplete deletion extrinsic for {} files",
            file_keys.len()
        );

        // Determine BSP ID from provider_id
        let maybe_bsp_id = match provider_id {
            Some(StorageProviderId::BackupStorageProvider(id)) => Some(id),
            Some(StorageProviderId::MainStorageProvider(_)) | None => None,
        };

        // Convert file keys to the required format and wrap in BoundedVec
        let file_keys_vec: Vec<_> = file_keys.iter().map(|k| (*k).into()).collect();
        let file_keys_bounded = file_keys_vec
            .try_into()
            .map_err(|_| anyhow!("Batch size exceeds MaxFileDeletionsPerExtrinsic limit"))?;

        // Build the delete_files_for_incomplete_storage_request extrinsic call
        let call =
            pallet_file_system::Call::<Runtime>::delete_files_for_incomplete_storage_request {
                file_keys: file_keys_bounded,
                bsp_id: maybe_bsp_id,
                forest_proof: forest_proof.proof,
            };

        // Submit the extrinsic
        self.storage_hub_handler
            .blockchain
            .send_extrinsic(
                call.into(),
                SendExtrinsicOptions::new(Duration::from_secs(120)), // Longer timeout for batch
            )
            .await
            .map_err(|e| {
                error!(
                    target: LOG_TARGET,
                    "Failed to submit delete_files_for_incomplete_storage_request extrinsic for {} files: {:?}",
                    file_keys.len(),
                    e
                );
                anyhow!(
                    "Failed to submit delete_files_for_incomplete_storage_request extrinsic: {:?}",
                    e
                )
            })?;

        info!(
            target: LOG_TARGET,
            "🎣 Successfully submitted delete_files_for_incomplete_storage_request extrinsic for {} files",
            file_keys.len()
        );

        Ok(())
    }

    /// Get pending deletions grouped by BSP and Bucket.
    ///
    /// Queries the indexer database for files marked with `deletion_status = InProgress`,
    /// filtered by the specified deletion type.
    ///
    /// # Parameters
    /// * `deletion_type` - Type of deletion to query ([`shc_indexer_db::models::FileDeletionType::User`] or [`shc_indexer_db::models::FileDeletionType::Incomplete`])
    /// * `bucket_id` - Optional filter to only return files from a specific bucket
    /// * `bsp_id` - Optional filter to only return files from a specific BSP
    /// * `limit` - Maximum number of files to return (default: 1000)
    /// * `offset` - Number of files to skip for pagination (default: 0)
    async fn get_pending_deletions(
        &self,
        deletion_type: shc_indexer_db::models::FileDeletionType,
        bucket_id: Option<shc_common::types::BucketId<Runtime>>,
        bsp_id: Option<shc_common::types::BackupStorageProviderId<Runtime>>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> anyhow::Result<PendingDeletionsGrouped<Runtime>> {
        trace!(
            target: LOG_TARGET,
            "🎣 Fetching pending {:?} deletions (bucket_id: {:?}, bsp_id: {:?}, limit: {:?}, offset: {:?})",
            deletion_type, bucket_id, bsp_id, limit, offset
        );

        // Get indexer DB pool
        let indexer_db_pool = self
            .storage_hub_handler
            .indexer_db_pool
            .as_ref()
            .ok_or_else(|| anyhow!("Indexer is disabled but batch deletion event was received"))?;

        // Clone connection pools for parallel tasks
        let pool_for_bucket = indexer_db_pool.clone();
        let pool_for_bsp = indexer_db_pool.clone();

        // Execute both pipelines concurrently: each queries + converts its own data
        let bucket_task = async move {
            // Get DB connection for concurrent query
            let mut bucket_conn = pool_for_bucket
                .get()
                .await
                .map_err(|e| anyhow!("Failed to get bucket DB connection: {:?}", e))?;

            // Convert bucket_id from Runtime type to DB type
            let bucket_id_bytes = bucket_id.as_ref().map(|id| id.as_ref() as &[u8]);

            // Query bucket files from DB
            let bucket_files =
                shc_indexer_db::models::File::get_files_pending_deletion_grouped_by_bucket(
                    &mut bucket_conn,
                    deletion_type,
                    bucket_id_bytes,
                    limit,
                    offset,
                )
                .await?;

            drop(bucket_conn);

            // Convert bucket files to Runtime types
            convert_bucket_files_to_runtime::<Runtime>(bucket_files)
        };

        let bsp_task = async move {
            // Get DB connection for concurrent query
            let mut bsp_conn = pool_for_bsp
                .get()
                .await
                .map_err(|e| anyhow!("Failed to get BSP DB connection: {:?}", e))?;

            // Convert bsp_id from Runtime type to DB type
            let bsp_id_db = bsp_id.map(shc_indexer_db::OnchainBspId::new);

            // Query BSP files from DB
            let bsp_files = BspFile::get_files_pending_deletion_grouped_by_bsp(
                &mut bsp_conn,
                deletion_type,
                bsp_id_db,
                limit,
                offset,
            )
            .await?;

            drop(bsp_conn);

            // Convert BSP files to Runtime types
            convert_bsp_files_to_runtime::<Runtime>(bsp_files)
        };

        // Execute both pipelines concurrently
        let (bucket_deletions, bsp_deletions) = tokio::try_join!(bucket_task, bsp_task)?;

        debug!(
            target: LOG_TARGET,
            "🎣 Found {} bucket groups and {} BSP groups with pending {:?} deletions",
            bucket_deletions.len(),
            bsp_deletions.len(),
            deletion_type
        );

        Ok(PendingDeletionsGrouped {
            bsp_deletions,
            bucket_deletions,
        })
    }
}

impl<NT, Runtime> EventHandler<BatchFileDeletions> for FishermanTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: BatchFileDeletions) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "🎣 Processing batch file deletions for {:?} deletion type",
            event.deletion_type
        );

        // Query pending deletions with limit of 1000 files
        let grouped_deletions = self
            .get_pending_deletions(event.deletion_type, None, None, Some(1000), None)
            .await?;

        debug!(
            target: LOG_TARGET,
            "🎣 Found {} BSP groups and {} bucket groups to process",
            grouped_deletions.bsp_deletions.len(),
            grouped_deletions.bucket_deletions.len()
        );

        // Spawn futures for each target
        let mut futures = Vec::new();

        // Spawn for each BSP group
        for (bsp_id, files) in grouped_deletions.bsp_deletions {
            debug!(
                target: LOG_TARGET,
                "🎣 Spawning future for BSP {:?} with {} files",
                bsp_id,
                files.len()
            );

            let future = self.batch_delete_files_for_target(
                FileDeletionTarget::BspId(bsp_id),
                files,
                event.deletion_type,
            );
            futures.push(future);
        }

        // Spawn for each Bucket group
        for (bucket_id, files) in grouped_deletions.bucket_deletions {
            debug!(
                target: LOG_TARGET,
                "🎣 Spawning future for Bucket {:?} with {} files",
                bucket_id,
                files.len()
            );

            let future = self.batch_delete_files_for_target(
                FileDeletionTarget::BucketId(bucket_id),
                files,
                event.deletion_type,
            );
            futures.push(future);
        }

        // Check if there's any work to do
        if futures.is_empty() {
            debug!(
                target: LOG_TARGET,
                "🎣 No pending {:?} deletions found, releasing lock",
                event.deletion_type
            );
            // Release lock and return early
            self.fisherman_service.release_batch_lock().await?;
            return Ok(());
        }

        // Await all futures
        debug!(
            target: LOG_TARGET,
            "🎣 Awaiting {} target processing futures",
            futures.len()
        );
        let results = join_all(futures).await;

        // Log results
        let successes = results.iter().filter(|r| r.is_ok()).count();
        let failures = results.iter().filter(|r| r.is_err()).count();

        if failures > 0 {
            warn!(
                target: LOG_TARGET,
                "🎣 Batch processing complete: {} successes, {} failures",
                successes,
                failures
            );

            // Log individual errors
            for (idx, result) in results.iter().enumerate() {
                if let Err(e) = result {
                    error!(
                        target: LOG_TARGET,
                        "🎣 Target {} failed: {:?}",
                        idx,
                        e
                    );
                }
            }
        } else {
            info!(
                target: LOG_TARGET,
                "🎣 Batch processing complete: {} successes, 0 failures",
                successes
            );
        }

        // Always release lock, even if some targets failed
        info!(
            target: LOG_TARGET,
            "🎣 Releasing batch processing lock"
        );
        self.fisherman_service.release_batch_lock().await?;

        Ok(())
    }
}

// ============================================================================
// Helper functions for database conversion
// ============================================================================

/// Convert DB bucket files to Runtime types.
///
/// Transforms the database representation to runtime types and converts
/// individual files to [`BatchFileDeletionData`].
fn convert_bucket_files_to_runtime<Runtime: StorageEnableRuntime>(
    db_files: std::collections::HashMap<Vec<u8>, Vec<shc_indexer_db::models::File>>,
) -> anyhow::Result<
    std::collections::HashMap<
        shc_common::types::BucketId<Runtime>,
        Vec<BatchFileDeletionData<Runtime>>,
    >,
> {
    let mut result = std::collections::HashMap::new();

    for (bucket_id_bytes, files) in db_files {
        // Convert bucket ID from database type to Runtime type using SCALE codec
        let bucket_id =
            shc_common::types::BucketId::<Runtime>::decode(&mut bucket_id_bytes.as_slice())
                .map_err(|e| anyhow!("Failed to decode bucket ID: {:?}", e))?;

        // Convert files
        let file_data: Result<Vec<_>, _> = files
            .into_iter()
            .map(|file| convert_file_to_deletion_data::<Runtime>(file))
            .collect();

        result.insert(bucket_id, file_data?);
    }

    Ok(result)
}

/// Convert DB BSP files to Runtime types.
///
/// Transforms the database representation to runtime types and converts
/// individual files to [`BatchFileDeletionData`].
fn convert_bsp_files_to_runtime<Runtime: StorageEnableRuntime>(
    db_files: std::collections::HashMap<
        shc_indexer_db::OnchainBspId,
        Vec<shc_indexer_db::models::File>,
    >,
) -> anyhow::Result<
    std::collections::HashMap<
        shc_common::types::BackupStorageProviderId<Runtime>,
        Vec<BatchFileDeletionData<Runtime>>,
    >,
> {
    let mut result = std::collections::HashMap::new();

    for (bsp_id, files) in db_files {
        // Convert BSP ID from database type to Runtime type
        let provider_id = bsp_id.into_h256();

        // Convert files
        let file_data: Result<Vec<_>, _> = files
            .into_iter()
            .map(|file| convert_file_to_deletion_data::<Runtime>(file))
            .collect();

        result.insert(provider_id, file_data?);
    }

    Ok(result)
}

/// Convert single DB File to [`BatchFileDeletionData`].
///
/// Handles conversion of all file metadata and decodes signatures for user deletions.
/// For user deletions, reconstructs the [`FileOperationIntention`] from the file key.
fn convert_file_to_deletion_data<Runtime: StorageEnableRuntime>(
    file: shc_indexer_db::models::File,
) -> anyhow::Result<BatchFileDeletionData<Runtime>> {
    // Convert file key from database type to Runtime type using SCALE codec
    let file_key = Runtime::Hash::decode(&mut file.file_key.as_slice())
        .map_err(|e| anyhow!("Failed to decode file key: {:?}", e))?;

    // Convert to FileMetadata
    let file_metadata = file
        .to_file_metadata(file.onchain_bucket_id.clone())
        .map_err(|e| anyhow!("Failed to convert file to metadata: {:?}", e))?;

    // Decode signature if present (user deletions)
    let (signature, signed_intention) = if let Some(sig_bytes) = &file.deletion_signature {
        // Decode signature from SCALE-encoded bytes
        let signature = OffchainSignature::<Runtime>::decode(&mut &sig_bytes[..])
            .map_err(|e| anyhow!("Failed to decode signature: {:?}", e))?;

        // Reconstruct FileOperationIntention
        let intention = shc_common::types::FileOperationIntention {
            file_key,
            operation: shc_common::types::FileOperation::Delete,
        };

        (Some(signature), Some(intention))
    } else {
        // No signature for incomplete deletions
        (None, None)
    };

    Ok(BatchFileDeletionData {
        file_key,
        file_metadata,
        signature,
        signed_intention,
    })
}
