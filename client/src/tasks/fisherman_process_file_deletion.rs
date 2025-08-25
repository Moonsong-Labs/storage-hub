use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::events::FileDeletionRequest;
use shc_common::traits::StorageEnableRuntime;
use shc_telemetry_service::{
    create_base_event, BaseTelemetryEvent, TelemetryEvent, TelemetryServiceCommandInterfaceExt,
};
use shc_common::task_context::{TaskContext, classify_error};
use serde::{Deserialize, Serialize};

// Local Fisherman telemetry event definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FishermanDeletionRequestReceivedEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    task_name: String,
    file_key: String,
    user: String,
    bucket_id: String,
}

impl TelemetryEvent for FishermanDeletionRequestReceivedEvent {
    fn event_type(&self) -> &str {
        "fisherman_deletion_request_received"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FishermanVerificationCompletedEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    file_key: String,
    verification_result: String,
    duration_ms: u64,
    storage_providers_checked: u32,
}

impl TelemetryEvent for FishermanVerificationCompletedEvent {
    fn event_type(&self) -> &str {
        "fisherman_verification_completed"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FishermanDeletionProcessedEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    file_key: String,
    deletion_result: String,
    duration_ms: u64,
    error_type: Option<String>,
    error_message: Option<String>,
}

impl TelemetryEvent for FishermanDeletionProcessedEvent {
    fn event_type(&self) -> &str {
        "fisherman_deletion_processed"
    }
}

use crate::{
    handler::StorageHubHandler,
    types::{FishermanForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "fisherman-process-file-deletion-task";

pub struct FishermanProcessFileDeletionTask<NT, Runtime>
where
    NT: ShNodeType,
    NT::FSH: FishermanForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for FishermanProcessFileDeletionTask<NT, Runtime>
where
    NT: ShNodeType,
    NT::FSH: FishermanForestStorageHandlerT,
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
    NT: ShNodeType,
    NT::FSH: FishermanForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<NT, Runtime> EventHandler<FileDeletionRequest>
    for FishermanProcessFileDeletionTask<NT, Runtime>
where
    NT: ShNodeType + 'static,
    NT::FSH: FishermanForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: FileDeletionRequest) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing file deletion request for file key: {:?}",
            event.file_key,
        );

        // Create task context for tracking
        let ctx = TaskContext::new("fisherman_process_file_deletion");

        // Send deletion request received telemetry event
        if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
            let deletion_request_event = FishermanDeletionRequestReceivedEvent {
                base: create_base_event("fisherman_deletion_request_received", "storage-hub-fisherman".to_string(), None),
                task_id: ctx.task_id.clone(),
                task_name: ctx.task_name.clone(),
                file_key: format!("{:?}", event.file_key),
                user: format!("{:?}", event.user),
                bucket_id: format!("{:?}", event.bucket_id),
            };
            telemetry_service.queue_typed_event(deletion_request_event).await.ok();
        }

        let result = self.process_file_deletion(&event, &ctx).await;

        // Send completion telemetry
        if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
            let deletion_processed_event = match &result {
                Ok(_) => FishermanDeletionProcessedEvent {
                    base: create_base_event("fisherman_deletion_processed", "storage-hub-fisherman".to_string(), None),
                    task_id: ctx.task_id.clone(),
                    file_key: format!("{:?}", event.file_key),
                    deletion_result: "success".to_string(),
                    duration_ms: ctx.elapsed_ms(),
                    error_type: None,
                    error_message: None,
                },
                Err(e) => FishermanDeletionProcessedEvent {
                    base: create_base_event("fisherman_deletion_processed", "storage-hub-fisherman".to_string(), None),
                    task_id: ctx.task_id.clone(),
                    file_key: format!("{:?}", event.file_key),
                    deletion_result: "failure".to_string(),
                    duration_ms: ctx.elapsed_ms(),
                    error_type: Some(classify_error(&e)),
                    error_message: Some(e.to_string()),
                },
            };
            telemetry_service.queue_typed_event(deletion_processed_event).await.ok();
        }

        result
    }
}

impl<NT, Runtime> FishermanProcessFileDeletionTask<NT, Runtime>
where
    NT: ShNodeType,
    NT::FSH: FishermanForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    async fn process_file_deletion(
        &mut self,
        event: &FileDeletionRequest,
        ctx: &TaskContext,
    ) -> anyhow::Result<()> {
        // Start verification process
        info!(
            target: LOG_TARGET,
            "Starting file deletion verification for file key: {:?}",
            event.file_key
        );

        let verification_start_time = std::time::Instant::now();

        // TODO: Implement file deletion request handling (non-exhaustive):
        // 1. Fetch file metadata and identify storage providers
        let storage_providers_checked = 0u32; // Placeholder for actual implementation
        
        // 2. Construct Bucket/BSP forest based on deletion target
        // 3. Construct proof of inclusion for file key
        // 4. Submit proof to blockchain

        let verification_duration = verification_start_time.elapsed();
        let verification_result = "pending_implementation".to_string();

        // Send verification completed telemetry event
        if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
            let verification_event = FishermanVerificationCompletedEvent {
                base: create_base_event("fisherman_verification_completed", "storage-hub-fisherman".to_string(), None),
                task_id: ctx.task_id.clone(),
                file_key: format!("{:?}", event.file_key),
                verification_result: verification_result.clone(),
                duration_ms: verification_duration.as_millis() as u64,
                storage_providers_checked,
            };
            telemetry_service.queue_typed_event(verification_event).await.ok();
        }

        Ok(())
    }
}
