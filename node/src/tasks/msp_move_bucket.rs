use anyhow::anyhow;
use futures::future::join_all;
use rand::{rngs::StdRng, SeedableRng};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

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
    BucketId, HashT, ProviderId, StorageProofsMerkleTrieLayout, StorageProviderId,
};
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};

use crate::services::{
    handler::StorageHubHandler,
    types::{MspForestStorageHandlerT, ShNodeType},
};

// Constants
const LOG_TARGET: &str = "storage-hub::msp-move-bucket";
lazy_static::lazy_static! {
    // A global RNG available for peer selection
    static ref GLOBAL_RNG: Mutex<StdRng> = Mutex::new(StdRng::from_entropy());
}

/// Handles requests for MSP (Main Storage Provider) to respond to bucket move requests.
/// Downloads bucket files from BSPs (Backup Storage Providers).
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
        MspRespondMoveBucketTask {
            storage_hub_handler: self.storage_hub_handler.clone(),
            pending_bucket_id: self.pending_bucket_id,
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

                    // Download file using the simplified download method
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

    /// Downloads a file from BSPs (Backup Storage Providers).
    ///
    /// Uses the FileDownloadManager which implements multi-level parallelism:
    /// - File-level parallelism: Multiple files can be downloaded simultaneously
    /// - Chunk-level parallelism: For each file, multiple chunk batches are downloaded in parallel
    /// - Peer selection and retry strategy: Selects and tries multiple peers
    async fn download_file(
        &self,
        file: &shc_indexer_db::models::File,
        bucket: &BucketId,
    ) -> anyhow::Result<()> {
        let file_metadata = file
            .to_file_metadata(bucket.as_ref().to_vec())
            .map_err(|e| anyhow!("Failed to convert file to file metadata: {:?}", e))?;
        let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();

        info!(
            target: LOG_TARGET,
            "Starting file download for file_key: {:?}", file_key
        );

        // Register BSP peers with the peer manager for this file
        let bsp_peer_ids = file
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

        for &peer_id in &bsp_peer_ids {
            self.storage_hub_handler
                .peer_manager
                .add_peer(peer_id, file_key)
                .await;
        }

        // Get the file storage reference
        let file_storage = self.storage_hub_handler.file_storage.clone();

        // Use the simplified FileDownloadManager interface
        self.storage_hub_handler
            .file_download_manager
            .download_file(
                file_metadata,
                *bucket,
                self.storage_hub_handler.file_transfer.clone(),
                file_storage,
            )
            .await
    }
}
