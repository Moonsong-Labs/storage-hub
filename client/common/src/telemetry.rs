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

pub mod events;

use async_trait::async_trait;
use axiom_rs::Client as AxiomClient;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    env,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

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
    client: Option<Arc<Mutex<AxiomClient>>>,
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
            client: Some(Arc::new(Mutex::new(client))),
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
struct TelemetryEventWrapper {
    event: serde_json::Value,
    strategy: TelemetryStrategy,
    retry_count: u32,
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
    fn new() -> Self {
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

/// Main telemetry service with batching and bounded channels
pub struct TelemetryService {
    sender: mpsc::Sender<TelemetryEventWrapper>,
    metrics: TelemetryMetrics,
    shutdown_tx: Option<oneshot::Sender<()>>,
    worker_handle: Option<tokio::task::JoinHandle<()>>,
    service_name: String,
    node_id: Option<String>,
    is_shutting_down: Arc<AtomicBool>,
}

impl TelemetryService {
    /// Create a new telemetry service
    pub fn new(
        service_name: String,
        node_id: Option<String>,
        backend: Arc<dyn TelemetryBackend>,
        config: TelemetryConfig,
    ) -> Self {
        let (tx, rx) = mpsc::channel(config.buffer_size);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let metrics = TelemetryMetrics::new();
        let is_shutting_down = Arc::new(AtomicBool::new(false));

        // Spawn worker task
        let worker_handle = tokio::spawn(Self::worker_task(
            rx,
            backend,
            config,
            metrics.clone(),
            shutdown_rx,
            is_shutting_down.clone(),
        ));

        Self {
            sender: tx,
            metrics,
            shutdown_tx: Some(shutdown_tx),
            worker_handle: Some(worker_handle),
            service_name,
            node_id,
            is_shutting_down,
        }
    }

    /// Worker task that processes events from the channel
    async fn worker_task(
        mut rx: mpsc::Receiver<TelemetryEventWrapper>,
        backend: Arc<dyn TelemetryBackend>,
        config: TelemetryConfig,
        metrics: TelemetryMetrics,
        mut shutdown_rx: oneshot::Receiver<()>,
        _is_shutting_down: Arc<AtomicBool>,
    ) {
        let mut batch = Vec::with_capacity(config.batch_size);
        let mut flush_interval = interval(Duration::from_secs(config.flush_interval_secs));
        flush_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        info!("Telemetry worker started for backend: {}", backend.name());

        loop {
            tokio::select! {
                Some(wrapper) = rx.recv() => {
                    batch.push(wrapper);

                    // Send batch if it's full
                    if batch.len() >= config.batch_size {
                        Self::send_batch(&backend, &mut batch, &config, &metrics).await;
                    }
                }
                _ = flush_interval.tick() => {
                    // Periodic flush
                    if !batch.is_empty() {
                        Self::send_batch(&backend, &mut batch, &config, &metrics).await;
                    }
                }
                _ = &mut shutdown_rx => {
                    info!("Telemetry worker received shutdown signal");

                    // Final flush before shutdown
                    if !batch.is_empty() {
                        info!("Flushing {} events before shutdown", batch.len());
                        Self::send_batch(&backend, &mut batch, &config, &metrics).await;
                    }

                    // Drain remaining events with timeout
                    let drain_timeout = tokio::time::sleep(Duration::from_secs(5));
                    tokio::pin!(drain_timeout);

                    loop {
                        tokio::select! {
                            Some(wrapper) = rx.recv() => {
                                batch.push(wrapper);
                                if batch.len() >= config.batch_size {
                                    Self::send_batch(&backend, &mut batch, &config, &metrics).await;
                                }
                            }
                            _ = &mut drain_timeout => {
                                if !batch.is_empty() {
                                    info!("Final flush of {} events", batch.len());
                                    Self::send_batch(&backend, &mut batch, &config, &metrics).await;
                                }
                                break;
                            }
                        }
                    }

                    info!("Telemetry worker shutdown complete");
                    break;
                }
            }
        }
    }

    /// Send a batch of events with retry logic
    async fn send_batch(
        backend: &Arc<dyn TelemetryBackend>,
        batch: &mut Vec<TelemetryEventWrapper>,
        config: &TelemetryConfig,
        metrics: &TelemetryMetrics,
    ) {
        if batch.is_empty() {
            return;
        }

        let events: Vec<serde_json::Value> = batch.iter().map(|w| w.event.clone()).collect();
        let batch_size = events.len();

        // Try to send with timeout
        let send_result = tokio::time::timeout(
            Duration::from_secs(config.backend_timeout_secs),
            backend.send_batch(events),
        )
        .await;

        match send_result {
            Ok(Ok(())) => {
                debug!("Successfully sent batch of {} events", batch_size);
                metrics
                    .events_sent
                    .fetch_add(batch_size as u64, Ordering::Relaxed);
                metrics.batches_sent.fetch_add(1, Ordering::Relaxed);
                batch.clear();
            }
            Ok(Err(e)) => {
                error!("Failed to send batch of {} events: {}", batch_size, e);
                metrics.backend_errors.fetch_add(1, Ordering::Relaxed);

                // Handle retries based on strategy
                let mut retry_batch = Vec::new();
                for mut wrapper in batch.drain(..) {
                    match wrapper.strategy {
                        TelemetryStrategy::Guaranteed | TelemetryStrategy::Critical
                            if wrapper.retry_count < config.max_retries =>
                        {
                            wrapper.retry_count += 1;
                            retry_batch.push(wrapper);
                        }
                        _ => {
                            metrics.events_dropped.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                *batch = retry_batch;
            }
            Err(_) => {
                error!(
                    "Backend timeout while sending batch of {} events",
                    batch_size
                );
                metrics.backend_errors.fetch_add(1, Ordering::Relaxed);

                // Handle retries based on strategy
                let mut retry_batch = Vec::new();
                for mut wrapper in batch.drain(..) {
                    match wrapper.strategy {
                        TelemetryStrategy::BestEffort => {
                            metrics.events_dropped.fetch_add(1, Ordering::Relaxed);
                        }
                        TelemetryStrategy::Guaranteed => {
                            wrapper.retry_count += 1;
                            if wrapper.retry_count <= config.max_retries {
                                retry_batch.push(wrapper);
                            } else {
                                metrics.events_failed.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        TelemetryStrategy::Critical => {
                            // For critical events, we would implement blocking retry
                            // For now, treat as guaranteed
                            wrapper.retry_count += 1;
                            if wrapper.retry_count <= config.max_retries * 2 {
                                retry_batch.push(wrapper);
                            } else {
                                error!("Critical event failed after maximum retries");
                                metrics.events_failed.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }

                // Add retry events back to batch for next attempt
                *batch = retry_batch;
            }
        }
    }

    /// Check if telemetry is enabled
    pub fn is_enabled(&self) -> bool {
        !self.is_shutting_down.load(Ordering::Relaxed)
    }

    /// Create a base telemetry event with common fields
    pub fn create_base_event(&self, event_type: &str) -> BaseTelemetryEvent {
        BaseTelemetryEvent {
            timestamp: Utc::now(),
            service: self.service_name.clone(),
            event_type: event_type.to_string(),
            node_id: self.node_id.clone(),
            correlation_id: None,
            span_id: None,
            parent_span_id: None,
        }
    }

    /// Send a telemetry event (non-blocking)
    pub fn send_event<T: TelemetryEvent + 'static>(&self, event: T) {
        if self.is_shutting_down.load(Ordering::Relaxed) {
            return;
        }

        let wrapper = TelemetryEventWrapper {
            event: event.to_json(),
            strategy: event.strategy(),
            retry_count: 0,
        };

        // Try to send, handle backpressure
        match self.sender.try_send(wrapper) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                self.metrics.events_dropped.fetch_add(1, Ordering::Relaxed);
                warn!("Telemetry buffer full, dropping event");
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                // Channel closed, we're shutting down
            }
        }
    }

    /// Send a raw telemetry event with custom data (non-blocking)
    pub fn send_raw_event(&self, event_type: &str, data: serde_json::Value) {
        let event = GeneralTelemetryEvent {
            base: self.create_base_event(event_type),
            data,
        };
        self.send_event(event);
    }

    /// Get current metrics
    pub fn metrics(&self) -> TelemetryMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Gracefully shutdown the telemetry service
    pub async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Initiating telemetry service shutdown");
        self.is_shutting_down.store(true, Ordering::Relaxed);

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        // Wait for worker to finish
        if let Some(handle) = self.worker_handle.take() {
            tokio::time::timeout(Duration::from_secs(10), handle).await??;
        }

        info!(
            "Telemetry service shutdown complete. Final metrics: {:?}",
            self.metrics.snapshot()
        );
        Ok(())
    }
}

/// Global telemetry instance (lazy-initialized)
use std::sync::OnceLock;

static TELEMETRY: OnceLock<Arc<Mutex<TelemetryService>>> = OnceLock::new();

/// Initialize global telemetry instance with custom backend and config
pub async fn init_telemetry_with_backend(
    service_name: String,
    node_id: Option<String>,
    backend: Arc<dyn TelemetryBackend>,
    config: TelemetryConfig,
) {
    let service = TelemetryService::new(service_name, node_id, backend, config);
    let _ = TELEMETRY.set(Arc::new(Mutex::new(service)));
}

/// Initialize global telemetry instance with Axiom backend
pub async fn init_telemetry(service_name: String, node_id: Option<String>) {
    let backend: Arc<dyn TelemetryBackend> = if let Some(axiom_config) = AxiomConfig::from_env() {
        match AxiomBackend::new(axiom_config) {
            Ok(backend) => {
                info!("Axiom telemetry backend initialized successfully");
                Arc::new(backend)
            }
            Err(e) => {
                error!("Failed to initialize Axiom backend: {}", e);
                Arc::new(NoOpBackend)
            }
        }
    } else {
        warn!("Axiom configuration not found in environment variables. Telemetry disabled.");
        Arc::new(NoOpBackend)
    };

    init_telemetry_with_backend(service_name, node_id, backend, TelemetryConfig::default()).await;
}

/// Get global telemetry instance
pub async fn telemetry() -> Option<Arc<Mutex<TelemetryService>>> {
    TELEMETRY.get().cloned()
}

/// Shutdown global telemetry instance
pub async fn shutdown_telemetry() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(service) = TELEMETRY.get() {
        let mut service_guard = service.lock().await;
        service_guard.shutdown().await?;
    }
    Ok(())
}

#[cfg(test)]
mod telemetry_test;

#[cfg(test)]
mod telemetry_integration_test;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex as TokioMutex;

    /// Mock backend for testing
    struct MockBackend {
        events: Arc<TokioMutex<Vec<serde_json::Value>>>,
        enabled: bool,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                events: Arc::new(TokioMutex::new(Vec::new())),
                enabled: true,
            }
        }
    }

    #[async_trait]
    impl TelemetryBackend for MockBackend {
        async fn send_batch(
            &self,
            events: Vec<serde_json::Value>,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let mut stored = self.events.lock().await;
            stored.extend(events);
            Ok(())
        }

        fn is_enabled(&self) -> bool {
            self.enabled
        }

        async fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        fn name(&self) -> &str {
            "mock"
        }
    }

    #[tokio::test]
    async fn test_service_telemetry_batching() {
        let mock = Arc::new(MockBackend::new());
        let config = TelemetryConfig {
            buffer_size: 100,
            batch_size: 5,
            flush_interval_secs: 1,
            ..Default::default()
        };

        let mut service = TelemetryService::new(
            "test-service".to_string(),
            Some("node-1".to_string()),
            mock.clone(),
            config,
        );

        // Send 10 events
        for i in 0..10 {
            service.send_raw_event(&format!("event_{}", i), serde_json::json!({"index": i}));
        }

        // Wait for batching
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Check events were batched
        let events = mock.events.lock().await;
        assert_eq!(events.len(), 10);

        // Check metrics
        let metrics = service.metrics();
        assert_eq!(metrics.events_sent, 10);
        assert_eq!(metrics.batches_sent, 2); // 2 batches of 5

        // Shutdown
        service.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_graceful_shutdown() {
        let mock = Arc::new(MockBackend::new());
        let config = TelemetryConfig {
            buffer_size: 100,
            batch_size: 10,
            flush_interval_secs: 60, // Long interval to test shutdown flush
            ..Default::default()
        };

        let mut service =
            TelemetryService::new("test-service".to_string(), None, mock.clone(), config);

        // Send events that won't trigger batch
        for i in 0..5 {
            service.send_raw_event(&format!("event_{}", i), serde_json::json!({"index": i}));
        }

        // Shutdown should flush remaining events
        service.shutdown().await.unwrap();

        // Check all events were sent
        let events = mock.events.lock().await;
        assert_eq!(events.len(), 5);
    }

    #[tokio::test]
    async fn test_backpressure_handling() {
        let mock = Arc::new(MockBackend::new());
        let config = TelemetryConfig {
            buffer_size: 5,
            batch_size: 10,
            flush_interval_secs: 60,
            overflow_strategy: OverflowStrategy::DropNewest,
            ..Default::default()
        };

        let service = TelemetryService::new("test-service".to_string(), None, mock.clone(), config);

        // Fill buffer beyond capacity
        for i in 0..10 {
            service.send_raw_event(&format!("event_{}", i), serde_json::json!({"index": i}));
        }

        // Some events should be dropped
        let metrics = service.metrics();
        assert!(metrics.events_dropped > 0);
    }
}
