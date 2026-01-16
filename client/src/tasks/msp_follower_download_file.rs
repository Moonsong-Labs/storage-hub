//! # MSP Follower Download File Task
//!
//! This module handles the file download flow for MSP Followers.
//!
//! ### Event Handlers
//!
//! - [`FollowerFileKeyToDownload`]: Emitted when a file key needs to be downloaded.
//!   The handler adds the file key to the internal download list.
//!
//! - [`ProcessFollowerDownloads`]: Emitted every block to process pending downloads.
//!   The handler attempts to download each file once per block. Failed downloads remain
//!   in the queue and will be retried on the next block.

use anyhow::anyhow;
use std::collections::HashSet;
use std::sync::Arc;

use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface,
    events::{FollowerFileKeyToDownload, ProcessFollowerDownloads},
};
use shc_common::traits::StorageEnableRuntime;
use shc_file_manager::traits::FileStorage;
use sp_core::H256;
use tokio::sync::RwLock;

use crate::{
    handler::StorageHubHandler,
    trusted_file_transfer::utils::download_file_from_peer,
    types::{MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "msp-follower-download-file-task";

/// Handles the file download flow for MSP Followers.
///
/// This task processes events to download files from the leader MSP.
/// See [module documentation](self) for the full architecture and event flow diagram.
pub struct MspFollowerDownloadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
    /// Internal list of file keys to download
    file_keys_to_download: Arc<RwLock<HashSet<H256>>>,
}

impl<NT, Runtime> Clone for MspFollowerDownloadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> MspFollowerDownloadFileTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            file_keys_to_download: self.file_keys_to_download.clone(),
        }
    }
}

impl<NT, Runtime> MspFollowerDownloadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler,
            file_keys_to_download: Arc::new(RwLock::new(HashSet::new())),
        }
    }
}

/// Handles the [`FollowerFileKeyToDownload`] event.
///
/// This event is emitted when a file key needs to be downloaded from the leader.
/// The handler adds the file key to the internal download list.
impl<NT, Runtime> EventHandler<FollowerFileKeyToDownload>
    for MspFollowerDownloadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: FollowerFileKeyToDownload) -> anyhow::Result<String> {
        let file_key_hash = event.file_key;

        info!(
            target: LOG_TARGET,
            "Adding file key [{:x}] to download list",
            file_key_hash
        );

        // Add file key to the download list
        {
            let mut keys = self.file_keys_to_download.write().await;
            keys.insert(file_key_hash);
        }

        Ok(format!(
            "Added file key [{:x}] to download list",
            file_key_hash
        ))
    }
}

/// Handles the [`ProcessFollowerDownloads`] event.
///
/// This event triggers processing all pending downloads.
/// The handler processes all files from the download list, continuing even if individual
/// downloads fail. Only successfully downloaded files are removed from the list.
/// Failed downloads remain in the list and will be retried on the next block.
impl<NT, Runtime> EventHandler<ProcessFollowerDownloads>
    for MspFollowerDownloadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, _event: ProcessFollowerDownloads) -> anyhow::Result<String> {
        // Get a snapshot of all file keys to download
        let file_keys_to_download: Vec<H256> = {
            let keys = self.file_keys_to_download.read().await;
            if keys.is_empty() {
                trace!(
                    target: LOG_TARGET,
                    "No files to download from leader"
                );
                return Ok("No files to download".to_string());
            }
            keys.iter().cloned().collect()
        };

        info!(
            target: LOG_TARGET,
            count = file_keys_to_download.len(),
            "Processing {} files to download from peer",
            file_keys_to_download.len()
        );

        // Get peer URL from blockchain service leadership DB
        // Note: In the follower scenario, this is the leader's URL, but the function
        // is generic and can download from any peer
        let peer_url = match self
            .storage_hub_handler
            .blockchain_service
            .get_leader_info()
            .await?
        {
            Some(endpoints) => endpoints.trusted_file_transfer_server_url,
            None => {
                error!(
                    target: LOG_TARGET,
                    "No leader info found in database. Leader may not be available yet."
                );
                return Err(anyhow!(
                    "No leader info found in database. Leader may not be available yet."
                ));
            }
        };

        let mut success_count = 0;
        let mut error_count = 0;
        let mut last_error: Option<anyhow::Error> = None;

        // Process each file once per event
        for file_key in file_keys_to_download {
            info!(
                target: LOG_TARGET,
                file_key = %file_key,
                "Attempting to download file from peer"
            );

            // Download the file from the peer
            match download_file_from_peer(
                &peer_url,
                &file_key,
                &self.storage_hub_handler.file_storage,
            )
            .await
            {
                Ok(()) => {
                    // Remove from download list on success
                    let mut keys = self.file_keys_to_download.write().await;
                    keys.remove(&file_key);
                    success_count += 1;
                    info!(
                        target: LOG_TARGET,
                        file_key = %file_key,
                        "Successfully downloaded file from peer"
                    );
                }
                Err(e) => {
                    error_count += 1;
                    last_error = Some(e.clone());
                    warn!(
                        target: LOG_TARGET,
                        file_key = %file_key,
                        error = %e,
                        "Failed to download file from peer, will retry on next block"
                    );
                    // Keep the file in the download list for retry on next block
                }
            }
        }

        // Return summary
        if error_count > 0 {
            warn!(
                target: LOG_TARGET,
                success_count = success_count,
                error_count = error_count,
                "Completed download processing with some errors"
            );
            Err(last_error.unwrap_or_else(|| anyhow!("Unknown error occurred")))
        } else {
            info!(
                target: LOG_TARGET,
                success_count = success_count,
                "Successfully processed all downloads"
            );
            Ok(format!(
                "Processed {} files: {} successful, {} errors",
                success_count + error_count,
                success_count,
                error_count
            ))
        }
    }
}
