use crate::tasks::StorageHubHandler;
use log::{debug, error, info};
use sc_network::PeerId;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::events::AcceptedBspVolunteer;
use shc_common::types::FileMetadata;
use shc_file_manager::traits::FileStorage;
use shc_file_transfer_service::commands::FileTransferServiceInterface;
use shc_forest_manager::traits::ForestStorage;
use shp_file_metadata::ChunkId;
use sp_runtime::AccountId32;
use sp_trie::TrieLayout;

const LOG_TARGET: &str = "user-sends-file-task";

/// Handles the events related to users sending a file to be stored by BSPs
/// volunteering for that file.
/// It can serve multiple BSPs volunteering to store each file, since
/// it reacts to every `AcceptedBspVolunteer` from the runtime.
pub struct UserSendsFileTask<T, FL, FS>
where
    T: Send + Sync + TrieLayout + 'static,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T> + 'static,
{
    storage_hub_handler: StorageHubHandler<T, FL, FS>,
}

impl<T, FL, FS> Clone for UserSendsFileTask<T, FL, FS>
where
    T: Send + Sync + TrieLayout + 'static,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T> + 'static,
{
    fn clone(&self) -> Self {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<T, FL, FS> UserSendsFileTask<T, FL, FS>
where
    T: Send + Sync + TrieLayout + 'static,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T> + 'static,
{
    pub fn new(storage_hub_handler: StorageHubHandler<T, FL, FS>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<T, FL, FS> EventHandler<AcceptedBspVolunteer> for UserSendsFileTask<T, FL, FS>
where
    T: Send + Sync + TrieLayout + 'static,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T> + 'static,
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

        let chunk_count = file_metadata.chunks_count();
        let file_key = file_metadata.file_key::<T::Hash>();

        // Adds the multiaddresses of the BSP volunteering to store the file to the known addresses of the file transfer service.
        // This is required to establish a connection to the BSP.
        let mut peer_ids = Vec::new();
        for multiaddress in &event.multiaddresses {
            if let Some(peer_id) = PeerId::try_from_multiaddr(&multiaddress) {
                if let Err(error) = self
                    .storage_hub_handler
                    .file_transfer
                    .add_known_address(peer_id, multiaddress.clone())
                    .await
                {
                    error!(target: LOG_TARGET, "Failed to add known address {:?} for peer {:?} due to {:?}", multiaddress, peer_id, error);
                }
                peer_ids.push(peer_id);
            }
        }

        // TODO: Check how we can improve this.
        // We could either make sure this scenario doesn't happen beforehand,
        // by implementing formatting checks for multiaddresses in the runtime,
        // or try to fetch new peer ids from the runtime at this point.
        if peer_ids.is_empty() {
            info!(target: LOG_TARGET, "No peers were found to receive file {:?}", file_metadata.fingerprint);
        }

        // Iterates and tries to send file to peer.
        // Breaks loop after first successful attempt,
        // since all peer ids belong to the same BSP.
        for peer_id in peer_ids {
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

                let upload_response = self
                    .storage_hub_handler
                    .file_transfer
                    .upload_request(peer_id, file_key.as_ref().into(), proof)
                    .await;

                match upload_response {
                    Ok(_) => {
                        debug!(target: LOG_TARGET, "Successfully uploaded chunk id {:?} of file {:?} to peer {:?}", chunk_id, file_metadata.fingerprint, peer_id);
                    }
                    Err(e) => {
                        error!(target: LOG_TARGET, "Failed to upload chunk_id {:?} to peer {:?}\n Error: {:?}", chunk_id, peer_id, e);
                        // In case of an error, we break the inner loop
                        // and try to connect to the next peer id.
                        break;
                    }
                }
            }
            info!(target: LOG_TARGET, "Successfully sent file {:?} to peer {:?}", file_metadata.fingerprint, peer_id);
            return Ok(());
        }

        // If we reach this point, it means that we couldn't send the file to any of the peers.
        return Err(anyhow::anyhow!(
            "Failed to send file {:?} to any of the peers",
            file_metadata.fingerprint
        ));
    }
}
