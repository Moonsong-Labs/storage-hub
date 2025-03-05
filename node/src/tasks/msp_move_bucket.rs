use anyhow::anyhow;
use codec::Decode;
use futures::future::join_all;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Duration,
};

use sc_network::PeerId;
use sc_tracing::tracing::*;
use sp_core::H256;

use pallet_file_system::types::BucketMoveRequestResponse;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    capacity_manager::CapacityRequestData,
    commands::BlockchainServiceInterface,
    events::{MoveBucketRequestedForMsp, StartMovedBucketDownload},
    types::RetryStrategy,
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

use crate::services::{
    bsp_peer_manager::BspPeerManager,
    handler::StorageHubHandler,
    types::{MspForestStorageHandlerT, ShNodeType},
};

lazy_static::lazy_static! {
    static ref GLOBAL_RNG: Mutex<StdRng> = Mutex::new(StdRng::from_entropy());
}

const LOG_TARGET: &str = "msp-move-bucket-task";

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

        // Get the file semaphore from the download manager
        let file_semaphore = self
            .storage_hub_handler
            .file_download_manager
            .file_semaphore();
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

                    let file_metadata = file
                        .to_file_metadata(bucket_id.as_ref().to_vec())
                        .map_err(|e| anyhow!("Failed to convert file to file metadata: {:?}", e))?;
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
                    for &peer_id in &bsp_peer_ids {
                        task.storage_hub_handler
                            .peer_manager
                            .add_peer(peer_id, file_key)
                            .await;
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
            let file_metadata = file
                .to_file_metadata(bucket.clone())
                .map_err(|e| anyhow!("Failed to convert file to file metadata: {:?}", e))?;
            let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();

            self.storage_hub_handler
                .file_storage
                .write()
                .await
                .insert_file(file_key, file_metadata.clone())
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
                .capacity_config
                .max_capacity();

            if max_storage_capacity <= current_capacity {
                let err_msg =
                    "Reached maximum storage capacity limit. Unable to add more storage capacity.";
                error!(
                    target: LOG_TARGET, "{}", err_msg
                );
                return Err(anyhow::anyhow!(err_msg));
            }

            self.storage_hub_handler
                .blockchain
                .increase_capacity(CapacityRequestData::new(required_size))
                .await?;

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

    /// Processes a single chunk download response
    async fn process_chunk_download_response(
        &self,
        file_key: H256,
        file_metadata: &FileMetadata,
        chunk_batch: &HashSet<ChunkId>,
        peer_id: PeerId,
        download_request: RemoteDownloadDataResponse,
        peer_manager: &Arc<BspPeerManager>,
        batch_size_bytes: u64,
        start_time: std::time::Instant,
    ) -> Result<bool, anyhow::Error> {
        let file_key_proof = FileKeyProof::decode(&mut download_request.file_key_proof.as_ref())
            .map_err(|e| anyhow!("Failed to decode file key proof: {:?}", e))?;

        // Verify fingerprint
        let expected_fingerprint = file_metadata.fingerprint();
        if file_key_proof.file_metadata.fingerprint() != expected_fingerprint {
            peer_manager.record_failure(peer_id).await;
            return Err(anyhow!(
                "Fingerprint mismatch. Expected: {:?}, got: {:?}",
                expected_fingerprint,
                file_key_proof.file_metadata.fingerprint()
            ));
        }

        let proven = file_key_proof
            .proven::<StorageProofsMerkleTrieLayout>()
            .map_err(|e| anyhow!("Failed to get proven data: {:?}", e))?;

        if proven.len() != chunk_batch.len() {
            peer_manager.record_failure(peer_id).await;
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
        peer_manager
            .record_success(peer_id, batch_size_bytes, download_time.as_millis() as u64)
            .await;

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
        peer_manager: &Arc<BspPeerManager>,
        batch_size_bytes: u64,
    ) -> Result<bool, anyhow::Error> {
        // Get retry attempts from the FileDownloadManager
        let download_retry_attempts = self
            .storage_hub_handler
            .file_download_manager
            .download_retry_attempts();

        // Retry the download up to the configured number of times
        for attempt in 0..=download_retry_attempts {
            if attempt > 0 {
                warn!(
                    target: LOG_TARGET,
                    "Retrying download with peer {:?} (attempt {}/{})",
                    peer_id,
                    attempt + 1,
                    download_retry_attempts + 1
                );
            }

            let start_time = std::time::Instant::now();

            match self
                .storage_hub_handler
                .file_transfer
                .download_request(peer_id, file_key.into(), chunk_batch.clone(), Some(*bucket))
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
                        Err(e) if attempt < download_retry_attempts => {
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
                            peer_manager.record_failure(peer_id).await;
                            return Err(e);
                        }
                    }
                }
                Err(e) if attempt < download_retry_attempts => {
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
                    peer_manager.record_failure(peer_id).await;
                    return Err(anyhow!(
                        "Download request failed after {} attempts to peer {:?}: {:?}",
                        download_retry_attempts + 1,
                        peer_id,
                        e
                    ));
                }
            }
        }

        Ok(false)
    }

    /// Creates a batch of chunk IDs to request together
    fn create_chunk_batch(
        chunk_start: u64,
        chunks_count: u64,
        max_chunks_per_request: usize,
    ) -> HashSet<ChunkId> {
        let chunk_end = std::cmp::min(chunk_start + (max_chunks_per_request as u64), chunks_count);
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
        let file_metadata = file
            .to_file_metadata(bucket.as_ref().to_vec())
            .map_err(|e| anyhow!("Failed to convert file to file metadata: {:?}", e))?;
        let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();
        let chunks_count = file_metadata.chunks_count();

        info!(
            target: LOG_TARGET,
            "MSP: downloading file {:?}", file_key,
        );

        // Get a new chunk semaphore from the download manager
        let chunk_semaphore = self
            .storage_hub_handler
            .file_download_manager
            .new_chunk_semaphore();
        let peer_manager = Arc::clone(&self.storage_hub_handler.peer_manager);
        let download_manager = self.storage_hub_handler.file_download_manager.clone();

        let chunk_tasks: Vec<_> = (0..chunks_count)
            .step_by(download_manager.max_chunks_per_request())
            .map(|chunk_start| {
                let semaphore = Arc::clone(&chunk_semaphore);
                let task = self.clone();
                let file_metadata = file_metadata.clone();
                let bucket = *bucket;
                let peer_manager = Arc::clone(&peer_manager);

                tokio::spawn(async move {
                    let _permit = semaphore
                        .acquire()
                        .await
                        .map_err(|e| anyhow!("Failed to acquire chunk semaphore: {:?}", e))?;

                    let chunk_batch = Self::create_chunk_batch(
                        chunk_start,
                        chunks_count,
                        task.storage_hub_handler
                            .file_download_manager
                            .max_chunks_per_request(),
                    );
                    let batch_size_bytes = chunk_batch.len() as u64 * FILE_CHUNK_SIZE as u64;

                    // Get the best performing peers for this request and shuffle them
                    let peer_retry_attempts = task
                        .storage_hub_handler
                        .file_download_manager
                        .chunk_request_peer_retry_attempts();
                    let mut peers = peer_manager
                        .select_peers(2, peer_retry_attempts - 2, &file_key)
                        .await;
                    peers.shuffle(&mut *GLOBAL_RNG.lock().unwrap());

                    // Try each selected peer
                    for peer_id in peers {
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
