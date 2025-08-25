use std::time::Instant;

use anyhow::anyhow;
use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::events::{FinalisedBucketMovedAway, FinalisedMspStoppedStoringBucket};
use shc_common::task_context::{classify_error, TaskContext};
use shc_common::traits::StorageEnableRuntime;
use shc_common::types::BucketId;
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::ForestStorageHandler;
use shc_telemetry_service::{
    create_base_event, BaseTelemetryEvent, TelemetryEvent, TelemetryServiceCommandInterfaceExt,
};
use serde::{Deserialize, Serialize};

use crate::{
    handler::StorageHubHandler,
    types::{MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "msp-stopped-storing-task";

// Local MSP telemetry event definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MspBucketDeletionStartedEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    task_name: String,
    bucket_id: String,
    trigger: String, // "bucket_moved" or "stopped_storing"
}

impl TelemetryEvent for MspBucketDeletionStartedEvent {
    fn event_type(&self) -> &str {
        "msp_bucket_deletion_started"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MspBucketDeletionCompletedEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    bucket_id: String,
    trigger: String,
    duration_ms: u64,
}

impl TelemetryEvent for MspBucketDeletionCompletedEvent {
    fn event_type(&self) -> &str {
        "msp_bucket_deletion_completed"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MspBucketDeletionFailedEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    bucket_id: String,
    trigger: String,
    error_type: String,
    error_message: String,
    duration_ms: Option<u64>,
}

impl TelemetryEvent for MspBucketDeletionFailedEvent {
    fn event_type(&self) -> &str {
        "msp_bucket_deletion_failed"
    }
}

/// Task that handles bucket deletion for an MSP in two scenarios:
/// 1. When a bucket is moved away to another MSP ([`BucketMovedAway`])
/// 2. When the MSP stops storing a bucket ([`FinalisedMspStoppedStoringBucket`])
///
/// The task will:
/// 1. Delete all files with the bucket prefix from [`FileStorage`]
/// 2. Remove the bucket's [`ForestStorageHandler`] instance
///
/// # Note
/// The cleanup happens immediately after the events are confirmed in a finalized block.
///
/// [`FileStorage`]: shc_file_manager::traits::FileStorage
/// [`ForestStorageHandler`]: shc_forest_manager::traits::ForestStorageHandler
pub struct MspDeleteBucketTask<NT, Runtime>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for MspDeleteBucketTask<NT, Runtime>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> MspDeleteBucketTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, Runtime> MspDeleteBucketTask<NT, Runtime>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<NT, Runtime> EventHandler<FinalisedBucketMovedAway> for MspDeleteBucketTask<NT, Runtime>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: FinalisedBucketMovedAway) -> anyhow::Result<()> {
        let start_time = Instant::now();
        let ctx = TaskContext::new("msp_delete_bucket");
        
        info!(
            target: LOG_TARGET,
            "MSP: bucket {:?} moved to MSP {:?}, starting cleanup",
            event.bucket_id,
            event.new_msp_id,
        );

        // Send telemetry event for bucket deletion started
        if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
            let start_event = MspBucketDeletionStartedEvent {
                base: create_base_event("msp_bucket_deletion_started", "storage-hub-msp".to_string(), None),
                task_id: ctx.task_id.clone(),
                task_name: "msp_delete_bucket".to_string(),
                bucket_id: event.bucket_id.to_string(),
                trigger: "bucket_moved".to_string(),
            };
            telemetry_service.queue_typed_event(start_event).await.ok();
        }

        match self.delete_bucket(&event.bucket_id).await {
            Ok(()) => {
                info!(
                    target: LOG_TARGET,
                    "MSP: successfully deleted bucket {:?} after move",
                    event.bucket_id,
                );

                // Send telemetry event for bucket deletion completed
                if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
                    let completed_event = MspBucketDeletionCompletedEvent {
                        base: create_base_event("msp_bucket_deletion_completed", "storage-hub-msp".to_string(), None),
                        task_id: ctx.task_id.clone(),
                        bucket_id: event.bucket_id.to_string(),
                        trigger: "bucket_moved".to_string(),
                        duration_ms: start_time.elapsed().as_millis() as u64,
                    };
                    telemetry_service.queue_typed_event(completed_event).await.ok();
                }

                Ok(())
            }
            Err(e) => {
                error!(
                    target: LOG_TARGET,
                    "Failed to delete bucket {:?} after move: {:?}",
                    event.bucket_id,
                    e
                );

                // Send telemetry event for bucket deletion failed
                if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
                    let error_type = classify_error(&e);
                    let error_message = e.to_string();
                    let failed_event = MspBucketDeletionFailedEvent {
                        base: create_base_event("msp_bucket_deletion_failed", "storage-hub-msp".to_string(), None),
                        task_id: ctx.task_id.clone(),
                        bucket_id: event.bucket_id.to_string(),
                        trigger: "bucket_moved".to_string(),
                        error_type,
                        error_message,
                        duration_ms: Some(start_time.elapsed().as_millis() as u64),
                    };
                    telemetry_service.queue_typed_event(failed_event).await.ok();
                }

                Err(e)
            }
        }
    }
}

impl<NT, Runtime> EventHandler<FinalisedMspStoppedStoringBucket>
    for MspDeleteBucketTask<NT, Runtime>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: FinalisedMspStoppedStoringBucket,
    ) -> anyhow::Result<()> {
        let start_time = Instant::now();
        let ctx = TaskContext::new("msp_delete_bucket");
        
        info!(
            target: LOG_TARGET,
            "MSP: deleting bucket {:?} for MSP {:?}",
            event.bucket_id,
            event.msp_id
        );

        // Send telemetry event for bucket deletion started
        if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
            let start_event = MspBucketDeletionStartedEvent {
                base: create_base_event("msp_bucket_deletion_started", "storage-hub-msp".to_string(), None),
                task_id: ctx.task_id.clone(),
                task_name: "msp_delete_bucket".to_string(),
                bucket_id: event.bucket_id.to_string(),
                trigger: "stopped_storing".to_string(),
            };
            telemetry_service.queue_typed_event(start_event).await.ok();
        }

        match self.delete_bucket(&event.bucket_id).await {
            Ok(()) => {
                info!(
                    target: LOG_TARGET,
                    "MSP: successfully deleted bucket {:?} after stop storing",
                    event.bucket_id,
                );

                // Send telemetry event for bucket deletion completed
                if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
                    let completed_event = MspBucketDeletionCompletedEvent {
                        base: create_base_event("msp_bucket_deletion_completed", "storage-hub-msp".to_string(), None),
                        task_id: ctx.task_id.clone(),
                        bucket_id: event.bucket_id.to_string(),
                        trigger: "stopped_storing".to_string(),
                        duration_ms: start_time.elapsed().as_millis() as u64,
                    };
                    telemetry_service.queue_typed_event(completed_event).await.ok();
                }

                Ok(())
            }
            Err(e) => {
                error!(
                    target: LOG_TARGET,
                    "Failed to delete bucket {:?} after stop storing: {:?}",
                    event.bucket_id,
                    e
                );

                // Send telemetry event for bucket deletion failed
                if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
                    let error_type = classify_error(&e);
                    let error_message = e.to_string();
                    let failed_event = MspBucketDeletionFailedEvent {
                        base: create_base_event("msp_bucket_deletion_failed", "storage-hub-msp".to_string(), None),
                        task_id: ctx.task_id.clone(),
                        bucket_id: event.bucket_id.to_string(),
                        trigger: "stopped_storing".to_string(),
                        error_type,
                        error_message,
                        duration_ms: Some(start_time.elapsed().as_millis() as u64),
                    };
                    telemetry_service.queue_typed_event(failed_event).await.ok();
                }

                Err(e)
            }
        }
    }
}

impl<NT, Runtime> MspDeleteBucketTask<NT, Runtime>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    /// Deletes all files in a bucket and removes the bucket's forest storage
    async fn delete_bucket(&mut self, bucket_id: &BucketId) -> anyhow::Result<()> {
        self.storage_hub_handler
            .file_storage
            .write()
            .await
            .delete_files_with_prefix(
                &bucket_id
                    .as_ref()
                    .try_into()
                    .map_err(|_| anyhow!("Invalid bucket id"))?,
            )
            .map_err(|e| anyhow!("Failed to delete files with prefix: {:?}", e))?;

        self.storage_hub_handler
            .forest_storage_handler
            .remove_forest_storage(&bucket_id.as_ref().to_vec())
            .await;

        Ok(())
    }
}
