// TODO: Remove this once we don't need the examples in this file
#![allow(dead_code)]

pub mod bsp_upload_file_task;
pub mod bsp_volunteer_mock;
pub mod user_sends_file;

use sc_tracing::tracing::info;
use storage_hub_infra::event_bus::EventHandler;

use crate::services::blockchain::events::{AcceptedBspVolunteer, NewStorageRequest};
use crate::services::file_transfer::events::RemoteUploadRequest;
use crate::services::handler::{StorageHubHandler, StorageHubHandlerConfig};

// ! The following are examples of task definitions.
pub struct ResolveRemoteUploadRequest<SHC: StorageHubHandlerConfig> {
    _storage_hub_handler: StorageHubHandler<SHC>,
}

impl<SHC: StorageHubHandlerConfig> Clone for ResolveRemoteUploadRequest<SHC> {
    fn clone(&self) -> ResolveRemoteUploadRequest<SHC> {
        Self {
            _storage_hub_handler: self._storage_hub_handler.clone(),
        }
    }
}

impl<SHC: StorageHubHandlerConfig> ResolveRemoteUploadRequest<SHC> {
    pub fn new(storage_hub_handler: StorageHubHandler<SHC>) -> Self {
        Self {
            _storage_hub_handler: storage_hub_handler,
        }
    }
}

impl<SHC> EventHandler<RemoteUploadRequest> for ResolveRemoteUploadRequest<SHC>
where
    SHC: StorageHubHandlerConfig,
{
    async fn handle_event(&self, event: RemoteUploadRequest) -> anyhow::Result<()> {
        info!(
            "[ResolveRemoteUploadRequest] - file location: {:?}",
            event.file_key
        );

        // self.storage_hub_handler.storage.store_chunk().await?;

        Ok(())
    }
}

pub struct NewStorageRequestHandler<SHC: StorageHubHandlerConfig> {
    _storage_hub_handler: StorageHubHandler<SHC>,
}

impl<SHC: StorageHubHandlerConfig> NewStorageRequestHandler<SHC> {
    pub fn new(storage_hub_handler: StorageHubHandler<SHC>) -> Self {
        Self {
            _storage_hub_handler: storage_hub_handler,
        }
    }
}

impl<SHC: StorageHubHandlerConfig> Clone for NewStorageRequestHandler<SHC> {
    fn clone(&self) -> NewStorageRequestHandler<SHC> {
        Self {
            _storage_hub_handler: self._storage_hub_handler.clone(),
        }
    }
}

impl<SHC: StorageHubHandlerConfig> EventHandler<NewStorageRequest>
    for NewStorageRequestHandler<SHC>
{
    async fn handle_event(&self, event: NewStorageRequest) -> anyhow::Result<()> {
        info!("[NewStorageRequestHandler] - received event: {:?}", event);

        // TODO: implement

        Ok(())
    }
}

pub struct AcceptedBspVolunteerHandler<SHC: StorageHubHandlerConfig> {
    _storage_hub_handler: StorageHubHandler<SHC>,
}

impl<SHC: StorageHubHandlerConfig> Clone for AcceptedBspVolunteerHandler<SHC> {
    fn clone(&self) -> AcceptedBspVolunteerHandler<SHC> {
        Self {
            _storage_hub_handler: self._storage_hub_handler.clone(),
        }
    }
}

impl<SHC: StorageHubHandlerConfig> AcceptedBspVolunteerHandler<SHC> {
    pub fn new(storage_hub_handler: StorageHubHandler<SHC>) -> Self {
        Self {
            _storage_hub_handler: storage_hub_handler,
        }
    }
}

impl<SHC: StorageHubHandlerConfig> EventHandler<AcceptedBspVolunteer>
    for AcceptedBspVolunteerHandler<SHC>
{
    async fn handle_event(&self, event: AcceptedBspVolunteer) -> anyhow::Result<()> {
        info!("[NewStorageRequestHandler] - received event: {:?}", event);

        // TODO: implement

        Ok(())
    }
}
