use std::collections::HashSet;

use sc_network::{PeerId, RequestFailure};
use sc_tracing::tracing::*;
use sp_core::H256;

use shc_common::{
    traits::StorageEnableRuntime,
    types::{
        FileMetadata, HashT, StorageProofsMerkleTrieLayout, BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE,
    },
};
use shp_file_metadata::ChunkId;

use crate::{
    handler::StorageHubHandler,
    metrics::{STATUS_FAILURE, STATUS_SUCCESS},
    observe_histogram,
    types::ShNodeType,
};
use shc_blockchain_service::commands::BlockchainServiceCommandInterface;
use shc_file_manager::traits::FileStorage;
use shc_file_transfer_service::commands::{
    FileTransferServiceCommandInterface, FileTransferServiceCommandInterfaceExt,
};

const LOG_TARGET: &str = "chunk-uploader";

pub trait ChunkUploaderExt<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn as_handler(&self) -> &StorageHubHandler<NT, Runtime>;

    /// Attempts to upload `file_metadata` to the first peer in `peer_ids` that
    /// successfully accepts its chunks.
    ///
    /// Behaviour:
    /// - Computes the file key and total chunk count from `file_metadata`.
    /// - Iterates peers in the provided order, delegating to [`send_chunks`].
    /// - Returns `Ok(())` immediately on the first successful upload to a peer
    ///   (including the case where the peer already has the full file),
    ///   otherwise tries the next peer.
    /// - If all peers fail, returns an error referencing the file fingerprint.
    ///
    /// Notes:
    /// - Peer order matters; the first successful peer short‑circuits the loop.
    /// - Transient errors are logged and the next peer is attempted.
    ///
    /// Returns a future that resolves when either an upload succeeds or all
    /// peers have been attempted.
    fn upload_file_to_peer_ids<'a>(
        &'a self,
        peer_ids: Vec<PeerId>,
        file_metadata: &'a FileMetadata,
    ) -> impl core::future::Future<Output = Result<(), anyhow::Error>> + 'a {
        async move {
            let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();
            let chunk_count = file_metadata.chunks_count();

            for peer_id in peer_ids {
                match self
                    .send_chunks(peer_id, file_metadata, file_key, chunk_count)
                    .await
                {
                    Ok(()) => return Ok(()),
                    Err(err) => {
                        warn!(target: LOG_TARGET, "{:?}", err);
                        continue;
                    }
                }
            }

            Err(anyhow::anyhow!(
                "Failed to send file {:?} to any of the peers",
                file_metadata.fingerprint()
            ))
        }
    }

    /// Sends the chunks of a single file to a specific `peer_id` in bounded
    /// batches, generating Merkle proofs per batch and retrying on transient
    /// failures.
    ///
    /// Behaviour:
    /// - Reads chunks from local storage and accumulates them into batches not
    ///   exceeding `BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE`.
    /// - For each batch, generates a proof and calls the file‑transfer upload
    ///   request. Parses the remote response to detect whether the peer already
    ///   has the entire file (short‑circuit success).
    /// - Implements limited retries:
    ///   - `RequestFailure::Refused`: up to 3 retries with short sleep.
    ///   - `RequestFailure::Network(_) | NotConnected`: up to 10 retries,
    ///     waiting for several blocks between attempts.
    /// - On the final batch, logs completion if the remote reports that the
    ///   file is complete.
    ///
    /// Returns a future that resolves to `Ok(())` if the peer accepts all
    /// batches (or already has the file), or an error if persistent failures
    /// occur.
    fn send_chunks<'a>(
        &'a self,
        peer_id: PeerId,
        file_metadata: &'a FileMetadata,
        file_key: H256,
        chunk_count: u64,
    ) -> impl core::future::Future<Output = Result<(), anyhow::Error>> + 'a {
        async move {
            let start_time = std::time::Instant::now();

            debug!(target: LOG_TARGET, "Attempting to send chunks of file key [{:x}] to peer {:?}", file_key, peer_id);

            let mut current_batch = Vec::new();
            let mut current_batch_size = 0usize;

            let fingerprint = file_metadata.fingerprint();

            for chunk_id in 0..chunk_count {
                let chunk_data = self
                    .as_handler()
                    .file_storage
                    .read()
                    .await
                    .get_chunk(&file_key, &ChunkId::new(chunk_id))
                    .map_err(|e| anyhow::anyhow!("Failed to get chunk: {:?}", e))?;

                if current_batch_size + chunk_data.len() > BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE {
                    debug!(
                        target: LOG_TARGET,
                        "Sending batch of {} chunks (total size: {} bytes) for file [{:x}] to peer {:?}",
                        current_batch.len(),
                        current_batch_size,
                        file_key,
                        peer_id
                    );

                    let proof = self
                        .as_handler()
                        .file_storage
                        .read()
                        .await
                        .generate_proof(
                            &file_key,
                            &HashSet::from_iter(current_batch.iter().cloned()),
                        )
                        .map_err(|e| {
                            anyhow::anyhow!(
                                "Failed to generate proof for batch of file {:?}\n Error: {:?}",
                                file_key,
                                e
                            )
                        })?;

                    let mut retry_attempts = 0;
                    loop {
                        let upload_response = self
                            .as_handler()
                            .file_transfer
                            .upload_request(peer_id, file_key.as_ref().into(), proof.clone(), None)
                            .await;

                        match upload_response {
                            Ok(r) => {
                                debug!(
                                    target: LOG_TARGET,
                                    "Successfully uploaded batch for file fingerprint {:x} to peer {:?}",
                                    fingerprint,
                                    peer_id
                                );

                                let r = self
                                    .as_handler()
                                    .file_transfer
                                    .parse_remote_upload_data_response(&r.0)
                                    .map_err(|e| {
                                        anyhow::anyhow!(
                                            "Failed to parse remote upload data response: {:?}",
                                            e
                                        )
                                    })?;

                                if r.file_complete {
                                    info!(
                                        target: LOG_TARGET,
                                        "Stopping file upload process. Peer {:?} has the entire file fingerprint {:x}",
                                        peer_id,
                                        fingerprint
                                    );
                                    return Ok(());
                                }

                                break;
                            }
                            Err(RequestFailure::Refused) if retry_attempts < 3 => {
                                warn!(
                                    target: LOG_TARGET,
                                    "Final batch upload rejected by peer {:?}, retrying... (attempt {})",
                                    peer_id,
                                    retry_attempts + 1
                                );
                                retry_attempts += 1;
                                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                            }
                            Err(RequestFailure::Network(_)) | Err(RequestFailure::NotConnected)
                                if retry_attempts < 10 =>
                            {
                                warn!(
                                    target: LOG_TARGET,
                                    "Unable to upload final batch to peer {:?}, retrying... (attempt {})",
                                    peer_id,
                                    retry_attempts + 1
                                );
                                retry_attempts += 1;

                                self.as_handler()
                                    .blockchain
                                    .wait_for_num_blocks(5u32.into())
                                    .await?;
                            }
                            Err(RequestFailure::Refused)
                            | Err(RequestFailure::Network(_))
                            | Err(RequestFailure::NotConnected) => {
                                observe_histogram!(
                                    handler: self.as_handler(),
                                    file_transfer_seconds,
                                    STATUS_FAILURE,
                                    start_time.elapsed().as_secs_f64()
                                );
                                return Err(anyhow::anyhow!("Failed to send file {:?}", file_key));
                            }
                            Err(e) => {
                                observe_histogram!(
                                    handler: self.as_handler(),
                                    file_transfer_seconds,
                                    STATUS_FAILURE,
                                    start_time.elapsed().as_secs_f64()
                                );
                                return Err(anyhow::anyhow!(
                                    "Unexpected error while trying to upload final batch to peer {:?} (Error: {:?})",
                                    peer_id,
                                    e
                                ));
                            }
                        }
                    }

                    current_batch.clear();
                    current_batch_size = 0;
                }

                current_batch.push(ChunkId::new(chunk_id));
                current_batch_size += chunk_data.len();

                if chunk_id == chunk_count - 1 && !current_batch.is_empty() {
                    debug!(
                        target: LOG_TARGET,
                        "Sending final batch of {} chunks (total size: {} bytes) for file {:?} to peer {:?}",
                        current_batch.len(),
                        current_batch_size,
                        file_key,
                        peer_id
                    );

                    let proof = self
                        .as_handler()
                        .file_storage
                        .read()
                        .await
                        .generate_proof(
                            &file_key,
                            &HashSet::from_iter(current_batch.iter().cloned()),
                        )
                        .map_err(|e| {
                            anyhow::anyhow!(
                                "Failed to generate proof for final batch of file {:?}\n Error: {:?}",
                                file_key,
                                e
                            )
                        })?;

                    let mut retry_attempts = 0;
                    loop {
                        let upload_response = self
                            .as_handler()
                            .file_transfer
                            .upload_request(peer_id, file_key.as_ref().into(), proof.clone(), None)
                            .await;

                        match upload_response.as_ref() {
                            Ok(r) => {
                                debug!(
                                    target: LOG_TARGET,
                                    "Successfully uploaded final batch for file fingerprint {:x} to peer {:?}",
                                    fingerprint,
                                    peer_id
                                );

                                let r = self
                                    .as_handler()
                                    .file_transfer
                                    .parse_remote_upload_data_response(&r.0)
                                    .map_err(|e| {
                                        anyhow::anyhow!(
                                            "Failed to parse remote upload data response: {:?}",
                                            e
                                        )
                                    })?;

                                if r.file_complete {
                                    info!(
                                        target: LOG_TARGET,
                                        "File upload complete. Peer {:?} has the entire file fingerprint {:x}",
                                        peer_id,
                                        fingerprint
                                    );
                                }
                                break;
                            }
                            Err(RequestFailure::Refused) if retry_attempts < 3 => {
                                warn!(
                                    target: LOG_TARGET,
                                    "Final batch upload rejected by peer {:?}, retrying... (attempt {})",
                                    peer_id,
                                    retry_attempts + 1
                                );
                                retry_attempts += 1;
                                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                            }
                            Err(RequestFailure::Network(_)) | Err(RequestFailure::NotConnected)
                                if retry_attempts < 10 =>
                            {
                                warn!(
                                    target: LOG_TARGET,
                                    "Unable to upload final batch to peer {:?}, retrying... (attempt {})",
                                    peer_id,
                                    retry_attempts + 1
                                );
                                retry_attempts += 1;
                                self.as_handler()
                                    .blockchain
                                    .wait_for_num_blocks(5u32.into())
                                    .await?;
                            }
                            Err(RequestFailure::Refused)
                            | Err(RequestFailure::Network(_))
                            | Err(RequestFailure::NotConnected) => {
                                observe_histogram!(
                                    handler: self.as_handler(),
                                    file_transfer_seconds,
                                    STATUS_FAILURE,
                                    start_time.elapsed().as_secs_f64()
                                );
                                return Err(anyhow::anyhow!("Failed to send file {:?}", file_key));
                            }
                            Err(e) => {
                                observe_histogram!(
                                    handler: self.as_handler(),
                                    file_transfer_seconds,
                                    STATUS_FAILURE,
                                    start_time.elapsed().as_secs_f64()
                                );
                                return Err(anyhow::anyhow!(
                                    "Unexpected error while trying to upload final batch to peer {:?} (Error: {:?})",
                                    peer_id,
                                    e
                                ));
                            }
                        }
                    }
                }
            }

            observe_histogram!(
                handler: self.as_handler(),
                file_transfer_seconds,
                STATUS_SUCCESS,
                start_time.elapsed().as_secs_f64()
            );

            info!(target: LOG_TARGET, "Successfully sent file fingerprint {:x} to peer {:?}", fingerprint, peer_id);
            Ok(())
        }
    }
}

impl<NT, Runtime> ChunkUploaderExt<NT, Runtime> for StorageHubHandler<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn as_handler(&self) -> &StorageHubHandler<NT, Runtime> {
        self
    }
}
