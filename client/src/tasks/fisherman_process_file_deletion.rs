use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_common::traits::{StorageEnableApiCollection, StorageEnableRuntimeApi};
use shc_fisherman_service::events::ProcessFileDeletionRequest;

use crate::{
    handler::StorageHubHandler,
    types::{BspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "fisherman-process-file-deletion-task";

pub struct FishermanProcessFileDeletionTask<NT, RuntimeApi>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    storage_hub_handler: StorageHubHandler<NT, RuntimeApi>,
}

impl<NT, RuntimeApi> Clone for FishermanProcessFileDeletionTask<NT, RuntimeApi>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    fn clone(&self) -> FishermanProcessFileDeletionTask<NT, RuntimeApi> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, RuntimeApi> FishermanProcessFileDeletionTask<NT, RuntimeApi>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, RuntimeApi>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<NT, RuntimeApi> EventHandler<ProcessFileDeletionRequest>
    for FishermanProcessFileDeletionTask<NT, RuntimeApi>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    async fn handle_event(&mut self, event: ProcessFileDeletionRequest) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing file deletion request for file key: {:?}",
            event.signed_file_operation_intention.file_key,
        );

        // TODO: Implement file deletion request handling (non-exhaustive):
        // 1. Fetch file metadata and identify storage providers
        // 2. Construct Bucket/BSP forest based on deletion target
        // 3. Construct proof of inclusion for file key
        // 4. Submit proof to blockchain

        Ok(())
    }
}
