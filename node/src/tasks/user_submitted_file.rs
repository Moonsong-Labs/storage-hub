use crate::services::file_transfer::commands::FileTransferServiceInterface;
use crate::tasks::AcceptedBspVolunteer;
use crate::tasks::StorageHubHandler;
use crate::tasks::StorageHubHandlerConfig;
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
            "Handling file submitted by user to BSP {:?} with location {:?}",
            event.who,
            event.location,
        );

        let multiaddresses = event.multiaddresses;

        let _ = self.storage_hub_handler
            .file_transfer
            .establish_connection(multiaddresses);

        // Command for file transfer service
        // Open P2P
        // Send file

        Ok(())
    }
}
