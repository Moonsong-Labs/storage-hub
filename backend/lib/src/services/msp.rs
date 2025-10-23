//! MSP service implementation with mock data
//!
//! TODO(MOCK): many of methods of the MspService returns mocked data

use std::{collections::HashSet, str::FromStr, sync::Arc};

use axum_extra::extract::multipart::Field;
use bigdecimal::BigDecimal;
use codec::{Decode, Encode};
use sc_network::PeerId;
use serde::{Deserialize, Serialize};
use shc_common::types::{
    ChunkId, FileKeyProof, FileMetadata, StorageProofsMerkleTrieLayout,
    BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE, FILE_CHUNK_SIZE,
};
use shc_file_manager::{in_memory::InMemoryFileDataTrie, traits::FileDataTrie};
use shc_rpc::{
    GetFileFromFileStorageResult, GetValuePropositionsResult, RpcProviderId, SaveFileToDisk,
};
use sp_core::{Blake2Hasher, H256};
use tracing::{debug, warn};

use shc_indexer_db::{models::Bucket as DBBucket, OnchainMspId};
use shp_types::Hash;

use crate::{
    constants::{
        mocks::{PLACEHOLDER_BUCKET_FILE_COUNT, PLACEHOLDER_BUCKET_SIZE_BYTES},
        retry::get_retry_delay,
    },
    data::{
        indexer_db::{client::DBClient, repository::PaymentStreamKind},
        rpc::StorageHubRpcClient,
        storage::BoxedStorage,
    },
    error::Error,
    models::{
        buckets::{Bucket, FileTree},
        files::{DistributeResponse, FileInfo, FileUploadResponse},
        msp_info::{Capacity, InfoResponse, StatsResponse, ValuePropositionWithId},
        payment::{PaymentStreamInfo, PaymentStreamsResponse},
    },
};

#[derive(Debug, Deserialize, Serialize)]
pub struct FileDownloadResult {
    pub file_size: u64,
    pub location: String,
    pub fingerprint: [u8; 32],
}

/// Service for handling MSP-related operations
//TODO: remove dead_code annotations when we actually use these items
// storage: anything that the backend will need to store temporarily
// rpc: anything that the backend needs to request to the underlying MSP node
#[derive(Clone)]
pub struct MspService {
    msp_id: OnchainMspId,

    #[allow(dead_code)]
    storage: Arc<dyn BoxedStorage>,
    postgres: Arc<DBClient>,
    #[allow(dead_code)]
    rpc: Arc<StorageHubRpcClient>,
    msp_callback_url: String,
}

impl MspService {
    /// Create a new MSP service
    ///
    /// This function tries to discover the MSP's provider ID and, if the node is not yet
    /// registered as an MSP, it retries indefinitely with a stepped backoff strategy.
    ///
    /// Note: Keep in mind that if the node is never registered as an MSP, this function
    /// will keep retrying indefinitely and the backend will fail to start. Monitor the
    /// retry attempt count in logs to detect potential configuration issues.
    pub async fn new(
        storage: Arc<dyn BoxedStorage>,
        postgres: Arc<DBClient>,
        rpc: Arc<StorageHubRpcClient>,
        msp_callback_url: String,
    ) -> Result<Self, Error> {
        let mut attempt = 0;

        // Discover the Provider ID of the connected node.
        let msp_id = loop {
            let provider_id: RpcProviderId = rpc.get_provider_id().await.map_err(|e| {
                Error::BadRequest(format!("Failed to get provider ID from RPC: {}", e))
            })?;

            match provider_id {
                RpcProviderId::Msp(id) => break OnchainMspId::new(Hash::from_slice(id.as_ref())),
                RpcProviderId::Bsp(_) => {
                    return Err(Error::BadRequest(
                        "Connected node is a BSP; expected an MSP".to_string(),
                    ));
                }
                RpcProviderId::NotAProvider => {
                    // Calculate the retry delay before the next attempt based on the attempt number
                    let delay_secs = get_retry_delay(attempt);
                    warn!(
                        target: "msp_service::new",
                                                delay_secs = delay_secs,
                                                attempt = attempt + 1,
                        "Connected node is not yet a registered MSP; retrying provider discovery in {delay_secs} seconds... (attempt {attempt})"
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
                    attempt += 1;
                    continue;
                }
            }
        };

        Ok(Self {
            msp_id,
            storage,
            postgres,
            rpc,
            // TODO: dedicated config struct
            // see: https://github.com/Moonsong-Labs/storage-hub/pull/459/files#r2369596519
            msp_callback_url,
        })
    }

    /// Get MSP information
    pub async fn get_info(&self) -> Result<InfoResponse, Error> {
        debug!(target: "msp_service::get_info", "Getting MSP info");

        // Fetch the MSP's local listen multiaddresses via RPC
        let multiaddresses: Vec<String> =
            self.rpc.get_multiaddresses().await.map_err(|e| {
                Error::BadRequest(format!("Failed to get MSP multiaddresses: {}", e))
            })?;

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
    pub async fn get_stats(&self) -> Result<StatsResponse, Error> {
        debug!(target: "msp_service::get_stats", "Getting MSP stats");

        Ok(StatsResponse {
            capacity: Capacity {
                total_bytes: 1099511627776,
                available_bytes: 879609302220,
                used_bytes: 219902325556,
            },
            active_users: 152,
            last_capacity_change: 123,
            value_props_amount: 42,
            buckets_amount: 1024,
        })
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
        user_address: &str,
    ) -> Result<impl Iterator<Item = Bucket>, Error> {
        debug!(target: "msp_service::list_user_buckets", user = %user_address, "Listing user buckets");

        // TODO: request by page
        self.postgres
            .get_user_buckets(&self.msp_id, user_address, None, None)
            .await
            .map(|buckets| {
                buckets.into_iter().map(|entry| {
                    Bucket::from_db(
                        &entry,
                        PLACEHOLDER_BUCKET_SIZE_BYTES,
                        PLACEHOLDER_BUCKET_FILE_COUNT,
                    )
                })
            })
    }

    /// Get a specific bucket by ID
    ///
    /// Verifies ownership of bucket is `user`
    pub async fn get_bucket(&self, bucket_id: &str, user: &str) -> Result<Bucket, Error> {
        debug!(target: "msp_service::get_bucket", bucket_id = %bucket_id, user = %user, "Getting bucket");

        self.get_db_bucket(bucket_id, user).await.map(|bucket| {
            Bucket::from_db(
                &bucket,
                PLACEHOLDER_BUCKET_SIZE_BYTES,
                PLACEHOLDER_BUCKET_FILE_COUNT,
            )
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
        user: &str,
        path: &str,
    ) -> Result<FileTree, Error> {
        debug!(target: "msp_service::get_file_tree", bucket_id = %bucket_id, user = %user, "Getting file tree");

        // first, get the bucket from the db and determine if user can view the bucket
        let bucket = self.get_db_bucket(bucket_id, user).await?;

        // TODO: request by page
        // TODO: optimize query by requesting only matching paths
        let files = self
            .postgres
            .get_bucket_files(bucket.id, None, None)
            .await?;

        // Create hierarchy based on location segments
        Ok(FileTree::from_files_filtered(files, path))
    }

    /// Get file information
    pub async fn get_file_info(
        &self,
        bucket_id: &str,
        user: &str,
        file_key: &str,
    ) -> Result<FileInfo, Error> {
        debug!(target: "msp_service::get_file_info", bucket_id = %bucket_id, user = %user, file_key = %file_key, "Getting file info");

        let file_key_hex = file_key.trim_start_matches("0x");

        let file_key = hex::decode(file_key_hex)
            .map_err(|e| Error::BadRequest(format!("Invalid File Key hex encoding: {}", e)))?;

        if file_key.len() != 32 {
            return Err(Error::BadRequest(format!(
                "Invalid File Key length. Expected 32 bytes, got {}",
                file_key.len()
            )));
        }

        // get bucket determine if user can view it
        let bucket = self.get_bucket(bucket_id, user).await?;

        self.postgres
            .get_file_info(&file_key)
            .await
            .map(|file| FileInfo::from_db(&file, bucket.is_public))
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

    /// Distribute a file to BSPs
    pub async fn distribute_file(
        &self,
        _bucket_id: &str,
        file_key: &str,
    ) -> Result<DistributeResponse, Error> {
        // Mock implementation
        Ok(DistributeResponse {
            status: "distribution_initiated".to_string(),
            file_key: file_key.to_string(),
            message: "File distribution to volunteering BSPs has been initiated".to_string(),
        })
    }

    /// Get all payment streams for a user
    pub async fn get_payment_streams(
        &self,
        user_address: &str,
    ) -> Result<PaymentStreamsResponse, Error> {
        debug!(target: "msp_service::get_payment_streams", user = %user_address, "Getting payment streams");

        // Get all payment streams for the user from the database
        let payment_stream_data = self
            .postgres
            .get_payment_streams_for_user(user_address)
            .await?;

        // Get current price per giga unit per tick from RPC (for dynamic rate calculations)
        let current_price_per_giga_unit_per_tick = self
            .rpc
            .get_current_price_per_giga_unit_per_tick()
            .await
            .map_err(|e| Error::BadRequest(format!("Failed to get price per unit: {}", e)))?;
        let unit_to_giga_unit =
            BigDecimal::from_str("1e-9").expect("Inverse of GIGA to be parsed correctly");

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

                    // Convert u128 price to BigDecimal and multiply
                    let price_bd = BigDecimal::from(current_price_per_giga_unit_per_tick);
                    let cost = amount_provided * &unit_to_giga_unit * price_bd;

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

    /// Calls is_file_in_file_storage rpc to get file metadata if present
    /// Resturns error in any other case
    /// pub enum GetFileFromFileStorageResult {
    //     FileNotFound,
    //     IncompleteFile(IncompleteFileStatus),
    //     FileFound(FileMetadata),
    //     FileFoundWithInconsistency(FileMetadata),
    // }
    pub async fn check_file_status(&self, file_key: &str) -> Result<FileMetadata, Error> {
        let file_status: GetFileFromFileStorageResult = self
            .rpc
            .is_file_in_file_storage(file_key)
            .await
            .map_err(|e| Error::BadRequest(e.to_string()))?;

        match file_status {
            GetFileFromFileStorageResult::FileNotFound => {
                Err(Error::BadRequest("File not found".to_string()))
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

    /// Download a file by `file_key` via the MSP RPC into `/tmp/uploads/<file_key>` and
    /// return its size, UTF-8 location, fingerprint, and temp path.
    /// Returns BadRequest on RPC/parse errors.
    ///
    /// We provide an URL as saveFileToDisk RPC requires it to stream the file.
    /// We also implemented the internal_upload_by_key handler to handle this temporary file upload.
    pub async fn get_file_from_key(&self, file_key: &str) -> Result<FileDownloadResult, Error> {
        debug!(target: "msp_service::get_file_from_key", file_key = %file_key, "Downloading file by key");
        // TODO: authenticate user
        let upload_url = format!("{}/internal/uploads/{}", self.msp_callback_url, file_key);

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
            SaveFileToDisk::Success(file_metadata) => {
                // Convert location bytes to string
                let location = String::from_utf8_lossy(file_metadata.location()).to_string();
                let fingerprint: [u8; 32] = file_metadata.fingerprint().as_hash();
                let file_size = file_metadata.file_size();

                debug!(
                    "File download prepared - file_key: {}, size: {} bytes",
                    file_key, file_size
                );

                Ok(FileDownloadResult {
                    file_size,
                    location,
                    fingerprint,
                })
            }
        }
    }

    /// Process a streamed file upload: validate metadata, chunk into trie, batch proofs, and send to MSP.
    pub async fn process_and_upload_file(
        &self,
        bucket_id: &str,
        file_key: &str,
        mut file_data_stream: Field,
        file_metadata: FileMetadata,
    ) -> Result<FileUploadResponse, Error> {
        debug!(
            target: "msp_service::process_and_upload_file",
            bucket_id = %bucket_id,
            file_key = %file_key,
            file_size = file_metadata.file_size(),
            "Starting file upload"
        );

        // Validate bucket id and file key against metadata
        let expected_bucket_id = hex::encode(file_metadata.bucket_id());
        if bucket_id.trim_start_matches("0x") != expected_bucket_id {
            return Err(Error::BadRequest(
                format!("Bucket ID in URL does not match file metadata: {expected_bucket_id} != {bucket_id}"),
            ));
        }

        let expected_file_key = hex::encode(file_metadata.file_key::<Blake2Hasher>());
        if file_key.trim_start_matches("0x") != expected_file_key {
            return Err(Error::BadRequest(format!(
                "File key in URL does not match file metadata: {expected_file_key} != {file_key}"
            )));
        }

        // Initialize the trie that will hold the chunked file data.
        let mut trie = InMemoryFileDataTrie::<StorageProofsMerkleTrieLayout>::new();

        // Prepare the overflow buffer that will hold any data that doesn't exactly fit in a chunk.
        let mut overflow_buffer = Vec::new();

        // Initialize the chunk index.
        let mut chunk_index = 0;

        // Start streaming the file data into the trie, chunking it into FILE_CHUNK_SIZE chunks in the process.
        while let Some(bytes_read) = file_data_stream.chunk().await.map_err(|e| {
            Error::BadRequest(format!("Failed to read multipart stream chunk: {}", e))
        })? {
            // Load the bytes read from the file into the overflow buffer.
            overflow_buffer.extend_from_slice(&bytes_read);

            // While the overflow buffer is larger than FILE_CHUNK_SIZE, process a chunk.
            while overflow_buffer.len() >= FILE_CHUNK_SIZE as usize {
                let chunk = overflow_buffer[..FILE_CHUNK_SIZE as usize].to_vec();

                // Insert the chunk into the trie.
                trie.write_chunk(&ChunkId::new(chunk_index as u64), &chunk)
                    .map_err(|e| {
                        Error::BadRequest(format!(
                            "Failed to write chunk {} to trie: {}",
                            chunk_index, e
                        ))
                    })?;

                // Increment the chunk index.
                chunk_index += 1;

                // Remove the chunk from the overflow buffer.
                overflow_buffer.drain(..FILE_CHUNK_SIZE as usize);
            }
        }

        // Check the overflow buffer to see if the file didn't fit exactly in an integer number of chunks.
        if !overflow_buffer.is_empty() {
            // Insert the chunk into the trie.
            trie.write_chunk(&ChunkId::new(chunk_index as u64), &overflow_buffer)
                .map_err(|e| {
                    Error::BadRequest(format!(
                        "Failed to write final chunk {} to trie: {}",
                        chunk_index, e
                    ))
                })?;

            // Increment the chunk index to get the total amount of chunks.
            chunk_index += 1;
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

        // At this point, the trie contains the entire file data and we can start generating the proofs for the chunk batches
        // and sending them to the MSP.

        // Get how many chunks fit in a batch of BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE, rounding down.
        const CHUNKS_PER_BATCH: u64 = BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE as u64 / FILE_CHUNK_SIZE;

        // Initialize the index of the initial chunk to process in this batch.
        let mut batch_start_chunk_index = 0;
        let total_batches = (total_chunks + CHUNKS_PER_BATCH - 1) / CHUNKS_PER_BATCH;
        let mut batch_number = 1;

        // Start processing batches, until all chunks have been processed.
        while batch_start_chunk_index < total_chunks {
            // Get the chunks to send in this batch, capping at the total amount of chunks of the file.
            let chunks = (batch_start_chunk_index
                ..(batch_start_chunk_index + CHUNKS_PER_BATCH).min(total_chunks))
                .map(|chunk_index| ChunkId::new(chunk_index as u64))
                .collect::<HashSet<_>>();
            let chunks_in_batch = chunks.len() as u64;

            debug!(
                target: "msp_service::process_and_upload_file",
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
            self.upload_to_msp(&chunks, &file_key_proof)
                .await
                .map_err(|e| {
                    Error::BadRequest(format!(
                        "Failed to upload batch {} to MSP: {}",
                        batch_number, e
                    ))
                })?;

            debug!(
                target: "msp_service::process_and_upload_file",
                batch_number = batch_number,
                total_batches = total_batches,
                "Batch uploaded successfully"
            );

            // Update the initial chunk index for the next batch.
            batch_start_chunk_index += chunks_in_batch;
            batch_number += 1;
        }

        // If the complete file was uploaded to the MSP successfully, we can return the response.
        let bytes_location = file_metadata.location().clone();
        let location = str::from_utf8(&bytes_location)
            .unwrap_or(file_key)
            .to_string();

        debug!(
            bucket_id = %bucket_id,
            file_key = %file_key,
            chunks = total_chunks,
            "File upload completed"
        );

        Ok(FileUploadResponse {
            status: "upload_successful".to_string(),
            file_key: file_key.to_string(),
            bucket_id: bucket_id.to_string(),
            fingerprint: format!("0x{}", hex::encode(trie.get_root())),
            location,
        })
    }

    /// Upload a batch of file chunks with their FileKeyProof to the MSP via its RPC.
    ///
    /// This implementation:
    /// 1. Gets the MSP info to get its multiaddresses.
    /// 2. Extracts the peer IDs from the multiaddresses.
    /// 3. Sends the FileKeyProof with the batch of chunks to the MSP through the `receiveBackendFileChunks` RPC method.
    ///
    /// Note: obtaining the peer ID previous to sending the request is needed as this is the peer ID that the MSP
    /// will send the file to. If it's different than its local one, it will probably fail.
    pub async fn upload_to_msp(
        &self,
        chunk_ids: &HashSet<ChunkId>,
        file_key_proof: &FileKeyProof,
    ) -> Result<(), Error> {
        debug!(
            target: "msp_service::upload_to_msp",
            chunk_count = chunk_ids.len(),
            "Uploading chunks to MSP"
        );

        // Ensure we are not incorrectly trying to upload an empty file.
        if chunk_ids.is_empty() {
            return Err(Error::BadRequest(
                "Cannot upload file with no chunks".to_string(),
            ));
        }

        // Get the MSP's info including its multiaddresses.
        let msp_info = self.get_info().await?;

        // Extract the peer IDs from the multiaddresses.
        let peer_ids = self.extract_peer_ids_from_multiaddresses(&msp_info.multiaddresses)?;

        // Try to send the chunks batch to each peer until one succeeds.
        debug!(target: "msp_service::upload_to_msp", "Trying to send the chunks batch to each peer until one succeeds");
        let mut last_err = None;
        for peer_id in peer_ids {
            match self
                .send_upload_request_to_msp_peer(peer_id, file_key_proof.clone())
                .await
            {
                Ok(()) => {
                    debug!(
                        target: "msp_service::upload_to_msp",
                        chunk_count = chunk_ids.len(),
                        msp_id = %msp_info.msp_id,
                        file_key = %format!("0x{}", hex::encode(file_key_proof.file_metadata.file_key::<Blake2Hasher>())),
                        bucket_id = %format!("0x{}", hex::encode(file_key_proof.file_metadata.bucket_id())),
                        "Successfully uploaded chunks to MSP"
                    );
                    return Ok(());
                }
                Err(e) => {
                    warn!(target: "msp_service::upload_to_msp", peer_id = ?peer_id, error = ?e, "Failed to send chunks to peer");
                    last_err = Some(e);
                    continue;
                }
            }
        }

        Err(last_err.expect("At least one peer_id was tried, so last_err must be Some"))
    }

    /// Extract peer IDs from multiaddresses
    fn extract_peer_ids_from_multiaddresses(
        &self,
        multiaddresses: &[String],
    ) -> Result<Vec<PeerId>, Error> {
        debug!(target: "msp_service::extract_peer_ids_from_multiaddresses", "Extracting peer IDs from MSP's multiaddresses");
        let mut peer_ids = HashSet::new();

        for multiaddr_str in multiaddresses {
            // Parse multiaddress string to extract peer ID
            // Format example: "/ip4/192.168.0.10/tcp/30333/p2p/12D3KooWJAgnKUrQkGsKxRxojxcFRhtH6ovWfJTPJjAkhmAz2yC8"
            if let Some(p2p_part) = multiaddr_str.split("/p2p/").nth(1) {
                // Extract the peer ID part (everything after /p2p/)
                let peer_id_str = p2p_part.split('/').next().unwrap_or(p2p_part);

                match peer_id_str.parse::<PeerId>() {
                    Ok(peer_id) => {
                        debug!(
                            target: "msp_service::extract_peer_ids_from_multiaddresses",
                            peer_id = ?peer_id,
                            multiaddress = %multiaddr_str,
                            "Extracted peer ID from multiaddress"
                        );
                        peer_ids.insert(peer_id);
                    }
                    Err(e) => {
                        warn!(
                            target: "msp_service::extract_peer_ids_from_multiaddresses",
                            multiaddress = %multiaddr_str,
                            error = ?e,
                            "Failed to parse peer ID from multiaddress"
                        );
                    }
                }
            } else {
                warn!(target: "msp_service::extract_peer_ids_from_multiaddresses", multiaddress = %multiaddr_str, "No /p2p/ section found in multiaddress");
            }
        }

        if peer_ids.is_empty() {
            return Err(Error::BadRequest(
                "No valid peer IDs found in multiaddresses".to_string(),
            ));
        }

        Ok(peer_ids.into_iter().collect())
    }

    /// Send an upload request to a specific peer ID of the MSP with retry logic.
    /// TODO: Make the number of retries configurable.
    async fn send_upload_request_to_msp_peer(
        &self,
        peer_id: PeerId,
        file_key_proof: FileKeyProof,
    ) -> Result<(), Error> {
        debug!(
            target: "msp_service::send_upload_request_to_msp_peer",
            peer_id = ?peer_id,
            "Attempting to send upload request to MSP peer"
        );

        // Get fhe file metadata from the received FileKeyProof.
        let file_metadata = file_key_proof.clone().file_metadata;

        // Get the file key from the file metadata.
        let file_key: H256 = file_metadata.file_key::<Blake2Hasher>();
        let file_key_hexstr = format!("{file_key:x}");

        // Encode the FileKeyProof as SCALE for transport
        let encoded_proof = file_key_proof.encode();

        // TODO: We should make these configurable.
        let mut retry_attempts = 0;
        let max_retries = 3;
        let delay_between_retries_secs = 1;

        while retry_attempts < max_retries {
            debug!(target: "msp_service::send_upload_request_to_msp_peer", peer_id = ?peer_id, retry_attempt = retry_attempts, "Sending file chunks to MSP peer via RPC");
            let result: Result<Vec<u8>, _> = self
                .rpc
                .receive_file_chunks(&file_key_hexstr, encoded_proof.clone())
                .await;

            match result {
                Ok(_raw) => {
                    debug!(peer_id = ?peer_id, "Successfully sent upload request to MSP peer");
                    return Ok(());
                }
                Err(e) => {
                    retry_attempts += 1;
                    if retry_attempts < max_retries {
                        warn!(
                            target: "msp_service::send_upload_request_to_msp_peer",
                            peer_id = ?peer_id,
                            retry_attempt = retry_attempts,
                            error = ?e,
                            "Upload request to MSP peer {peer_id} failed via RPC, retrying... (attempt {retry_attempts})",
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
}

impl MspService {
    /// Verifies user can access the given bucket
    fn can_user_view_bucket(&self, bucket: DBBucket, user: &str) -> Result<DBBucket, Error> {
        // TODO: NFT ownership
        if bucket.private {
            if bucket.account.as_str() == user {
                Ok(bucket)
            } else {
                Err(Error::Unauthorized(format!(
                    "Specified user is not authorized to view this bucket"
                )))
            }
        } else {
            Ok(bucket)
        }
    }

    /// Retrieve a bucket from the DB and verify read permission
    async fn get_db_bucket(
        &self,
        bucket_id: &str,
        user: &str,
    ) -> Result<shc_indexer_db::models::Bucket, Error> {
        let bucket_id_hex = bucket_id.trim_start_matches("0x");

        let bucket_id = hex::decode(bucket_id_hex)
            .map_err(|e| Error::BadRequest(format!("Invalid Bucket ID hex encoding: {}", e)))?;

        if bucket_id.len() != 32 {
            return Err(Error::BadRequest(format!(
                "Invalid Bucket ID length. Expected 32 bytes, got {}",
                bucket_id.len()
            )));
        }

        self.postgres
            .get_bucket(&bucket_id)
            .await
            .and_then(|bucket| self.can_user_view_bucket(bucket, user))
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use std::sync::Arc;

    use bigdecimal::BigDecimal;
    use serde_json::Value;

    use shc_common::types::{FileKeyProof, FileMetadata};
    use shp_types::Hash;

    use super::*;
    use crate::{
        config::Config,
        constants::{
            mocks::{MOCK_ADDRESS, MOCK_PRICE_PER_GIGA_UNIT},
            rpc::DUMMY_MSP_ID,
            test::{bucket::DEFAULT_BUCKET_NAME, file::DEFAULT_SIZE},
        },
        data::{
            indexer_db::{
                client::DBClient, mock_repository::MockRepository, repository::PaymentStreamKind,
            },
            rpc::{AnyRpcConnection, MockConnection, StorageHubRpcClient},
            storage::{BoxedStorageWrapper, InMemoryStorage},
        },
        test_utils::random_bytes_32,
    };

    /// Builder for creating MspService instances with mock dependencies for testing
    struct MockMspServiceBuilder {
        storage: Arc<BoxedStorageWrapper<InMemoryStorage>>,
        postgres: Arc<DBClient>,
        rpc: Arc<StorageHubRpcClient>,
    }

    impl MockMspServiceBuilder {
        /// Create a new builder with default empty mocks
        pub fn new() -> Self {
            Self {
                storage: Arc::new(BoxedStorageWrapper::new(InMemoryStorage::new())),
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

            MspService::new(
                self.storage,
                self.postgres,
                self.rpc,
                cfg.storage_hub.msp_callback_url,
            )
            .await
            .expect("Mocked MSP service builder should succeed")
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
        let service = MockMspServiceBuilder::new().build().await;
        let stats = service.get_stats().await.unwrap();

        assert!(stats.capacity.total_bytes > 0);
        assert!(stats.capacity.available_bytes <= stats.capacity.total_bytes);
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
                            MOCK_ADDRESS,
                            OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
                        )
                        .await
                        .expect("should create MSP");

                    // Create a test bucket for the mock user
                    client
                        .create_bucket(
                            MOCK_ADDRESS,
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
            .list_user_buckets(MOCK_ADDRESS)
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
                            MOCK_ADDRESS,
                            OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
                        )
                        .await
                        .expect("should create MSP");

                    // Create a test bucket for the mock user
                    let bucket = client
                        .create_bucket(
                            MOCK_ADDRESS,
                            Some(msp.id),
                            bucket_name.as_bytes(),
                            &bucket_id,
                            false,
                        )
                        .await
                        .expect("should create bucket");

                    client
                        .create_file(
                            MOCK_ADDRESS.as_bytes(),
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
        let bucket = service.get_bucket(&bucket_id, MOCK_ADDRESS).await.unwrap();

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
                            MOCK_ADDRESS,
                            OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
                        )
                        .await
                        .expect("should create MSP");

                    // Create a test bucket for the mock user
                    let bucket = client
                        .create_bucket(
                            MOCK_ADDRESS,
                            Some(msp.id),
                            DEFAULT_BUCKET_NAME.as_bytes(),
                            &bucket_id,
                            false,
                        )
                        .await
                        .expect("should create bucket");

                    client
                        .create_file(
                            MOCK_ADDRESS.as_bytes(),
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

        let tree = service
            .get_file_tree(hex::encode(bucket_id).as_ref(), MOCK_ADDRESS, "/")
            .await
            .unwrap();

        tree.entry.folder().expect("first entry to be a folder");
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
                            MOCK_ADDRESS,
                            OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
                        )
                        .await
                        .expect("should create MSP");

                    // Create a test bucket for the mock user
                    let bucket = client
                        .create_bucket(
                            MOCK_ADDRESS,
                            Some(msp.id),
                            DEFAULT_BUCKET_NAME.as_bytes(),
                            &bucket_id,
                            false,
                        )
                        .await
                        .expect("should create bucket");

                    client
                        .create_file(
                            MOCK_ADDRESS.as_bytes(),
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
            .get_file_info(&bucket_id, MOCK_ADDRESS, &file_key)
            .await
            .expect("get_file_info should succeed");

        assert_eq!(info.bucket_id, bucket_id);
        assert_eq!(info.file_key, file_key);
        assert!(!info.location.is_empty());
        assert!(info.size > 0);
    }

    #[tokio::test]
    async fn test_distribute_file() {
        let service = MockMspServiceBuilder::new().build().await;
        let file_key = "abc123";
        let resp = service
            .distribute_file("bucket123", file_key)
            .await
            .expect("distribute_file should succeed");

        assert_eq!(resp.status, "distribution_initiated");
        assert_eq!(resp.file_key, file_key);
        assert!(!resp.message.is_empty());
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
                            MOCK_ADDRESS,
                            "0x1234567890abcdef1234567890abcdef12345678",
                            BigDecimal::from(500000),
                            PaymentStreamKind::Fixed { rate },
                        )
                        .await
                        .expect("should create fixed payment stream");

                    client
                        .create_payment_stream(
                            MOCK_ADDRESS,
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
            .get_payment_streams(MOCK_ADDRESS)
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

        assert_eq!(
            BigDecimal::from_str(&dynamic.cost_per_tick)
                .expect("cost per tick to be a valid number"),
            expected_cost_per_tick,
            "Dynamic payment stream cost per tick should be a function of amount provided and price per giga unit"
        )
    }

    #[tokio::test]
    async fn test_upload_to_msp() {
        let service = MockMspServiceBuilder::new().build().await;

        // Provide at least one chunk id (upload_to_msp rejects empty sets)
        let mut chunk_ids = HashSet::new();
        chunk_ids.insert(ChunkId::new(0));

        // Create test file metadata
        let file_metadata = FileMetadata::new(
            vec![0u8; 32],
            vec![0u8; 32],
            b"test_location".to_vec(),
            1000,
            [0u8; 32].into(),
        )
        .unwrap();

        // Create test FileKeyProof
        let file_key_proof = FileKeyProof::new(
            file_metadata.owner().clone(),
            file_metadata.bucket_id().clone(),
            file_metadata.location().clone(),
            file_metadata.file_size(),
            *file_metadata.fingerprint(),
            sp_trie::CompactProof {
                encoded_nodes: vec![],
            },
        )
        .unwrap();

        service
            .upload_to_msp(&chunk_ids, &file_key_proof)
            .await
            .expect("able to upload file");
    }
}
