use crate::services::file_transfer::commands::FileTransferServiceInterface;
use crate::tasks::AcceptedBspVolunteer;
use crate::tasks::StorageHubHandler;
use crate::tasks::StorageHubHandlerConfig;
use file_manager::traits::FileStorage;
use log::{debug, error, info};
use shc_common::types::Metadata;

use sc_network::PeerId;

use sp_trie::TrieLayout;
use storage_hub_infra::event_bus::EventHandler;

const LOG_TARGET: &str = "user-sends-file-task";

/// Handles the events related to users sending a file to be stored by BSPs
/// volunteering for that file.
/// It can serve multiple BSPs volunteering to store each file, since
/// it reacts to every `AcceptedBspVolunteer` from the runtime.
pub struct UserSendsFileTask<SHC: StorageHubHandlerConfig> {
    storage_hub_handler: StorageHubHandler<SHC>,
}

impl<SHC: StorageHubHandlerConfig> Clone for UserSendsFileTask<SHC> {
    fn clone(&self) -> Self {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<SHC: StorageHubHandlerConfig> UserSendsFileTask<SHC> {
    pub fn new(storage_hub_handler: StorageHubHandler<SHC>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<SHC: StorageHubHandlerConfig> EventHandler<AcceptedBspVolunteer> for UserSendsFileTask<SHC> {
    /// Reacts to BSPs volunteering (`AcceptedBspVolunteer` from the runtime) to store the user's file,
    /// establishes a connection to each BSPs through the p2p network and sends the file.
    /// At this point we assume that the file is merkleised and already in file storage, and
    /// for this reason the file transfer to the BSP should not fail unless the p2p connection fails.
    async fn handle_event(&self, event: AcceptedBspVolunteer) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Handling BSP volunteering to store a file from user [{:?}], with location [{:?}]",
            event.owner,
            event.location,
        );

        let file_metadata = Metadata {
            owner: event.owner.to_string(),
            size: event.size.into(),
            location: event.location.into_inner(),
            fingerprint: event.fingerprint,
        };

        let chunk_count = file_metadata.chunk_count();
        let file_key = file_metadata.key::<<SHC::TrieLayout as TrieLayout>::Hash>();

        let peer_ids = event
            .multiaddresses
            .iter()
            .filter_map(|multiaddr| PeerId::try_from_multiaddr(&multiaddr))
            .collect::<Vec<PeerId>>();

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
                let proof = self
                    .storage_hub_handler
                    .file_storage
                    .read()
                    .await
                    .generate_proof(&file_key, &chunk_id)
                    .expect("File is not in storage, or proof does not exist.");

                let upload_response = self
                    .storage_hub_handler
                    .file_transfer
                    .upload_request(peer_id, file_key.as_ref().to_vec(), proof)
                    .await;

                match upload_response {
                    Ok(_) => {
                        debug!(target: LOG_TARGET, "Successfully uploaded chunk id {:?} of file {:?} to peer {:?}", chunk_id, file_metadata.fingerprint, peer_id)
                    }
                    Err(e) => {
                        error!(target: LOG_TARGET, "Failed to upload chunk_id {:?} to peer {:?} due to {:?}", chunk_id, peer_id, e);
                        // In case of an error, we break the inner loop
                        // and try to connect to the next peer id.
                        break;
                    }
                }
            }
            info!(target: LOG_TARGET, "Successfully sent file {:?} to peer {:?}", file_metadata.fingerprint, peer_id);
            break;
        }

        Ok(())
    }
}
