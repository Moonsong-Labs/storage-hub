use crate::services::file_transfer::commands::FileTransferServiceInterface;
use crate::tasks::AcceptedBspVolunteer;
use crate::tasks::StorageHubHandler;
use crate::tasks::StorageHubHandlerConfig;
use crate::services::file_transfer::schema;
use log::info;
use storage_hub_infra::event_bus::EventHandler;

const LOG_TARGET: &str = "user-submitted-file-task";

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
            event.file_metadata.location,
        );

        let multiaddresses = event.multiaddresses;
        let peer_id = event.peer_id;

        let payload = "Mocked Data".to_string();
        let request = schema::v1::provider::Request::decode(&payload[..])?;

        // let _ = self.storage_hub_handler.file_transfer.send_request(peer_id, protocol_name, request);

        // let _ = self.storage_hub_handler
        //     .file_transfer
        //     .establish_connection(multiaddresses);

        // iterate through all chunks of this file in FileStorage.

        // OK - I need add the Metadata struct to runtime events.
        // - add peer_id as requirement as well and send_request
        // - get RequestReponseBehavior? substrate has a send_request()
        // - we need the file storage client to get proofs, send file etc
        // By the time we are here, we are guaranteed to have the file in File Storage.
        // FileStorage trait is not implemented yet and will be changed.

        
        // Command for file transfer service
        // Open P2P
        // Send file

        Ok(())
    }
}
