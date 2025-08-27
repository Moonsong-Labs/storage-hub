use async_trait::async_trait;
use futures::prelude::*;
use log::{debug, error, info, warn};
use serde_json::Value as JsonValue;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    sync::{mpsc, oneshot},
    time::interval,
};

use shc_actors_framework::actor::Actor;

use crate::{
    commands::TelemetryServiceCommand,
    types::{
        TelemetryBackend, TelemetryConfig, TelemetryEventWrapper, TelemetryMetrics,
        TelemetryStrategy,
    },
};

const LOG_TARGET: &str = "telemetry-service";

/// Main telemetry service actor with batching and bounded channels.
pub struct TelemetryService {
    /// Service name for telemetry identification
    service_name: String,
    /// Node ID for telemetry identification
    node_id: Option<String>,
    /// Backend for sending telemetry
    backend: Arc<dyn TelemetryBackend>,
    /// Configuration for telemetry behavior
    config: TelemetryConfig,
    /// Metrics tracking
    metrics: TelemetryMetrics,
    /// Channel for sending events to worker
    sender: mpsc::Sender<TelemetryEventWrapper>,
    /// Channel for receiving events in worker
    receiver: Option<mpsc::Receiver<TelemetryEventWrapper>>,
    /// Shutdown signal sender
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// Flag indicating if service is shutting down
    is_shutting_down: Arc<AtomicBool>,
}

impl TelemetryService {
    /// Create a new telemetry service.
    pub fn new(
        service_name: String,
        node_id: Option<String>,
        backend: Arc<dyn TelemetryBackend>,
        config: TelemetryConfig,
    ) -> Self {
        let (tx, rx) = mpsc::channel(config.buffer_size);
        let metrics = TelemetryMetrics::new();
        let is_shutting_down = Arc::new(AtomicBool::new(false));

        Self {
            service_name,
            node_id,
            backend,
            config,
            metrics,
            sender: tx,
            receiver: Some(rx),
            shutdown_tx: None,
            is_shutting_down,
        }
    }

    /// Process events from the channel in batches.
    async fn process_events(
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

        info!(target: LOG_TARGET, "Telemetry worker started for backend: {}", backend.name());

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
                    info!(target: LOG_TARGET, "Telemetry worker received shutdown signal");

                    // Final flush before shutdown
                    if !batch.is_empty() {
                        info!(target: LOG_TARGET, "Flushing {} events before shutdown", batch.len());
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
                                    info!(target: LOG_TARGET, "Final flush of {} events", batch.len());
                                    Self::send_batch(&backend, &mut batch, &config, &metrics).await;
                                }
                                break;
                            }
                        }
                    }

                    info!(target: LOG_TARGET, "Telemetry worker shutdown complete");
                    break;
                }
            }
        }
    }

    /// Send a batch of events with retry logic.
    async fn send_batch(
        backend: &Arc<dyn TelemetryBackend>,
        batch: &mut Vec<TelemetryEventWrapper>,
        config: &TelemetryConfig,
        metrics: &TelemetryMetrics,
    ) {
        if batch.is_empty() {
            return;
        }

        let events: Vec<JsonValue> = batch.iter().map(|w| w.event.clone()).collect();
        let batch_size = events.len();

        // Try to send with timeout
        let send_result = tokio::time::timeout(
            Duration::from_secs(config.backend_timeout_secs),
            backend.send_batch(events),
        )
        .await;

        match send_result {
            Ok(Ok(())) => {
                debug!(target: LOG_TARGET, "Successfully sent batch of {} events", batch_size);
                metrics
                    .events_sent
                    .fetch_add(batch_size as u64, Ordering::Relaxed);
                metrics.batches_sent.fetch_add(1, Ordering::Relaxed);
                batch.clear();
            }
            Ok(Err(e)) => {
                error!(target: LOG_TARGET, "Failed to send batch of {} events: {}", batch_size, e);
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
                    target: LOG_TARGET,
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
                                error!(target: LOG_TARGET, "Critical event failed after maximum retries");
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

    /// Queue an event for processing (non-blocking).
    fn queue_event_internal(&self, event: JsonValue, strategy: TelemetryStrategy) {
        if self.is_shutting_down.load(Ordering::Relaxed) {
            return;
        }

        let wrapper = TelemetryEventWrapper {
            event,
            strategy,
            retry_count: 0,
        };

        // Try to send, handle backpressure
        match self.sender.try_send(wrapper) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                self.metrics.events_dropped.fetch_add(1, Ordering::Relaxed);
                warn!(target: LOG_TARGET, "Telemetry buffer full, dropping event");
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                // Channel closed, we're shutting down
            }
        }
    }
}

#[async_trait]
impl Actor for TelemetryService {
    type Message = TelemetryServiceCommand;
    type EventLoop = TelemetryServiceEventLoop;
    type EventBusProvider = ();

    fn handle_message(
        &mut self,
        message: Self::Message,
    ) -> impl std::future::Future<Output = ()> + Send {
        async move {
            match message {
                TelemetryServiceCommand::QueueEvent { event, strategy } => {
                    self.queue_event_internal(event, strategy);
                }
            }
        }
    }

    fn get_event_bus_provider(&self) -> &Self::EventBusProvider {
        &()
    }
}

/// Custom event loop for telemetry service to handle background processing.
pub struct TelemetryServiceEventLoop {
    actor: TelemetryService,
    receiver: sc_utils::mpsc::TracingUnboundedReceiver<TelemetryServiceCommand>,
}

impl shc_actors_framework::actor::ActorEventLoop<TelemetryService> for TelemetryServiceEventLoop {
    fn new(
        mut actor: TelemetryService,
        receiver: sc_utils::mpsc::TracingUnboundedReceiver<TelemetryServiceCommand>,
    ) -> Self {
        // Log service initialization with service name and node ID
        info!(
            target: LOG_TARGET,
            "Initializing telemetry service '{}' for node {:?}",
            actor.service_name,
            actor.node_id
        );

        // Take the receiver for the background worker
        let worker_rx = actor.receiver.take().expect("Receiver should be available");

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        actor.shutdown_tx = Some(shutdown_tx);

        // Spawn the background worker task
        let backend = actor.backend.clone();
        let config = actor.config.clone();
        let metrics = actor.metrics.clone();
        let is_shutting_down = actor.is_shutting_down.clone();

        tokio::spawn(async move {
            TelemetryService::process_events(
                worker_rx,
                backend,
                config,
                metrics,
                shutdown_rx,
                is_shutting_down,
            )
            .await;
        });

        Self { actor, receiver }
    }

    async fn run(mut self) {
        info!(target: LOG_TARGET, "Starting telemetry service event loop");

        while let Some(message) = self.receiver.next().await {
            self.actor.handle_message(message).await;
        }

        info!(target: LOG_TARGET, "Telemetry service event loop stopped");

        // Shutdown the background worker
        self.actor.is_shutting_down.store(true, Ordering::Relaxed);
        if let Some(tx) = self.actor.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}
