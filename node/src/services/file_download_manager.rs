use std::sync::Arc;
use tokio::sync::Semaphore;

/// Constants for file download rate-limiting
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
}

impl Default for FileDownloadLimits {
    fn default() -> Self {
        Self {
            max_concurrent_file_downloads: 10,
            max_concurrent_chunks_per_file: 5,
            max_chunks_per_request: 10,
            chunk_request_peer_retry_attempts: 5,
            download_retry_attempts: 2,
        }
    }
}

/// Manages file downloads with rate limiting
pub struct FileDownloadManager {
    /// Configuration for download limits
    limits: FileDownloadLimits,
    /// Semaphore for controlling file-level parallelism
    file_semaphore: Arc<Semaphore>,
}

impl FileDownloadManager {
    /// Creates a new FileDownloadManager with default limits
    pub fn new() -> Self {
        Self::with_limits(FileDownloadLimits::default())
    }

    /// Creates a new FileDownloadManager with custom limits
    pub fn with_limits(limits: FileDownloadLimits) -> Self {
        let file_semaphore = Arc::new(Semaphore::new(limits.max_concurrent_file_downloads));

        Self {
            limits,
            file_semaphore,
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

    /// Get the maximum number of chunks to request in a single batch
    pub fn max_chunks_per_request(&self) -> usize {
        self.limits.max_chunks_per_request
    }

    /// Get the number of peer retry attempts for chunk downloads
    pub fn chunk_request_peer_retry_attempts(&self) -> usize {
        self.limits.chunk_request_peer_retry_attempts
    }

    /// Get the number of download retry attempts per peer
    pub fn download_retry_attempts(&self) -> usize {
        self.limits.download_retry_attempts
    }
}

impl Clone for FileDownloadManager {
    fn clone(&self) -> Self {
        Self {
            limits: self.limits.clone(),
            file_semaphore: Arc::clone(&self.file_semaphore),
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
        }
    }
}
