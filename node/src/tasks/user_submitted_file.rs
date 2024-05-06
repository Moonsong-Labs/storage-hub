use std::str::FromStr;
use crate::services::file_transfer::commands::FileTransferServiceInterface;
use crate::tasks::AcceptedBspVolunteer;
use crate::tasks::StorageHubHandler;
use crate::tasks::StorageHubHandlerConfig;
use log::info;
use sc_network::Multiaddr;
use sc_network::PeerId;
use storage_hub_infra::event_bus::EventHandler;

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
            "Handling file submitted by user to BSP with location {:?}",
            event.location,
        );

        let multiaddresses = event.multiaddresses;
        let _file_location = event.location;
        let peer_ids = multiaddresses.iter().map(|multiaddr| {
            let multiaddr_str = multiaddr.split("/").last().expect("Multiaddress of Bsp should not be empty.");
            // TODO: this won’t be necessary once we switch from `String` to `Multiaddr`.
            let multiaddr = Multiaddr::from_str(multiaddr_str).expect("Failed to convert into Multiaddr");
            PeerId::try_from_multiaddr(&multiaddr).expect("Multiaddr without PeerId")
        });

        // let chunk = self.storage_hub_handler.file_storage.get_chunk();
        // get chunk and proof.
        // just iterate over every single chunk and produce proof.
        //  query the pallet storage to get metadata
        // need a function to query blockchain storage (should be in the blockchain service).
        // let _ = self
        //     .storage_hub_handler
        //     .file_transfer
        //     .upload_request(peer_id, chunk.into());
        Ok(())
    }
}
