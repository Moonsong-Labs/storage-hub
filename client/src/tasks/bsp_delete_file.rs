use anyhow::anyhow;
use sc_tracing::tracing::*;
use serde::{Deserialize, Serialize};
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::events::FinalisedBspConfirmStoppedStoring;
use shc_common::consts::CURRENT_FOREST_KEY;
use shc_common::task_context::TaskContext;
use shc_common::telemetry_error::TelemetryErrorCategory;
use shc_common::traits::StorageEnableRuntime;
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use shc_telemetry_service::{
    create_base_event, BaseTelemetryEvent, TelemetryEvent, TelemetryServiceCommandInterfaceExt,
};
use sp_core::H256;

use crate::{
    handler::StorageHubHandler,
    types::{BspForestStorageHandlerT, ShNodeType},
};

// Local BSP telemetry event definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BspFileDeletionStartedEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    task_name: String,
    file_key: String,
    bsp_id: String,
}

impl TelemetryEvent for BspFileDeletionStartedEvent {
    fn event_type(&self) -> &str {
        "bsp_file_deletion_started"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BspFileDeletionCompletedEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    file_key: String,
    bsp_id: String,
    duration_ms: u64,
    removed_from_forest: bool,
    removed_from_file_storage: bool,
}

impl TelemetryEvent for BspFileDeletionCompletedEvent {
    fn event_type(&self) -> &str {
        "bsp_file_deletion_completed"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BspFileDeletionFailedEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    file_key: String,
    bsp_id: String,
    error_type: String,
    error_message: String,
    duration_ms: Option<u64>,
}

impl TelemetryEvent for BspFileDeletionFailedEvent {
    fn event_type(&self) -> &str {
        "bsp_file_deletion_failed"
    }
}

const LOG_TARGET: &str = "bsp-delete-file-task";

pub struct BspDeleteFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for BspDeleteFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> BspDeleteFileTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, Runtime> BspDeleteFileTask<NT, Runtime>
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

    async fn handle_file_deletion_event(
        &mut self,
        event: FinalisedBspConfirmStoppedStoring<Runtime>,
    ) -> anyhow::Result<(bool, bool)> {
        // Check that the file_key is not in the Forest.
        let current_forest_key = CURRENT_FOREST_KEY.to_vec();
        let read_fs = self
            .storage_hub_handler
            .forest_storage_handler
            .get(&current_forest_key)
            .await
            .ok_or_else(|| anyhow!("Failed to get forest storage."))?;

        let removed_from_forest = !read_fs
            .read()
            .await
            .contains_file_key(&event.file_key.into())?;

        let mut removed_from_file_storage = false;

        if removed_from_forest {
            // If file key is not in Forest, we can now safely remove it from the File Storage.
            self.remove_file_from_file_storage(&event.file_key.into())
                .await?;
            removed_from_file_storage = true;
        } else {
            warn!(
                target: LOG_TARGET,
                "FinalisedBspConfirmStoppedStoring applied and finalised for file key {:x}, but file key is still in Forest. This can only happen if the same file key was added again after deleted by this BSP.",
                event.file_key,
            );
        }

        Ok((removed_from_forest, removed_from_file_storage))
    }

    async fn remove_file_from_file_storage(&self, file_key: &H256) -> anyhow::Result<()> {
        // Remove the file from the File Storage.
        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;
        write_file_storage.delete_file(file_key).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to remove file from File Storage after it was removed from the Forest. \nError: {:?}", e);
            anyhow!(
                "Failed to delete file from File Storage after it was removed from the Forest: {:?}",
                e
            )
        })?;

        // Release the file storage write lock.
        drop(write_file_storage);

        Ok(())
    }
}

impl<NT, Runtime> EventHandler<FinalisedBspConfirmStoppedStoring<Runtime>>
    for BspDeleteFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: FinalisedBspConfirmStoppedStoring<Runtime>,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Deleting file {:x} for BSP {:?}",
            event.file_key,
            event.bsp_id
        );

        // Create task context for tracking
        let ctx = TaskContext::new("bsp_delete_file");

        // Send task started telemetry event
        if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
            let start_event = BspFileDeletionStartedEvent {
                base: create_base_event(
                    "bsp_file_deletion_started",
                    "storage-hub-bsp".to_string(),
                    None,
                ),
                task_id: ctx.task_id.clone(),
                task_name: ctx.task_name.clone(),
                file_key: format!("{:?}", event.file_key),
                bsp_id: format!("{:?}", event.bsp_id),
            };
            telemetry_service.queue_typed_event(start_event).await.ok();
        }

        let result = self.handle_file_deletion_event(event.clone()).await;

        // Send completion or failure telemetry
        if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
            match &result {
                Ok((removed_from_forest, removed_from_file_storage)) => {
                    let completed_event = BspFileDeletionCompletedEvent {
                        base: create_base_event(
                            "bsp_file_deletion_completed",
                            "storage-hub-bsp".to_string(),
                            None,
                        ),
                        task_id: ctx.task_id.clone(),
                        file_key: format!("{:?}", event.file_key),
                        bsp_id: format!("{:?}", event.bsp_id),
                        duration_ms: ctx.elapsed_ms(),
                        removed_from_forest: *removed_from_forest,
                        removed_from_file_storage: *removed_from_file_storage,
                    };
                    telemetry_service
                        .queue_typed_event(completed_event)
                        .await
                        .ok();
                }
                Err(e) => {
                    let failed_event = BspFileDeletionFailedEvent {
                        base: create_base_event(
                            "bsp_file_deletion_failed",
                            "storage-hub-bsp".to_string(),
                            None,
                        ),
                        task_id: ctx.task_id.clone(),
                        file_key: format!("{:?}", event.file_key),
                        bsp_id: format!("{:?}", event.bsp_id),
                        error_type: e.telemetry_category().to_string(),
                        error_message: e.to_string(),
                        duration_ms: Some(ctx.elapsed_ms()),
                    };
                    telemetry_service.queue_typed_event(failed_event).await.ok();
                }
            }
        }

        result.map(|_| ())
    }
}
