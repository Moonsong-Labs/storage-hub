use sc_tracing::tracing::info;
use storage_hub_infra::event_bus::EventHandler;

use crate::services::file_transfer::events::RemoteUploadRequest;
use crate::services::StorageHubHandler;

#[derive(Clone)]
pub struct ResolveRemoteUploadRequest {
    _storage_hub_handler: StorageHubHandler,
}

impl ResolveRemoteUploadRequest {
    pub fn new(storage_hub_handler: StorageHubHandler) -> Self {
        Self {
            _storage_hub_handler: storage_hub_handler,
        }
    }
}

impl EventHandler<RemoteUploadRequest> for ResolveRemoteUploadRequest {
    async fn handle_event(&self, event: RemoteUploadRequest) -> anyhow::Result<()> {
        info!(
            "[ResolveRemoteUploadRequest] - file location: {}",
            event.location
        );

        // self.storage_hub_handler.storage.store_chunk().await?;

        Ok(())
    }
}
