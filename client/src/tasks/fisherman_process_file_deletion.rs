use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_common::traits::StorageEnableRuntime;
use shc_fisherman_service::events::ProcessFileDeletionRequest;

use crate::{
    handler::StorageHubHandler,
    types::{BspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "fisherman-process-file-deletion-task";

pub struct FishermanProcessFileDeletionTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for FishermanProcessFileDeletionTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> FishermanProcessFileDeletionTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, Runtime> FishermanProcessFileDeletionTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<NT, Runtime> EventHandler<ProcessFileDeletionRequest<Runtime>>
    for FishermanProcessFileDeletionTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: ProcessFileDeletionRequest<Runtime>,
    ) -> anyhow::Result<()> {
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
