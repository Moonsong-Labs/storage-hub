use anyhow::anyhow;
use codec::Decode;
use futures::future::join_all;
use ordered_float::OrderedFloat;
use priority_queue::PriorityQueue;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use std::{
    cmp::max,
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{RwLock, Semaphore};

use sc_network::PeerId;
use sc_tracing::tracing::*;
use sp_core::H256;

use pallet_file_system::types::BucketMoveRequestResponse;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceInterface,
    events::{MoveBucketRequestedForMsp, StartMovedBucketDownload},
    types::{RetryStrategy, SendExtrinsicOptions},
};
use shc_common::types::{
    BucketId, FileKeyProof, FileMetadata, HashT, ProviderId, StorageProofsMerkleTrieLayout,
    StorageProviderId,
};
use shc_file_manager::traits::FileStorage;
use shc_file_transfer_service::{
    commands::FileTransferServiceInterface, schema::v1::provider::RemoteDownloadDataResponse,
};
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use shp_constants::FILE_CHUNK_SIZE;
use shp_file_metadata::{Chunk, ChunkId, Leaf as ProvenLeaf};
use storage_hub_runtime::StorageDataUnit;

use crate::services::{
    handler::StorageHubHandler,
    types::{MspForestStorageHandlerT, ShNodeType},
};

lazy_static::lazy_static! {
    static ref GLOBAL_RNG: Mutex<StdRng> = Mutex::new(StdRng::from_entropy());
}

const LOG_TARGET: &str = "msp-move-bucket-task";

/// Maximum number of files to download in parallel
const MAX_CONCURRENT_FILE_DOWNLOADS: usize = 10;
/// Maximum number of chunks requests to do in parallel per file
const MAX_CONCURRENT_CHUNKS_PER_FILE: usize = 5;
/// Maximum number of chunks to request in a single network request
const MAX_CHUNKS_PER_REQUEST: usize = 10;
/// Number of peers to select for each chunk download attempt (2 best + 3 random)
const CHUNK_REQUEST_PEER_RETRY_ATTEMPTS: usize = 5;
/// Number of retries per peer for a single chunk request
const DOWNLOAD_RETRY_ATTEMPTS: usize = 2;

/// [`MspRespondMoveBucketTask`] handles bucket move requests between MSPs.
///
/// # Event Handling
/// This task handles both:
/// - [`MoveBucketRequestedForMsp`] event which is emitted when a user requests to move their bucket
/// - [`StartMovedBucketDownload`] event which is emitted when a bucket move is confirmed
///
/// # Lifecycle
/// 1. When a move bucket request is received:
///    - Verifies that indexer is enabled and accessible
///    - Checks if there is sufficient storage capacity via [`MspMoveBucketTask::check_and_increase_capacity`]
///    - Validates that all files in the bucket can be handled
///    - Inserts file metadata into local storage and forest storage
///    - Verifies BSP peer IDs are available for each file
///
/// 2. If all validations pass:
///    - Accepts the move request via [`MspMoveBucketTask::accept_bucket_move`]
///    - Downloads all files in parallel using [`MspMoveBucketTask::download_file`] with controlled concurrency
///    - Updates local forest root to match on-chain state
///
/// 3. If any validation fails:
///    - Rejects the move request via [`MspMoveBucketTask::reject_bucket_move`]
///    - Cleans up any partially inserted file metadata
///    - Removes any created forest storage
///
/// # Error Handling
/// The task will reject the bucket move request if:
/// - Indexer is disabled or inaccessible
/// - Insufficient storage capacity and unable to increase
/// - File metadata insertion fails
/// - BSP peer IDs are unavailable
/// - Database connection issues occur
pub struct MspRespondMoveBucketTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<NT>,
    peer_manager: Arc<RwLock<BspPeerManager>>,
    pending_bucket_id: Option<BucketId>,
    file_storage_inserted_file_keys: Vec<H256>,
}

impl<NT> Clone for MspRespondMoveBucketTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
{
    fn clone(&self) -> MspRespondMoveBucketTask<NT> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            peer_manager: self.peer_manager.clone(),
            pending_bucket_id: self.pending_bucket_id.clone(),
            file_storage_inserted_file_keys: self.file_storage_inserted_file_keys.clone(),
        }
    }
}

impl<NT> MspRespondMoveBucketTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT>) -> Self {
        Self {
            storage_hub_handler,
            peer_manager: Arc::new(RwLock::new(BspPeerManager::new())),
            pending_bucket_id: None,
            file_storage_inserted_file_keys: Vec::new(),
        }
    }
}

impl<NT> EventHandler<MoveBucketRequestedForMsp> for MspRespondMoveBucketTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: MoveBucketRequestedForMsp) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "MSP: user requested to move bucket {:?} to us",
            event.bucket_id,
        );

        if let Err(error) = self.handle_move_bucket_request(event.clone()).await {
            // TODO: Based on the error, we should persist the bucket move request and retry later.
            error!(
                target: LOG_TARGET,
                "Failed to handle move bucket request: {:?}",
                error
            );
            return self.reject_bucket_move(event.bucket_id).await;
        }

        Ok(())
    }
}

impl<NT> EventHandler<StartMovedBucketDownload> for MspRespondMoveBucketTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: StartMovedBucketDownload) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "StartMovedBucketDownload: Starting download process for bucket {:?}",
            event.bucket_id
        );

        let indexer_db_pool = if let Some(indexer_db_pool) =
            self.storage_hub_handler.indexer_db_pool.clone()
        {
            indexer_db_pool
        } else {
            return Err(anyhow!("Indexer is disabled but a move bucket event was received. Please provide a database URL (and enable indexer) for it to use this feature."));
        };

        let mut indexer_connection = indexer_db_pool.get().await.map_err(|error| {
            anyhow!(
                "Failed to get indexer connection after timeout: {:?}",
                error
            )
        })?;

        let bucket = event.bucket_id.as_ref().to_vec();
        let files = shc_indexer_db::models::File::get_by_onchain_bucket_id(
            &mut indexer_connection,
            bucket.clone(),
        )
        .await?;

        let total_files = files.len();

        // Create forest storage for the bucket if it doesn't exist
        let _ = self
            .storage_hub_handler
            .forest_storage_handler
            .get_or_create(&bucket)
            .await;

        // Create semaphore for file-level parallelism
        let file_semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_FILE_DOWNLOADS));
        let file_tasks: Vec<_> = files
            .into_iter()
            .map(|file| {
                let semaphore = Arc::clone(&file_semaphore);
                let task = self.clone();
                let bucket_id = event.bucket_id.clone();

                tokio::spawn(async move {
                    let _permit = semaphore
                        .acquire()
                        .await
                        .map_err(|e| anyhow!("Failed to acquire file semaphore: {:?}", e))?;

                    let file_metadata = file.to_file_metadata(bucket_id.as_ref().to_vec());
                    let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();

                    // Get BSP peer IDs and register them
                    let bsp_peer_ids = file
                        .get_bsp_peer_ids(
                            &mut task
                                .storage_hub_handler
                                .indexer_db_pool
                                .as_ref()
                                .unwrap()
                                .get()
                                .await?,
                        )
                        .await?;

                    if bsp_peer_ids.is_empty() {
                        return Err(anyhow!("No BSP peer IDs found for file {:?}", file_key));
                    }

                    // Register BSP peers for file transfer
                    {
                        let mut peer_manager = task.peer_manager.write().await;
                        for &peer_id in &bsp_peer_ids {
                            peer_manager.add_peer(peer_id, file_key);
                        }
                    }

                    // Download file using existing download_file method
                    task.download_file(&file, &bucket_id).await
                })
            })
            .collect();

        // Wait for all file downloads to complete
        let results = join_all(file_tasks).await;

        // Process results and count failures
        let mut failed_downloads = 0;
        for result in results {
            match result {
                Ok(download_result) => {
                    if let Err(e) = download_result {
                        error!(
                            target: LOG_TARGET,
                            "File download task failed: {:?}", e
                        );
                        failed_downloads += 1;
                    }
                }
                Err(e) => {
                    error!(
                        target: LOG_TARGET,
                        "File download task panicked: {:?}", e
                    );
                    failed_downloads += 1;
                }
            }
        }

        if failed_downloads > 0 {
            return Err(anyhow!(
                "Failed to download {} out of {} files",
                failed_downloads,
                total_files
            ));
        } else {
            info!(
                target: LOG_TARGET,
                "Successfully completed bucket move with all files downloaded"
            );
        }

        Ok(())
    }
}

impl<NT> MspRespondMoveBucketTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
{
    /// Internal implementation of the move bucket request handling.
    /// This function contains the core logic for processing a bucket move request.
    /// If it returns an error, the caller (handle_event) will reject the bucket move request.
    async fn handle_move_bucket_request(
        &mut self,
        event: MoveBucketRequestedForMsp,
    ) -> anyhow::Result<()> {
        let indexer_db_pool = if let Some(indexer_db_pool) =
            self.storage_hub_handler.indexer_db_pool.clone()
        {
            indexer_db_pool
        } else {
            return Err(anyhow!("Indexer is disabled but a move bucket event was received. Please provide a database URL (and enable indexer) for it to use this feature."));
        };

        let mut indexer_connection = indexer_db_pool.get().await.map_err(|error| {
            anyhow!(
                "CRITICAL ❗️❗️❗️: Failed to get indexer connection after timeout: {:?}",
                error
            )
        })?;
        let bucket = event.bucket_id.as_ref().to_vec();

        let forest_storage = self
            .storage_hub_handler
            .forest_storage_handler
            .get_or_create(&bucket)
            .await;

        self.pending_bucket_id = Some(event.bucket_id);

        let files = shc_indexer_db::models::File::get_by_onchain_bucket_id(
            &mut indexer_connection,
            bucket.clone(),
        )
        .await?;

        let total_size: u64 = files
            .iter()
            .try_fold(0u64, |acc, file| acc.checked_add(file.size as u64))
            .ok_or_else(|| {
                anyhow!("Total size calculation overflowed u64 - bucket is too large")
            })?;

        let own_provider_id = self
            .storage_hub_handler
            .blockchain
            .query_storage_provider_id(None)
            .await?;

        let own_msp_id = match own_provider_id {
            Some(StorageProviderId::MainStorageProvider(id)) => id,
            Some(StorageProviderId::BackupStorageProvider(_)) => {
                return Err(anyhow!("CRITICAL ❗️❗️❗️: Current node account is a Backup Storage Provider. Expected a Main Storage Provider ID."));
            }
            None => {
                return Err(anyhow!("CRITICAL ❗️❗️❗️: Failed to get own MSP ID."));
            }
        };

        // Check and increase capacity if needed
        self.check_and_increase_capacity(total_size, own_msp_id)
            .await?;

        // Try to insert all files before accepting the request
        for file in &files {
            let file_metadata = file.to_file_metadata(bucket.clone());
            let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();

            self.storage_hub_handler
                .file_storage
                .write()
                .await
                .insert_file(file_key.clone(), file_metadata.clone())
                .map_err(|error| {
                    anyhow!(
                        "CRITICAL ❗️❗️❗️: Failed to insert file {:?} into file storage: {:?}",
                        file_key,
                        error
                    )
                })?;

            self.file_storage_inserted_file_keys.push(file_key);

            forest_storage
                .write()
                .await
                .insert_files_metadata(&[file_metadata.clone()])
                .map_err(|error| {
                    anyhow!(
                        "CRITICAL ❗️❗️❗️: Failed to insert file {:?} into forest storage: {:?}",
                        file_key,
                        error
                    )
                })?;

            let bsp_peer_ids = file.get_bsp_peer_ids(&mut indexer_connection).await?;
            if bsp_peer_ids.is_empty() {
                return Err(anyhow!("No BSP peer IDs found for file {:?}", file_key));
            }
        }

        // Accept the request since we've verified we can handle all files
        self.accept_bucket_move(event.bucket_id).await?;

        Ok(())
    }

    /// Rejects a bucket move request and performs cleanup of any partially created resources.
    ///
    /// # Arguments
    /// - `bucket_id` - The ID of the bucket whose move request is being rejected
    ///
    /// # Cleanup Steps
    /// 1. Deletes any files that were inserted into file storage during validation
    /// 2. Removes the forest storage if it was created for this bucket
    /// 3. Sends an extrinsic to reject the move request on-chain
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to send or confirm the rejection extrinsic
    /// Note: Cleanup errors are logged but don't prevent the rejection from being sent
    async fn reject_bucket_move(&mut self, bucket_id: BucketId) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "MSP: rejecting move bucket request for bucket {:?}",
            bucket_id.as_ref(),
        );

        for file_key in self.file_storage_inserted_file_keys.iter() {
            if let Err(error) = self
                .storage_hub_handler
                .file_storage
                .write()
                .await
                .delete_file(file_key)
            {
                error!(
                    target: LOG_TARGET,
                    "IMPORTANT ❗️❗️❗️: Failed to delete (move bucket rollback) file {:?} from file storage: {:?}",
                    file_key, error
                );
            }
        }

        if let Some(bucket_id) = self.pending_bucket_id {
            self.storage_hub_handler
                .forest_storage_handler
                .remove_forest_storage(&bucket_id.as_ref().to_vec())
                .await;
        }

        let call = storage_hub_runtime::RuntimeCall::FileSystem(
            pallet_file_system::Call::msp_respond_move_bucket_request {
                bucket_id,
                response: BucketMoveRequestResponse::Rejected,
            },
        );

        self.storage_hub_handler
            .blockchain
            .submit_extrinsic_with_retry(
                call,
                RetryStrategy::default()
                    .with_max_retries(3)
                    .with_max_tip(10.0)
                    .with_timeout(Duration::from_secs(
                        self.storage_hub_handler
                            .provider_config
                            .extrinsic_retry_timeout,
                    )),
                false,
            )
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to submit move bucket rejection after 3 retries: {:?}",
                    e
                )
            })?;

        Ok(())
    }

    async fn accept_bucket_move(&self, bucket_id: BucketId) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "MSP: accepting move bucket request for bucket {:?}",
            bucket_id.as_ref(),
        );

        let call = storage_hub_runtime::RuntimeCall::FileSystem(
            pallet_file_system::Call::msp_respond_move_bucket_request {
                bucket_id,
                response: BucketMoveRequestResponse::Accepted,
            },
        );

        info!(
            target: LOG_TARGET,
            "MSP: accepting move bucket request for bucket {:?}",
            bucket_id.as_ref(),
        );

        self.storage_hub_handler
            .blockchain
            .submit_extrinsic_with_retry(
                call,
                RetryStrategy::default()
                    .with_max_retries(3)
                    .with_max_tip(10.0)
                    .with_timeout(Duration::from_secs(
                        self.storage_hub_handler
                            .provider_config
                            .extrinsic_retry_timeout,
                    )),
                false,
            )
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to submit move bucket acceptance after 3 retries: {:?}",
                    e
                )
            })?;

        Ok(())
    }

    async fn check_and_increase_capacity(
        &self,
        required_size: u64,
        own_msp_id: ProviderId,
    ) -> anyhow::Result<()> {
        let available_capacity = self
            .storage_hub_handler
            .blockchain
            .query_available_storage_capacity(own_msp_id)
            .await
            .map_err(|e| {
                let err_msg = format!("Failed to query available storage capacity: {:?}", e);
                error!(target: LOG_TARGET, err_msg);
                anyhow::anyhow!(err_msg)
            })?;

        // Increase storage capacity if the available capacity is less than the required size
        if available_capacity < required_size {
            warn!(
                target: LOG_TARGET,
                "Insufficient storage capacity to accept bucket move. Available: {}, Required: {}",
                available_capacity,
                required_size
            );

            let current_capacity = self
                .storage_hub_handler
                .blockchain
                .query_storage_provider_capacity(own_msp_id)
                .await
                .map_err(|e| {
                    let err_msg = format!("Failed to query storage provider capacity: {:?}", e);
                    error!(target: LOG_TARGET, err_msg);
                    anyhow::anyhow!(err_msg)
                })?;

            let max_storage_capacity = self
                .storage_hub_handler
                .provider_config
                .max_storage_capacity;

            if max_storage_capacity == current_capacity {
                let err_msg =
                    "Reached maximum storage capacity limit. Unable to add more storage capacity.";
                warn!(target: LOG_TARGET, err_msg);
                return Err(anyhow::anyhow!(err_msg));
            }

            let new_capacity = self.calculate_capacity(required_size, current_capacity)?;

            let call = storage_hub_runtime::RuntimeCall::Providers(
                pallet_storage_providers::Call::change_capacity { new_capacity },
            );

            let earliest_change_capacity_block = self
                .storage_hub_handler
                .blockchain
                .query_earliest_change_capacity_block(own_msp_id)
                .await
                .map_err(|e| {
                    error!(
                        target: LOG_TARGET,
                        "Failed to query earliest change capacity block: {:?}", e
                    );
                    anyhow::anyhow!("Failed to query earliest change capacity block: {:?}", e)
                })?;

            // Wait for the earliest block where the capacity can be changed
            self.storage_hub_handler
                .blockchain
                .wait_for_block(earliest_change_capacity_block)
                .await?;

            self.storage_hub_handler
                .blockchain
                .send_extrinsic(call, SendExtrinsicOptions::default())
                .await?
                .with_timeout(Duration::from_secs(60))
                .watch_for_success(&self.storage_hub_handler.blockchain)
                .await?;

            info!(
                target: LOG_TARGET,
                "Increased storage capacity to {:?} bytes",
                new_capacity
            );

            let available_capacity = self
                .storage_hub_handler
                .blockchain
                .query_available_storage_capacity(own_msp_id)
                .await
                .map_err(|e| {
                    error!(
                        target: LOG_TARGET,
                        "Failed to query available storage capacity: {:?}", e
                    );
                    anyhow::anyhow!("Failed to query available storage capacity: {:?}", e)
                })?;

            // Reject bucket move if the new available capacity is still less than required
            if available_capacity < required_size {
                let err_msg =
                    "Increased storage capacity is still insufficient to accept bucket move.";
                warn!(target: LOG_TARGET, "{}", err_msg);
                return Err(anyhow::anyhow!(err_msg));
            }
        }

        Ok(())
    }

    fn calculate_capacity(
        &self,
        required_size: u64,
        current_capacity: StorageDataUnit,
    ) -> Result<StorageDataUnit, anyhow::Error> {
        let jump_capacity = self.storage_hub_handler.provider_config.jump_capacity;
        let jumps_needed = (required_size + jump_capacity - 1) / jump_capacity;
        let jumps = max(jumps_needed, 1);
        let bytes_to_add = jumps * jump_capacity;
        let required_capacity = current_capacity.checked_add(bytes_to_add).ok_or_else(|| {
            anyhow::anyhow!("Reached maximum storage capacity limit. Cannot accept bucket move.")
        })?;

        let max_storage_capacity = self
            .storage_hub_handler
            .provider_config
            .max_storage_capacity;

        let new_capacity = std::cmp::min(required_capacity, max_storage_capacity);

        Ok(new_capacity)
    }

    /// Processes a single chunk download response
    async fn process_chunk_download_response(
        &self,
        file_key: H256,
        file_metadata: &FileMetadata,
        chunk_batch: &HashSet<ChunkId>,
        peer_id: PeerId,
        download_request: RemoteDownloadDataResponse,
        peer_manager: &Arc<RwLock<BspPeerManager>>,
        batch_size_bytes: u64,
        start_time: std::time::Instant,
    ) -> Result<bool, anyhow::Error> {
        let file_key_proof = FileKeyProof::decode(&mut download_request.file_key_proof.as_ref())
            .map_err(|e| anyhow!("Failed to decode file key proof: {:?}", e))?;

        // Verify fingerprint
        let expected_fingerprint = file_metadata.fingerprint;
        if file_key_proof.file_metadata.fingerprint != expected_fingerprint {
            let mut peer_manager = peer_manager.write().await;
            peer_manager.record_failure(peer_id);
            return Err(anyhow!(
                "Fingerprint mismatch. Expected: {:?}, got: {:?}",
                expected_fingerprint,
                file_key_proof.file_metadata.fingerprint
            ));
        }

        let proven = file_key_proof
            .proven::<StorageProofsMerkleTrieLayout>()
            .map_err(|e| anyhow!("Failed to get proven data: {:?}", e))?;

        if proven.len() != chunk_batch.len() {
            let mut peer_manager = peer_manager.write().await;
            peer_manager.record_failure(peer_id);
            return Err(anyhow!(
                "Expected {} proven chunks but got {}",
                chunk_batch.len(),
                proven.len()
            ));
        }

        // Process each proven chunk
        for proven_chunk in proven {
            self.process_proven_chunk(file_key, file_metadata, proven_chunk)
                .await?;
        }

        let download_time = start_time.elapsed();
        let mut peer_manager = peer_manager.write().await;
        peer_manager.record_success(peer_id, batch_size_bytes, download_time.as_millis() as u64);

        Ok(true)
    }

    /// Processes a single proven chunk
    async fn process_proven_chunk(
        &self,
        file_key: H256,
        file_metadata: &FileMetadata,
        proven_chunk: ProvenLeaf<ChunkId, Chunk>,
    ) -> Result<(), anyhow::Error> {
        let chunk_id = proven_chunk.key;
        let chunk_data = proven_chunk.data;

        // Validate chunk size
        let chunk_idx = chunk_id.as_u64();
        let expected_chunk_size = file_metadata.chunk_size_at(chunk_idx);

        if chunk_data.len() != expected_chunk_size {
            return Err(anyhow!(
                "Invalid chunk size for chunk {}: Expected: {}, got: {}",
                chunk_idx,
                expected_chunk_size,
                chunk_data.len()
            ));
        }

        self.storage_hub_handler
            .file_storage
            .write()
            .await
            .write_chunk(&file_key, &chunk_id, &chunk_data)
            .map_err(|error| anyhow!("Failed to write chunk {}: {:?}", chunk_idx, error))?;

        Ok(())
    }

    /// Attempts to download a batch of chunks from a specific peer
    async fn try_download_chunk_batch(
        &self,
        peer_id: PeerId,
        file_key: H256,
        file_metadata: &FileMetadata,
        chunk_batch: &HashSet<ChunkId>,
        bucket: &BucketId,
        peer_manager: &Arc<RwLock<BspPeerManager>>,
        batch_size_bytes: u64,
    ) -> Result<bool, anyhow::Error> {
        for attempt in 0..=DOWNLOAD_RETRY_ATTEMPTS {
            if attempt > 0 {
                warn!(
                    target: LOG_TARGET,
                    "Retrying download with peer {:?} (attempt {}/{})",
                    peer_id,
                    attempt + 1,
                    DOWNLOAD_RETRY_ATTEMPTS + 1
                );
            }

            let start_time = std::time::Instant::now();

            match self
                .storage_hub_handler
                .file_transfer
                .download_request(
                    peer_id,
                    file_key.into(),
                    chunk_batch.clone(),
                    Some(bucket.clone()),
                )
                .await
            {
                Ok(download_request) => {
                    match self
                        .process_chunk_download_response(
                            file_key,
                            file_metadata,
                            chunk_batch,
                            peer_id,
                            download_request,
                            peer_manager,
                            batch_size_bytes,
                            start_time,
                        )
                        .await
                    {
                        Ok(success) => return Ok(success),
                        Err(e) if attempt < DOWNLOAD_RETRY_ATTEMPTS => {
                            warn!(
                                target: LOG_TARGET,
                                "Download attempt {} failed for peer {:?}: {:?}",
                                attempt + 1,
                                peer_id,
                                e
                            );
                            continue;
                        }
                        Err(e) => {
                            let mut peer_manager = peer_manager.write().await;
                            peer_manager.record_failure(peer_id);
                            return Err(e);
                        }
                    }
                }
                Err(e) if attempt < DOWNLOAD_RETRY_ATTEMPTS => {
                    warn!(
                        target: LOG_TARGET,
                        "Download attempt {} failed for peer {:?}: {:?}",
                        attempt + 1,
                        peer_id,
                        e
                    );
                    continue;
                }
                Err(e) => {
                    let mut peer_manager = peer_manager.write().await;
                    peer_manager.record_failure(peer_id);
                    return Err(anyhow!(
                        "Download request failed after {} attempts to peer {:?}: {:?}",
                        DOWNLOAD_RETRY_ATTEMPTS + 1,
                        peer_id,
                        e
                    ));
                }
            }
        }

        Ok(false)
    }

    /// Creates a batch of chunk IDs to request together
    fn create_chunk_batch(chunk_start: u64, chunks_count: u64) -> HashSet<ChunkId> {
        let chunk_end = std::cmp::min(chunk_start + (MAX_CHUNKS_PER_REQUEST as u64), chunks_count);
        (chunk_start..chunk_end).map(ChunkId::new).collect()
    }

    /// Downloads a file from BSPs (Backup Storage Providers) chunk by chunk.
    ///
    /// # Parallelism Implementation
    /// The download process uses a multi-level parallelism approach:
    ///
    /// 1. File-Level Parallelism:
    ///    - Up to [`MAX_CONCURRENT_FILE_DOWNLOADS`] files can be downloaded simultaneously
    ///    - Controlled by a top-level semaphore to prevent system overload
    ///
    /// 2. Chunk-Level Parallelism:
    ///    - For each file, up to [`MAX_CONCURRENT_CHUNKS_PER_FILE`] chunk batches can be downloaded in parallel
    ///    - Each chunk download is managed by a separate task
    ///    - Chunk downloads are batched ([`MAX_CHUNKS_PER_REQUEST`] chunks per request) for efficiency
    ///
    /// 3. Peer Selection and Retry Strategy:
    ///    - For each chunk batch:
    ///      - Selects [`CHUNK_REQUEST_PEER_RETRY_ATTEMPTS`] peers (2 best performing + remaining random)
    ///      - Tries each selected peer up to [`DOWNLOAD_RETRY_ATTEMPTS`] times
    ///      - First successful download stops the retry process
    ///    - Total retry attempts per chunk = [`CHUNK_REQUEST_PEER_RETRY_ATTEMPTS`] * [`DOWNLOAD_RETRY_ATTEMPTS`]
    async fn download_file(
        &self,
        file: &shc_indexer_db::models::File,
        bucket: &BucketId,
    ) -> anyhow::Result<()> {
        let file_metadata = file.to_file_metadata(bucket.as_ref().to_vec());
        let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();
        let chunks_count = file_metadata.chunks_count();

        info!(
            target: LOG_TARGET,
            "MSP: downloading file {:?}", file_key,
        );

        // Create semaphore for chunk-level parallelism
        let chunk_semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_CHUNKS_PER_FILE));
        let peer_manager = Arc::clone(&self.peer_manager);

        let chunk_tasks: Vec<_> = (0..chunks_count)
            .step_by(MAX_CHUNKS_PER_REQUEST)
            .map(|chunk_start| {
                let semaphore = Arc::clone(&chunk_semaphore);
                let task = self.clone();
                let file_metadata = file_metadata.clone();
                let file_key = file_key.clone();
                let bucket = bucket.clone();
                let peer_manager = Arc::clone(&peer_manager);

                tokio::spawn(async move {
                    let _permit = semaphore
                        .acquire()
                        .await
                        .map_err(|e| anyhow!("Failed to acquire chunk semaphore: {:?}", e))?;

                    let chunk_batch = Self::create_chunk_batch(chunk_start, chunks_count);
                    let batch_size_bytes = chunk_batch.len() as u64 * FILE_CHUNK_SIZE as u64;

                    // Get the best performing peers for this request and shuffle them
                    let selected_peers = {
                        let peer_manager = peer_manager.read().await;
                        let mut peers = peer_manager.select_peers(
                            2,
                            CHUNK_REQUEST_PEER_RETRY_ATTEMPTS - 2,
                            &file_key,
                        );
                        peers.shuffle(&mut *GLOBAL_RNG.lock().unwrap());
                        peers
                    };

                    // Try each selected peer
                    for peer_id in selected_peers {
                        match task
                            .try_download_chunk_batch(
                                peer_id,
                                file_key,
                                &file_metadata,
                                &chunk_batch,
                                &bucket,
                                &peer_manager,
                                batch_size_bytes,
                            )
                            .await
                        {
                            Ok(true) => return Ok(()),
                            Ok(false) | Err(_) => continue,
                        }
                    }

                    Err(anyhow!(
                        "Failed to download chunk {} after all retries",
                        chunk_start
                    ))
                })
            })
            .collect();

        // Wait for all downloads to complete and collect results
        let results = join_all(chunk_tasks).await;

        // Process results and count failures
        let mut failed_downloads = 0;
        for result in results {
            match result {
                Ok(download_result) => {
                    if let Err(e) = download_result {
                        error!(
                            target: LOG_TARGET,
                            "File download chunk task failed: {:?}", e
                        );
                        failed_downloads += 1;
                    }
                }
                Err(e) => {
                    error!(
                        target: LOG_TARGET,
                        "File download chunk task panicked: {:?}", e
                    );
                    failed_downloads += 1;
                }
            }
        }

        // Log summary of download results
        if failed_downloads > 0 {
            error!(
                target: LOG_TARGET,
                "Failed to download {}/{} chunks for file {:?}",
                failed_downloads,
                chunks_count,
                file_key
            );
            return Err(anyhow!(
                "Failed to download all chunks for file {:?}",
                file_key
            ));
        } else {
            info!(
                target: LOG_TARGET,
                "Successfully downloaded {} chunks for file {:?}",
                chunks_count,
                file_key
            );
        }

        Ok(())
    }
}

/// Tracks performance metrics for a BSP peer.
/// This struct is used to track the performance metrics for a BSP peer.
/// It is used to select the best performing peers for a given file.
#[derive(Debug, Clone)]
struct BspPeerStats {
    /// The number of successful downloads for the peer
    pub successful_downloads: u64,

    /// The number of failed downloads for the peer
    pub failed_downloads: u64,

    /// The total number of bytes downloaded for the peer
    pub total_bytes_downloaded: u64,

    /// The total download time for the peer
    pub total_download_time_ms: u64,

    /// The time of the last successful download for the peer
    pub last_success_time: Option<std::time::Instant>,

    /// The set of file keys that the peer can provide. This is used to
    /// update the right priority queue in [`BspPeerManager::peer_queues`].
    pub file_keys: HashSet<H256>,
}

impl BspPeerStats {
    fn new() -> Self {
        Self {
            successful_downloads: 0,
            failed_downloads: 0,
            total_bytes_downloaded: 0,
            total_download_time_ms: 0,
            last_success_time: None,
            file_keys: HashSet::new(),
        }
    }

    fn record_success(&mut self, bytes_downloaded: u64, download_time_ms: u64) {
        self.successful_downloads += 1;
        self.total_bytes_downloaded += bytes_downloaded;
        self.total_download_time_ms += download_time_ms;
        self.last_success_time = Some(std::time::Instant::now());
    }

    fn record_failure(&mut self) {
        self.failed_downloads += 1;
    }

    fn get_success_rate(&self) -> f64 {
        let total = self.successful_downloads + self.failed_downloads;
        if total == 0 {
            return 0.0;
        }
        self.successful_downloads as f64 / total as f64
    }

    fn get_average_speed_bytes_per_sec(&self) -> f64 {
        if self.total_download_time_ms == 0 {
            return 0.0;
        }
        (self.total_bytes_downloaded as f64 * 1000.0) / self.total_download_time_ms as f64
    }

    /// Calculates a score for the peer based on its success rate and average speed
    ///
    /// The score is a weighted combination of the peer's success rate and average speed.
    /// The success rate is weighted more heavily (70%) compared to the average speed (30%).
    ///
    fn get_score(&self) -> f64 {
        // Combine success rate and speed into a single score
        // Weight success rate more heavily (70%) compared to speed (30%)
        let success_weight = 0.7;
        let speed_weight = 0.3;

        let success_score = self.get_success_rate();
        let speed_score = if self.successful_downloads == 0 {
            0.0
        } else {
            // Normalize speed score between 0 and 1
            // Using 30MB/s as a reference for max speed
            let max_speed = 50.0 * 1024.0 * 1024.0;
            (self.get_average_speed_bytes_per_sec() / max_speed).min(1.0)
        };

        (success_score * success_weight) + (speed_score * speed_weight)
    }

    /// Register that this peer is requested for a file key
    fn add_file_key(&mut self, file_key: H256) {
        self.file_keys.insert(file_key);
    }
}

/// Manages BSP peer selection and performance tracking
///
/// This struct maintains performance metrics for each peer and provides improved peer selection
/// based on historical performance. It uses a priority queue system to rank peers based on their
/// performance scores, which are calculated using a weighted combination of success rate (70%) and
/// download speed (30%).
///
/// # Peer Selection Strategy
/// - Selects a mix of best-performing peers and random peers for each request
/// - Uses priority queues to maintain peer rankings per file
/// - Implements a hybrid selection approach:
///   - Best performers: Selected based on weighted scoring
///   - Random selection: Ensures network diversity and prevents starvation
///
/// # Performance Tracking
/// - Tracks success/failure rates
/// - Monitors download speeds
/// - Records total bytes transferred
#[derive(Debug)]
struct BspPeerManager {
    peers: HashMap<PeerId, BspPeerStats>,
    // Map from file_key to a priority queue of peers sorted by score
    peer_queues: HashMap<H256, PriorityQueue<PeerId, OrderedFloat<f64>>>,
}

impl BspPeerManager {
    /// Creates a new BspPeerManager with empty peer stats and queues
    fn new() -> Self {
        Self {
            peers: HashMap::new(),
            peer_queues: HashMap::new(),
        }
    }

    /// Registers a new peer for a specific file and initializes its performance tracking
    fn add_peer(&mut self, peer_id: PeerId, file_key: H256) {
        let stats = self
            .peers
            .entry(peer_id)
            .or_insert_with(|| BspPeerStats::new());
        stats.add_file_key(file_key.clone());

        // Add to the priority queue for this file key
        let queue = self
            .peer_queues
            .entry(file_key)
            .or_insert_with(PriorityQueue::new);
        queue.push(peer_id, OrderedFloat::from(stats.get_score()));
    }

    /// Records a successful download attempt and updates peer's performance metrics
    fn record_success(&mut self, peer_id: PeerId, bytes_downloaded: u64, download_time_ms: u64) {
        if let Some(stats) = self.peers.get_mut(&peer_id) {
            stats.record_success(bytes_downloaded, download_time_ms);
            let new_score = stats.get_score();

            // Update scores in all queues containing this peer
            for file_key in stats.file_keys.iter() {
                if let Some(queue) = self.peer_queues.get_mut(file_key) {
                    queue.change_priority(&peer_id, OrderedFloat::from(new_score));
                }
            }
        }
    }

    /// Records a failed download attempt and updates peer's performance metrics
    fn record_failure(&mut self, peer_id: PeerId) {
        if let Some(stats) = self.peers.get_mut(&peer_id) {
            stats.record_failure();
            let new_score = stats.get_score();

            // Update scores in all queues containing this peer
            for file_key in stats.file_keys.iter() {
                if let Some(queue) = self.peer_queues.get_mut(file_key) {
                    queue.change_priority(&peer_id, OrderedFloat::from(new_score));
                }
            }
        }
    }

    /// Selects a list of peers for downloading chunks of a specific file
    ///
    /// # Arguments
    /// - `count_best` - Number of top-performing peers to select based on scores
    /// - `count_random` - Number of additional random peers to select for diversity
    /// - `file_key` - The file key for which peers are being selected
    ///
    /// # Selection Strategy
    /// 1. First selects the top `count_best` peers based on their performance scores
    /// 2. Then randomly selects `count_random` additional peers from the remaining pool
    /// 3. Returns a combined list of selected peers in a randomized order
    ///
    /// This hybrid approach ensures both performance (by selecting proven peers) and
    /// network health (by giving chances to other peers through random selection).
    fn select_peers(&self, count_best: usize, count_random: usize, file_key: &H256) -> Vec<PeerId> {
        let queue = match self.peer_queues.get(file_key) {
            Some(queue) => queue,
            None => return Vec::new(),
        };

        let mut selected_peers = Vec::with_capacity(count_best + count_random);
        let mut queue_clone = queue.clone();

        // Extract top count_best peers
        let actual_best_count = count_best.min(queue_clone.len());
        for _ in 0..actual_best_count {
            if let Some((peer_id, _)) = queue_clone.pop() {
                selected_peers.push(peer_id);
            }
        }

        // Randomly select additional peers from the remaining pool
        if count_random > 0 && !queue_clone.is_empty() {
            use rand::seq::SliceRandom;
            let remaining_peers: Vec<_> = queue_clone
                .into_iter()
                .map(|(peer_id, _)| peer_id)
                .collect();
            let mut remaining_peers = remaining_peers;
            remaining_peers.shuffle(&mut *GLOBAL_RNG.lock().unwrap());

            let actual_random_count = count_random.min(remaining_peers.len());
            selected_peers.extend(remaining_peers.iter().take(actual_random_count));
        }

        selected_peers
    }
}
