use log::{error, info, warn};
use shc_actors_framework::event_bus::EventHandler;
use shc_common::traits::StorageEnableRuntime;
use shc_common::types::{HashT, StorageProofsMerkleTrieLayout};
use shc_file_transfer_service::events::RetryBucketMoveDownload;
use std::sync::Arc;

use crate::{
    download_state_store::DownloadStateStore,
    file_download_manager::BucketDownloadError,
    handler::StorageHubHandler,
    types::{MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "retry-bucket-move-task";

/// Task that handles retrying and resuming bucket move downloads
/// that might have been interrupted.
pub struct MspRetryBucketMoveTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
    download_state_store: Arc<DownloadStateStore<Runtime>>,
}

impl<NT, Runtime> MspRetryBucketMoveTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler: storage_hub_handler.clone(),
            download_state_store: storage_hub_handler
                .file_download_manager
                .download_state_store(),
        }
    }
}

impl<NT, Runtime> Clone for MspRetryBucketMoveTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> Self {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            download_state_store: self.download_state_store.clone(),
        }
    }
}

impl<NT, Runtime> EventHandler<RetryBucketMoveDownload> for MspRetryBucketMoveTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, _event: RetryBucketMoveDownload) -> anyhow::Result<String> {
        info!(
            target: LOG_TARGET,
            "Checking for pending bucket downloads to resume"
        );

        // Get all pending bucket downloads from the state store
        let context = self.download_state_store.open_rw_context();
        let pending_buckets = context.get_all_pending_bucket_downloads();

        if pending_buckets.is_empty() {
            info!(
                target: LOG_TARGET,
                "No pending bucket downloads to resume"
            );
            return Ok("No pending bucket downloads to resume".to_string());
        }

        info!(
            target: LOG_TARGET,
            "Found {} pending bucket downloads to resume", pending_buckets.len()
        );

        // Get indexer DB pool for fetching file metadata
        let indexer_db_pool =
            if let Some(indexer_db_pool) = self.storage_hub_handler.indexer_db_pool.clone() {
                indexer_db_pool
            } else {
                warn!(
                    target: LOG_TARGET,
                    "Indexer is disabled but there are pending bucket downloads"
                );
                return Ok("Indexer disabled; cannot resume pending bucket downloads".to_string());
            };

        // For each pending bucket, try to resume its download
        for bucket_id in pending_buckets {
            info!(
                target: LOG_TARGET,
                "Attempting to resume download for bucket [0x{:x}]", bucket_id
            );

            // Get connection to indexer DB
            let mut indexer_connection = match indexer_db_pool.get().await {
                Ok(conn) => conn,
                Err(e) => {
                    error!(
                        target: LOG_TARGET,
                        "Failed to get indexer connection: {:?}", e
                    );
                    continue;
                }
            };

            // Check if there are missing files for this bucket
            let context = self.download_state_store.open_rw_context();
            let missing_files = context.get_missing_files_for_bucket(&bucket_id);

            if missing_files.is_empty() {
                info!(
                    target: LOG_TARGET,
                    "No missing files found for bucket [0x{:x}], marking as completed", bucket_id
                );

                // Mark as completed if no missing files in download state
                context.mark_bucket_download_completed(&bucket_id);
                context.commit();
                continue;
            }
            context.commit();

            // Get files for this bucket from indexer
            let indexer_files = match shc_indexer_db::models::File::get_by_onchain_bucket_id(
                &mut indexer_connection,
                bucket_id.as_ref().to_vec(),
            )
            .await
            {
                Ok(files) => files,
                Err(e) => {
                    error!(
                        target: LOG_TARGET,
                        "Failed to get files for bucket [0x{:x}] from indexer: {:?}", bucket_id, e
                    );
                    // Continue with download attempt even if we couldn't get indexer info
                    // We'll just have fewer peers to try
                    Vec::new()
                }
            };

            // Register BSP peers from indexer files
            for file in &indexer_files {
                if let Ok(metadata) = file.to_file_metadata(bucket_id.as_ref().to_vec()) {
                    // Register BSP peers for this file
                    let file_key = metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();
                    if let Ok(peer_ids) =
                        futures::executor::block_on(file.get_bsp_peer_ids(&mut indexer_connection))
                    {
                        for peer_id in &peer_ids {
                            // Register peer for file
                            let _ = self
                                .storage_hub_handler
                                .peer_manager
                                .add_peer(*peer_id, file_key);
                        }
                    }
                }
            }

            // Get list of file metadatas from indexer files
            let file_metadatas = indexer_files
                .iter()
                .filter_map(
                    |file| match file.to_file_metadata(bucket_id.as_ref().to_vec()) {
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

            info!(
                target: LOG_TARGET,
                "Starting download of {} files for bucket [0x{:x}]",
                file_metadatas.len(), bucket_id
            );

            let file_transfer_service = self.storage_hub_handler.file_transfer.clone();
            let file_storage = self.storage_hub_handler.file_storage.clone();

            // Try to download the bucket using our new method with internal locking
            match self
                .storage_hub_handler
                .file_download_manager
                .try_lock_and_download_bucket(
                    bucket_id,
                    file_metadatas,
                    file_transfer_service,
                    file_storage,
                )
                .await
            {
                Ok(()) => {
                    info!(
                        target: LOG_TARGET,
                        "Successfully resumed bucket download for {:?}", bucket_id
                    );
                }
                Err(BucketDownloadError::AlreadyBeingDownloaded(_)) => {
                    info!(
                        target: LOG_TARGET,
                        "Bucket {:?} is already being downloaded by another task", bucket_id
                    );
                }
                Err(BucketDownloadError::DownloadFailed(e)) => {
                    error!(
                        target: LOG_TARGET,
                        "Failed to resume bucket download for {:?}: {:?}", bucket_id, e
                    );
                    // Note: We don't mark as completed here so it can be retried later
                }
            }
        }

        Ok("RetryBucketMoveDownload processing completed".to_string())
    }
}
