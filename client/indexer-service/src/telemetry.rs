//! Indexer-specific telemetry module for the storage hub indexer service.
//!
//! This module defines telemetry events and helper functions specific to the indexer
//! service, such as block processing, event handling, and handler execution.

use serde::{Deserialize, Serialize};
use shc_common::telemetry::{
    BaseTelemetryEvent, TelemetryEvent, TelemetryService, TelemetryStrategy,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

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
        match self.status {
            IndexerStatus::Failed => TelemetryStrategy::Guaranteed,
            _ => TelemetryStrategy::BestEffort,
        }
    }
}

/// Indexer processing status
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
    inner: Arc<Mutex<TelemetryService>>,
}

impl IndexerServiceTelemetry {
    /// Create a new indexer service telemetry wrapper
    pub fn new(service: Arc<Mutex<TelemetryService>>) -> Self {
        Self { inner: service }
    }

    /// Send an indexer started event
    pub async fn indexer_started(
        &self,
        handler_name: &str,
        block_number: Option<u64>,
        block_hash: Option<String>,
        custom_metrics: serde_json::Value,
    ) {
        if let Ok(service) = self.inner.try_lock() {
            let event = IndexerTelemetryEvent {
                base: service.create_base_event("indexer_started"),
                handler_name: handler_name.to_string(),
                block_number,
                block_hash,
                event_count: None,
                processing_duration_ms: None,
                status: IndexerStatus::Started,
                error_message: None,
                custom_metrics,
            };
            service.send_event(event);
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
        if let Ok(service) = self.inner.try_lock() {
            let event = IndexerTelemetryEvent {
                base: service.create_base_event("indexer_completed"),
                handler_name: handler_name.to_string(),
                block_number,
                block_hash,
                event_count: Some(event_count),
                processing_duration_ms: Some(duration.as_millis() as u64),
                status: IndexerStatus::Success,
                error_message: None,
                custom_metrics,
            };
            service.send_event(event);
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
        if let Ok(service) = self.inner.try_lock() {
            let event = IndexerTelemetryEvent {
                base: service.create_base_event("indexer_failed"),
                handler_name: handler_name.to_string(),
                block_number,
                block_hash,
                event_count: None,
                processing_duration_ms: duration.map(|d| d.as_millis() as u64),
                status: IndexerStatus::Failed,
                error_message: Some(error_message),
                custom_metrics,
            };
            service.send_event(event);
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
        if let Ok(service) = self.inner.try_lock() {
            let event = IndexerTelemetryEvent {
                base: service.create_base_event("indexer_skipped"),
                handler_name: handler_name.to_string(),
                block_number,
                block_hash,
                event_count: None,
                processing_duration_ms: None,
                status: IndexerStatus::Skipped,
                error_message: Some(reason),
                custom_metrics,
            };
            service.send_event(event);
        }
    }

    /// Get telemetry metrics
    pub async fn metrics(&self) -> Option<shc_common::telemetry::TelemetryMetricsSnapshot> {
        if let Ok(service) = self.inner.try_lock() {
            Some(service.metrics())
        } else {
            None
        }
    }
}
