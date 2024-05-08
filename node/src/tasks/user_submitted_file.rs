use crate::services::file_transfer::commands::FileTransferServiceInterface;
use crate::tasks::AcceptedBspVolunteer;
use crate::tasks::StorageHubHandler;
use crate::tasks::StorageHubHandlerConfig;
use file_manager::traits::FileStorage;
use log::{debug, error, info};

use sc_network::PeerId;

use storage_hub_infra::event_bus::EventHandler;
use storage_hub_infra::types::Metadata;

const LOG_TARGET: &str = "user-submitted-file-task";

/// Handles the events related to users submitting files to be stored.
pub struct UserSubmittedFileTask<SHC: StorageHubHandlerConfig> {
    storage_hub_handler: StorageHubHandler<SHC>,
}

impl<SHC: StorageHubHandlerConfig> Clone for UserSubmittedFileTask<SHC> {
    fn clone(&self) -> Self {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<SHC: StorageHubHandlerConfig> UserSubmittedFileTask<SHC> {
    pub fn new(storage_hub_handler: StorageHubHandler<SHC>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<SHC: StorageHubHandlerConfig> EventHandler<AcceptedBspVolunteer>
    for UserSubmittedFileTask<SHC>
{
    async fn handle_event(&self, event: AcceptedBspVolunteer) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Handling file submitted by user {:?} to BSP with location {:?}",
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
        let file_key = file_metadata.key();

        let peer_ids = event
            .multiaddresses
            .iter()
            .filter_map(|multiaddr| PeerId::try_from_multiaddr(&multiaddr))
            .collect::<Vec<PeerId>>();

        // TODO: Check how we can improve this.
        // We could either make sure this scenario doesn't happen beforehand,
        // or try to fetch new peer_ids from the runtime at this point.
        if peer_ids.is_empty() {
            info!(target: LOG_TARGET, "No peers were found to receive file {:?}", file_metadata.fingerprint);
        }

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
                    .upload_request(peer_id, file_key, proof)
                    .await;

                match upload_response {
                    Ok(_) => {
                        debug!(target: LOG_TARGET, "Successfully uploaded chunk id {:?} of file {:?} to peer {:?}", chunk_id, file_metadata.fingerprint, peer_id)
                    }
                    Err(e) => {
                        error!(target: LOG_TARGET, "Failed to upload chunk_id {:?} to peer {:?} due to {:?}", chunk_id, peer_id, e);
                        // In case of an error, we stop sending to this peer and go to the next one.
                        break;
                    }
                }
            }
            info!(target: LOG_TARGET, "Succesfully sent file {:?} to peer {:?}", file_metadata.fingerprint, peer_id);
        }

        Ok(())
    }
}
