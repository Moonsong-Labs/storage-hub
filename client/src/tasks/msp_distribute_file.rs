use anyhow::anyhow;
use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface, events::DistributeFileToBsp,
};
use shc_common::{traits::StorageEnableRuntime, types::StorageProviderId};
use sp_core::Get;

use crate::{
    handler::StorageHubHandler,
    types::{MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "msp-distribute-file-task";

pub struct MspDistributeFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for MspDistributeFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> MspDistributeFileTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, Runtime> MspDistributeFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler: storage_hub_handler.clone(),
        }
    }
}

/// Handles the [`DistributeFileToBsp`] event.
///
/// TODO: Document this
impl<NT, Runtime> EventHandler<DistributeFileToBsp<Runtime>> for MspDistributeFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, _event: DistributeFileToBsp<Runtime>) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Distributing file to BSP",
        );

        todo!("HELLO THERE. Implement this.");
    }
}
