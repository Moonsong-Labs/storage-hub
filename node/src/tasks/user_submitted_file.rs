const LOG_TARGET: &str = "user-submitted-file-task";

pub struct UserSubmittedFile<SHC: StorageHubHandlerConfig> {
    storage_hub_handler: StorageHubHandler<SHC>,
}

impl<SHC: StorageHubHandlerConfig> Clone for UserSubmittedFile<SHC> {
    fn clone(&self) -> Self {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone()
        }
    }
}

impl<SHC: StorageHubHandlerConfig> UserSubmittedFile<SHC> {
    pub fn new(storage_hub_handler: StorageHubHandler<SHC>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<SHC: StorageHubHandlerConfig> EventHandler<AcceptedBspVolunteer> for UserSubmittedFile<SHC> {
    async fn handle_event(&self, event: AcceptedBspVolunteer) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Handling file submitted by user to BSP {:?} with location {:?}",
            event.who,
            event.location,
        );

        let multiaddresses = event.multiaddresses;

        self._storage_hub_handler.file_transfer.establish_connection();

        // Command for file transfer service
        // Open P2P
        // Send file
    }
}