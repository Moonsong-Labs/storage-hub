//! Trusted File Transfer Utilities
//!
//! This module contains utility functions for downloading files from peers
//! via the trusted file transfer server.

use anyhow::{anyhow, Result};
use sc_tracing::tracing::info;
use shc_file_manager::traits::FileStorage;
use sp_core::H256;

use crate::{trusted_file_transfer::files::process_chunk_stream, types::FileStorageT};

const LOG_TARGET: &str = "trusted-file-transfer-utils";

/// Downloads a file from a peer's trusted file transfer server and writes it to local storage.
///
/// # Arguments
/// * `peer_url` - The base URL of the peer's trusted file transfer server (e.g., "http://192.168.1.100:7070")
/// * `file_key` - The file key to download
/// * `file_storage` - The file storage to write chunks to
///
/// # Returns
/// * `Ok(())` if the file was downloaded and stored successfully
/// * `Err` if there was an error downloading or storing the file
pub async fn download_file_from_peer<FL: FileStorageT>(
    peer_url: &str,
    file_key: &H256,
    file_storage: &tokio::sync::RwLock<FL>,
) -> Result<()>
where
    FL: FileStorage<shc_common::types::StorageProofsMerkleTrieLayout> + Send + Sync,
{
    info!(
        target: LOG_TARGET,
        file_key = %file_key,
        peer_url = %peer_url,
        "Downloading file from peer"
    );

    // Build the download URL
    let download_url = format!("{}/download/{:x}", peer_url.trim_end_matches('/'), file_key);

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

    let bytes_stream = response.bytes_stream();
    process_chunk_stream(file_storage, file_key, bytes_stream).await?;

    info!(
        target: LOG_TARGET,
        file_key = %file_key,
        "Successfully downloaded and stored file from peer"
    );

    Ok(())
}
