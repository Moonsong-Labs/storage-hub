use crate::services::file_transfer::commands::FileTransferServiceInterface;
use crate::tasks::AcceptedBspVolunteer;
use crate::tasks::StorageHubHandler;
use crate::tasks::StorageHubHandlerConfig;

use log::info;

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
            event.file_metadata.location,
        );

        let _multiaddresses = event.multiaddresses;
        let _peer_id = event.peer_id;
        let _file_location = event.file_metadata.location;
        // let chunk_count = event.file_metadata.chunk_count();
        // Mocked count:
        let chunk_count = 100u64;

        for _chunk_idx in 0..chunk_count {
            // Depends on FileStorage trait implementation
            // let chunk = self.storage_hub_handler.file_storage.get_chunk();
            let chunk = "Mocked Data".to_string();
            let _ = self.storage_hub_handler.file_transfer.upload_request(chunk);
        }

        Ok(())
    }
}
