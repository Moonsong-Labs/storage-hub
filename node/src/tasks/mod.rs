pub mod bsp_volunteer_mock;

use sc_tracing::tracing::info;
use storage_hub_infra::event_bus::EventHandler;

use crate::services::blockchain::events::{AcceptedBspVolunteer, NewStorageRequest};
use crate::services::file_transfer::events::RemoteUploadRequest;
use crate::services::StorageHubHandler;

// ! The following are examples of task definitions.
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

#[derive(Clone)]
pub struct NewStorageRequestHandler {
    _storage_hub_handler: StorageHubHandler,
}

impl NewStorageRequestHandler {
    pub fn new(storage_hub_handler: StorageHubHandler) -> Self {
        Self {
            _storage_hub_handler: storage_hub_handler,
        }
    }
}

impl EventHandler<NewStorageRequest> for NewStorageRequestHandler {
    async fn handle_event(&self, event: NewStorageRequest) -> anyhow::Result<()> {
        info!("[NewStorageRequestHandler] - received event: {:?}", event);

        // TODO: implement

        Ok(())
    }
}

#[derive(Clone)]
pub struct AcceptedBspVolunteerHandler {
    _storage_hub_handler: StorageHubHandler,
}

impl AcceptedBspVolunteerHandler {
    pub fn new(storage_hub_handler: StorageHubHandler) -> Self {
        Self {
            _storage_hub_handler: storage_hub_handler,
        }
    }
}

impl EventHandler<AcceptedBspVolunteer> for AcceptedBspVolunteerHandler {
    async fn handle_event(&self, event: AcceptedBspVolunteer) -> anyhow::Result<()> {
        info!("[NewStorageRequestHandler] - received event: {:?}", event);

        // TODO: implement

        Ok(())
    }
}
