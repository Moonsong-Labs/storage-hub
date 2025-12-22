use anyhow::{anyhow, Result};
use futures::future::join_all;
use log::*;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use thiserror::Error;
use tokio::sync::{RwLock, Semaphore};

use codec::Decode;
use sc_network::PeerId;
use sp_core::H256;

use shc_common::{
    traits::StorageEnableRuntime,
    types::{
        BucketId, FileKeyProof, FileMetadata, Fingerprint, HashT, Proven,
        StorageProofsMerkleTrieLayout,
    },
};
use shc_file_manager::traits::FileStorage;
use shc_file_transfer_service::{
    commands::{FileTransferServiceCommandInterface, FileTransferServiceCommandInterfaceExt},
    schema::v1::provider::RemoteDownloadDataResponse,
};
use shp_file_metadata::{Chunk, ChunkId};

use crate::{
    bsp_peer_manager::BspPeerManager,
    download_state_store::DownloadStateStore,
    inc_counter_by,
    metrics::{MetricsLink, STATUS_FAILURE, STATUS_SUCCESS},
    observe_histogram,
};

const LOG_TARGET: &str = "file_download_manager";

/// Constants for file download and operation rate-limiting
const MAX_CONCURRENT_FILE_DOWNLOADS: usize = 10;
const MAX_CONCURRENT_CHUNKS_PER_FILE: usize = 5;
const MAX_CHUNKS_PER_REQUEST: usize = 10;
const CHUNK_REQUEST_PEER_RETRY_ATTEMPTS: usize = 5;
const DOWNLOAD_RETRY_ATTEMPTS: usize = 2;
const BEST_PEERS_TO_SELECT: usize = 2;
const RANDOM_PEERS_TO_SELECT: usize = 3;
const MAX_CONCURRENT_BUCKET_DOWNLOADS: usize = 2;

/// Configuration for file download limits and parallelism settings.
#[derive(Copy)]
pub struct FileDownloadLimits {
    /// Maximum number of files to download in parallel
    pub max_concurrent_file_downloads: usize,
    /// Maximum number of chunks requests to do in parallel per file
    pub max_concurrent_chunks_per_file: usize,
    /// Maximum number of chunks to request in a single network request
    pub max_chunks_per_request: usize,
    /// Number of peers to select for each chunk download attempt
    pub chunk_request_peer_retry_attempts: usize,
    /// Number of retries per peer for a single chunk request
    pub download_retry_attempts: usize,
    /// Number of best performing peers to select
    pub best_peers_to_select: usize,
    /// Number of random peers to select
    pub random_peers_to_select: usize,
    /// Maximum number of bucket downloads to process in parallel
    pub max_concurrent_bucket_downloads: usize,
}

impl Default for FileDownloadLimits {
    fn default() -> Self {
        Self {
            max_concurrent_file_downloads: MAX_CONCURRENT_FILE_DOWNLOADS,
            max_concurrent_chunks_per_file: MAX_CONCURRENT_CHUNKS_PER_FILE,
            max_chunks_per_request: MAX_CHUNKS_PER_REQUEST,
            chunk_request_peer_retry_attempts: CHUNK_REQUEST_PEER_RETRY_ATTEMPTS,
            download_retry_attempts: DOWNLOAD_RETRY_ATTEMPTS,
            best_peers_to_select: BEST_PEERS_TO_SELECT,
            random_peers_to_select: RANDOM_PEERS_TO_SELECT,
            max_concurrent_bucket_downloads: MAX_CONCURRENT_BUCKET_DOWNLOADS,
        }
    }
}

impl Clone for FileDownloadLimits {
    fn clone(&self) -> Self {
        Self {
            max_concurrent_file_downloads: self.max_concurrent_file_downloads,
            max_concurrent_chunks_per_file: self.max_concurrent_chunks_per_file,
            max_chunks_per_request: self.max_chunks_per_request,
            chunk_request_peer_retry_attempts: self.chunk_request_peer_retry_attempts,
            download_retry_attempts: self.download_retry_attempts,
            best_peers_to_select: self.best_peers_to_select,
            random_peers_to_select: self.random_peers_to_select,
            max_concurrent_bucket_downloads: self.max_concurrent_bucket_downloads,
        }
    }
}

/// A bucket lock with metadata about its active status
struct BucketLockInfo {
    /// Whether this lock is currently actively downloading (has acquired the mutex)
    is_downloading: bool,
}

impl BucketLockInfo {
    fn new() -> Self {
        Self {
            is_downloading: false,
        }
    }

    fn set_downloading(&mut self, is_downloading: bool) {
        self.is_downloading = is_downloading;
    }
}

/// Possible errors that can occur during bucket download
#[derive(Error, Debug)]
pub enum BucketDownloadError<Runtime: StorageEnableRuntime> {
    #[error("Bucket {0:?} is already being downloaded by another task")]
    AlreadyBeingDownloaded(BucketId<Runtime>),

    #[error("Failed to download bucket: {0}")]
    DownloadFailed(anyhow::Error),
}

impl<Runtime: StorageEnableRuntime> From<anyhow::Error> for BucketDownloadError<Runtime> {
    fn from(error: anyhow::Error) -> Self {
        BucketDownloadError::DownloadFailed(error)
    }
}

/// Manages file downloads and operations with rate limiting
///
/// # Parallelism Implementation
/// The download process uses a multi-level parallelism approach:
///
/// 1. File-Level Parallelism:
///    - Multiple files can be downloaded simultaneously
///    - Controlled by a top-level semaphore to prevent system overload
///
/// 2. Chunk-Level Parallelism:
///    - For each file, multiple chunk batches can be downloaded in parallel
///    - Each chunk download is managed by a separate task
///    - Chunk downloads are batched (multiple chunks per request) for efficiency
///
/// 3. Peer Selection and Retry Strategy:
///    - For each chunk batch:
///      - Selects peers (best performing + random)
///      - Tries each selected peer multiple times
///      - First successful download stops the retry process
///
/// 4. Bucket-Level Locking:
///    - Per-bucket locks prevent multiple tasks from downloading the same bucket
///    - Lock status tracked to avoid premature cleanup
///    - Locks automatically expire after downloads complete
pub struct FileDownloadManager<Runtime: StorageEnableRuntime> {
    /// Configuration for download limits
    pub limits: FileDownloadLimits,
    /// Semaphore for controlling file-level parallelism
    file_semaphore: Arc<Semaphore>,
    /// Semaphore for controlling bucket-level parallelism
    bucket_semaphore: Arc<Semaphore>,
    /// Per-bucket locks with status info to prevent concurrent downloads of the same bucket
    bucket_locks: Arc<RwLock<HashMap<BucketId<Runtime>, BucketLockInfo>>>,
    /// BSP peer manager for tracking and selecting peers
    peer_manager: Arc<BspPeerManager>,
    /// Download state store for persistence
    download_state_store: Arc<DownloadStateStore<Runtime>>,
    /// Prometheus metrics for tracking download throughput
    metrics: MetricsLink,
}

impl<Runtime: StorageEnableRuntime> FileDownloadManager<Runtime> {
    /// Create a new [`FileDownloadManager`] with default limits.
    ///
    /// # Arguments
    /// * `peer_manager` - The peer manager to use for peer selection and tracking
    /// * `data_dir` - The directory to store download state
    /// * `metrics` - The Prometheus metrics link for tracking download throughput
    pub fn new(
        peer_manager: Arc<BspPeerManager>,
        data_dir: PathBuf,
        metrics: MetricsLink,
    ) -> Result<Self> {
        Self::with_limits(
            FileDownloadLimits::default(),
            peer_manager,
            data_dir,
            metrics,
        )
    }

    /// Create a new [`FileDownloadManager`] with specified limits.
    ///
    /// # Arguments
    /// * `limits` - The download limits to use
    /// * `peer_manager` - The peer manager to use for peer selection and tracking
    /// * `data_dir` - The directory to store download state
    /// * `metrics` - The Prometheus metrics link for tracking download throughput
    pub fn with_limits(
        limits: FileDownloadLimits,
        peer_manager: Arc<BspPeerManager>,
        data_dir: PathBuf,
        metrics: MetricsLink,
    ) -> Result<Self> {
        // Create a new download state store
        let download_state_store = Arc::new(DownloadStateStore::new(data_dir)?);

        Ok(Self {
            file_semaphore: Arc::new(Semaphore::new(limits.max_concurrent_file_downloads)),
            bucket_semaphore: Arc::new(Semaphore::new(limits.max_concurrent_bucket_downloads)),
            bucket_locks: Arc::new(RwLock::new(HashMap::new())),
            limits,
            peer_manager,
            download_state_store,
            metrics,
        })
    }

    /// Get a reference to the file semaphore for file-level parallelism
    pub fn file_semaphore(&self) -> Arc<Semaphore> {
        Arc::clone(&self.file_semaphore)
    }

    /// Returns a reference to the download state store
    pub fn download_state_store(&self) -> Arc<DownloadStateStore<Runtime>> {
        self.download_state_store.clone()
    }

    /// Create a new chunk semaphore for chunk-level parallelism within a file
    pub fn new_chunk_semaphore(&self) -> Arc<Semaphore> {
        Arc::new(Semaphore::new(self.limits.max_concurrent_chunks_per_file))
    }

    /// Process a single proven chunk, writing it to file storage
    async fn process_proven_chunk<FS>(
        &self,
        file_key: H256,
        proven_chunk: Proven<ChunkId, Chunk>,
        file_storage: &mut FS,
    ) -> Result<()>
    where
        FS: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    {
        // Handle the proven chunk based on its variant
        match proven_chunk {
            Proven::ExactKey(leaf) => {
                let chunk_id = leaf.key;
                let chunk_data = leaf.data;
                let chunk_idx = chunk_id.as_u64();

                // Chunk size has already been validated in process_chunk_download_response
                file_storage
                    .write_chunk(&file_key, &chunk_id, &chunk_data)
                    .map_err(|error| anyhow!("Failed to write chunk {}: {:?}", chunk_idx, error))?;

                // Mark chunk as downloaded in persistent state
                let context = self.download_state_store.open_rw_context();
                context
                    .missing_chunks_map()
                    .mark_chunk_downloaded(&file_key, chunk_id);
                context.commit();

                Ok(())
            }
            unexpected => {
                warn!(
                    target: LOG_TARGET,
                    "Unexpected Proven variant encountered: {:?}", unexpected
                );
                Err(anyhow!(
                    "Unexpected Proven variant: only ExactKey is supported for file chunks"
                ))
            }
        }
    }

    /// Extract proven chunks from a download response, validating the fingerprint
    fn extract_chunks_from_response(
        response: &RemoteDownloadDataResponse,
        file_key: &H256,
        expected_fingerprint: &Fingerprint,
    ) -> Result<Vec<Proven<ChunkId, Chunk>>> {
        // Access the file_key_proof bytes
        let file_key_proof_bytes = &response.file_key_proof;

        // Decode the file key proof
        let file_key_proof = FileKeyProof::decode(&mut file_key_proof_bytes.as_slice())
            .map_err(|e| anyhow!("Failed to decode FileKeyProof: {:?}", e))?;

        // Verify that the fingerprint in the response matches the expected fingerprint
        if file_key_proof.file_metadata.fingerprint() != expected_fingerprint {
            return Err(anyhow!(
                "Fingerprint mismatch for file {:?}. Expected: {:?}, got: {:?}",
                file_key,
                expected_fingerprint,
                file_key_proof.file_metadata.fingerprint()
            ));
        }

        // Extract proven chunks from the proof
        let proven_leaves = file_key_proof
            .proven::<StorageProofsMerkleTrieLayout>()
            .map_err(|e| anyhow!("Failed to extract proven chunks from proof: {:?}", e))?;

        // Convert Leaf<ChunkId, Chunk> to Proven<ChunkId, Chunk>
        let proven_chunks = proven_leaves
            .into_iter()
            .map(|leaf| Proven::new_exact_key(leaf.key, leaf.data))
            .collect();

        Ok(proven_chunks)
    }

    /// Process the response from a chunk download request
    async fn process_chunk_download_response<FS>(
        &self,
        file_key: H256,
        file_metadata: &FileMetadata,
        chunk_batch: &HashSet<ChunkId>,
        peer_id: PeerId,
        download_request: RemoteDownloadDataResponse,
        start_time: std::time::Instant,
        file_storage: &mut FS,
    ) -> Result<bool>
    where
        FS: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    {
        let elapsed = start_time.elapsed();

        // Extract chunks from the response, including fingerprint validation
        let chunks = Self::extract_chunks_from_response(
            &download_request,
            &file_key,
            file_metadata.fingerprint(),
        )
        .map_err(|e| anyhow!("Error processing response from peer {:?}: {:?}", peer_id, e))?;

        if chunks.is_empty() {
            return Err(anyhow!("No chunks in response from peer {:?}", peer_id));
        }

        let mut total_bytes = 0;
        let mut processed_chunks = 0;

        for proven_chunk in chunks {
            // Validate the chunk before processing it
            if let Proven::ExactKey(leaf) = &proven_chunk {
                let chunk_id = &leaf.key;
                let chunk_data = &leaf.data;
                let chunk_idx = chunk_id.as_u64();

                // Validate chunk size using is_valid_chunk_size
                if !file_metadata.is_valid_chunk_size(chunk_idx, chunk_data.len()) {
                    let expected_size = file_metadata
                        .chunk_size_at(chunk_idx)
                        .map_err(|e| anyhow!("Failed to get expected chunk size: {:?}", e))?;
                    return Err(anyhow!(
                        "Invalid chunk size for chunk {}: Expected: {}, got: {}",
                        chunk_idx,
                        expected_size,
                        chunk_data.len()
                    ));
                }

                total_bytes += chunk_data.len();

                // Only process chunks that were requested
                if !chunk_batch.contains(chunk_id) {
                    warn!(
                        target: LOG_TARGET,
                        "Received chunk {:?} that was not requested from peer {:?}", chunk_id, peer_id
                    );
                    continue;
                }
            } else {
                warn!(
                    target: LOG_TARGET,
                    "Unexpected Proven variant encountered: {:?}", proven_chunk
                );
                continue;
            }

            self.process_proven_chunk(file_key, proven_chunk, file_storage)
                .await?;
            processed_chunks += 1;
        }

        info!(
            target: LOG_TARGET,
            "Downloaded {} chunks ({} bytes) from peer {:?} in {:?} ({:.2} MB/s)",
            processed_chunks,
            total_bytes,
            peer_id,
            elapsed,
            (total_bytes as f64 / 1024.0 / 1024.0) / elapsed.as_secs_f64()
        );

        self.peer_manager
            .record_success(peer_id, total_bytes as u64, elapsed.as_millis() as u64)
            .await;

        // Record successful download throughput metrics
        inc_counter_by!(
            metrics: self.metrics.as_ref(),
            bytes_downloaded_total,
            STATUS_SUCCESS,
            total_bytes as u64
        );
        inc_counter_by!(
            metrics: self.metrics.as_ref(),
            chunks_downloaded_total,
            STATUS_SUCCESS,
            processed_chunks as u64
        );

        Ok(true)
    }

    /// Attempts to download a batch of chunks from a specific peer with retries
    pub async fn try_download_chunk_batch<FS, FT>(
        &self,
        peer_id: PeerId,
        file_key: H256,
        file_metadata: &FileMetadata,
        chunk_batch: &HashSet<ChunkId>,
        bucket: &BucketId<Runtime>,
        file_transfer: &FT,
        file_storage: &mut FS,
    ) -> Result<bool>
    where
        FT: FileTransferServiceCommandInterface<Runtime>
            + FileTransferServiceCommandInterfaceExt
            + Send
            + Sync,
        FS: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    {
        // Retry the download up to the configured number of times
        for attempt in 0..=self.limits.download_retry_attempts {
            if attempt > 0 {
                warn!(
                    target: LOG_TARGET,
                    "Retrying download from peer {:?} (attempt {}/{})",
                    peer_id,
                    attempt + 1,
                    self.limits.download_retry_attempts + 1
                );
            }

            let start_time = std::time::Instant::now();

            match file_transfer
                .download_request(peer_id, file_key.into(), chunk_batch.clone(), Some(*bucket))
                .await
            {
                Ok(response) => {
                    let response = file_transfer
                        .parse_remote_download_data_response(&response.0)
                        .map_err(|e| {
                            anyhow!("Failed to parse remote download data response: {:?}", e)
                        })?;
                    return self
                        .process_chunk_download_response(
                            file_key,
                            file_metadata,
                            chunk_batch,
                            peer_id,
                            response,
                            start_time,
                            file_storage,
                        )
                        .await;
                }
                Err(e) => {
                    warn!(
                        target: LOG_TARGET,
                        "Download attempt {} failed with peer {:?}: {:?}",
                        attempt + 1,
                        peer_id,
                        e
                    );

                    if attempt == self.limits.download_retry_attempts {
                        self.peer_manager.record_failure(peer_id).await;

                        // Track failed download metrics
                        let expected_bytes: u64 = chunk_batch
                            .iter()
                            .filter_map(|chunk_id| {
                                file_metadata
                                    .chunk_size_at(chunk_id.as_u64())
                                    .ok()
                                    .map(|size| size as u64)
                            })
                            .sum();

                        inc_counter_by!(
                            metrics: self.metrics.as_ref(),
                            bytes_downloaded_total,
                            STATUS_FAILURE,
                            expected_bytes
                        );
                        inc_counter_by!(
                            metrics: self.metrics.as_ref(),
                            chunks_downloaded_total,
                            STATUS_FAILURE,
                            chunk_batch.len() as u64
                        );

                        return Err(anyhow!(
                            "Failed to download after {} attempts: {:?}",
                            attempt + 1,
                            e
                        ));
                    }
                }
            }

            // Delay before retry
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        // This should not be reachable due to the return in the loop
        Err(anyhow!("Failed to download chunk batch after all attempts"))
    }

    /// Downloads a file by breaking it into chunk batches and downloading them in parallel
    pub async fn download_file<FS, FT>(
        &self,
        file_metadata: FileMetadata,
        bucket: BucketId<Runtime>,
        file_transfer: FT,
        file_storage: Arc<RwLock<FS>>,
    ) -> Result<()>
    where
        FT: FileTransferServiceCommandInterface<Runtime>
            + FileTransferServiceCommandInterfaceExt
            + Send
            + Sync
            + Clone
            + 'static,
        FS: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync + 'static,
    {
        // Acquire the file semaphore permit
        let semaphore = self.file_semaphore();
        let _permit = semaphore
            .acquire()
            .await
            .map_err(|e| anyhow!("Failed to acquire file semaphore: {:?}", e))?;

        // Track file download start time for metrics
        let download_start = std::time::Instant::now();

        let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();
        let chunks_count = file_metadata.chunks_count();

        info!(
            target: LOG_TARGET,
            "Downloading file {:?} with {} chunks", file_key, chunks_count
        );

        // Check if we already have state for this file
        let context = self.download_state_store.open_rw_context();
        let missing_chunks = {
            let existing_metadata = context.get_file_metadata(&file_key);

            if let Some(_existing_metadata) = existing_metadata {
                info!(
                    target: LOG_TARGET,
                    "Resuming download of file {:?} with {} chunks", file_key, chunks_count
                );
                // We already have state for this file, use it
            } else {
                // New file download, initialize state
                info!(
                    target: LOG_TARGET,
                    "Starting new download of file {:?} with {} chunks", file_key, chunks_count
                );
                context.store_file_metadata(&file_key, &file_metadata);
                context.missing_chunks_map().initialize_file(&file_metadata);
            }

            // Get missing chunks from the store in both cases
            context.missing_chunks_map().get_missing_chunks(&file_key)
        };
        context.commit();

        if missing_chunks.is_empty() {
            info!(
                target: LOG_TARGET,
                "File {:?} is already fully downloaded", file_key
            );
            return Ok(());
        }

        info!(
            target: LOG_TARGET,
            "File {:?} has {} missing chunks to download",
            file_key,
            missing_chunks.len()
        );

        // Create a new chunk semaphore for this file
        let chunk_semaphore = self.new_chunk_semaphore();
        let manager = self.clone();

        // Group missing chunks into batches
        let max_chunks_per_request = self.limits.max_chunks_per_request as u64;
        let mut missing_chunks_sorted = missing_chunks;
        missing_chunks_sorted.sort();

        // Create tasks for missing chunk batches
        let chunk_tasks: Vec<_> = missing_chunks_sorted
            .chunks(max_chunks_per_request as usize)
            .map(|chunk_ids| {
                let semaphore = Arc::clone(&chunk_semaphore);
                let file_metadata = file_metadata.clone();
                let file_transfer = file_transfer.clone();
                let file_storage = Arc::clone(&file_storage);
                let manager = manager.clone();
                let chunk_batch: HashSet<ChunkId> = chunk_ids.iter().copied().collect();

                tokio::spawn(async move {
                    // Acquire semaphore permit for this chunk batch
                    let _permit = semaphore
                        .acquire()
                        .await
                        .map_err(|e| anyhow!("Failed to acquire chunk semaphore: {:?}", e))?;

                    // Get peers to try for this download
                    let mut peers = manager
                        .peer_manager
                        .select_peers(
                            manager.limits.best_peers_to_select,
                            manager.limits.random_peers_to_select,
                            &file_key,
                        )
                        .await;

                    // Shuffle peers for randomization using a thread-safe RNG
                    let mut rng = StdRng::from_entropy();
                    peers.shuffle(&mut rng);

                    // Try each selected peer
                    for peer_id in peers {
                        let download_result = {
                            let mut file_storage_guard = file_storage.write().await;
                            manager
                                .try_download_chunk_batch(
                                    peer_id,
                                    file_key,
                                    &file_metadata,
                                    &chunk_batch,
                                    &bucket,
                                    &file_transfer,
                                    &mut *file_storage_guard,
                                )
                                .await
                        };

                        match download_result {
                            Ok(_) => return Ok(()),
                            Err(e) => {
                                warn!(
                                    target: LOG_TARGET,
                                    "Failed to download chunk batch from peer {:?}: {:?}",
                                    peer_id,
                                    e
                                );
                                // Try next peer
                                continue;
                            }
                        }
                    }

                    // All peers failed
                    Err(anyhow!(
                        "Failed to download chunk batch after trying all peers"
                    ))
                })
            })
            .collect();

        // Wait for all chunk tasks to complete
        let results = join_all(chunk_tasks).await;

        // Check if download is complete
        let is_complete = {
            let context = self.download_state_store.open_rw_context();
            let is_complete = context.missing_chunks_map().is_file_complete(&file_key);

            if is_complete {
                // Clean up metadata if download is complete
                context.delete_file_metadata(&file_key);
            }

            is_complete
        };

        // Process results and check for errors
        let mut errors = Vec::new();
        for (i, result) in results.into_iter().enumerate() {
            match result {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => {
                    errors.push(format!("Chunk task {} failed: {:?}", i, e));
                }
                Err(e) => {
                    errors.push(format!("Chunk task {} panicked: {:?}", i, e));
                }
            }
        }

        if !errors.is_empty() && !is_complete {
            // Record failed file download duration in histogram
            observe_histogram!(
                metrics: self.metrics.as_ref(),
                file_download_seconds,
                STATUS_FAILURE,
                download_start.elapsed().as_secs_f64()
            );
            Err(anyhow!(
                "Failed to download file {:?}: {}",
                file_key,
                errors.join(", ")
            ))
        } else {
            // Record successful file download duration in histogram
            observe_histogram!(
                metrics: self.metrics.as_ref(),
                file_download_seconds,
                STATUS_SUCCESS,
                download_start.elapsed().as_secs_f64()
            );

            info!(
                target: LOG_TARGET,
                "Successfully downloaded all chunks for file {:?}", file_key
            );
            Ok(())
        }
    }

    /// Mark a bucket download as started
    pub fn mark_bucket_download_started(&self, bucket_id: &BucketId<Runtime>) {
        let context = self.download_state_store.open_rw_context();
        context.mark_bucket_download_started(bucket_id);
        context.commit();
    }

    /// Mark a bucket lock as inactive
    async fn mark_bucket_inactive(&self, bucket_id: &BucketId<Runtime>) {
        let mut locks = self.bucket_locks.write().await;
        if let Some(lock_info) = locks.get_mut(bucket_id) {
            // Mark that the bucket is no longer actively downloading
            lock_info.set_downloading(false);
        }
    }

    /// Check if a bucket download is in progress
    pub fn is_bucket_download_in_progress(&self, bucket_id: &BucketId<Runtime>) -> bool {
        let context = self.download_state_store.open_rw_context();
        let result = context.is_bucket_download_in_progress(bucket_id);
        result
    }

    /// Attempt to lock a bucket and download its files
    /// This method handles all locking internally and returns a specific error
    /// if the bucket is already being downloaded
    pub async fn try_lock_and_download_bucket<FS, FT>(
        &self,
        bucket_id: BucketId<Runtime>,
        file_metadatas: Vec<FileMetadata>,
        file_transfer: FT,
        file_storage: Arc<RwLock<FS>>,
    ) -> Result<(), BucketDownloadError<Runtime>>
    where
        FT: FileTransferServiceCommandInterface<Runtime>
            + FileTransferServiceCommandInterfaceExt
            + Send
            + Sync
            + Clone
            + 'static,
        FS: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync + 'static,
    {
        // Check if bucket is already being downloaded
        {
            let locks = self.bucket_locks.read().await;
            if let Some(lock_info) = locks.get(&bucket_id) {
                if lock_info.is_downloading {
                    return Err(BucketDownloadError::AlreadyBeingDownloaded(bucket_id));
                }
            }
        }

        // Mark bucket as downloading
        {
            let mut locks = self.bucket_locks.write().await;
            let lock_info = locks.entry(bucket_id).or_insert_with(BucketLockInfo::new);

            // Check again in case it became downloading while we were waiting
            if lock_info.is_downloading {
                return Err(BucketDownloadError::AlreadyBeingDownloaded(bucket_id));
            }

            // Mark as downloading
            lock_info.set_downloading(true);
        }

        // Check if bucket download is already in progress
        if self.is_bucket_download_in_progress(&bucket_id) {
            info!(
                target: LOG_TARGET,
                "Resuming download of bucket {:?}", bucket_id
            );
        } else {
            info!(
                target: LOG_TARGET,
                "Starting new download of bucket {:?}", bucket_id
            );
            self.mark_bucket_download_started(&bucket_id);
        }

        // Acquire a bucket download permit
        let _bucket_permit = match self.bucket_semaphore.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(e) => {
                self.mark_bucket_inactive(&bucket_id).await;
                return Err(anyhow!("Failed to acquire bucket semaphore: {:?}", e).into());
            }
        };

        info!(
            target: LOG_TARGET,
            "Downloading {} files for bucket {:?}", file_metadatas.len(), bucket_id
        );

        // Try to download all files in the bucket
        let download_result = async {
            // Process all files in parallel
            let file_tasks: Vec<_> = file_metadatas
                .into_iter()
                .map(|file_metadata| {
                    let file_transfer = file_transfer.clone();
                    let file_storage = Arc::clone(&file_storage);
                    let manager = self.clone();

                    // Spawn a task for each file download
                    tokio::spawn(async move {
                        manager
                            .download_file(file_metadata, bucket_id, file_transfer, file_storage)
                            .await
                    })
                })
                .collect();

            // Wait for all file downloads to complete
            let results = join_all(file_tasks).await;

            // Process results and collect errors
            let mut errors = Vec::new();
            for (i, result) in results.into_iter().enumerate() {
                match result {
                    Ok(Ok(_)) => {} // Successfully downloaded
                    Ok(Err(e)) => errors.push(format!("File {} download failed: {}", i, e)),
                    Err(e) => errors.push(format!("File {} task panicked: {}", i, e)),
                }
            }

            // If there were any errors, return an error with details
            if !errors.is_empty() {
                Err(anyhow!(
                    "Failed to download some files: {}",
                    errors.join(", ")
                ))
            } else {
                Ok(())
            }
        }
        .await;

        // Always mark the bucket inactive at the end
        self.mark_bucket_inactive(&bucket_id).await;

        // Handle download result
        match download_result {
            Ok(()) => {
                // Update persistent state
                let context = self.download_state_store.open_rw_context();
                context.mark_bucket_download_completed(&bucket_id);
                context.commit();

                info!(
                    target: LOG_TARGET,
                    "Completed download of bucket {:?}", bucket_id
                );
                Ok(())
            }
            Err(error) => {
                // Propagate the error
                Err(error.into())
            }
        }
    }
}

impl<Runtime: StorageEnableRuntime> Clone for FileDownloadManager<Runtime> {
    fn clone(&self) -> Self {
        Self {
            limits: self.limits.clone(),
            file_semaphore: Arc::clone(&self.file_semaphore),
            bucket_semaphore: Arc::clone(&self.bucket_semaphore),
            bucket_locks: Arc::clone(&self.bucket_locks),
            peer_manager: Arc::clone(&self.peer_manager),
            download_state_store: Arc::clone(&self.download_state_store),
            metrics: self.metrics.clone(),
        }
    }
}
