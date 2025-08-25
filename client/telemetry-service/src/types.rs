//! Generic telemetry module with batching and bounded channels for production-scale performance.
//!
//! This module provides a generic telemetry infrastructure that allows any component
//! in the codebase to define and send custom telemetry events to configurable backends.
//!
//! Key features:
//! - Bounded channel architecture to prevent resource exhaustion
//! - Automatic batching for network efficiency
//! - Graceful shutdown with event flushing
//! - Backpressure handling with configurable drop strategies
//! - Health monitoring and metrics
//! - Strongly-typed, queryable event fields (no JSON blobs)

use async_trait::async_trait;
use axiom_rs::Client as AxiomClient;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    env,
    sync::atomic::{AtomicU64, Ordering},
    sync::Arc,
};
use log::error;

/// Telemetry configuration
#[derive(Clone, Debug)]
pub struct TelemetryConfig {
    /// Maximum events to buffer before applying backpressure
    pub buffer_size: usize,
    /// Maximum events per batch
    pub batch_size: usize,
    /// Maximum time to wait before flushing a batch (in seconds)
    pub flush_interval_secs: u64,
    /// Timeout for backend operations (in seconds)
    pub backend_timeout_secs: u64,
    /// Maximum number of retries for failed sends
    pub max_retries: u32,
    /// Strategy for handling buffer overflow
    pub overflow_strategy: OverflowStrategy,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            buffer_size: 10_000,
            batch_size: 100,
            flush_interval_secs: 5,
            backend_timeout_secs: 10,
            max_retries: 3,
            overflow_strategy: OverflowStrategy::DropOldest,
        }
    }
}

/// Strategy for handling buffer overflow
#[derive(Clone, Debug)]
pub enum OverflowStrategy {
    /// Drop the oldest events when buffer is full
    DropOldest,
    /// Drop new events when buffer is full
    DropNewest,
    /// Block until space is available (not recommended)
    Block,
}

/// Telemetry delivery strategy
#[derive(Clone, Debug)]
pub enum TelemetryStrategy {
    /// Best effort - drop events on failure
    BestEffort,
    /// Guaranteed - retry with exponential backoff
    Guaranteed,
    /// Critical - block until sent (use sparingly)
    Critical,
}

/// Configuration for Axiom telemetry
#[derive(Clone, Debug)]
pub struct AxiomConfig {
    pub token: String,
    pub dataset: String,
}

impl AxiomConfig {
    /// Create configuration from explicit parameters
    pub fn new(token: String, dataset: String) -> Self {
        Self { token, dataset }
    }

    /// Create configuration from environment variables
    pub fn from_env() -> Option<Self> {
        let token = env::var("AXIOM_TOKEN").ok()?;
        let dataset = env::var("AXIOM_DATASET").ok()?;

        Some(Self { token, dataset })
    }
}

/// Base telemetry event with common fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseTelemetryEvent {
    pub timestamp: DateTime<Utc>,
    pub service: String,
    pub event_type: String,
    pub node_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
}

/// Trait that all telemetry events must implement
pub trait TelemetryEvent: Serialize + Send + Sync {
    /// Get the event type for this telemetry event
    fn event_type(&self) -> &str;

    /// Get the delivery strategy for this event
    fn strategy(&self) -> TelemetryStrategy {
        TelemetryStrategy::BestEffort
    }

    /// Convert the event to JSON for sending to backend
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|e| {
            error!("Failed to serialize telemetry event: {}", e);
            serde_json::json!({})
        })
    }
}


/// Telemetry backend trait for extensibility
#[async_trait]
pub trait TelemetryBackend: Send + Sync {
    /// Send a batch of events to the backend
    async fn send_batch(
        &self,
        events: Vec<serde_json::Value>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Check if the backend is enabled
    fn is_enabled(&self) -> bool;

    /// Flush any pending events
    async fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Get backend name for logging
    fn name(&self) -> &str;
}

/// Axiom backend implementation
pub struct AxiomBackend {
    client: Option<Arc<tokio::sync::Mutex<AxiomClient>>>,
    dataset: String,
}

impl AxiomBackend {
    /// Create a new Axiom backend
    pub fn new(config: AxiomConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let client = AxiomClient::builder()
            .no_env()
            .with_token(config.token)
            .build()?;

        Ok(Self {
            client: Some(Arc::new(tokio::sync::Mutex::new(client))),
            dataset: config.dataset,
        })
    }
}

#[async_trait]
impl TelemetryBackend for AxiomBackend {
    async fn send_batch(
        &self,
        events: Vec<serde_json::Value>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(client) = &self.client {
            let client_guard = client.lock().await;
            client_guard.ingest(&self.dataset, events).await?;
        }
        Ok(())
    }

    fn is_enabled(&self) -> bool {
        self.client.is_some()
    }

    async fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Axiom client doesn't require explicit flush
        Ok(())
    }

    fn name(&self) -> &str {
        "axiom"
    }
}

/// No-op backend for testing or when telemetry is disabled
pub struct NoOpBackend;

#[async_trait]
impl TelemetryBackend for NoOpBackend {
    async fn send_batch(
        &self,
        _events: Vec<serde_json::Value>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    fn is_enabled(&self) -> bool {
        false
    }

    async fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    fn name(&self) -> &str {
        "noop"
    }
}

/// Internal event wrapper with metadata
pub struct TelemetryEventWrapper {
    pub event: serde_json::Value,
    pub strategy: TelemetryStrategy,
    pub retry_count: u32,
}

/// Telemetry health metrics
#[derive(Debug, Clone)]
pub struct TelemetryMetrics {
    pub events_sent: Arc<AtomicU64>,
    pub events_dropped: Arc<AtomicU64>,
    pub events_failed: Arc<AtomicU64>,
    pub batches_sent: Arc<AtomicU64>,
    pub backend_errors: Arc<AtomicU64>,
}

impl TelemetryMetrics {
    pub fn new() -> Self {
        Self {
            events_sent: Arc::new(AtomicU64::new(0)),
            events_dropped: Arc::new(AtomicU64::new(0)),
            events_failed: Arc::new(AtomicU64::new(0)),
            batches_sent: Arc::new(AtomicU64::new(0)),
            backend_errors: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get a snapshot of current metrics
    pub fn snapshot(&self) -> TelemetryMetricsSnapshot {
        TelemetryMetricsSnapshot {
            events_sent: self.events_sent.load(Ordering::Relaxed),
            events_dropped: self.events_dropped.load(Ordering::Relaxed),
            events_failed: self.events_failed.load(Ordering::Relaxed),
            batches_sent: self.batches_sent.load(Ordering::Relaxed),
            backend_errors: self.backend_errors.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of telemetry metrics
#[derive(Debug, Clone, Serialize)]
pub struct TelemetryMetricsSnapshot {
    pub events_sent: u64,
    pub events_dropped: u64,
    pub events_failed: u64,
    pub batches_sent: u64,
    pub backend_errors: u64,
}

// The TelemetryService implementation has been moved to handler.rs as an Actor

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_axiom_config_from_env() {
        // Test environment variable loading
        std::env::set_var("AXIOM_TOKEN", "test-token");
        std::env::set_var("AXIOM_DATASET", "test-dataset");
        
        let config = AxiomConfig::from_env();
        assert!(config.is_some());
        
        let config = config.unwrap();
        assert_eq!(config.token, "test-token");
        assert_eq!(config.dataset, "test-dataset");
        
        // Clean up
        std::env::remove_var("AXIOM_TOKEN");
        std::env::remove_var("AXIOM_DATASET");
    }
}