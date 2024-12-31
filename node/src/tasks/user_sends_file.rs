use log::{debug, error, info, warn};
use sc_network::{PeerId, RequestFailure};
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceInterface,
    events::{AcceptedBspVolunteer, NewStorageRequest},
};
use shc_common::types::{FileMetadata, HashT, StorageProofsMerkleTrieLayout};
use shc_file_transfer_service::commands::{FileTransferServiceInterface, RequestError};
use shc_forest_manager::traits::ForestStorageHandler;
use shp_file_metadata::ChunkId;
use sp_runtime::AccountId32;

use crate::services::{handler::StorageHubHandler, types::FileStorageT};

const LOG_TARGET: &str = "user-sends-file-task";

/// [`UserSendsFileTask`]: Handles the events related to users sending a file to be stored by BSPs
/// volunteering for that file.
/// It can serve multiple BSPs volunteering to store each file, since
/// it reacts to every [`AcceptedBspVolunteer`] from the runtime.
pub struct UserSendsFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    storage_hub_handler: StorageHubHandler<FL, FSH>,
}

impl<FL, FSH> Clone for UserSendsFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<FL, FSH> UserSendsFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    pub fn new(storage_hub_handler: StorageHubHandler<FL, FSH>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<FL, FSH> EventHandler<NewStorageRequest> for UserSendsFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    /// Reacts to a new storage request from the runtime, which is triggered by a user sending a file to be stored.
    /// It generates the file metadata and sends it to the BSPs volunteering to store the file.
    async fn handle_event(&mut self, event: NewStorageRequest) -> anyhow::Result<()> {
        let node_pub_key = self
            .storage_hub_handler
            .blockchain
            .get_node_public_key()
            .await;

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

        let file_metadata = FileMetadata {
            owner: <AccountId32 as AsRef<[u8]>>::as_ref(&event.who).to_vec(),
            bucket_id: event.bucket_id.as_ref().to_vec(),
            file_size: event.size.into(),
            fingerprint: event.fingerprint,
            location: event.location.into_inner(),
        };

        let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();

        // TODO: Check how we can improve this.
        // We could either make sure this scenario doesn't happen beforehand,
        // by implementing formatting checks for multiaddresses in the runtime,
        // or try to fetch new peer ids from the runtime at this point.
        if peer_ids.is_empty() {
            info!(target: LOG_TARGET, "No peers were found to receive file key {:?}", file_key);
        }

        self.send_chunks_to_provider(peer_ids, &file_metadata)
            .await?;

        Ok(())
    }
}

impl<FL, FSH> EventHandler<AcceptedBspVolunteer> for UserSendsFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    /// Reacts to BSPs volunteering (`AcceptedBspVolunteer` from the runtime) to store the user's file,
    /// establishes a connection to each BSPs through the p2p network and sends the file.
    /// At this point we assume that the file is merkleised and already in file storage, and
    /// for this reason the file transfer to the BSP should not fail unless the p2p connection fails.
    async fn handle_event(&mut self, event: AcceptedBspVolunteer) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Handling BSP volunteering to store a file from user [{:?}], with location [{:?}]",
            event.owner,
            event.location,
        );

        let file_metadata = FileMetadata {
            owner: <AccountId32 as AsRef<[u8]>>::as_ref(&event.owner).to_vec(),
            bucket_id: event.bucket_id.as_ref().to_vec(),
            file_size: event.size.into(),
            fingerprint: event.fingerprint,
            location: event.location.into_inner(),
        };

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

impl<FL, FSH> UserSendsFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
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
            debug!(target: LOG_TARGET, "Attempting to send chunks of file key {:?} to peer {:?}", file_key, peer_id);

            for chunk_id in 0..chunk_count {
                debug!(target: LOG_TARGET, "Trying to send chunk id {:?} of file {:?} to peer {:?}", chunk_id, file_key, peer_id);
                let proof = match self
                    .storage_hub_handler
                    .file_storage
                    .read()
                    .await
                    .generate_proof(&file_key, &vec![ChunkId::new(chunk_id)])
                {
                    Ok(proof) => proof,
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "Failed to generate proof for chunk id {:?} of file {:?}\n Error: {:?}",
                            chunk_id,
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
                        Ok(_) => {
                            debug!(target: LOG_TARGET, "Successfully uploaded chunk id {:?} of file {:?} to peer {:?}", chunk_id, file_metadata.fingerprint, peer_id);
                            break;
                        }
                        // Retry if the request was refused by the peer (MSP). This could happen if the user was too fast
                        // before the MSP registered the NewStorageRequest event and was ready to receive the file.
                        Err(RequestError::RequestFailure(RequestFailure::Refused))
                            if retry_attempts < 3 =>
                        {
                            warn!(target: LOG_TARGET, "Chunk id {:?} upload rejected by peer {:?}, retrying... (attempt {})", chunk_id, peer_id, retry_attempts + 1);
                            retry_attempts += 1;

                            // Wait for a short time before retrying.
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        }
                        Err(RequestError::RequestFailure(RequestFailure::Refused)) => {
                            // TODO: Handle MSP not receiving file after multiple retries.
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to upload chunk_id {:?} to peer {:?}\n Error: {:?}", chunk_id, peer_id, e);
                            // In case of an error, we break the inner loop
                            // and try to connect to the next peer id.
                            break;
                        }
                    }
                }
            }
            info!(target: LOG_TARGET, "Successfully sent file {:?} to peer {:?}", file_metadata.fingerprint, peer_id);
            return Ok(());
        }

        Err(anyhow::anyhow!(
            "Failed to send file {:?} to any of the peers",
            file_metadata.fingerprint
        ))
    }
}
