//! Task-specific telemetry module for the storage hub client with service identity support.
//!
//! This module defines telemetry events and helper functions specific to tasks
//! in the storage hub client, such as file uploads, downloads, and storage operations.
//! It integrates with the service identity system to automatically attribute events.

use serde::{Deserialize, Serialize};
use shc_actors_framework::actor::ActorHandle;
use shc_telemetry_service::{
    create_base_event, BaseTelemetryEvent, TelemetryEvent, TelemetryService,
    TelemetryServiceCommandInterfaceExt, TelemetryStrategy,
};
use std::time::Duration;
use uuid::Uuid;

/// Task execution telemetry event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTelemetryEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    pub task_name: String,
    pub task_id: Option<String>,
    pub status: TaskStatus,
    pub duration_ms: Option<u64>,
    pub error_message: Option<String>,
    pub file_size_bytes: Option<u64>,
    pub transfer_rate_mbps: Option<f64>,
    pub custom_metrics: serde_json::Value,
}

impl TelemetryEvent for TaskTelemetryEvent {
    fn event_type(&self) -> &str {
        &self.base.event_type
    }

    fn strategy(&self) -> TelemetryStrategy {
        match self.status {
            TaskStatus::Failed => TelemetryStrategy::Guaranteed,
            _ => TelemetryStrategy::BestEffort,
        }
    }
}

/// Task execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Started,
    Success,
    Failed,
    Timeout,
    Cancelled,
}

/// Service-scoped telemetry wrapper that provides automatic service attribution
pub struct ServiceTelemetry {
    inner: Option<ActorHandle<TelemetryService>>,
    service_name: String,
}

impl ServiceTelemetry {
    /// Create a new service telemetry wrapper
    pub fn new(
        telemetry_handle: Option<ActorHandle<TelemetryService>>,
        service_name: String,
    ) -> Self {
        Self {
            inner: telemetry_handle,
            service_name,
        }
    }

    /// Generate a new task ID
    pub fn generate_task_id() -> String {
        Uuid::new_v4().to_string()
    }

    /// Send a task started event
    pub async fn task_started(
        &self,
        task_name: &str,
        task_id: Option<String>,
        file_size_bytes: Option<u64>,
        custom_metrics: serde_json::Value,
    ) {
        if let Some(telemetry) = &self.inner {
            let event = TaskTelemetryEvent {
                base: create_base_event("task_started", self.service_name.clone(), None),
                task_name: task_name.to_string(),
                task_id,
                status: TaskStatus::Started,
                duration_ms: None,
                error_message: None,
                file_size_bytes,
                transfer_rate_mbps: None,
                custom_metrics,
            };
            telemetry.queue_typed_event(event).await.ok();
        }
    }

    /// Send a task completed event
    pub async fn task_completed(
        &self,
        task_name: &str,
        task_id: Option<String>,
        duration: Duration,
        file_size_bytes: Option<u64>,
        transfer_rate_mbps: Option<f64>,
        custom_metrics: serde_json::Value,
    ) {
        if let Some(telemetry) = &self.inner {
            let event = TaskTelemetryEvent {
                base: create_base_event("task_completed", self.service_name.clone(), None),
                task_name: task_name.to_string(),
                task_id,
                status: TaskStatus::Success,
                duration_ms: Some(duration.as_millis() as u64),
                error_message: None,
                file_size_bytes,
                transfer_rate_mbps,
                custom_metrics,
            };
            telemetry.queue_typed_event(event).await.ok();
        }
    }

    /// Send a task failed event
    pub async fn task_failed(
        &self,
        task_name: &str,
        task_id: Option<String>,
        duration: Option<Duration>,
        error_message: String,
        custom_metrics: serde_json::Value,
    ) {
        if let Some(telemetry) = &self.inner {
            let event = TaskTelemetryEvent {
                base: create_base_event("task_failed", self.service_name.clone(), None),
                task_name: task_name.to_string(),
                task_id,
                status: TaskStatus::Failed,
                duration_ms: duration.map(|d| d.as_millis() as u64),
                error_message: Some(error_message),
                file_size_bytes: None,
                transfer_rate_mbps: None,
                custom_metrics,
            };
            telemetry.queue_typed_event(event).await.ok();
        }
    }

    /// Send a task timeout event
    pub async fn task_timeout(
        &self,
        task_name: &str,
        task_id: Option<String>,
        duration: Duration,
        custom_metrics: serde_json::Value,
    ) {
        if let Some(telemetry) = &self.inner {
            let event = TaskTelemetryEvent {
                base: create_base_event("task_timeout", self.service_name.clone(), None),
                task_name: task_name.to_string(),
                task_id,
                status: TaskStatus::Timeout,
                duration_ms: Some(duration.as_millis() as u64),
                error_message: Some("Task exceeded timeout".to_string()),
                file_size_bytes: None,
                transfer_rate_mbps: None,
                custom_metrics,
            };
            telemetry.queue_typed_event(event).await.ok();
        }
    }

    /// Send a task cancelled event
    pub async fn task_cancelled(
        &self,
        task_name: &str,
        task_id: Option<String>,
        duration: Option<Duration>,
        reason: String,
        custom_metrics: serde_json::Value,
    ) {
        if let Some(telemetry) = &self.inner {
            let event = TaskTelemetryEvent {
                base: create_base_event("task_cancelled", self.service_name.clone(), None),
                task_name: task_name.to_string(),
                task_id,
                status: TaskStatus::Cancelled,
                duration_ms: duration.map(|d| d.as_millis() as u64),
                error_message: Some(reason),
                file_size_bytes: None,
                transfer_rate_mbps: None,
                custom_metrics,
            };
            telemetry.queue_typed_event(event).await.ok();
        }
    }
}
