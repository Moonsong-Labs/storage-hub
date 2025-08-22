use log::{debug, info, warn};
use sc_network::{PeerId, RequestFailure};
use sp_core::H256;
use sp_runtime::traits::SaturatedConversion;
use std::collections::HashSet;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface,
    events::{AcceptedBspVolunteer, NewStorageRequest},
};
use shc_common::{
    traits::StorageEnableRuntime,
    types::{
        FileMetadata, HashT, StorageProofsMerkleTrieLayout, BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE,
    },
};
use shc_file_manager::traits::FileStorage;
use shc_file_transfer_service::commands::{
    FileTransferServiceCommandInterface, FileTransferServiceCommandInterfaceExt,
};
use shp_file_metadata::ChunkId;

use crate::{handler::StorageHubHandler, types::ShNodeType};

const LOG_TARGET: &str = "user-sends-file-task";

/// [`UserSendsFileTask`]: Handles the events related to users sending a file to be stored by BSPs
/// volunteering for that file.
/// It can serve multiple BSPs volunteering to store each file, since
/// it reacts to every [`AcceptedBspVolunteer`] from the runtime.
pub struct UserSendsFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for UserSendsFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> Self {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, Runtime> UserSendsFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<NT, Runtime> EventHandler<NewStorageRequest<Runtime>> for UserSendsFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    Runtime: StorageEnableRuntime,
{
    /// Reacts to a new storage request from the runtime, which is triggered by a user sending a file to be stored.
    /// It generates the file metadata and sends it to the BSPs volunteering to store the file.
    async fn handle_event(&mut self, event: NewStorageRequest<Runtime>) -> anyhow::Result<()> {
        let node_pub_key = self
            .storage_hub_handler
            .blockchain
            .get_node_public_key()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get node public key: {:?}", e))?;

        if event.who != node_pub_key.into() {
            // Skip if the storage request was not created by this user node.
            return Ok(());
        }

        info!(
            target: LOG_TARGET,
            "Handling new storage request from user [{:?}], with location [{:?}]",
            event.who,
            event.location,
        );

        let Some(msp_id) = self
            .storage_hub_handler
            .blockchain
            .query_msp_id_of_bucket_id(event.bucket_id)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to query MSP ID of bucket ID {:?}\n Error: {:?}",
                    event.bucket_id,
                    e
                )
            })?
        else {
            warn!(
                target: LOG_TARGET,
                "Skipping storage request - no MSP ID found for bucket ID {:?}",
                event.bucket_id
            );
            return Ok(());
        };

        let multiaddress_vec = self
            .storage_hub_handler
            .blockchain
            .query_provider_multiaddresses(msp_id)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to query MSP multiaddresses of MSP ID {:?}\n Error: {:?}",
                    msp_id,
                    e
                )
            })?;

        // Adds the multiaddresses of the MSP to the known addresses of the file transfer service.
        // This is required to establish a connection to the MSP.
        let peer_ids = self
            .storage_hub_handler
            .file_transfer
            .extract_peer_ids_and_register_known_addresses(multiaddress_vec)
            .await;

        let who = event.who.as_ref().to_vec();
        let file_metadata = FileMetadata::new(
            who,
            event.bucket_id.as_ref().to_vec(),
            event.location.into_inner(),
            event.size.saturated_into(),
            event.fingerprint,
        )
        .map_err(|_| anyhow::anyhow!("Invalid file metadata"))?;

        let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();

        // TODO: Check how we can improve this.
        // We could either make sure this scenario doesn't happen beforehand,
        // by implementing formatting checks for multiaddresses in the runtime,
        // or try to fetch new peer ids from the runtime at this point.
        if peer_ids.is_empty() {
            info!(target: LOG_TARGET, "No peers were found to receive file key {:?}", file_key);
        }

        self.send_chunks_to_provider(peer_ids, &file_metadata).await
    }
}

impl<NT, Runtime> EventHandler<AcceptedBspVolunteer<Runtime>> for UserSendsFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    Runtime: StorageEnableRuntime,
{
    /// Reacts to BSPs volunteering (`AcceptedBspVolunteer` from the runtime) to store the user's file,
    /// establishes a connection to each BSPs through the p2p network and sends the file.
    /// At this point we assume that the file is merkleised and already in file storage, and
    /// for this reason the file transfer to the BSP should not fail unless the p2p connection fails.
    async fn handle_event(&mut self, event: AcceptedBspVolunteer<Runtime>) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Handling BSP volunteering to store a file from user [{:?}], with location [{:?}]",
            event.owner,
            event.location,
        );

        let owner = event.owner.as_ref().to_vec();
        let file_metadata = FileMetadata::new(
            owner,
            event.bucket_id.as_ref().to_vec(),
            event.location.into_inner(),
            event.size.saturated_into(),
            event.fingerprint,
        )
        .map_err(|_| anyhow::anyhow!("Invalid file metadata"))?;

        // Adds the multiaddresses of the BSP volunteering to store the file to the known addresses of the file transfer service.
        // This is required to establish a connection to the BSP.
        let peer_ids = self
            .storage_hub_handler
            .file_transfer
            .extract_peer_ids_and_register_known_addresses(event.multiaddresses)
            .await;

        let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();

        // TODO: Check how we can improve this.
        // We could either make sure this scenario doesn't happen beforehand,
        // by implementing formatting checks for multiaddresses in the runtime,
        // or try to fetch new peer ids from the runtime at this point.
        if peer_ids.is_empty() {
            info!(target: LOG_TARGET, "No peers were found to receive file key {:?}", file_key);
        }

        self.send_chunks_to_provider(peer_ids, &file_metadata).await
    }
}

impl<NT, Runtime> UserSendsFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
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
