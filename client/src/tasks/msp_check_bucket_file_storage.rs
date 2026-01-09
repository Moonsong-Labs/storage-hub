use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::events::CheckBucketFileStorage;
use shc_common::traits::StorageEnableRuntime;

use crate::{
    handler::StorageHubHandler,
    types::{MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "msp-check-bucket-file-storage-task";

/// MSP task that handles [`CheckBucketFileStorage`] events.
///
/// This is boilerplate wiring only. Behaviour will be implemented separately.
pub struct MspCheckBucketFileStorageTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for MspCheckBucketFileStorageTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> Self {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, Runtime> MspCheckBucketFileStorageTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<NT, Runtime> EventHandler<CheckBucketFileStorage<Runtime>>
    for MspCheckBucketFileStorageTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: CheckBucketFileStorage<Runtime>,
    ) -> anyhow::Result<String> {
        // Placeholder: behaviour will be implemented separately.
        info!(
            target: LOG_TARGET,
            "Received CheckBucketFileStorage for bucket [0x{:x}]",
            event.bucket_id
        );

        // Keep `storage_hub_handler` referenced to avoid unused-field warnings
        // in case logging is compiled out.
        let _ = &self.storage_hub_handler;

        Ok(format!(
            "Handled CheckBucketFileStorage for bucket [0x{:x}]",
            event.bucket_id
        ))
    }
}
