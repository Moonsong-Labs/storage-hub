use std::str::FromStr;
use crate::services::file_transfer::commands::FileTransferServiceInterface;
use file_manager::traits::FileStorage;
use crate::tasks::AcceptedBspVolunteer;
use crate::tasks::StorageHubHandler;
use crate::tasks::StorageHubHandlerConfig;
use log::{error, info};
use sc_network::Multiaddr;
use sc_network::PeerId;
use sp_core::Blake2Hasher;
use sp_core::Hasher;
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

        // TODO: use `Multiaddr` instead of `String`.
        let multiaddresses = event.multiaddresses;
        let file_metadata = Metadata { 
            owner: event.owner.to_string(), 
            size: event.size.into(), 
            // TODO: use `FileLocation` instead of `String`.
            location: format!("{:?}", event.location.into_inner()), 
            fingerprint: event.fingerprint, 
        };
        let chunk_count = file_metadata.chunk_count();
        
        // TODO(Arthur): double check this, I'm assuming Blake2 as the Trie hash function.
        let file_key = Blake2Hasher::hash(&serde_json::to_vec(&file_metadata)?.to_owned());

        let peer_ids = multiaddresses.iter().map(|multiaddr| {
            let multiaddr_str = multiaddr.split("/").last().expect("Multiaddress of Bsp should not be empty.");
            let multiaddr = Multiaddr::from_str(multiaddr_str).expect("Failed to convert into Multiaddr");
            PeerId::try_from_multiaddr(&multiaddr).expect("Multiaddr without PeerId")
        }).collect::<Vec<PeerId>>();

        for peer_id in peer_ids {
            // Acquire the write lock for the total duration of the file transmission.
            let file_storage_lock = self.storage_hub_handler.file_storage.write().await;

            for chunk_id in 0..chunk_count {    
                let proof = file_storage_lock.generate_proof(&file_key, &chunk_id).expect("File is not in storage, or proof does not exist.");
                let upload_response = self.storage_hub_handler.file_transfer.upload_request(peer_id, file_key, proof).await;
    
                match upload_response {
                    Ok(_) => {
                        info!(target: LOG_TARGET, "Successfully uploaded chunk id {:?} to peer {:?}", chunk_id, peer_id)
                    },
                    Err(e) => {
                        error!(target: LOG_TARGET, "Failed to upload chunk_id {:?} to peer {:?}", chunk_id, peer_id);
                        return Err(anyhow::anyhow!("{:?}", e));
                    }
                }
            }
        }

        Ok(())
    }
}
