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

use crate::{handler::StorageHubHandler, types::ShNodeType};
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

    fn send_chunks<'a>(
        &'a self,
        peer_id: PeerId,
        file_metadata: &'a FileMetadata,
        file_key: H256,
        chunk_count: u64,
    ) -> impl core::future::Future<Output = Result<(), anyhow::Error>> + 'a {
        async move {
            debug!(target: LOG_TARGET, "Attempting to send chunks of file key {:?} to peer {:?}", file_key, peer_id);

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
                        "Sending batch of {} chunks (total size: {} bytes) for file {:?} to peer {:?}",
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
                                return Err(anyhow::anyhow!("Failed to send file {:?}", file_key));
                            }
                            Err(e) => {
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
                                return Err(anyhow::anyhow!("Failed to send file {:?}", file_key));
                            }
                            Err(e) => {
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
