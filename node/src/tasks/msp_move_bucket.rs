use anyhow::anyhow;
use codec::Decode;
use rand::seq::SliceRandom;
use sc_tracing::tracing::*;
use sp_core::H256;
use std::{cmp::max, time::Duration};

use pallet_file_system::types::BucketMoveRequestResponse;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceInterface,
    events::MoveBucketRequestedForMsp,
    types::{RetryStrategy, SendExtrinsicOptions},
};
use shc_common::types::{
    BucketId, FileKeyProof, HashT, ProviderId, StorageProofsMerkleTrieLayout, StorageProviderId,
};
use shc_file_manager::traits::FileStorage;
use shc_file_transfer_service::commands::FileTransferServiceInterface;
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use shp_file_metadata::ChunkId;
use storage_hub_runtime::StorageDataUnit;

use crate::services::{
    handler::StorageHubHandler,
    types::{MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "msp-move-bucket-task";
const DOWNLOAD_REQUEST_RETRY_COUNT: usize = 30;

/// [`MspMoveBucketTask`] handles bucket move requests between MSPs.
///
/// # Event Handling
/// This task handles the [`MoveBucketRequestedForNewMsp`] event which is emitted when a user
/// requests to move their bucket from one MSP to this MSP.
///
/// # Lifecycle
/// 1. When a move bucket request is received:
///    - Verifies that indexer is enabled and accessible
///    - Checks if there is sufficient storage capacity, increasing it if needed
///    - Validates that all files in the bucket can be handled
///    - Inserts file metadata into local storage and forest storage
///    - Verifies BSP peer IDs are available for each file
///
/// 2. If all validations pass:
///    - Accepts the move request by sending [`BucketMoveRequestResponse::Accepted`]
///    - Downloads all files from BSPs
///    - Updates local forest root to match on-chain state
///
/// 3. If any validation fails:
///    - Rejects the move request by sending [`BucketMoveRequestResponse::Rejected`]
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
pub struct MspMoveBucketTask<NT>
where
    NT: ShNodeType,
    NT::FSH: MspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<NT>,
    file_storage_inserted_file_keys: Vec<H256>,
    pending_bucket_id: Option<BucketId>,
}

impl<NT> EventHandler<MoveBucketRequestedForMsp> for MspMoveBucketTask<NT>
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

impl<NT> MspMoveBucketTask<NT>
where
    NT: ShNodeType,
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

        // Now download all the files
        for file in files {
            if let Err(error) = self.download_file(&file, &event.bucket_id).await {
                error!(
                    target: LOG_TARGET,
                    "Failed to download file: {:?}",
                    error
                );
                // Continue with other files even if one fails
                continue;
            }
        }

        Ok(())
    }
}

impl<NT> Clone for MspMoveBucketTask<NT>
where
    NT: ShNodeType,
    NT::FSH: MspForestStorageHandlerT,
{
    fn clone(&self) -> MspMoveBucketTask<NT> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            file_storage_inserted_file_keys: self.file_storage_inserted_file_keys.clone(),
            pending_bucket_id: self.pending_bucket_id.clone(),
        }
    }
}

impl<NT> MspMoveBucketTask<NT>
where
    NT: ShNodeType,
    NT::FSH: MspForestStorageHandlerT,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT>) -> Self {
        Self {
            storage_hub_handler,
            file_storage_inserted_file_keys: Vec::new(),
            pending_bucket_id: None,
        }
    }

    /// Rejects a bucket move request and performs cleanup of any partially created resources.
    ///
    /// # Arguments
    /// * `bucket_id` - The ID of the bucket whose move request is being rejected
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

    /// Downloads a file from BSPs (Backup Storage Providers) chunk by chunk.
    ///
    /// # Flow
    /// 1. Constructs file metadata and key from the provided file and bucket information
    /// 2. Retrieves and shuffles BSP peer IDs to distribute load across providers
    /// 3. For each chunk in the file:
    ///    - Cycles through BSP peers attempting to download the chunk
    ///    - Retries failed downloads up to DOWNLOAD_REQUEST_RETRY_COUNT times
    ///    - Verifies proof and chunk data integrity before storage
    ///
    /// # Verification Steps
    /// - Decodes and validates the file key proof from the download response
    /// - Ensures exactly one proven chunk is received
    /// - Verifies chunk ID matches the expected chunk being downloaded
    /// - Validates chunk data before writing to storage
    ///
    /// # Error Handling
    /// - Logs errors for failed download attempts
    /// - Continues to next BSP peer on failure
    /// - Tracks download success/failure per chunk
    /// - Continues to next chunk even if current chunk fails all retry attempts
    ///
    /// # Arguments
    /// * `file` - The file model containing metadata and BSP information
    /// * `bucket` - The bucket ID where the file belongs
    ///
    /// # Returns
    /// Returns `Ok(())` if the download process completes (even with some failed chunks)
    /// Returns `Err` for critical failures that prevent the download process from continuing
    ///
    /// # Note
    /// The function implements a round-robin approach to BSP selection with random initial
    /// distribution.
    async fn download_file(
        &self,
        file: &shc_indexer_db::models::File,
        bucket: &BucketId,
    ) -> anyhow::Result<()> {
        let file_metadata = file.to_file_metadata(bucket.as_ref().to_vec());
        let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();

        info!(
            target: LOG_TARGET,
            "MSP: downloading file {:?}",
            file_key,
        );

        let mut bsp_peer_ids = file
            .get_bsp_peer_ids(
                &mut self
                    .storage_hub_handler
                    .indexer_db_pool
                    .as_ref()
                    .unwrap()
                    .get()
                    .await?,
            )
            .await?;

        // Shuffle BSP peer IDs to distribute load randomly across BSPs and avoid
        // overwhelming a single BSP with sequential requests
        bsp_peer_ids.shuffle(&mut rand::thread_rng());

        // TODO: Consider optimizing BSP selection strategy - current approach uses round-robin,
        // but we could potentially stick with a responsive BSP until it fails
        let mut bsp_peer_ids_iter = bsp_peer_ids.iter().cycle();

        let chunks_count = file_metadata.chunks_count();
        for chunk in 0..chunks_count {
            let mut downloaded = false;
            for _ in 0..DOWNLOAD_REQUEST_RETRY_COUNT {
                let peer_id = bsp_peer_ids_iter
                    .next()
                    .expect("Iterator will never be empty due to .cycle()");

                let download_request = match self
                    .storage_hub_handler
                    .file_transfer
                    .download_request(
                        *peer_id,
                        file_key.into(),
                        ChunkId::new(chunk),
                        Some(*bucket),
                    )
                    .await
                {
                    Ok(request) => request,
                    Err(error) => {
                        error!(
                            target: LOG_TARGET,
                            "Failed to download chunk {:?} from peer {:?}: {:?}",
                            chunk, peer_id, error
                        );
                        continue;
                    }
                };

                let file_key_proof =
                    match FileKeyProof::decode(&mut download_request.file_key_proof.as_ref()) {
                        Ok(proof) => proof,
                        Err(error) => {
                            error!(
                                target: LOG_TARGET,
                                "Failed to decode file key proof: {:?}",
                                error
                            );
                            continue;
                        }
                    };

                let proven = match file_key_proof.proven::<StorageProofsMerkleTrieLayout>() {
                    Ok(data) => data,
                    Err(error) => {
                        error!(
                            target: LOG_TARGET,
                            "Failed to get proven data: {:?}",
                            error
                        );
                        continue;
                    }
                };

                if proven.len() != 1 {
                    error!(
                        target: LOG_TARGET,
                        "Expected exactly one proven chunk but got {}",
                        proven.len()
                    );
                    continue;
                }

                let chunk_data = proven[0].data.clone();
                let chunk_id = ChunkId::new(chunk);

                if chunk_id != proven[0].key {
                    error!(
                        target: LOG_TARGET,
                        "Expected chunk id {:?} but got {:?}",
                        chunk, proven[0].key
                    );
                    continue;
                }

                if let Err(error) = self
                    .storage_hub_handler
                    .file_storage
                    .write()
                    .await
                    .write_chunk(&file_key, &chunk_id, &chunk_data)
                {
                    error!(
                        target: LOG_TARGET,
                        "Failed to write chunk: {:?}",
                        error
                    );
                } else {
                    downloaded = true;
                    break;
                }
            }

            if !downloaded {
                error!(
                    target: LOG_TARGET,
                    "Failed to download chunk {:?} after {} retries",
                    chunk,
                    DOWNLOAD_REQUEST_RETRY_COUNT
                );
                // TODO: Improve handling of failed chunk downloads:
                // - Consider implementing a retry mechanism with backoff
                // - Track failed chunks for later retry attempts
                // - Potentially notify the user about partial downloads
                // - Add metrics/monitoring for failed downloads
            }
        }

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
}
