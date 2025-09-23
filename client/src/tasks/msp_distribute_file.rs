use std::collections::HashSet;

use sc_network::{PeerId, RequestFailure};
use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface, events::DistributeFileToBsp,
};
use shc_common::{
    traits::StorageEnableRuntime,
    types::{
        ChunkId, FileMetadata, HashT, StorageProofsMerkleTrieLayout,
        BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE,
    },
};
use shc_file_manager::traits::FileStorage;
use shc_file_transfer_service::commands::{
    FileTransferServiceCommandInterface, FileTransferServiceCommandInterfaceExt,
};
use sp_core::H256;

use crate::{
    handler::StorageHubHandler,
    types::{MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "msp-distribute-file-task";

pub struct MspDistributeFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for MspDistributeFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> MspDistributeFileTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, Runtime> MspDistributeFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler: storage_hub_handler.clone(),
        }
    }
}

/// Handles the [`DistributeFileToBsp`] event.
///
/// TODO: Document this
impl<NT, Runtime> EventHandler<DistributeFileToBsp<Runtime>> for MspDistributeFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: DistributeFileToBsp<Runtime>) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Distributing file to BSP",
        );

        // This function handles the whole process of distributing the file to the BSP.
        // If anything fails, we unregister the BSP as distributing file, thus allowing
        // for a retry.
        self.handle_distribute_file_to_bsp(event)
            .await
            .map_err(|e| {
                // TODO: Unregister BSP as distributing file.
                error!(target: LOG_TARGET, "Failed to distribute file to BSP: {:?}", e);
                e
            })
    }
}

impl<NT, Runtime> MspDistributeFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_distribute_file_to_bsp(
        &mut self,
        event: DistributeFileToBsp<Runtime>,
    ) -> anyhow::Result<()> {
        let file_key = event.file_key;
        let bsp_id = event.bsp_id;

        self.storage_hub_handler
            .blockchain
            .register_bsp_distributing(file_key, bsp_id)
            .await?;

        // TODO: Get file metadata from local file storage.

        // TODO: Get MSP multiaddresses from BSP from runtime.

        // TODO: Get peer ids from multiaddresses and register them as known addresses.

        // TODO: Send chunks to provider.

        // TODO: Implement this.
        Ok(())
    }

    async fn send_chunks_to_provider(
        &mut self,
        peer_ids: Vec<PeerId>,
        file_metadata: &FileMetadata,
    ) -> Result<(), anyhow::Error> {
        let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();
        let chunk_count = file_metadata.chunks_count();

        // Iterates and tries to send file to peer.
        // Breaks loop after first successful attempt since all peer ids belong to the same provider.
        for peer_id in peer_ids {
            match self
                .send_chunks(peer_id, file_metadata, file_key, chunk_count)
                .await
            {
                Err(err) => {
                    // If sending chunk failed with one peer id, we try with the next one.
                    warn!(target: LOG_TARGET, "{:?}", err);
                    continue;
                }
                Ok(()) => {
                    // If successful our job is done. No need to try with the next peer id.
                    return Ok(());
                }
            };
        }

        Err(anyhow::anyhow!(
            "Failed to send file {:?} to any of the peers",
            file_metadata.fingerprint()
        ))
    }

    async fn send_chunks(
        &mut self,
        peer_id: PeerId,
        file_metadata: &FileMetadata,
        file_key: H256,
        chunk_count: u64,
    ) -> Result<(), anyhow::Error> {
        debug!(target: LOG_TARGET, "Attempting to send chunks of file key {:?} to peer {:?}", file_key, peer_id);

        let mut current_batch = Vec::new();
        let mut current_batch_size = 0;

        let fingerprint = file_metadata.fingerprint();

        for chunk_id in 0..chunk_count {
            let chunk_data = self
                .storage_hub_handler
                .file_storage
                .read()
                .await
                .get_chunk(&file_key, &ChunkId::new(chunk_id))
                .map_err(|e| anyhow::anyhow!("Failed to get chunk: {:?}", e))?;

            // Check if adding this chunk would exceed the batch size limit
            if current_batch_size + chunk_data.len() > BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE {
                // Send current batch before adding new chunk
                debug!(
                    target: LOG_TARGET,
                    "Sending batch of {} chunks (total size: {} bytes) for file {:?} to peer {:?}",
                    current_batch.len(),
                    current_batch_size,
                    file_key,
                    peer_id
                );

                // Generate proof for the entire batch
                let proof = match self
                    .storage_hub_handler
                    .file_storage
                    .read()
                    .await
                    .generate_proof(
                        &file_key,
                        &HashSet::from_iter(current_batch.iter().cloned()),
                    ) {
                    Ok(proof) => proof,
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "Failed to generate proof for batch of file {:?}\n Error: {:?}",
                            file_key,
                            e
                        ));
                    }
                };

                let mut retry_attempts = 0;
                loop {
                    let upload_response = self
                        .storage_hub_handler
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
                                .storage_hub_handler
                                .file_transfer
                                .parse_remote_upload_data_response(&r.0)
                                .map_err(|e| {
                                    anyhow::anyhow!(
                                        "Failed to parse remote upload data response: {:?}",
                                        e
                                    )
                                })?;

                            // If the provider signals they have the entire file, we can stop
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

                            // Wait for a short time before retrying
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

                            // Wait a bit for the MSP to be online
                            self.storage_hub_handler
                                .blockchain
                                .wait_for_num_blocks(5u32.into())
                                .await?;
                        }
                        Err(RequestFailure::Refused)
                        | Err(RequestFailure::Network(_))
                        | Err(RequestFailure::NotConnected) => {
                            // Return an error if the provider refused to answer.
                            return Err(anyhow::anyhow!("Failed to send file {:?}", file_key));
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!(
                                "Unexpected error while trying to upload batch to peer {:?} (Error: {:?})",
                                peer_id,
                                e
                            ));
                        }
                    }
                }

                // Clear the batch for next iteration
                current_batch.clear();
                current_batch_size = 0;
            }

            // Add chunk to current batch
            current_batch.push(ChunkId::new(chunk_id));
            current_batch_size += chunk_data.len();

            // If this is the last chunk, send the final batch
            if chunk_id == chunk_count - 1 && !current_batch.is_empty() {
                debug!(
                    target: LOG_TARGET,
                    "Sending final batch of {} chunks (total size: {} bytes) for file {:?} to peer {:?}",
                    current_batch.len(),
                    current_batch_size,
                    file_key,
                    peer_id
                );

                // Generate proof for the final batch
                let proof = match self
                    .storage_hub_handler
                    .file_storage
                    .read()
                    .await
                    .generate_proof(
                        &file_key,
                        &HashSet::from_iter(current_batch.iter().cloned()),
                    ) {
                    Ok(proof) => proof,
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "Failed to generate proof for final batch of file {:?}\n Error: {:?}",
                            file_key,
                            e
                        ));
                    }
                };

                let mut retry_attempts = 0;
                loop {
                    let upload_response = self
                        .storage_hub_handler
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
                                .storage_hub_handler
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

                            // Wait for a short time before retrying
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

                            // Wait a bit for the MSP to be online
                            self.storage_hub_handler
                                .blockchain
                                .wait_for_num_blocks(5u32.into())
                                .await?;
                        }
                        Err(RequestFailure::Refused)
                        | Err(RequestFailure::Network(_))
                        | Err(RequestFailure::NotConnected) => {
                            // Return an error if the provider refused to answer.
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
