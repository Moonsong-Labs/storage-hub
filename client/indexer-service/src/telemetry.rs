//! Indexer-specific telemetry module for the storage hub indexer service.
//!
//! This module defines telemetry events and helper functions specific to the indexer
//! service, such as block processing, event handling, and handler execution.

use serde::{Deserialize, Serialize};
use shc_actors_framework::actor::ActorHandle;
use shc_telemetry_service::{
    create_base_event, BaseTelemetryEvent, TelemetryEvent, TelemetryMetricsSnapshot,
    TelemetryService, TelemetryServiceCommandInterface, TelemetryServiceCommandInterfaceExt,
    TelemetryStrategy,
};
use std::time::Duration;

/// Indexer handler telemetry event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerTelemetryEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    pub handler_name: String,
    pub block_number: Option<u64>,
    pub block_hash: Option<String>,
    pub event_count: Option<u64>,
    pub processing_duration_ms: Option<u64>,
    pub status: IndexerStatus,
    pub error_message: Option<String>,
    pub custom_metrics: serde_json::Value,
}

impl TelemetryEvent for IndexerTelemetryEvent {
    fn event_type(&self) -> &str {
        &self.base.event_type
    }

    fn strategy(&self) -> TelemetryStrategy {
        TelemetryStrategy::BestEffort
    }
}

/// Status of an indexer operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexerStatus {
    Started,
    Success,
    Failed,
    Skipped,
}

/// Service-scoped telemetry wrapper for indexer service
pub struct IndexerServiceTelemetry {
    inner: Option<ActorHandle<TelemetryService>>,
    service_name: String,
}

impl IndexerServiceTelemetry {
    /// Create a new indexer service telemetry wrapper
    pub fn new(telemetry_handle: Option<ActorHandle<TelemetryService>>) -> Self {
        Self {
            inner: telemetry_handle,
            service_name: "storage-hub-indexer".to_string(),
        }
    }

    /// Send an indexer started event
    pub async fn indexer_started(
        &self,
        handler_name: &str,
        block_number: Option<u64>,
        block_hash: Option<String>,
        custom_metrics: serde_json::Value,
    ) {
        if let Some(telemetry) = &self.inner {
            let event = IndexerTelemetryEvent {
                base: create_base_event("indexer_started", self.service_name.clone(), None),
                handler_name: handler_name.to_string(),
                block_number,
                block_hash,
                event_count: None,
                processing_duration_ms: None,
                status: IndexerStatus::Started,
                error_message: None,
                custom_metrics,
            };
            telemetry.queue_typed_event(event).await.ok();
        }
    }

    /// Send an indexer completed event
    pub async fn indexer_completed(
        &self,
        handler_name: &str,
        block_number: Option<u64>,
        block_hash: Option<String>,
        event_count: u64,
        duration: Duration,
        custom_metrics: serde_json::Value,
    ) {
        if let Some(telemetry) = &self.inner {
            let event = IndexerTelemetryEvent {
                base: create_base_event("indexer_completed", self.service_name.clone(), None),
                handler_name: handler_name.to_string(),
                block_number,
                block_hash,
                event_count: Some(event_count),
                processing_duration_ms: Some(duration.as_millis() as u64),
                status: IndexerStatus::Success,
                error_message: None,
                custom_metrics,
            };
            telemetry.queue_typed_event(event).await.ok();
        }
    }

    /// Send an indexer failed event
    pub async fn indexer_failed(
        &self,
        handler_name: &str,
        block_number: Option<u64>,
        block_hash: Option<String>,
        duration: Option<Duration>,
        error_message: String,
        custom_metrics: serde_json::Value,
    ) {
        if let Some(telemetry) = &self.inner {
            let event = IndexerTelemetryEvent {
                base: create_base_event("indexer_failed", self.service_name.clone(), None),
                handler_name: handler_name.to_string(),
                block_number,
                block_hash,
                event_count: None,
                processing_duration_ms: duration.map(|d| d.as_millis() as u64),
                status: IndexerStatus::Failed,
                error_message: Some(error_message),
                custom_metrics,
            };
            telemetry.queue_typed_event(event).await.ok();
        }
    }

    /// Send an indexer skipped event
    pub async fn indexer_skipped(
        &self,
        handler_name: &str,
        block_number: Option<u64>,
        block_hash: Option<String>,
        reason: String,
        custom_metrics: serde_json::Value,
    ) {
        if let Some(telemetry) = &self.inner {
            let event = IndexerTelemetryEvent {
                base: create_base_event("indexer_skipped", self.service_name.clone(), None),
                handler_name: handler_name.to_string(),
                block_number,
                block_hash,
                event_count: None,
                processing_duration_ms: None,
                status: IndexerStatus::Skipped,
                error_message: Some(reason),
                custom_metrics,
            };
            telemetry.queue_typed_event(event).await.ok();
        }
    }

    /// Get telemetry metrics
    pub async fn metrics(&self) -> Option<TelemetryMetricsSnapshot> {
        if let Some(telemetry) = &self.inner {
            telemetry.get_metrics().await.ok()
        } else {
            None
        }
    }
}
