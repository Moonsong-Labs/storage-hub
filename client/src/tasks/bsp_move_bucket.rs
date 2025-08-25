use anyhow::anyhow;
use sc_tracing::tracing::*;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface,
    events::{MoveBucketAccepted, MoveBucketExpired, MoveBucketRejected, MoveBucketRequested},
};
use shc_common::{
    task_context::{classify_error, TaskContext},
    traits::StorageEnableRuntime,
};
use shc_file_transfer_service::commands::{
    FileTransferServiceCommandInterface, FileTransferServiceCommandInterfaceExt,
};
use shc_telemetry_service::{
    create_base_event, BaseTelemetryEvent, TelemetryEvent, TelemetryServiceCommandInterfaceExt,
};
use serde::{Deserialize, Serialize};

// Local BSP bucket move telemetry event definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BspBucketMoveStartedEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    task_name: String,
    bucket_id: String,
    event_type: String,
    new_msp_id: Option<String>,
    old_msp_id: Option<String>,
}

impl TelemetryEvent for BspBucketMoveStartedEvent {
    fn event_type(&self) -> &str {
        "bsp_bucket_move_started"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BspBucketMoveProgressEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    bucket_id: String,
    progress_type: String,
    new_msp_id: Option<String>,
    old_msp_id: Option<String>,
    grace_period_seconds: Option<u64>,
}

impl TelemetryEvent for BspBucketMoveProgressEvent {
    fn event_type(&self) -> &str {
        "bsp_bucket_move_progress"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BspBucketMoveCompletedEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    bucket_id: String,
    completion_type: String,
    duration_ms: u64,
    new_msp_id: Option<String>,
    old_msp_id: Option<String>,
}

impl TelemetryEvent for BspBucketMoveCompletedEvent {
    fn event_type(&self) -> &str {
        "bsp_bucket_move_completed"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BspBucketMoveFailedEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    bucket_id: String,
    error_type: String,
    error_message: String,
    duration_ms: Option<u64>,
    event_context: String,
    new_msp_id: Option<String>,
    old_msp_id: Option<String>,
}

impl TelemetryEvent for BspBucketMoveFailedEvent {
    fn event_type(&self) -> &str {
        "bsp_bucket_move_failed"
    }
}

use crate::{
    handler::StorageHubHandler,
    types::{BspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "bsp-move-bucket-task";

/// Configuration for the BspMoveBucketTask
#[derive(Debug, Clone)]
pub struct BspMoveBucketConfig {
    /// Grace period in seconds to accept download requests after a bucket move is accepted
    pub move_bucket_accepted_grace_period: u64,
}

impl Default for BspMoveBucketConfig {
    fn default() -> Self {
        Self {
            move_bucket_accepted_grace_period: 4 * 60 * 60, // 4 hours - Default value that was in command.rs
        }
    }
}

/// Task that handles the [`MoveBucketRequested`], [`MoveBucketAccepted`], [`MoveBucketRejected`]
/// and [`MoveBucketExpired`] events from the BSP point of view.
pub struct BspMoveBucketTask<NT, Runtime>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
    /// Configuration for this task
    config: BspMoveBucketConfig,
}

impl<NT, Runtime> Clone for BspMoveBucketTask<NT, Runtime>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> BspMoveBucketTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            config: self.config.clone(),
        }
    }
}

impl<NT, Runtime> BspMoveBucketTask<NT, Runtime>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler: storage_hub_handler.clone(),
            config: storage_hub_handler.provider_config.bsp_move_bucket.clone(),
        }
    }
}

/// Handles the [`MoveBucketRequested`] event.
///
/// This event is triggered when an user requests to move a bucket to a new MSP.
/// As a BSP, we need to allow the new MSP to download the files we have from the bucket.
impl<NT, Runtime> EventHandler<MoveBucketRequested> for BspMoveBucketTask<NT, Runtime>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: MoveBucketRequested) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "MoveBucketRequested: BSP will accept download requests for files in bucket {:?} from MSP {:?}",
            event.bucket_id,
            event.new_msp_id
        );

        // Create task context for tracking
        let ctx = TaskContext::new("bsp_move_bucket");

        // Send task started telemetry event
        if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
            let start_event = BspBucketMoveStartedEvent {
                base: create_base_event("bsp_bucket_move_started", "storage-hub-bsp".to_string(), None),
                task_id: ctx.task_id.clone(),
                task_name: ctx.task_name.clone(),
                bucket_id: format!("{:?}", event.bucket_id),
                event_type: "requested".to_string(),
                new_msp_id: Some(format!("{:?}", event.new_msp_id)),
                old_msp_id: None,
            };
            telemetry_service.queue_typed_event(start_event).await.ok();
        }

        let result = self.handle_move_bucket_requested_event(event.clone(), &ctx).await;

        // Send completion or failure telemetry
        if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
            match &result {
                Ok(_) => {
                    let completed_event = BspBucketMoveCompletedEvent {
                        base: create_base_event("bsp_bucket_move_completed", "storage-hub-bsp".to_string(), None),
                        task_id: ctx.task_id.clone(),
                        bucket_id: format!("{:?}", event.bucket_id),
                        completion_type: "requested_handled".to_string(),
                        duration_ms: ctx.elapsed_ms(),
                        new_msp_id: Some(format!("{:?}", event.new_msp_id)),
                        old_msp_id: None,
                    };
                    telemetry_service.queue_typed_event(completed_event).await.ok();
                }
                Err(e) => {
                    let failed_event = BspBucketMoveFailedEvent {
                        base: create_base_event("bsp_bucket_move_failed", "storage-hub-bsp".to_string(), None),
                        task_id: ctx.task_id.clone(),
                        bucket_id: format!("{:?}", event.bucket_id),
                        error_type: classify_error(&e),
                        error_message: e.to_string(),
                        duration_ms: Some(ctx.elapsed_ms()),
                        event_context: "requested".to_string(),
                        new_msp_id: Some(format!("{:?}", event.new_msp_id)),
                        old_msp_id: None,
                    };
                    telemetry_service.queue_typed_event(failed_event).await.ok();
                }
            }
        }

        result
    }
}

/// Handles the [`MoveBucketAccepted`] event.
///
/// This event is triggered when the new MSP accepts the move bucket request.
/// This does not mean that the move bucket request is complete, but that the new MSP has committed.
/// For this to be complete, we need to wait for the new MSP to download all the files from the
/// bucket.
impl<NT, Runtime> EventHandler<MoveBucketAccepted> for BspMoveBucketTask<NT, Runtime>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: MoveBucketAccepted) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "MoveBucketAccepted: New MSP {:?} accepted move bucket request for bucket {:?} from old MSP {:?}. Will keep accepting download requests for a window of time.",
            event.new_msp_id,
            event.bucket_id,
            event.old_msp_id
        );

        // Create task context for tracking
        let ctx = TaskContext::new("bsp_move_bucket");

        // Send progress telemetry event
        if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
            let progress_event = BspBucketMoveProgressEvent {
                base: create_base_event("bsp_bucket_move_progress", "storage-hub-bsp".to_string(), None),
                task_id: ctx.task_id.clone(),
                bucket_id: format!("{:?}", event.bucket_id),
                progress_type: "accepted".to_string(),
                new_msp_id: Some(format!("{:?}", event.new_msp_id)),
                old_msp_id: Some(format!("{:?}", event.old_msp_id)),
                grace_period_seconds: Some(self.config.move_bucket_accepted_grace_period),
            };
            telemetry_service.queue_typed_event(progress_event).await.ok();
        }

        let result = self.storage_hub_handler
            .file_transfer
            .schedule_unregister_bucket(
                event.bucket_id,
                Some(self.config.move_bucket_accepted_grace_period),
            )
            .await
            .map_err(|e| anyhow!("Failed to unregister bucket: {:?}", e));

        // Send completion or failure telemetry
        if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
            match &result {
                Ok(_) => {
                    let completed_event = BspBucketMoveCompletedEvent {
                        base: create_base_event("bsp_bucket_move_completed", "storage-hub-bsp".to_string(), None),
                        task_id: ctx.task_id.clone(),
                        bucket_id: format!("{:?}", event.bucket_id),
                        completion_type: "accepted_scheduled".to_string(),
                        duration_ms: ctx.elapsed_ms(),
                        new_msp_id: Some(format!("{:?}", event.new_msp_id)),
                        old_msp_id: Some(format!("{:?}", event.old_msp_id)),
                    };
                    telemetry_service.queue_typed_event(completed_event).await.ok();
                }
                Err(e) => {
                    let failed_event = BspBucketMoveFailedEvent {
                        base: create_base_event("bsp_bucket_move_failed", "storage-hub-bsp".to_string(), None),
                        task_id: ctx.task_id.clone(),
                        bucket_id: format!("{:?}", event.bucket_id),
                        error_type: classify_error(&e),
                        error_message: e.to_string(),
                        duration_ms: Some(ctx.elapsed_ms()),
                        event_context: "accepted".to_string(),
                        new_msp_id: Some(format!("{:?}", event.new_msp_id)),
                        old_msp_id: Some(format!("{:?}", event.old_msp_id)),
                    };
                    telemetry_service.queue_typed_event(failed_event).await.ok();
                }
            }
        }

        result
    }
}

/// Handles the [`MoveBucketRejected`] event.
///
/// This event is triggered when the new MSP rejects the move bucket request.
/// In this case, we need to stop accepting download requests for the bucket.
impl<NT, Runtime> EventHandler<MoveBucketRejected> for BspMoveBucketTask<NT, Runtime>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: MoveBucketRejected) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "MoveBucketRejected: BSP will no longer accept download requests for files in bucket {:?} from MSP {:?}",
            event.bucket_id,
            event.new_msp_id
        );

        // Create task context for tracking
        let ctx = TaskContext::new("bsp_move_bucket");

        // Send progress telemetry event
        if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
            let progress_event = BspBucketMoveProgressEvent {
                base: create_base_event("bsp_bucket_move_progress", "storage-hub-bsp".to_string(), None),
                task_id: ctx.task_id.clone(),
                bucket_id: format!("{:?}", event.bucket_id),
                progress_type: "rejected".to_string(),
                new_msp_id: Some(format!("{:?}", event.new_msp_id)),
                old_msp_id: None,
                grace_period_seconds: None,
            };
            telemetry_service.queue_typed_event(progress_event).await.ok();
        }

        let result = self.storage_hub_handler
            .file_transfer
            .schedule_unregister_bucket(event.bucket_id, None)
            .await
            .map_err(|e| anyhow!("Failed to unregister bucket: {:?}", e));

        // Send completion or failure telemetry
        if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
            match &result {
                Ok(_) => {
                    let completed_event = BspBucketMoveCompletedEvent {
                        base: create_base_event("bsp_bucket_move_completed", "storage-hub-bsp".to_string(), None),
                        task_id: ctx.task_id.clone(),
                        bucket_id: format!("{:?}", event.bucket_id),
                        completion_type: "rejected_unregistered".to_string(),
                        duration_ms: ctx.elapsed_ms(),
                        new_msp_id: Some(format!("{:?}", event.new_msp_id)),
                        old_msp_id: None,
                    };
                    telemetry_service.queue_typed_event(completed_event).await.ok();
                }
                Err(e) => {
                    let failed_event = BspBucketMoveFailedEvent {
                        base: create_base_event("bsp_bucket_move_failed", "storage-hub-bsp".to_string(), None),
                        task_id: ctx.task_id.clone(),
                        bucket_id: format!("{:?}", event.bucket_id),
                        error_type: classify_error(&e),
                        error_message: e.to_string(),
                        duration_ms: Some(ctx.elapsed_ms()),
                        event_context: "rejected".to_string(),
                        new_msp_id: Some(format!("{:?}", event.new_msp_id)),
                        old_msp_id: None,
                    };
                    telemetry_service.queue_typed_event(failed_event).await.ok();
                }
            }
        }

        result
    }
}

/// Handles the [`MoveBucketExpired`] event.
///
/// This event is triggered when the move bucket request expires.
/// In this case, we need to stop accepting download requests for the bucket.
impl<NT, Runtime> EventHandler<MoveBucketExpired> for BspMoveBucketTask<NT, Runtime>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: MoveBucketExpired) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "MoveBucketExpired: BSP will no longer accept download requests for files in bucket {:?}",
            event.bucket_id,
        );

        // Create task context for tracking
        let ctx = TaskContext::new("bsp_move_bucket");

        // Send progress telemetry event
        if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
            let progress_event = BspBucketMoveProgressEvent {
                base: create_base_event("bsp_bucket_move_progress", "storage-hub-bsp".to_string(), None),
                task_id: ctx.task_id.clone(),
                bucket_id: format!("{:?}", event.bucket_id),
                progress_type: "expired".to_string(),
                new_msp_id: None,
                old_msp_id: None,
                grace_period_seconds: None,
            };
            telemetry_service.queue_typed_event(progress_event).await.ok();
        }

        let result = self.storage_hub_handler
            .file_transfer
            .schedule_unregister_bucket(event.bucket_id, None)
            .await
            .map_err(|e| anyhow!("Failed to unregister bucket: {:?}", e));

        // Send completion or failure telemetry
        if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
            match &result {
                Ok(_) => {
                    let completed_event = BspBucketMoveCompletedEvent {
                        base: create_base_event("bsp_bucket_move_completed", "storage-hub-bsp".to_string(), None),
                        task_id: ctx.task_id.clone(),
                        bucket_id: format!("{:?}", event.bucket_id),
                        completion_type: "expired_unregistered".to_string(),
                        duration_ms: ctx.elapsed_ms(),
                        new_msp_id: None,
                        old_msp_id: None,
                    };
                    telemetry_service.queue_typed_event(completed_event).await.ok();
                }
                Err(e) => {
                    let failed_event = BspBucketMoveFailedEvent {
                        base: create_base_event("bsp_bucket_move_failed", "storage-hub-bsp".to_string(), None),
                        task_id: ctx.task_id.clone(),
                        bucket_id: format!("{:?}", event.bucket_id),
                        error_type: classify_error(&e),
                        error_message: e.to_string(),
                        duration_ms: Some(ctx.elapsed_ms()),
                        event_context: "expired".to_string(),
                        new_msp_id: None,
                        old_msp_id: None,
                    };
                    telemetry_service.queue_typed_event(failed_event).await.ok();
                }
            }
        }

        result
    }
}

impl<NT, Runtime> BspMoveBucketTask<NT, Runtime>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    async fn handle_move_bucket_requested_event(
        &mut self,
        event: MoveBucketRequested,
        _ctx: &TaskContext,
    ) -> anyhow::Result<()> {
        let multiaddress_vec = self
            .storage_hub_handler
            .blockchain
            .query_provider_multiaddresses(event.new_msp_id)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to query MSP multiaddresses of MSP ID {:?}\n Error: {:?}",
                    event.new_msp_id,
                    e
                )
            })?;

        let peer_ids = self
            .storage_hub_handler
            .file_transfer
            .extract_peer_ids_and_register_known_addresses(multiaddress_vec)
            .await;

        for peer_id in peer_ids {
            self.storage_hub_handler
                .file_transfer
                .register_new_bucket_peer(peer_id, event.bucket_id)
                .await
                .map_err(|e| anyhow!("Failed to register new bucket peer: {:?}", e))?;
        }

        Ok(())
    }
}
