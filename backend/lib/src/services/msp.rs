//! MSP service implementation

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use alloy_core::{hex::ToHexExt, primitives::Address};
use axum_extra::extract::multipart::Field;
use bigdecimal::{BigDecimal, RoundingMode};
use bytes::BytesMut;
use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use sp_core::Blake2Hasher;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::RwLock;
use tokio_util::io::ReaderStream;
use tracing::{debug, warn};
use uuid::Uuid;

use shc_common::types::{
    ChunkId, FileKeyProof, FileMetadata, StorageProofsMerkleTrieLayout,
    BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE, FILE_CHUNK_SIZE,
};
use shc_file_manager::{in_memory::InMemoryFileDataTrie, traits::FileDataTrie};
use shc_indexer_db::{models::Bucket as DBBucket, OnchainMspId};
use shc_rpc::{
    GetFileFromFileStorageResult, GetValuePropositionsResult, RpcProviderId, SaveFileToDisk,
};
use shp_types::Hash;

use crate::{
    config::MspConfig,
    constants::{retry::get_retry_delay, stats::STATS_CACHE_TTL_SECS},
    data::{
        indexer_db::{client::DBClient, repository::PaymentStreamKind},
        rpc::StorageHubRpcClient,
    },
    error::Error,
    models::{
        buckets::{Bucket, FileTree},
        files::{FileInfo, FileUploadResponse},
        msp_info::{Capacity, InfoResponse, StatsResponse, ValuePropositionWithId},
        payment::{PaymentStreamInfo, PaymentStreamsResponse},
    },
};

/// Result of [`MspService::get_file_from_key`]
#[derive(Debug, Deserialize, Serialize)]
pub struct FileDownloadResult {
    pub file_size: u64,
    pub location: String,
    pub fingerprint: [u8; 32],
}

/// Cache entry for MSP stats
struct StatsCacheEntry {
    stats: StatsResponse,
    last_refreshed: Instant,
}

const UPLOAD_SPOOL_FILE_PREFIX: &str = "sh-upload-";
const UPLOAD_SPOOL_FILE_SUFFIX: &str = ".bin";
const UPLOAD_SPOOL_STARTUP_CLEANUP_MAX_AGE: Duration = Duration::from_secs(60 * 60);

/// Temporary on-disk spool for trusted upload forwarding.
///
/// This stores chunk records in the wire format expected by the trusted transfer server:
/// `[ChunkId: 8 bytes (u64, little-endian)][Chunk data: N bytes]...`
///
/// The file is automatically deleted on drop.
struct UploadSpool {
    path: PathBuf,
    writer: Option<BufWriter<tokio::fs::File>>,
    bytes_written: u64,
}

impl UploadSpool {
    async fn create() -> Result<Self, Error> {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "{}{}{}",
            UPLOAD_SPOOL_FILE_PREFIX,
            Uuid::now_v7(),
            UPLOAD_SPOOL_FILE_SUFFIX
        ));

        let file = tokio::fs::File::create(&path)
            .await
            .map_err(|e| Error::Storage(Box::new(e)))?;

        Ok(Self {
            path,
            writer: Some(BufWriter::with_capacity(1024 * 1024, file)),
            bytes_written: 0,
        })
    }

    async fn write_chunk_record(&mut self, chunk_id: u64, chunk: &[u8]) -> Result<(), Error> {
        let writer = self.writer.as_mut().ok_or(Error::Internal)?;
        self.bytes_written = self
            .bytes_written
            .saturating_add(8)
            .saturating_add(chunk.len() as u64);

        writer
            .write_all(&chunk_id.to_le_bytes())
            .await
            .map_err(|e| Error::Storage(Box::new(e)))?;
        writer
            .write_all(chunk)
            .await
            .map_err(|e| Error::Storage(Box::new(e)))?;
        Ok(())
    }

    async fn finish_for_reading(&mut self) -> Result<(), Error> {
        if let Some(writer) = self.writer.as_mut() {
            writer
                .flush()
                .await
                .map_err(|e| Error::Storage(Box::new(e)))?;
        }

        // Close file handle before re-opening for streaming.
        let _ = self.writer.take();
        Ok(())
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn bytes_written(&self) -> u64 {
        self.bytes_written
    }
}

impl Drop for UploadSpool {
    fn drop(&mut self) {
        // Ensure writer handle is closed before deleting the temp file.
        let _ = self.writer.take();
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Service for handling MSP-related operations
#[derive(Clone)]
pub struct MspService {
    msp_id: OnchainMspId,

    postgres: Arc<DBClient>,
    rpc: Arc<StorageHubRpcClient>,
    http_client: reqwest::Client,
    msp_config: MspConfig,
    stats_cache: Arc<RwLock<Option<StatsCacheEntry>>>,
}

impl MspService {
    /// Create a new MSP service
    ///
    /// The `msp_id` will be fetched once at startup using [`Self::discover_provider_id`]
    /// and cached for the lifetime of the backend service.
    pub async fn new(
        postgres: Arc<DBClient>,
        rpc: Arc<StorageHubRpcClient>,
        msp_config: MspConfig,
    ) -> Self {
        Self::cleanup_stale_upload_spools();
        let msp_id = Self::discover_provider_id(&rpc)
            .await
            .expect("MSP must be available when starting the backend's services");
        Self {
            msp_id,
            postgres,
            rpc,
            http_client: reqwest::Client::new(),
            msp_config,
            stats_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Best-effort startup cleanup for stale trusted-upload spool temp files.
    ///
    /// Files are deleted only if their filename matches this service's spool naming pattern
    /// and they are older than `UPLOAD_SPOOL_STARTUP_CLEANUP_MAX_AGE`.
    fn cleanup_stale_upload_spools() {
        let temp_dir = std::env::temp_dir();
        let now = std::time::SystemTime::now();
        let entries = match std::fs::read_dir(&temp_dir) {
            Ok(entries) => entries,
            Err(e) => {
                debug!(
                    target: "msp_service::cleanup_stale_upload_spools",
                    temp_dir = %temp_dir.display(),
                    error = %e,
                    "Failed to read temp directory while cleaning startup upload spools"
                );
                return;
            }
        };

        let mut removed = 0u64;

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    debug!(
                        target: "msp_service::cleanup_stale_upload_spools",
                        error = %e,
                        "Failed to read a temp directory entry while cleaning startup upload spools"
                    );
                    continue;
                }
            };

            let file_name = match entry.file_name().to_str() {
                Some(name) => name.to_string(),
                None => continue,
            };
            if !file_name.starts_with(UPLOAD_SPOOL_FILE_PREFIX)
                || !file_name.ends_with(UPLOAD_SPOOL_FILE_SUFFIX)
            {
                continue;
            }

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(e) => {
                    debug!(
                        target: "msp_service::cleanup_stale_upload_spools",
                        path = %path.display(),
                        error = %e,
                        "Failed to read metadata for startup upload spool candidate"
                    );
                    continue;
                }
            };

            let modified = match metadata.modified() {
                Ok(modified) => modified,
                Err(e) => {
                    debug!(
                        target: "msp_service::cleanup_stale_upload_spools",
                        path = %path.display(),
                        error = %e,
                        "Failed to read modified timestamp for startup upload spool candidate"
                    );
                    continue;
                }
            };

            let age = match now.duration_since(modified) {
                Ok(age) => age,
                Err(_) => continue,
            };
            if age < UPLOAD_SPOOL_STARTUP_CLEANUP_MAX_AGE {
                continue;
            }

            match std::fs::remove_file(&path) {
                Ok(()) => {
                    removed = removed.saturating_add(1);
                }
                Err(e) => {
                    debug!(
                        target: "msp_service::cleanup_stale_upload_spools",
                        path = %path.display(),
                        error = %e,
                        "Failed to delete stale startup upload spool"
                    );
                }
            }
        }

        if removed > 0 {
            debug!(
                target: "msp_service::cleanup_stale_upload_spools",
                removed = removed,
                "Removed stale trusted upload spool files at startup"
            );
        }
    }

    /// Discover the MSP provider ID from the connected node
    ///
    /// This function tries to discover the MSP's provider ID and, if the node is not yet
    /// registered as an MSP, it retries indefinitely with a stepped backoff strategy.
    ///
    /// The provider ID is queried once at startup and should be cached for the lifetime
    /// of the backend service. If the provider needs to change, the backend must be restarted.
    ///
    /// Note: Keep in mind that if the node is never registered as an MSP, this function
    /// will keep retrying indefinitely and the backend will fail to start. Monitor the
    /// retry attempt count in logs to detect potential configuration issues.
    pub async fn discover_provider_id(rpc: &StorageHubRpcClient) -> Result<OnchainMspId, Error> {
        let mut attempt = 0;

        loop {
            let provider_id: RpcProviderId = rpc.get_provider_id().await.map_err(|e| {
                Error::BadRequest(format!("Failed to get provider ID from RPC: {}", e))
            })?;

            match provider_id {
                RpcProviderId::Msp(id) => {
                    return Ok(OnchainMspId::new(Hash::from_slice(id.as_ref())))
                }
                RpcProviderId::Bsp(_) => {
                    return Err(Error::BadRequest(
                        "Connected node is a BSP; expected an MSP".to_string(),
                    ));
                }
                RpcProviderId::NotAProvider => {
                    // Calculate the retry delay before the next attempt based on the attempt number
                    let delay_secs = get_retry_delay(attempt);
                    warn!(
                        target: "msp_service::discover_provider_id",
                        delay_secs = delay_secs,
                        attempt = attempt + 1,
                        "Connected node is not yet a registered MSP; retrying provider discovery in {delay_secs} seconds... (attempt {attempt})"
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
                    attempt += 1;
                    continue;
                }
            }
        }
    }

    /// Get MSP information
    pub async fn get_info(&self) -> Result<InfoResponse, Error> {
        debug!(target: "msp_service::get_info", "Getting MSP info");

        // Fetch the MSP's local listen multiaddresses via RPC
        let multiaddresses: Vec<String> =
            self.rpc.get_multiaddresses().await.map_err(|e| {
                Error::BadRequest(format!("Failed to get MSP multiaddresses: {}", e))
            })?;

        // TODO: fetch from RPC the appropriate details
        Ok(InfoResponse {
            client: "storagehub-node v1.0.0".to_string(),
            version: "StorageHub MSP v0.1.0".to_string(),
            msp_id: self.msp_id.to_string(),
            multiaddresses,
            owner_account: "0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac".to_string(),
            payment_account: "0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac".to_string(),
            status: "active".to_string(),
            active_since: 123,
            uptime: "2 days, 1 hour".to_string(),
        })
    }

    /// Get MSP statistics
    ///
    /// This method caches the stats to avoid repeated RPC calls.
    /// The cache is automatically refreshed after the configured TTL expires.
    pub async fn get_stats(&self) -> Result<StatsResponse, Error> {
        debug!(target: "msp_service::get_stats", "Getting MSP stats");

        // Check if we have a valid cached entry
        {
            let cache = self.stats_cache.read().await;
            if let Some(entry) = &*cache {
                let cache_ttl = std::time::Duration::from_secs(STATS_CACHE_TTL_SECS);
                if entry.last_refreshed.elapsed() < cache_ttl {
                    debug!(target: "msp_service::get_stats", "Returning cached stats");
                    return Ok(entry.stats.clone());
                }
            }
        }

        // Cache miss or expired, fetch fresh data
        let info = self
            .rpc
            .get_msp_info(self.msp_id)
            .await
            .map_err(|e| Error::BadRequest(format!("Failed to retrieve MSP stats from RPC: {e}")))?
            .ok_or(Error::BadRequest(
                "Unable to retrieve MSP info: MSP not found".to_string(),
            ))?;

        let active_users = self
            .rpc
            .get_number_of_active_users(self.msp_id)
            .await
            .map_err(|e| {
                Error::BadRequest(format!(
                    "Field to retrieve number of active users from RPC: {e}"
                ))
            })?;

        let files_amount = self
            .postgres
            .get_number_of_files_stored_by_msp(&self.msp_id)
            .await
            .map_err(|e| {
                Error::BadRequest(format!(
                    "Failed to retrieve number of files stored by MSP: {e}"
                ))
            })?;

        debug!(?info, %active_users, "msp stats");

        let stats = StatsResponse {
            capacity: Capacity {
                total_bytes: BigDecimal::from(info.capacity).to_string(),
                available_bytes: BigDecimal::from(info.capacity - info.capacity_used).to_string(),
                used_bytes: BigDecimal::from(info.capacity_used).to_string(),
            },
            active_users: active_users as u64,
            last_capacity_change: BigDecimal::from(info.last_capacity_change).to_string(),
            value_props_amount: BigDecimal::from(info.amount_of_value_props).to_string(),
            buckets_amount: BigDecimal::from(info.amount_of_buckets).to_string(),
            files_amount: BigDecimal::from(files_amount).to_string(),
        };

        // Update cache with the new value
        {
            let mut cache = self.stats_cache.write().await;
            *cache = Some(StatsCacheEntry {
                stats: stats.clone(),
                last_refreshed: Instant::now(),
            });
        }

        Ok(stats)
    }

    /// Get MSP value propositions
    pub async fn get_value_props(&self) -> Result<Vec<ValuePropositionWithId>, Error> {
        debug!(target: "msp_service::get_value_props", "Getting MSP value propositions");

        // Call RPC to get the value propositions
        let result: GetValuePropositionsResult = self.rpc.get_value_props().await.map_err(|e| {
            Error::BadRequest(format!("Failed to get value propositions from RPC: {}", e))
        })?;

        // Decode the SCALE-encoded ValuePropositionWithId entries
        match result {
            GetValuePropositionsResult::Success(encoded_props) => {
                let mut props = Vec::with_capacity(encoded_props.len());
                for encoded_value_proposition in encoded_props {
                    let value_prop_with_id =
                        ValuePropositionWithId::decode(&mut encoded_value_proposition.as_slice())
                            .map_err(|e| {
                            Error::BadRequest(format!(
                                "Failed to decode ValuePropositionWithId: {}",
                                e
                            ))
                        })?;

                    props.push(ValuePropositionWithId {
                        id: value_prop_with_id.id,
                        value_prop: value_prop_with_id.value_prop,
                    });
                }
                Ok(props)
            }
            GetValuePropositionsResult::NotAnMsp => Err(Error::BadRequest(
                "The node that we are connected to is not an MSP".to_string(),
            )),
        }
    }

    /// List buckets for a user
    pub async fn list_user_buckets(
        &self,
        user_address: &Address,
        offset: i64,
        limit: i64,
    ) -> Result<impl Iterator<Item = Bucket>, Error> {
        debug!(target: "msp_service::list_user_buckets", user = %user_address, %limit, %offset, "Listing user buckets");

        Ok(self
            .postgres
            .get_user_buckets(
                &self.msp_id,
                &user_address.to_string(),
                Some(limit),
                Some(offset),
            )
            .await?
            .into_iter()
            .map(|entry| {
                // Convert BigDecimal to u64 for size (may lose precision)
                let size_bytes = entry.total_size.to_string().parse::<u64>().unwrap_or(0);
                let file_count = entry.file_count as u64;

                Bucket::from_db(&entry, size_bytes, file_count)
            }))
    }

    /// Get a specific bucket by its ID
    ///
    /// Verifies that the owner of the bucket is `user`. If the bucket is public, this check always passes.
    pub async fn get_bucket(
        &self,
        bucket_id: &str,
        user: Option<&Address>,
    ) -> Result<Bucket, Error> {
        debug!(target: "msp_service::get_bucket", bucket_id = %bucket_id, user = ?user, "Getting bucket");

        self.get_db_bucket(bucket_id)
            .await
            .and_then(|bucket| self.can_user_view_bucket(bucket, user))
            .map(|bucket| {
                // Convert BigDecimal to u64 for size (may lose precision)
                let size_bytes = bucket.total_size.to_string().parse::<u64>().unwrap_or(0);
                let file_count = bucket.file_count as u64;

                Bucket::from_db(&bucket, size_bytes, file_count)
            })
    }

    /// Get file tree for a bucket
    ///
    /// Verifies ownership of bucket is `user`
    /// Returns only direct children of the given path
    ///
    /// ## Business Rules for File Location Handling
    ///
    /// The given path is normalized using the following rules:
    /// * root is implicit
    /// * duplicated slashes are collapsed
    /// * trailing slashes are trimmed
    pub async fn get_file_tree(
        &self,
        bucket_id: &str,
        user: Option<&Address>,
        path: &str,
        offset: i64,
        limit: i64,
    ) -> Result<FileTree, Error> {
        debug!(target: "msp_service::get_file_tree", bucket_id = %bucket_id, user = ?user, %limit, %offset,  "Getting file tree");

        // first, get the bucket from the db and determine if user can view the bucket
        let bucket = self
            .get_db_bucket(bucket_id)
            .await
            .and_then(|bucket| self.can_user_view_bucket(bucket, user))?;

        // TODO: optimize query by requesting only matching paths
        // TODO: pagination doesn't account for path filtering
        let files = self
            .postgres
            .get_bucket_files(bucket.id, Some(limit), Some(offset))
            .await?;

        // Create hierarchy based on location segments
        Ok(FileTree::from_files_filtered(files, path))
    }

    /// Get file information
    ///
    /// Verifies ownership of bucket that the file belongs to is `user`, if private
    pub async fn get_file_info(
        &self,
        user: Option<&Address>,
        file_key: &str,
    ) -> Result<FileInfo, Error> {
        debug!(target: "msp_service::get_file_info", user = ?user, file_key = %file_key, "Getting file info");

        let file_key_hex = file_key.trim_start_matches("0x");

        let file_key = hex::decode(file_key_hex)
            .map_err(|e| Error::BadRequest(format!("Invalid File Key hex encoding: {}", e)))?;

        if file_key.len() != 32 {
            return Err(Error::BadRequest(format!(
                "Invalid File Key length. Expected 32 bytes, got {}",
                file_key.len()
            )));
        }

        let db_file = self.postgres.get_file_info(&file_key).await?;

        // get bucket determine if user can view it
        let bucket = self
            .get_bucket(&hex::encode(&db_file.onchain_bucket_id), user)
            .await?;

        Ok(FileInfo::from_db(&db_file, bucket.is_public))
    }

    /// Check via MSP RPC if this node is expecting to receive the given file key
    pub async fn is_msp_expecting_file_key(&self, file_key: &str) -> Result<bool, Error> {
        debug!(target: "msp_service::is_msp_expecting_file_key", file_key = %file_key, "Checking if MSP is expecting file key");

        let expected: bool = self.rpc.is_file_key_expected(file_key).await.map_err(|e| {
            Error::BadRequest(format!("Failed to check if file key is expected: {}", e))
        })?;

        if !expected {
            warn!(target: "msp_service::is_msp_expecting_file_key", file_key = %file_key, "MSP not expecting file_key");
        }

        Ok(expected)
    }

    /// Get all payment streams for a user
    pub async fn get_payment_streams(
        &self,
        user_address: &Address,
    ) -> Result<PaymentStreamsResponse, Error> {
        debug!(target: "msp_service::get_payment_streams", user = %user_address, "Getting payment streams");

        // Get all payment streams for the user from the database
        let payment_stream_data = self
            .postgres
            .get_payment_streams_for_user(&user_address.to_string())
            .await?;

        // Get current price per giga unit per tick from RPC (for dynamic rate calculations)
        let current_price_per_giga_unit_per_tick = self
            .rpc
            .get_current_price_per_giga_unit_per_tick()
            .await
            .map_err(|e| Error::BadRequest(format!("Failed to get price per unit: {}", e)))?;

        // Process each payment stream
        let mut streams = Vec::new();
        for stream_data in payment_stream_data {
            let (provider_type, cost_per_tick) = match stream_data.kind {
                PaymentStreamKind::Fixed { rate } => {
                    // This is an MSP (fixed rate payment stream)
                    ("msp".to_string(), rate.to_string())
                }
                PaymentStreamKind::Dynamic { amount_provided } => {
                    // This is a BSP (dynamic rate payment stream)
                    // Cost per tick = amount_provided * 1e-9 * current_price_per_giga_unit_per_tick

                    // Matches the computation done in the runtime
                    //
                    // (price * amount) / gigaunit
                    let cost = (&current_price_per_giga_unit_per_tick * amount_provided)
                        / shp_constants::GIGAUNIT;

                    // Truncate the decimal digits of the cost per tick
                    let cost = cost.with_scale_round(0, RoundingMode::Down);

                    ("bsp".to_string(), cost.to_string())
                }
            };

            streams.push(PaymentStreamInfo {
                provider: stream_data.provider,
                provider_type,
                total_amount_paid: stream_data.total_amount_paid.to_string(),
                cost_per_tick,
            });
        }

        Ok(PaymentStreamsResponse { streams })
    }

    /// Calls is_file_in_file_storage RPC method from the MSP substrate node
    /// to get file metadata if present.
    ///
    /// Returns successfully only if the file is present and fully stored in
    /// the MSP node (i.e. all chunks of the file are present).
    /// Returns error in any other case, with descriptive message.
    ///
    /// ```ignore
    /// pub enum GetFileFromFileStorageResult {
    ///     FileNotFound, // returns Error
    ///     IncompleteFile(IncompleteFileStatus), // returns Error
    ///     FileFound(FileMetadata), // returns Ok
    ///     FileFoundWithInconsistency(FileMetadata), // returns Error
    /// }
    /// ```
    pub async fn check_file_status(&self, file_key: &str) -> Result<FileMetadata, Error> {
        let file_status: GetFileFromFileStorageResult = self
            .rpc
            .is_file_in_file_storage(file_key)
            .await
            .map_err(|e| Error::BadRequest(e.to_string()))?;

        match file_status {
            GetFileFromFileStorageResult::FileNotFound => {
                Err(Error::NotFound("File not found".to_string()))
            }
            GetFileFromFileStorageResult::FileFoundWithInconsistency(_inconsistent_metadata) => {
                Err(Error::BadRequest(
                    "File found with inconsistency".to_string(),
                ))
            }
            GetFileFromFileStorageResult::IncompleteFile(_status) => {
                Err(Error::BadRequest("File is incomplete".to_string()))
            }
            GetFileFromFileStorageResult::FileFound(metadata) => Ok(metadata),
        }
    }

    /// Download the given `file` via the MSP RPC to the specified `session_id`, and
    /// return its size, UTF-8 location and fingerprint.
    /// Returns BadRequest on RPC/parse errors.
    ///
    /// We provide an URL as saveFileToDisk RPC requires it to stream the file.
    /// We also implemented the trusted_upload_by_key handler to handle the upload to the client.
    pub async fn get_file(
        &self,
        session_id: &str,
        file: FileInfo,
    ) -> Result<FileDownloadResult, Error> {
        let file_key = &file.file_key;
        debug!(target: "msp_service::get_file_from_key", file_key = %file_key, "Downloading file by key");

        // TODO(AUTH): Add MSP Node authentication credentials
        // Currently this internal endpoint doesn't authenticate that
        // the client connecting to it is the MSP Node
        let upload_url = format!(
            "{}/internal/uploads/{}/{}",
            self.msp_config.callback_url, session_id, file_key
        );

        // Make the RPC call to download file and get metadata
        let rpc_response: SaveFileToDisk = self
            .rpc
            .save_file_to_disk(file_key, upload_url.as_str())
            .await
            .map_err(|e| {
                Error::BadRequest(format!("Failed to save file to disk via RPC: {}", e))
            })?;

        match rpc_response {
            SaveFileToDisk::FileNotFound => {
                warn!(target: "msp_service::get_file_from_key", file_key = %file_key, "File not found for download");
                Err(Error::NotFound("File not found".to_string()))
            }
            SaveFileToDisk::IncompleteFile(_status) => {
                warn!(
                    target: "msp_service::get_file_from_key",
                    file_key = %file_key,
                    "Incomplete file requested for download"
                );
                Err(Error::BadRequest("File is incomplete".to_string()))
            }
            SaveFileToDisk::Success(_file_metadata) => {
                // TODO: re-enable these checks once the Mock RPC returns the correct data
                // It's a defensive check to ensure the RPC returns correct data,
                // unfortunately, the mock RPC doesn't have access to the expected data
                // which makes the SDK Mock tests fail

                // // Convert location bytes to string
                // let location = String::from_utf8_lossy(file_metadata.location()).to_string();
                // let file_size = file_metadata.file_size();
                // let fingerprint = file_metadata.fingerprint().as_hash();

                // // Ensure data received from MSP matches what we expect
                // if location != file.location
                //     || file_size != file.size
                //     || fingerprint != file.fingerprint
                // {
                //     Err(Error::BadRequest(
                //         "Downloaded file doesn't match given file key".to_string(),
                //     ))
                // } else {

                debug!(
                    "File download prepared - file_key: {}, size: {} bytes",
                    file.file_key, file.size
                );

                Ok(FileDownloadResult {
                    file_size: file.size,
                    location: file.location,
                    fingerprint: file.fingerprint,
                })
                // }
            }
        }
    }

    /// Process a streamed file upload: validate metadata, chunk into trie, batch proofs, and send to MSP.
    ///
    /// Verifies that `user` owns the bucket that the file belongs to
    pub async fn process_and_upload_file(
        &self,
        user: Option<&Address>,
        file_key: &str,
        mut file_data_stream: Field,
        file_metadata: FileMetadata,
    ) -> Result<FileUploadResponse, Error> {
        debug!(
            target: "msp_service::process_and_upload_file",
            file_key = %file_key,
            file_size = file_metadata.file_size(),
            "Starting file upload"
        );

        // Validate the received file key against the one corresponding to the file metadata.
        let expected_file_key = hex::encode(file_metadata.file_key::<Blake2Hasher>());
        let file_key_without_prefix = file_key.trim_start_matches("0x");
        if file_key_without_prefix != expected_file_key {
            return Err(Error::BadRequest(format!(
                "File key in URL does not match file metadata: {expected_file_key} != {file_key_without_prefix}"
            )));
        }

        // Get the bucket ID from the metadata and verify that the user is its owner.
        // We check the bucket ownership instead of the file ownership as the file might not be in
        // the indexer at this point (since the storage request would have to have been finalised).
        // TODO: This could still fail as the bucket creation extrinsic might not have been finalised yet,
        // ideally we should have a way to directly check on-chain (like an RPC).
        let bucket_id = hex::encode(file_metadata.bucket_id());
        let bucket = self.get_db_bucket(&bucket_id).await?;
        self.can_user_view_bucket(bucket, user)?;

        // Initialize the trie that will hold the chunked file data.
        let mut trie = InMemoryFileDataTrie::<StorageProofsMerkleTrieLayout>::new();

        // For the trusted file transfer upload method, spool the outbound wire bytes locally
        // while ingesting the multipart stream. This preserves the policy:
        // "verify fingerprint first, only then contact MSP",
        // while avoiding a second pass that re-reads all chunks from the trie for sending.
        let mut spool = if self.msp_config.use_legacy_upload_method {
            None
        } else {
            Some(UploadSpool::create().await?)
        };

        // Prepare the overflow buffer that will hold any data that doesn't exactly fit in a chunk.
        //
        // Use `BytesMut` so we can consume a prefix in O(1) via `split_to`,
        // instead of repeatedly `drain(..k)`ing a `Vec` (which shifts remaining bytes and can
        // become quadratic over large uploads).
        let mut overflow_buffer = BytesMut::new();

        // Accumulate chunks into batches before inserting into the trie, to reduce per-chunk overhead
        // (allocator pressure + trie-mutator rebuild costs).
        let chunks_per_batch =
            (BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE / FILE_CHUNK_SIZE as usize).max(1);
        if BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE < FILE_CHUNK_SIZE as usize {
            warn!(
                target: "msp_service::process_and_upload_file",
                batch_max_bytes = BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE,
                chunk_size_bytes = FILE_CHUNK_SIZE,
                "BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE is smaller than FILE_CHUNK_SIZE; falling back to 1 chunk per batch"
            );
        }
        let mut pending_chunks: Vec<Vec<u8>> = Vec::with_capacity(chunks_per_batch);

        // Initialize the chunk index.
        let mut chunk_index: u64 = 0;

        // Start streaming the file data into the trie, chunking it into FILE_CHUNK_SIZE chunks in the process.
        while let Some(bytes_read) = file_data_stream.chunk().await.map_err(|e| {
            Error::BadRequest(format!("Failed to read multipart stream chunk: {}", e))
        })? {
            // Load the bytes read from the file into the overflow buffer.
            overflow_buffer.extend_from_slice(&bytes_read);

            // While the overflow buffer is larger than FILE_CHUNK_SIZE, process a chunk.
            while overflow_buffer.len() >= FILE_CHUNK_SIZE as usize {
                // Consume the next FILE_CHUNK_SIZE bytes without shifting the remaining buffer.
                let chunk = overflow_buffer
                    .split_to(FILE_CHUNK_SIZE as usize)
                    .as_ref()
                    .to_vec();

                if let Some(spool) = spool.as_mut() {
                    spool.write_chunk_record(chunk_index, &chunk).await?;
                }
                pending_chunks.push(chunk);

                // Increment the chunk index.
                chunk_index += 1;

                if pending_chunks.len() == chunks_per_batch {
                    let start_index = chunk_index - pending_chunks.len() as u64;
                    let batch = std::mem::take(&mut pending_chunks);

                    trie.write_chunks_batched(start_index, batch).map_err(|e| {
                        Error::BadRequest(format!(
                            "Failed to write chunk batch (start={}, count={}) to trie: {}",
                            start_index, chunks_per_batch, e
                        ))
                    })?;
                }
            }
        }

        // Check the overflow buffer to see if the file didn't fit exactly in an integer number of chunks.
        if !overflow_buffer.is_empty() {
            let overflow_bytes = overflow_buffer.as_ref().to_vec();

            if let Some(spool) = spool.as_mut() {
                spool
                    .write_chunk_record(chunk_index, &overflow_bytes)
                    .await?;
            }
            pending_chunks.push(overflow_bytes);

            // Increment the chunk index to get the total amount of chunks.
            chunk_index += 1;
        }

        // Flush any remaining pending chunks into the trie.
        if !pending_chunks.is_empty() {
            let start_index = chunk_index - pending_chunks.len() as u64;
            let batch = std::mem::take(&mut pending_chunks);

            trie.write_chunks_batched(start_index, batch).map_err(|e| {
                Error::BadRequest(format!(
                    "Failed to write final chunk batch (start={}, count={}) to trie: {}",
                    start_index,
                    chunk_index - start_index,
                    e
                ))
            })?;
        }

        // Validate that the file fingerprint matches the trie root.
        let computed_root = trie.get_root();
        if computed_root.as_ref() != file_metadata.fingerprint().as_ref() {
            return Err(Error::BadRequest(format!(
                "File fingerprint mismatch. Expected: {}, Computed: {}",
                hex::encode(file_metadata.fingerprint().as_ref()),
                hex::encode(computed_root)
            )));
        }

        // Validate that the received amount of chunks matches the amount of chunks corresponding to the file size in the metadata.
        let total_chunks = file_metadata.chunks_count();
        if chunk_index != total_chunks {
            return Err(Error::BadRequest(format!(
            "Received amount of chunks {} does not match the amount of chunks {} corresponding to the file size in the metadata",
            chunk_index, total_chunks
        )));
        }

        debug!(target: "msp_service::process_and_upload_file", total_chunks = total_chunks, "File chunking completed");

        // Choose upload method based on configuration
        if self.msp_config.use_legacy_upload_method {
            debug!(target: "msp_service::process_and_upload_file", "Using legacy RPC-based upload method (receiveBackendFileChunks)");
            self.legacy_upload_file_in_batches(&trie, &file_metadata, total_chunks)
                .await
                .map_err(|e| {
                    Error::BadRequest(format!(
                        "Failed to send chunks to MSP via legacy method: {}",
                        e
                    ))
                })?;
        } else {
            debug!(target: "msp_service::process_and_upload_file", "Using new trusted file transfer server upload method");
            let spool = spool.as_mut().ok_or(Error::Internal)?;
            spool.finish_for_reading().await?;
            self.send_spooled_chunks_to_msp(spool.path(), file_key, spool.bytes_written())
                .await
                .map_err(|e| Error::BadRequest(format!("Failed to send chunks to MSP: {}", e)))?;
        }

        // If the complete file was uploaded to the MSP successfully, we can return the response.
        let bytes_location = file_metadata.location();
        let location = str::from_utf8(bytes_location)
            .unwrap_or(file_key)
            .to_string();

        debug!(
            file_key = %file_key,
            chunks = total_chunks,
            "File upload completed"
        );

        Ok(FileUploadResponse {
            status: "upload_successful".to_string(),
            fingerprint: file_metadata.fingerprint().encode_hex_with_prefix(),
            file_key: file_key.to_string(),
            bucket_id,
            location,
        })
    }
}

impl MspService {
    /// Send pre-verified chunks to the MSP trusted file transfer server.
    ///
    /// The backend enforces "verify fingerprint first, only then contact MSP" by spooling the
    /// outbound wire bytes locally during ingestion. This method then streams that spool.
    ///
    /// Wire format:
    /// `[ChunkId: 8 bytes (u64, little-endian)][Chunk data: N bytes]...`
    async fn send_spooled_chunks_to_msp(
        &self,
        spool_path: &Path,
        file_key: &str,
        spooled_bytes: u64,
    ) -> Result<(), Error> {
        let url = format!(
            "{}/upload/{}",
            self.msp_config.trusted_file_transfer_server_url, file_key
        );

        let file = tokio::fs::File::open(spool_path)
            .await
            .map_err(|e| Error::Storage(Box::new(e)))?;
        let body = reqwest::Body::wrap_stream(ReaderStream::new(file));

        // Send the POST request using the shared client for connection pooling.
        let response = self
            .http_client
            .post(&url)
            .body(body)
            .send()
            .await
            .map_err(|e| Error::BadRequest(format!("Failed to send request to MSP: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read response".to_string());
            return Err(Error::BadRequest(format!(
                "MSP trusted file transfer server returned error: {} - {} (file_key={}, spooled_bytes={})",
                status, body, file_key, spooled_bytes
            )));
        }
        Ok(())
    }

    // TODO: Remove this method once legacy upload is deprecated
    /// Legacy method: Upload file in batches using RPC calls
    async fn legacy_upload_file_in_batches(
        &self,
        trie: &InMemoryFileDataTrie<StorageProofsMerkleTrieLayout>,
        file_metadata: &FileMetadata,
        total_chunks: u64,
    ) -> Result<(), Error> {
        // Get how many chunks fit in a batch of BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE, rounding down.
        let chunks_per_batch = (BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE as u64 / FILE_CHUNK_SIZE).max(1);
        if BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE < FILE_CHUNK_SIZE as usize {
            warn!(
                target: "msp_service::legacy_upload_file_in_batches",
                batch_max_bytes = BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE,
                chunk_size_bytes = FILE_CHUNK_SIZE,
                "BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE is smaller than FILE_CHUNK_SIZE; falling back to 1 chunk per batch"
            );
        }

        // Initialize the index of the initial chunk to process in this batch.
        let mut batch_start_chunk_index = 0;
        let total_batches = (total_chunks + chunks_per_batch - 1) / chunks_per_batch;
        let mut batch_number = 1;

        // Start processing batches, until all chunks have been processed.
        while batch_start_chunk_index < total_chunks {
            // Get the chunks to send in this batch, capping at the total amount of chunks of the file.
            let chunks = (batch_start_chunk_index
                ..(batch_start_chunk_index + chunks_per_batch).min(total_chunks))
                .map(ChunkId::new)
                .collect::<HashSet<_>>();
            let chunks_in_batch = chunks.len() as u64;

            debug!(
                target: "msp_service::legacy_upload_file_in_batches",
                batch_number = batch_number,
                total_batches = total_batches,
                chunk_start = batch_start_chunk_index,
                chunk_end = batch_start_chunk_index + chunks_in_batch - 1,
                "Processing batch"
            );

            // Generate the proof for the batch.
            let file_proof = trie.generate_proof(&chunks).map_err(|e| {
                Error::BadRequest(format!(
                    "Failed to generate proof for batch {}: {}",
                    batch_number, e
                ))
            })?;

            // Convert the generated proof to a FileKeyProof and send it to the MSP.
            let file_key_proof = file_proof
                .to_file_key_proof(file_metadata.clone())
                .map_err(|e| Error::BadRequest(format!("Failed to convert proof: {:?}", e)))?;

            // Send the proof with the chunks to the MSP.
            self.legacy_upload_to_msp(&chunks, &file_key_proof)
                .await
                .map_err(|e| {
                    Error::BadRequest(format!(
                        "Failed to upload batch {} to MSP: {}",
                        batch_number, e
                    ))
                })?;

            debug!(
                target: "msp_service::legacy_upload_file_in_batches",
                batch_number = batch_number,
                total_batches = total_batches,
                "Batch uploaded successfully"
            );

            // Update the initial chunk index for the next batch.
            batch_start_chunk_index += chunks_in_batch;
            batch_number += 1;
        }

        Ok(())
    }

    // TODO: Remove this method once legacy upload is deprecated
    /// Legacy method: Upload chunks to MSP via RPC
    async fn legacy_upload_to_msp(
        &self,
        chunk_ids: &HashSet<ChunkId>,
        file_key_proof: &FileKeyProof,
    ) -> Result<(), Error> {
        debug!(
            target: "msp_service::legacy_upload_to_msp",
            chunk_count = chunk_ids.len(),
            "Uploading chunks to MSP"
        );

        // Ensure we are not incorrectly trying to upload an empty file.
        if chunk_ids.is_empty() {
            return Err(Error::BadRequest(
                "Cannot upload file with no chunks".to_string(),
            ));
        }

        debug!(target: "msp_service::legacy_upload_to_msp", "Trying to send the chunks batch");
        let ret = self
            .legacy_send_upload_request_to_msp(file_key_proof.clone())
            .await;
        if ret.is_ok() {
            debug!(
                target: "msp_service::legacy_upload_to_msp",
                chunk_count = chunk_ids.len(),
                file_key = %format!("0x{}", hex::encode(file_key_proof.file_metadata.file_key::<Blake2Hasher>())),
                bucket_id = %format!("0x{}", hex::encode(file_key_proof.file_metadata.bucket_id())),
                "Successfully uploaded chunks to MSP"
            );
        }
        ret
    }

    // TODO: Remove this method once legacy upload is deprecated
    /// Legacy method: Send upload request to MSP via RPC
    async fn legacy_send_upload_request_to_msp(
        &self,
        file_key_proof: FileKeyProof,
    ) -> Result<(), Error> {
        debug!(
            target: "msp_service::legacy_send_upload_request_to_msp",
            "Attempting to send upload request to MSP"
        );

        // Get the file metadata from the received FileKeyProof.
        let file_metadata = file_key_proof.clone().file_metadata;

        // Get the file key from the file metadata.
        let file_key: shp_types::Hash = file_metadata.file_key::<Blake2Hasher>();
        let file_key_hexstr = format!("{file_key:x}");

        // Encode the FileKeyProof as SCALE for transport
        let encoded_proof = file_key_proof.encode();

        let max_retries = self.msp_config.upload_retry_attempts;
        let delay_between_retries_secs = self.msp_config.upload_retry_delay_secs;
        let mut retry_attempts = 0;

        while retry_attempts < max_retries {
            debug!(target: "msp_service::legacy_send_upload_request_to_msp", "Sending file chunks to MSP via RPC");
            let result: Result<Vec<u8>, _> = self
                .rpc
                .receive_file_chunks(&file_key_hexstr, encoded_proof.clone())
                .await;

            match result {
                Ok(_raw) => {
                    debug!("Successfully sent upload request to MSP");
                    return Ok(());
                }
                Err(e) => {
                    retry_attempts += 1;
                    if retry_attempts < max_retries {
                        warn!(
                            target: "msp_service::legacy_send_upload_request_to_msp",
                            retry_attempt = retry_attempts,
                            error = ?e,
                            "Upload request to MSP failed via RPC, retrying... (attempt {retry_attempts})",
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(
                            delay_between_retries_secs,
                        ))
                        .await;
                    } else {
                        return Err(Error::Internal);
                    }
                }
            }
        }

        Err(Error::Internal)
    }

    /// Verifies that a user can access the given bucket.
    ///
    /// If the bucket is public, this check always passes.
    ///
    /// Will return the bucket metadata if the user has the required permissions, or an error otherwise.
    fn can_user_view_bucket(
        &self,
        bucket: DBBucket,
        user: Option<&Address>,
    ) -> Result<DBBucket, Error> {
        // TODO: NFT ownership
        if bucket.private {
            let Some(user) = user else {
                return Err(Error::Unauthorized(format!(
                    "Bucket with ID {} is private and no user received.",
                    bucket.onchain_bucket_id.encode_hex_with_prefix()
                )));
            };

            if bucket.account.as_str() == user.to_string() {
                Ok(bucket)
            } else {
                Err(Error::Unauthorized(format!(
                    "User {} is not authorized to view bucket with ID {}",
                    user,
                    bucket.onchain_bucket_id.encode_hex_with_prefix()
                )))
            }
        } else {
            Ok(bucket)
        }
    }

    /// Retrieve a bucket from the DB
    ///
    /// Will NOT verify ownership, see [`can_user_view_bucket`]
    async fn get_db_bucket(&self, bucket_id: &str) -> Result<DBBucket, Error> {
        let bucket_id_hex = bucket_id.trim_start_matches("0x");

        let bucket_id = hex::decode(bucket_id_hex)
            .map_err(|e| Error::BadRequest(format!("Invalid Bucket ID hex encoding: {}", e)))?;

        if bucket_id.len() != 32 {
            return Err(Error::BadRequest(format!(
                "Invalid Bucket ID length. Expected 32 bytes, got {}",
                bucket_id.len()
            )));
        }

        self.postgres.get_bucket(&bucket_id).await
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use std::{str::FromStr, sync::Arc};

    use bigdecimal::{BigDecimal, Signed};
    use serde_json::Value;

    use shp_types::Hash;

    use super::*;
    use crate::{
        config::Config,
        constants::{
            database::DEFAULT_PAGE_LIMIT,
            mocks::{MOCK_ADDRESS, MOCK_PRICE_PER_GIGA_UNIT},
            rpc::DUMMY_MSP_ID,
            test::{bucket::DEFAULT_BUCKET_NAME, file::DEFAULT_SIZE},
        },
        data::{
            indexer_db::{
                client::DBClient, mock_repository::MockRepository, repository::PaymentStreamKind,
            },
            rpc::{AnyRpcConnection, MockConnection, StorageHubRpcClient},
        },
        test_utils::random_bytes_32,
    };

    /// Builder for creating MspService instances with mock dependencies for testing
    struct MockMspServiceBuilder {
        postgres: Arc<DBClient>,
        rpc: Arc<StorageHubRpcClient>,
    }

    impl MockMspServiceBuilder {
        /// Create a new builder with default empty mocks
        pub fn new() -> Self {
            Self {
                postgres: Arc::new(DBClient::new(Arc::new(MockRepository::new()))),
                rpc: Arc::new(StorageHubRpcClient::new(Arc::new(AnyRpcConnection::Mock(
                    MockConnection::new(),
                )))),
            }
        }

        /// Initialize repository with custom test data
        pub async fn init_repository_with<F>(self, init: F) -> Self
        where
            F: FnOnce(&DBClient) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + '_>>,
        {
            init(&self.postgres).await;
            self
        }

        #[allow(dead_code)] // useful helper if we are making requests that we don't mock yet
        /// Set RPC responses for the connection to use
        pub async fn with_rpc_responses(mut self, responses: Vec<(&str, Value)>) -> Self {
            let mock_conn = MockConnection::new();
            for (method, value) in responses {
                mock_conn.set_response(method, value).await;
            }
            self.rpc = Arc::new(StorageHubRpcClient::new(Arc::new(AnyRpcConnection::Mock(
                mock_conn,
            ))));
            self
        }

        /// Build the final MspService
        pub async fn build(self) -> MspService {
            let cfg = Config::default();

            MspService::new(self.postgres, self.rpc, cfg.msp).await
        }
    }

    #[tokio::test]
    async fn test_get_info() {
        let service = MockMspServiceBuilder::new().build().await;
        let info = service.get_info().await.unwrap();

        assert_eq!(info.status, "active");
        assert!(!info.multiaddresses.is_empty());
    }

    #[tokio::test]
    async fn test_get_stats() {
        let service = MockMspServiceBuilder::new()
            .init_repository_with(|client| {
                Box::pin(async move {
                    // Create MSP with the ID that matches the default config
                    let _ = client
                        .create_msp(
                            &MOCK_ADDRESS.to_string(),
                            OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
                        )
                        .await
                        .expect("should create MSP");
                })
            })
            .await
            .build()
            .await;
        let stats = service.get_stats().await.unwrap();

        let total_bytes = stats
            .capacity
            .total_bytes
            .parse::<BigDecimal>()
            .expect("total_bytes to be a number");
        let available_bytes = stats
            .capacity
            .available_bytes
            .parse::<BigDecimal>()
            .expect("available_bytes to be a number");
        let used_bytes = stats
            .capacity
            .used_bytes
            .parse::<BigDecimal>()
            .expect("used_bytes to be a number");

        assert!(
            total_bytes.is_positive(),
            "Total bytes should always be positive"
        );
        assert!(
            available_bytes <= total_bytes,
            "Available bytes should never be more than total bytes"
        );
        assert!(
            used_bytes <= total_bytes,
            "Used bytes should never be more than total bytes"
        );
        assert_eq!(
            used_bytes + available_bytes,
            total_bytes,
            "Available + used bytes should always equal total bytes"
        );

        // Verify last_capacity_change is a valid number string
        let last_capacity_change = stats
            .last_capacity_change
            .parse::<BigDecimal>()
            .expect("last_capacity_change to be a valid number");
        assert!(
            !last_capacity_change.is_negative(),
            "Last capacity change should be non-negative"
        );

        // Verify value_props_amount is a valid number string
        let value_props_amount = stats
            .value_props_amount
            .parse::<BigDecimal>()
            .expect("value_props_amount to be a valid number");
        assert!(
            !value_props_amount.is_negative(),
            "Value props amount should be non-negative"
        );

        // Verify buckets_amount is a valid number string
        let buckets_amount = stats
            .buckets_amount
            .parse::<BigDecimal>()
            .expect("buckets_amount to be a valid number");
        assert!(
            !buckets_amount.is_negative(),
            "Buckets amount should be non-negative"
        );
    }

    #[tokio::test]
    async fn test_get_stats_caching() {
        let service = MockMspServiceBuilder::new()
            .init_repository_with(|client| {
                Box::pin(async move {
                    // Create MSP with the ID that matches the default config
                    let _ = client
                        .create_msp(
                            &MOCK_ADDRESS.to_string(),
                            OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
                        )
                        .await
                        .expect("should create MSP");
                })
            })
            .await
            .build()
            .await;

        // 1. Verify cache is empty before first call
        {
            let cache = service.stats_cache.read().await;
            assert!(cache.is_none(), "Cache should be empty before first call");
        }

        // 2. First call should populate the cache
        let stats1 = service.get_stats().await.unwrap();

        // 3. Verify cache is now populated and capture the timestamp
        let first_timestamp = {
            let cache = service.stats_cache.read().await;
            assert!(
                cache.is_some(),
                "Cache should be populated after first call"
            );
            cache.as_ref().unwrap().last_refreshed
        };

        // 4. Second call should return cached data
        let stats2 = service.get_stats().await.unwrap();

        // 5. Verify cache timestamp hasn't changed (proving it was a cache hit)
        {
            let cache = service.stats_cache.read().await;
            let entry = cache.as_ref().unwrap();
            assert_eq!(
                entry.last_refreshed, first_timestamp,
                "Cache timestamp should not change on cache hit"
            );
        }

        // 6. Verify both calls return the same data
        assert_eq!(stats1.active_users, stats2.active_users);
        assert_eq!(stats1.capacity.total_bytes, stats2.capacity.total_bytes);
        assert_eq!(stats1.capacity.used_bytes, stats2.capacity.used_bytes);
        assert_eq!(
            stats1.capacity.available_bytes,
            stats2.capacity.available_bytes
        );
        assert_eq!(stats1.last_capacity_change, stats2.last_capacity_change);
        assert_eq!(stats1.value_props_amount, stats2.value_props_amount);
        assert_eq!(stats1.buckets_amount, stats2.buckets_amount);
    }

    #[tokio::test]
    async fn test_get_value_props() {
        let service = MockMspServiceBuilder::new().build().await;
        let props = service.get_value_props().await.unwrap();

        assert!(!props.is_empty());
        assert!(props.iter().any(|p| p.value_prop.available));
    }

    #[tokio::test]
    async fn test_list_user_buckets() {
        let service = MockMspServiceBuilder::new()
            .init_repository_with(|client| {
                Box::pin(async move {
                    // Create MSP with the ID that matches the default config
                    let msp = client
                        .create_msp(
                            &MOCK_ADDRESS.to_string(),
                            OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
                        )
                        .await
                        .expect("should create MSP");

                    // Create a test bucket for the mock user
                    client
                        .create_bucket(
                            &MOCK_ADDRESS.to_string(),
                            Some(msp.id),
                            DEFAULT_BUCKET_NAME.as_bytes(),
                            random_bytes_32().as_slice(),
                            false,
                        )
                        .await
                        .expect("should create bucket");
                })
            })
            .await
            .build()
            .await;

        let buckets = service
            .list_user_buckets(&MOCK_ADDRESS, 0, DEFAULT_PAGE_LIMIT)
            .await
            .unwrap()
            .collect::<Vec<_>>();

        assert!(!buckets.is_empty());
    }

    #[tokio::test]
    async fn test_get_bucket() {
        let bucket_name = "my-bucket";
        let bucket_id = random_bytes_32();

        let service = MockMspServiceBuilder::new()
            .init_repository_with(|client| {
                Box::pin(async move {
                    // Create MSP with the ID that matches the default config
                    let msp = client
                        .create_msp(
                            &MOCK_ADDRESS.to_string(),
                            OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
                        )
                        .await
                        .expect("should create MSP");

                    // Create a test bucket for the mock user
                    let bucket = client
                        .create_bucket(
                            &MOCK_ADDRESS.to_string(),
                            Some(msp.id),
                            bucket_name.as_bytes(),
                            &bucket_id,
                            false,
                        )
                        .await
                        .expect("should create bucket");

                    client
                        .create_file(
                            MOCK_ADDRESS.to_string().as_bytes(),
                            random_bytes_32().as_slice(),
                            bucket.id,
                            &bucket_id,
                            "sample-file.txt".as_bytes(),
                            random_bytes_32().as_slice(),
                            DEFAULT_SIZE,
                        )
                        .await
                        .expect("should create file");
                })
            })
            .await
            .build()
            .await;

        let bucket_id = hex::encode(bucket_id);
        let bucket = service
            .get_bucket(&bucket_id, Some(&MOCK_ADDRESS))
            .await
            .unwrap();

        assert_eq!(bucket.bucket_id, bucket_id);
        assert_eq!(bucket.name, bucket_name);
    }

    #[tokio::test]
    async fn test_get_files_root() {
        let bucket_id = random_bytes_32();

        let service = MockMspServiceBuilder::new()
            .init_repository_with(|client| {
                Box::pin(async move {
                    // Create MSP with the ID that matches the default config
                    let msp = client
                        .create_msp(
                            &MOCK_ADDRESS.to_string(),
                            OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
                        )
                        .await
                        .expect("should create MSP");

                    // Create a test bucket for the mock user
                    let bucket = client
                        .create_bucket(
                            &MOCK_ADDRESS.to_string(),
                            Some(msp.id),
                            DEFAULT_BUCKET_NAME.as_bytes(),
                            &bucket_id,
                            false,
                        )
                        .await
                        .expect("should create bucket");

                    client
                        .create_file(
                            MOCK_ADDRESS.to_string().as_bytes(),
                            random_bytes_32().as_slice(),
                            bucket.id,
                            &bucket_id,
                            "sample-file.txt".as_bytes(),
                            random_bytes_32().as_slice(),
                            DEFAULT_SIZE,
                        )
                        .await
                        .expect("should create file");
                })
            })
            .await
            .build()
            .await;

        let filter = "/";
        let tree = service
            .get_file_tree(
                hex::encode(bucket_id).as_ref(),
                Some(&MOCK_ADDRESS),
                filter,
                0,
                DEFAULT_PAGE_LIMIT,
            )
            .await
            .unwrap();

        assert_eq!(
            tree.name.as_str(),
            filter,
            "Folder name should match folder"
        );
        assert!(tree.children.len() > 0, "Shold have at least 1 entry");
    }

    #[tokio::test]
    async fn test_get_file_info() {
        let file_key = random_bytes_32();
        let bucket_id = random_bytes_32();

        let service = MockMspServiceBuilder::new()
            .init_repository_with(|client| {
                Box::pin(async move {
                    // Create MSP with the ID that matches the default config
                    let msp = client
                        .create_msp(
                            &MOCK_ADDRESS.to_string(),
                            OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
                        )
                        .await
                        .expect("should create MSP");

                    // Create a test bucket for the mock user
                    let bucket = client
                        .create_bucket(
                            &MOCK_ADDRESS.to_string(),
                            Some(msp.id),
                            DEFAULT_BUCKET_NAME.as_bytes(),
                            &bucket_id,
                            false,
                        )
                        .await
                        .expect("should create bucket");

                    client
                        .create_file(
                            MOCK_ADDRESS.to_string().as_bytes(),
                            &file_key,
                            bucket.id,
                            &bucket_id,
                            "sample-file.txt".as_bytes(),
                            random_bytes_32().as_slice(),
                            DEFAULT_SIZE,
                        )
                        .await
                        .expect("should create file");
                })
            })
            .await
            .build()
            .await;

        let bucket_id = hex::encode(bucket_id);
        let file_key = hex::encode(file_key);

        let info = service
            .get_file_info(Some(&MOCK_ADDRESS), &file_key)
            .await
            .expect("get_file_info should succeed");

        assert_eq!(info.bucket_id, bucket_id);
        assert_eq!(info.file_key, file_key);
        assert!(!info.location.is_empty());
        assert!(info.size > 0);
    }

    #[tokio::test]
    async fn test_get_payment_stream() {
        let rate = BigDecimal::from(5);
        let amount_provided = BigDecimal::from(10);

        let service = MockMspServiceBuilder::new()
            .init_repository_with(|client| {
                let rate = rate.clone();
                let amount_provided = amount_provided.clone();

                Box::pin(async move {
                    // Create 2 payment streams for MOCK_ADDRESS, one for MSP and one for BSP
                    client
                        .create_payment_stream(
                            &MOCK_ADDRESS.to_string(),
                            "0x1234567890abcdef1234567890abcdef12345678",
                            BigDecimal::from(500000),
                            PaymentStreamKind::Fixed { rate },
                        )
                        .await
                        .expect("should create fixed payment stream");

                    client
                        .create_payment_stream(
                            &MOCK_ADDRESS.to_string(),
                            "0xabcdef1234567890abcdef1234567890abcdef12",
                            BigDecimal::from(200000),
                            PaymentStreamKind::Dynamic { amount_provided },
                        )
                        .await
                        .expect("should create dynamic payment stream");
                })
            })
            .await
            .build()
            .await;

        let ps = service
            .get_payment_streams(&MOCK_ADDRESS)
            .await
            .expect("get_payment_stream should succeed");

        // Verify we have the expected payment streams
        assert_eq!(ps.streams.len(), 2);

        let fixed = ps
            .streams
            .iter()
            .find(|s| s.provider_type == "msp")
            .expect("a fixed stream");
        assert_eq!(
            BigDecimal::from_str(&fixed.cost_per_tick).expect("cost per tick to be a valid number"),
            rate,
            "Fixed payment stream cost per tick should match what it was crated with"
        );

        let dynamic = ps
            .streams
            .iter()
            .find(|s| s.provider_type == "bsp")
            .expect("a dynamic stream");

        let expected_cost_per_tick = amount_provided
            // mock environment sets price per giga unit to this value
            * BigDecimal::from(MOCK_PRICE_PER_GIGA_UNIT)
            * BigDecimal::from_str("1e-9").unwrap();
        let expected_cost_per_tick = expected_cost_per_tick.with_scale_round(0, RoundingMode::Down);

        assert_eq!(
            BigDecimal::from_str(&dynamic.cost_per_tick)
                .expect("cost per tick to be a valid number"),
            expected_cost_per_tick,
            "Dynamic payment stream cost per tick should be a function of amount provided and price per giga unit"
        )
    }
}
