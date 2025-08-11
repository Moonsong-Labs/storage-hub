use anyhow::anyhow;
use rand::{rngs::StdRng, SeedableRng};
use std::{sync::Mutex, time::Duration};

use sc_tracing::tracing::*;
use sp_core::H256;

use pallet_file_system::types::BucketMoveRequestResponse;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    capacity_manager::CapacityRequestData,
    commands::{BlockchainServiceCommandInterface, BlockchainServiceCommandInterfaceExt},
    events::{MoveBucketRequestedForMsp, StartMovedBucketDownload},
    types::{RetryStrategy, SendExtrinsicOptions},
};
use shc_common::traits::StorageEnableRuntime;
use shc_common::types::{
    BucketId, HashT, ProviderId, StorageProofsMerkleTrieLayout, StorageProviderId,
};
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};

use crate::{
    handler::StorageHubHandler,
    types::{MspForestStorageHandlerT, ShNodeType},
};

// Constants
const LOG_TARGET: &str = "storage-hub::msp-move-bucket";
lazy_static::lazy_static! {
    // A global RNG available for peer selection
    static ref GLOBAL_RNG: Mutex<StdRng> = Mutex::new(StdRng::from_entropy());
}

/// Configuration for the MspMoveBucketTask
#[derive(Debug, Clone)]
pub struct MspMoveBucketConfig {
    /// Maximum number of times to retry a move bucket request
    pub max_try_count: u32,
    /// Maximum tip amount to use when submitting a move bucket request extrinsic
    pub max_tip: f64,
}

impl Default for MspMoveBucketConfig {
    fn default() -> Self {
        Self {
            max_try_count: 5,
            max_tip: 500.0,
        }
    }
}

/// Handles requests for MSP (Main Storage Provider) to respond to bucket move requests.
/// Downloads bucket files from BSPs (Backup Storage Providers).
pub struct MspRespondMoveBucketTask<NT, Runtime>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
    pending_bucket_id: Option<BucketId<Runtime>>,
    file_storage_inserted_file_keys: Vec<H256>,
    /// Configuration for this task
    config: MspMoveBucketConfig,
}

impl<NT, Runtime> Clone for MspRespondMoveBucketTask<NT, Runtime>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> MspRespondMoveBucketTask<NT, Runtime> {
        MspRespondMoveBucketTask {
            storage_hub_handler: self.storage_hub_handler.clone(),
            pending_bucket_id: self.pending_bucket_id,
            file_storage_inserted_file_keys: self.file_storage_inserted_file_keys.clone(),
            config: self.config.clone(),
        }
    }
}

impl<NT, Runtime> MspRespondMoveBucketTask<NT, Runtime>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler: storage_hub_handler.clone(),
            pending_bucket_id: None,
            file_storage_inserted_file_keys: Vec::new(),
            config: storage_hub_handler.provider_config.msp_move_bucket.clone(),
        }
    }
}

impl<NT, Runtime> EventHandler<MoveBucketRequestedForMsp<Runtime>>
    for MspRespondMoveBucketTask<NT, Runtime>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: MoveBucketRequestedForMsp<Runtime>,
    ) -> anyhow::Result<()> {
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

impl<NT, Runtime> EventHandler<StartMovedBucketDownload<Runtime>>
    for MspRespondMoveBucketTask<NT, Runtime>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: StartMovedBucketDownload<Runtime>,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "StartMovedBucketDownload: Starting download process for bucket {:?}",
            event.bucket_id
        );

        // Important: Add a delay after receiving the on-chain confirmation
        // This gives the BSPs time to process the chain event and prepare to serve files
        info!(
            target: LOG_TARGET,
            "Waiting for BSPs to be ready to serve files for bucket {:?}", event.bucket_id
        );

        // Get all files for this bucket from the indexer
        let indexer_db_pool =
            if let Some(indexer_db_pool) = self.storage_hub_handler.indexer_db_pool.clone() {
                indexer_db_pool
            } else {
                return Err(anyhow!(
                    "Indexer is disabled but a StartMovedBucketDownload event was received"
                ));
            };

        let mut indexer_connection = indexer_db_pool.get().await?;

        let files = shc_indexer_db::models::File::get_by_onchain_bucket_id(
            &mut indexer_connection,
            event.bucket_id.as_ref().to_vec(),
        )
        .await?;

        if files.is_empty() {
            info!(
                target: LOG_TARGET,
                "No files to download for bucket {:?}", event.bucket_id
            );
            self.pending_bucket_id = None;
            return Ok(());
        }

        // Convert indexer files to FileMetadata
        let file_metadatas = files
            .iter()
            .filter_map(
                |file| match file.to_file_metadata(event.bucket_id.as_ref().to_vec()) {
                    Ok(metadata) => Some(metadata),
                    Err(e) => {
                        error!(
                            target: LOG_TARGET,
                            "Failed to convert file to metadata: {:?}", e
                        );
                        None
                    }
                },
            )
            .collect::<Vec<_>>();

        // Now download all files using the FileDownloadManager
        let file_download_manager = &self.storage_hub_handler.file_download_manager;
        let file_transfer_service = self.storage_hub_handler.file_transfer.clone();

        info!(
            target: LOG_TARGET,
            "Starting new download of bucket {:?}", event.bucket_id
        );

        // Use try_lock_and_download_bucket which handles locking internally
        let download_result = file_download_manager
            .try_lock_and_download_bucket(
                event.bucket_id,
                file_metadatas,
                file_transfer_service,
                self.storage_hub_handler.file_storage.clone(),
            )
            .await;

        match download_result {
            Ok(()) => {
                info!(
                    target: LOG_TARGET,
                    "Successfully downloaded bucket {:?}", event.bucket_id
                );
            }
            Err(crate::file_download_manager::BucketDownloadError::AlreadyBeingDownloaded(_)) => {
                info!(
                    target: LOG_TARGET,
                    "Bucket {:?} is already being downloaded by another task", event.bucket_id
                );
            }
            Err(crate::file_download_manager::BucketDownloadError::DownloadFailed(e)) => {
                error!(
                    target: LOG_TARGET,
                    "Failed to download bucket {:?}: {:?}", event.bucket_id, e
                );
            }
        }

        // After download is complete, update status
        self.pending_bucket_id = None;

        info!(
            target: LOG_TARGET,
            "Bucket move completed for bucket {:?}", event.bucket_id
        );

        Ok(())
    }
}

impl<NT, Runtime> MspRespondMoveBucketTask<NT, Runtime>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    /// Internal implementation of the move bucket request handling.
    /// This function contains the core logic for processing a bucket move request.
    /// If it returns an error, the caller (handle_event) will reject the bucket move request.
    async fn handle_move_bucket_request(
        &mut self,
        event: MoveBucketRequestedForMsp<Runtime>,
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

        // First, retrieve all the files for this bucket from the indexer
        let files = shc_indexer_db::models::File::get_by_onchain_bucket_id(
            &mut indexer_connection,
            event.bucket_id.as_ref().to_vec(),
        )
        .await?;

        if files.is_empty() {
            warn!(
                target: LOG_TARGET,
                "No files found for bucket {:?}", event.bucket_id
            );
            // We still accept since there's nothing to download
            self.accept_bucket_move(event.bucket_id).await?;
            return Ok(());
        }

        let bucket = event.bucket_id.as_ref().to_vec();

        let forest_storage = self
            .storage_hub_handler
            .forest_storage_handler
            .get_or_create(&bucket)
            .await;

        // Calculate total size to check capacity
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

        // Convert to the expected ProviderId type
        let own_msp_id = match own_provider_id {
            Some(StorageProviderId::MainStorageProvider(id)) => id,
            Some(StorageProviderId::BackupStorageProvider(_)) => {
                return Err(anyhow!("Current node is a BSP. Expected an MSP ID."));
            }
            None => {
                return Err(anyhow!("Failed to get own provider ID."));
            }
        };

        // Validate capacity - might trigger capacity increase
        self.check_and_increase_capacity(total_size, own_msp_id)
            .await?;

        // Register BSP peers and prepare file metadata
        let mut file_metadatas = Vec::with_capacity(files.len());

        for file in &files {
            let file_metadata = file
                .to_file_metadata(event.bucket_id.as_ref().to_vec())
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

            // Register the BSP peers with the peer manager for this file
            let bsp_peer_ids = file.get_bsp_peer_ids(&mut indexer_connection).await?;
            if bsp_peer_ids.is_empty() {
                return Err(anyhow!("No BSP peer IDs found for file {:?}", file_key));
            }

            for peer_id in &bsp_peer_ids {
                self.storage_hub_handler
                    .peer_manager
                    .add_peer(*peer_id, file_key)
                    .await;
            }

            // Add the file metadata to our list
            file_metadatas.push(file_metadata);
        }

        // Store bucket ID for tracking purposes
        self.pending_bucket_id = Some(event.bucket_id);

        // All validation passed, now accept the request
        self.accept_bucket_move(event.bucket_id).await?;

        // File downloads will be initiated by the StartMovedBucketDownload event handler
        info!(
            target: LOG_TARGET,
            "Bucket move request accepted for bucket {:?}, waiting for on-chain confirmation", event.bucket_id
        );

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
    async fn reject_bucket_move(&mut self, bucket_id: BucketId<Runtime>) -> anyhow::Result<()> {
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
                SendExtrinsicOptions::new(Duration::from_secs(
                    self.storage_hub_handler
                        .provider_config
                        .blockchain_service
                        .extrinsic_retry_timeout,
                )),
                RetryStrategy::default()
                    .with_max_retries(self.config.max_try_count)
                    .with_max_tip(self.config.max_tip),
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

    async fn accept_bucket_move(&self, bucket_id: BucketId<Runtime>) -> anyhow::Result<()> {
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
                SendExtrinsicOptions::new(Duration::from_secs(
                    self.storage_hub_handler
                        .provider_config
                        .blockchain_service
                        .extrinsic_retry_timeout,
                )),
                RetryStrategy::default()
                    .with_max_retries(self.config.max_try_count)
                    .with_max_tip(self.config.max_tip),
                false,
            )
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to submit move bucket acceptance after {} retries: {:?}",
                    self.config.max_try_count,
                    e
                )
            })?;

        Ok(())
    }

    async fn check_and_increase_capacity(
        &self,
        required_size: u64,
        own_msp_id: ProviderId<Runtime>,
    ) -> anyhow::Result<()> {
        let available_capacity = self
            .storage_hub_handler
            .blockchain
            .query_available_storage_capacity(own_msp_id)
            .await
            .map_err(|e| {
                error!(target: LOG_TARGET, "Failed to query available storage capacity: {:?}", e);
                anyhow::anyhow!("Failed to query available storage capacity: {:?}", e)
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
                    error!(target: LOG_TARGET, "Failed to query storage provider capacity: {:?}", e);
                    anyhow::anyhow!("Failed to query storage provider capacity: {:?}", e)
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
}
