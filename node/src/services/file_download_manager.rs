use anyhow::{anyhow, Result};
use codec::Decode;
use futures::future::join_all;
use log::*;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use sc_network::PeerId;
use shc_common::types::{
    BucketId, Chunk, ChunkId, FileKeyProof, FileMetadata, HashT, Proven,
    StorageProofsMerkleTrieLayout,
};
use shc_file_manager::traits::FileStorage;
use shc_file_transfer_service::{
    commands::FileTransferServiceInterface, schema::v1::provider::RemoteDownloadDataResponse,
};
use sp_core::H256;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, Semaphore};

use super::bsp_peer_manager::BspPeerManager;

const LOG_TARGET: &str = "file_download_manager";

/// Constants for file download and operation rate-limiting
const MAX_CONCURRENT_FILE_DOWNLOADS: usize = 10;
const MAX_CONCURRENT_CHUNKS_PER_FILE: usize = 5;
const MAX_CHUNKS_PER_REQUEST: usize = 10;
const CHUNK_REQUEST_PEER_RETRY_ATTEMPTS: usize = 5;
const DOWNLOAD_RETRY_ATTEMPTS: usize = 2;
const BEST_PEERS_TO_SELECT: usize = 2;
const RANDOM_PEERS_TO_SELECT: usize = 3;

/// Configuration for file download limits and parallelism settings
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
        }
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
pub struct FileDownloadManager {
    /// Configuration for download limits
    pub limits: FileDownloadLimits,
    /// Semaphore for controlling file-level parallelism
    file_semaphore: Arc<Semaphore>,
    /// BSP peer manager for tracking and selecting peers
    peer_manager: Arc<BspPeerManager>,
}

impl FileDownloadManager {
    /// Create a new FileDownloadManager with default limits
    ///
    /// # Arguments
    /// * `peer_manager` - The peer manager to use for peer selection and tracking
    pub fn new(peer_manager: Arc<BspPeerManager>) -> Self {
        Self::with_limits(FileDownloadLimits::default(), peer_manager)
    }

    /// Create a new FileDownloadManager with specified limits
    ///
    /// # Arguments
    /// * `limits` - The download limits to use
    /// * `peer_manager` - The peer manager to use for peer selection and tracking
    pub fn with_limits(limits: FileDownloadLimits, peer_manager: Arc<BspPeerManager>) -> Self {
        let file_semaphore = Arc::new(Semaphore::new(limits.max_concurrent_file_downloads));

        Self {
            limits,
            file_semaphore,
            peer_manager,
        }
    }

    /// Get a reference to the file semaphore for file-level parallelism
    pub fn file_semaphore(&self) -> Arc<Semaphore> {
        Arc::clone(&self.file_semaphore)
    }

    /// Create a new chunk semaphore for chunk-level parallelism within a file
    pub fn new_chunk_semaphore(&self) -> Arc<Semaphore> {
        Arc::new(Semaphore::new(self.limits.max_concurrent_chunks_per_file))
    }

    /// Create a batch of chunks to download
    pub fn create_chunk_batch(&self, chunk_start: u64, chunks_count: u64) -> HashSet<ChunkId> {
        let chunk_end = std::cmp::min(
            chunk_start + (self.limits.max_chunks_per_request as u64),
            chunks_count,
        );
        (chunk_start..chunk_end).map(ChunkId::new).collect()
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

    /// Extract proven chunks from a download response
    fn extract_chunks_from_response(
        response: &RemoteDownloadDataResponse,
    ) -> Result<Vec<Proven<ChunkId, Chunk>>> {
        // Access the file_key_proof field
        let file_key_proof_bytes = &response.file_key_proof;

        // Decode the file key proof
        let file_key_proof = FileKeyProof::decode(&mut file_key_proof_bytes.as_slice())
            .map_err(|e| anyhow!("Failed to decode FileKeyProof: {:?}", e))?;

        // Extract proven chunks from the proof using the correct layout type
        let proven_leaves = file_key_proof
            .proven::<StorageProofsMerkleTrieLayout>()
            .map_err(|e| anyhow!("Failed to extract chunks from proof: {:?}", e))?;

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

        let chunks = Self::extract_chunks_from_response(&download_request)?;

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
                    let expected_size = file_metadata.chunk_size_at(chunk_idx);
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
        Ok(true)
    }

    /// Attempts to download a batch of chunks from a specific peer with retries
    pub async fn try_download_chunk_batch<FS, FT>(
        &self,
        peer_id: PeerId,
        file_key: H256,
        file_metadata: &FileMetadata,
        chunk_batch: &HashSet<ChunkId>,
        bucket: &BucketId,
        file_transfer: &FT,
        file_storage: &mut FS,
    ) -> Result<bool>
    where
        FT: FileTransferServiceInterface + Send + Sync,
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
        bucket: BucketId,
        file_transfer: FT,
        file_storage: Arc<RwLock<FS>>,
    ) -> Result<()>
    where
        FT: FileTransferServiceInterface + Send + Sync + Clone + 'static,
        FS: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync + 'static,
    {
        // Acquire the file semaphore permit
        let semaphore = self.file_semaphore();
        let _permit = semaphore
            .acquire()
            .await
            .map_err(|e| anyhow!("Failed to acquire file semaphore: {:?}", e))?;

        let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();
        let chunks_count = file_metadata.chunks_count();

        info!(
            target: LOG_TARGET,
            "Downloading file {:?} with {} chunks", file_key, chunks_count
        );

        // Create a new chunk semaphore for this file
        let chunk_semaphore = self.new_chunk_semaphore();
        let manager = self.clone();

        // Create tasks for chunk batches
        let chunk_tasks: Vec<_> = (0..chunks_count)
            .step_by(self.limits.max_chunks_per_request)
            .map(|chunk_start| {
                let semaphore = Arc::clone(&chunk_semaphore);
                let file_metadata = file_metadata.clone();
                let bucket = bucket.clone();
                let file_transfer = file_transfer.clone();
                let file_storage = Arc::clone(&file_storage);
                let file_key = file_key;
                let manager = manager.clone();

                tokio::spawn(async move {
                    // Acquire semaphore permit for this chunk batch
                    let _permit = semaphore
                        .acquire()
                        .await
                        .map_err(|e| anyhow!("Failed to acquire chunk semaphore: {:?}", e))?;

                    // Create chunk batch
                    let chunk_batch = manager.create_chunk_batch(chunk_start, chunks_count);

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
                        "Failed to download chunk batch starting at {} after trying all peers",
                        chunk_start
                    ))
                })
            })
            .collect();

        // Wait for all chunk tasks to complete
        let results = join_all(chunk_tasks).await;

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

        if !errors.is_empty() {
            Err(anyhow!(
                "Failed to download file {:?}: {}",
                file_key,
                errors.join(", ")
            ))
        } else {
            info!(
                target: LOG_TARGET,
                "Successfully downloaded all chunks for file {:?}", file_key
            );
            Ok(())
        }
    }
}

impl Clone for FileDownloadManager {
    fn clone(&self) -> Self {
        Self {
            limits: self.limits.clone(),
            file_semaphore: Arc::clone(&self.file_semaphore),
            peer_manager: Arc::clone(&self.peer_manager),
        }
    }
}
