//! Follower file downloader
//!
//! This module contains the logic for MSP Followers to download files from the Leader
//! via the trusted file transfer server.

use anyhow::{anyhow, Result};
use log::{debug, error, info};
use shc_common::types::FileKey;
use shc_file_manager::traits::FileStorage;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use super::files::process_chunk_stream;

const LOG_TARGET: &str = "follower-downloader";

/// Downloads a file from the leader's trusted file transfer server and writes it to local storage.
///
/// # Arguments
/// * `leader_url` - The base URL of the leader's trusted file transfer server (e.g., "http://192.168.1.100:7070")
/// * `file_key` - The file key to download
/// * `file_storage` - The file storage to write chunks to
///
/// # Returns
/// * `Ok(())` if the file was downloaded and stored successfully
/// * `Err` if there was an error downloading or storing the file
pub async fn download_file_from_leader<FL>(
    leader_url: &str,
    file_key: &FileKey,
    file_storage: &tokio::sync::RwLock<FL>,
) -> Result<()>
where
    FL: FileStorage<shc_common::types::StorageProofsMerkleTrieLayout> + Send + Sync,
{
    info!(
        target: LOG_TARGET,
        file_key = %file_key,
        leader_url = %leader_url,
        "Downloading file from leader"
    );

    // Build the download URL
    let download_url = format!(
        "{}/download/{:x}",
        leader_url.trim_end_matches('/'),
        file_key
    );

    // Make HTTP GET request to download the file
    let client = reqwest::Client::new();
    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to send download request: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read error body>".to_string());
        return Err(anyhow!(
            "Download request failed with status {}: {}",
            status,
            error_body
        ));
    }

    // Process the byte stream using the shared chunk processing logic
    let bytes_stream = response.bytes_stream();
    process_chunk_stream(file_storage, file_key, bytes_stream).await?;

    info!(
        target: LOG_TARGET,
        file_key = %file_key,
        "Successfully downloaded and stored file from leader"
    );

    Ok(())
}

/// Spawns a background task that polls for files to download from the leader every 6 seconds.
///
/// # Arguments
/// * `file_keys_to_retrieve` - Arc<RwLock<HashSet>> of file keys that need to be downloaded
/// * `leader_url` - The base URL of the leader's trusted file transfer server
/// * `file_storage` - The file storage to write chunks to
///
/// # Behavior
/// - Polls every 6 seconds
/// - Processes files until the list is exhausted or an error occurs
/// - On error, logs the error and continues with the next file
/// - Removes successfully downloaded files from the tracking set
pub fn spawn_follower_download_task<FL>(
    file_keys_to_retrieve: Arc<RwLock<HashSet<FileKey>>>,
    leader_url: String,
    file_storage: Arc<tokio::sync::RwLock<FL>>,
) where
    FL: FileStorage<shc_common::types::StorageProofsMerkleTrieLayout> + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(6));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        info!(
            target: LOG_TARGET,
            "Follower download task started, polling every 6 seconds"
        );

        loop {
            interval.tick().await;

            // Get a snapshot of file keys to download
            let keys_to_download: Vec<FileKey> = {
                let keys_guard = file_keys_to_retrieve.read().unwrap();
                keys_guard.iter().cloned().collect()
            };

            if keys_to_download.is_empty() {
                debug!(
                    target: LOG_TARGET,
                    "No files to download from leader"
                );
                continue;
            }

            info!(
                target: LOG_TARGET,
                count = keys_to_download.len(),
                "Processing files to download from leader"
            );

            // Process each file
            for file_key in keys_to_download {
                debug!(
                    target: LOG_TARGET,
                    file_key = %file_key,
                    "Attempting to download file from leader"
                );

                match download_file_from_leader(&leader_url, &file_key, &file_storage).await {
                    Ok(()) => {
                        // Remove from tracking set on success
                        let mut keys_guard = file_keys_to_retrieve.write().unwrap();
                        keys_guard.remove(&file_key);
                        info!(
                            target: LOG_TARGET,
                            file_key = %file_key,
                            "Successfully downloaded and removed from tracking list"
                        );
                    }
                    Err(e) => {
                        error!(
                            target: LOG_TARGET,
                            file_key = %file_key,
                            error = %e,
                            "Failed to download file from leader, will retry on next poll"
                        );
                        // Keep the file in the tracking set to retry later
                    }
                }
            }
        }
    });
}
