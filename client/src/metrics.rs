//! Prometheus metrics for StorageHub client.
//!
//! This module provides a [`StorageHubMetrics`] struct that registers all metrics upfront
//! with a Prometheus [`Registry`]. Metrics are wrapped in [`MetricsLink`] to handle the
//! optional case when Prometheus is disabled.
//!
//! # Usage
//!
//! ```ignore
//! // In tasks, access metrics through the handler:
//! if let Some(m) = self.storage_hub_handler.metrics() {
//!     m.bytes_uploaded_total.with_label_values(&["success"]).inc();
//! }
//! ```

use shc_actors_framework::event_bus::LifecycleMetricRecorder;
use substrate_prometheus_endpoint::{
    register, CounterVec, Gauge, HistogramOpts, HistogramVec, Opts, PrometheusError, Registry, U64,
};
use sysinfo::{Pid, System};

pub const LOG_TARGET: &str = "storagehub::metrics";

/// System metrics collection interval in seconds.
const SYSTEM_METRICS_INTERVAL_SECS: u64 = 5;

/// Fast CPU-bound operations (proof generation): 1ms to 30s.
/// Provides finer granularity for sub-10ms operations.
const FAST_OP_BUCKETS: &[f64] = &[
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0,
];

/// Network I/O operations (file transfers/downloads): 100ms to 30min.
/// Extended range for large file operations that can take several minutes.
const TRANSFER_BUCKETS: &[f64] = &[
    0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1800.0,
];

/// General request processing: 10ms to 5min.
/// Balanced buckets for typical request-response patterns.
const REQUEST_BUCKETS: &[f64] = &[
    0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 300.0,
];

/// Metric status label for successful operations.
pub const STATUS_SUCCESS: &str = "success";
/// Metric status label for failed operations.
pub const STATUS_FAILURE: &str = "failure";
/// Metric status label for pending operations.
pub const STATUS_PENDING: &str = "pending";

/// Wrapper for optional metrics, similar to Substrate's `MetricsLink` pattern.
///
/// This wrapper allows metrics to be optional (when Prometheus is disabled) while
/// still providing a clean API for tasks to report metrics.
///
/// # Default
///
/// The default value has metrics disabled (`None`). Use [`MetricsLink::new`] with
/// a [`Registry`] to enable metrics collection.
#[derive(Clone, Default)]
pub struct MetricsLink(Option<StorageHubMetrics>);

impl MetricsLink {
    /// Creates a new [`MetricsLink`] from an optional [`Registry`].
    ///
    /// If the registry is `Some`, metrics are registered and a background task is spawned
    /// to periodically collect system metrics (CPU, memory). If registration fails,
    /// metrics will be `None` and a warning is logged.
    pub fn new(registry: Option<&Registry>) -> Self {
        match registry {
            Some(r) => match StorageHubMetrics::register(r) {
                Ok(metrics) => {
                    log::info!(target: LOG_TARGET, "StorageHub Prometheus metrics registered successfully");
                    let metrics_link = Self(Some(metrics));

                    // Spawn background task to collect system metrics
                    metrics_link.spawn_system_metrics_collector();

                    metrics_link
                }
                Err(e) => {
                    log::error!(target: LOG_TARGET, "Failed to register StorageHub Prometheus metrics: {}", e);
                    Self(None)
                }
            },
            None => {
                log::warn!(target: LOG_TARGET, "No Prometheus registry provided, StorageHub metrics disabled");
                Self(None)
            }
        }
    }

    /// Returns a reference to the metrics if available.
    #[must_use]
    pub fn as_ref(&self) -> Option<&StorageHubMetrics> {
        self.0.as_ref()
    }

    /// Spawns a background task that collects system metrics (CPU, memory) every 5 seconds.
    fn spawn_system_metrics_collector(&self) {
        let metrics = self.clone();
        tokio::spawn(async move {
            let mut system = System::new();
            let current_pid = Pid::from_u32(std::process::id());

            // Initial refresh to get accurate CPU readings on first iteration
            system.refresh_cpu_usage();
            system.refresh_memory();
            system.refresh_processes_specifics(
                sysinfo::ProcessesToUpdate::Some(&[current_pid]),
                true,
                sysinfo::ProcessRefreshKind::nothing()
                    .with_cpu()
                    .with_memory(),
            );

            loop {
                tokio::time::sleep(std::time::Duration::from_secs(SYSTEM_METRICS_INTERVAL_SECS))
                    .await;

                // Refresh system info
                system.refresh_cpu_usage();
                system.refresh_memory();
                system.refresh_processes_specifics(
                    sysinfo::ProcessesToUpdate::Some(&[current_pid]),
                    true,
                    sysinfo::ProcessRefreshKind::nothing()
                        .with_cpu()
                        .with_memory(),
                );

                if let Some(m) = metrics.as_ref() {
                    // System-wide CPU usage (average across cores)
                    m.system_cpu_usage_percent
                        .set(system.global_cpu_usage() as u64);

                    // System memory
                    m.system_memory_total_bytes.set(system.total_memory());
                    m.system_memory_used_bytes.set(system.used_memory());
                    m.system_memory_available_bytes
                        .set(system.available_memory());

                    // Process-specific metrics
                    if let Some(process) = system.process(current_pid) {
                        m.process_cpu_usage_percent.set(process.cpu_usage() as u64);
                        m.process_memory_rss_bytes.set(process.memory());
                    }
                }
            }
        });
    }
}

/// StorageHub Prometheus metrics.
///
/// All metrics are registered upfront to avoid duplicate registration errors.
/// Metrics follow Prometheus naming conventions:
/// - Prefix: `storagehub_`
/// - Counters: suffix `_total`
/// - Histograms: suffix `_seconds` or `_bytes`
#[derive(Clone)]
pub struct StorageHubMetrics {
    // === System Resource Metrics ===
    /// Current system-wide CPU usage percentage (0-100).
    /// Average across all CPU cores.
    pub system_cpu_usage_percent: Gauge<U64>,
    /// Total system memory in bytes.
    pub system_memory_total_bytes: Gauge<U64>,
    /// Used system memory in bytes.
    pub system_memory_used_bytes: Gauge<U64>,
    /// Available system memory in bytes.
    pub system_memory_available_bytes: Gauge<U64>,
    /// Current process CPU usage percentage.
    pub process_cpu_usage_percent: Gauge<U64>,
    /// Current process resident set size (RSS) in bytes.
    pub process_memory_rss_bytes: Gauge<U64>,

    // === Event Handler Lifecycle Metrics ===
    /// Total event handler invocations, labeled by event type and status.
    /// Automatically recorded by the event bus for all registered handlers.
    /// - `pending`: Event received and handler starting
    /// - `success`: Handler completed successfully
    /// - `failure`: Handler returned an error
    pub event_handler_total: CounterVec<U64>,
    /// Event handler processing duration, labeled by event type and status.
    /// Automatically recorded by the event bus on handler completion.
    pub event_handler_seconds: HistogramVec,

    // === BSP Metrics ===
    /// Time spent generating proofs for challenge responses, labeled by status.
    /// Measures the duration of generate_key_proof operations.
    pub bsp_proof_generation_seconds: HistogramVec,

    // === General Metrics ===
    /// Time spent sending file chunks to a peer (outbound transfer), labeled by status.
    /// Measures the duration of send_chunks operations used by MSP to distribute files to BSPs.
    /// Note: This tracks outbound transfers, not receiving uploads.
    pub file_transfer_seconds: HistogramVec,

    // === Download Metrics ===
    /// Total bytes downloaded from peers (inbound), labeled by status.
    /// Tracks bytes received during file downloads from other storage providers.
    pub bytes_downloaded_total: CounterVec<U64>,
    /// Total chunk batches downloaded from peers, labeled by status.
    /// Tracks the number of chunk batches successfully received during file downloads.
    pub chunks_downloaded_total: CounterVec<U64>,
    /// Time spent downloading a complete file from peers, labeled by status.
    /// Measures total duration from starting a file download to completion or failure.
    pub file_download_seconds: HistogramVec,

    // === Upload Metrics ===
    /// Total bytes received from upload requests (inbound), labeled by status.
    /// Tracks bytes received when users or MSPs upload file chunks to this provider.
    pub bytes_uploaded_total: CounterVec<U64>,
}

impl StorageHubMetrics {
    /// Registers all metrics with the given Prometheus [`Registry`].
    pub fn register(registry: &Registry) -> Result<Self, PrometheusError> {
        Ok(Self {
            // System resource metrics
            system_cpu_usage_percent: register(
                Gauge::new(
                    "storagehub_system_cpu_usage_percent",
                    "Current system-wide CPU usage percentage (0-100)",
                )?,
                registry,
            )?,
            system_memory_total_bytes: register(
                Gauge::new(
                    "storagehub_system_memory_total_bytes",
                    "Total system memory in bytes",
                )?,
                registry,
            )?,
            system_memory_used_bytes: register(
                Gauge::new(
                    "storagehub_system_memory_used_bytes",
                    "Used system memory in bytes",
                )?,
                registry,
            )?,
            system_memory_available_bytes: register(
                Gauge::new(
                    "storagehub_system_memory_available_bytes",
                    "Available system memory in bytes",
                )?,
                registry,
            )?,
            process_cpu_usage_percent: register(
                Gauge::new(
                    "storagehub_process_cpu_usage_percent",
                    "Current process CPU usage percentage",
                )?,
                registry,
            )?,
            process_memory_rss_bytes: register(
                Gauge::new(
                    "storagehub_process_memory_rss_bytes",
                    "Current process resident set size (RSS) in bytes",
                )?,
                registry,
            )?,
            // Event handler lifecycle metrics
            event_handler_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_event_handler_total",
                        "Event handler invocations by event type and status",
                    ),
                    &["event", "status"],
                )?,
                registry,
            )?,
            event_handler_seconds: register(
                HistogramVec::new(
                    HistogramOpts::new(
                        "storagehub_event_handler_seconds",
                        "Event handler processing duration by event type and status",
                    )
                    .buckets(REQUEST_BUCKETS.to_vec()),
                    &["event", "status"],
                )?,
                registry,
            )?,
            // BSP metrics
            bsp_proof_generation_seconds: register(
                HistogramVec::new(
                    HistogramOpts::new(
                        "storagehub_bsp_proof_generation_seconds",
                        "BSP proof generation duration for challenge responses",
                    )
                    .buckets(FAST_OP_BUCKETS.to_vec()),
                    &["status"],
                )?,
                registry,
            )?,
            // General metrics
            file_transfer_seconds: register(
                HistogramVec::new(
                    HistogramOpts::new(
                        "storagehub_file_transfer_seconds",
                        "Outbound file chunk transfer duration (sending to peers)",
                    )
                    .buckets(TRANSFER_BUCKETS.to_vec()),
                    &["status"],
                )?,
                registry,
            )?,
            // Download metrics
            bytes_downloaded_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_bytes_downloaded_total",
                        "Bytes downloaded from peers (inbound)",
                    ),
                    &["status"],
                )?,
                registry,
            )?,
            chunks_downloaded_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_chunks_downloaded_total",
                        "Chunk batches downloaded from peers",
                    ),
                    &["status"],
                )?,
                registry,
            )?,
            file_download_seconds: register(
                HistogramVec::new(
                    HistogramOpts::new(
                        "storagehub_file_download_seconds",
                        "Complete file download duration from peers",
                    )
                    .buckets(TRANSFER_BUCKETS.to_vec()),
                    &["status"],
                )?,
                registry,
            )?,
            // Upload metrics
            bytes_uploaded_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_bytes_uploaded_total",
                        "Bytes received from upload requests (inbound)",
                    ),
                    &["status"],
                )?,
                registry,
            )?,
        })
    }
}

/// Records event handler lifecycle metrics (pending/success/failure) with timing.
///
/// This struct implements [`LifecycleMetricRecorder`] and is used by the event bus
/// to automatically track handler invocations. Event names are derived from type
/// names (converted to snake_case) at subscription time.
///
/// # Example
///
/// ```ignore
/// // Created automatically by subscribe_actor_event_map! macro
/// let recorder = EventMetricRecorder::new(
///     metrics.clone(),
///     "new_storage_request",  // auto-derived from NewStorageRequest
/// );
/// ```
#[derive(Clone)]
pub struct EventMetricRecorder {
    metrics: MetricsLink,
    event_name: &'static str,
}

impl EventMetricRecorder {
    /// Creates a new [`EventMetricRecorder`] with the given labels.
    ///
    /// # Arguments
    ///
    /// * `metrics` - The metrics link (can be disabled)
    /// * `event_name` - The event type name in snake_case (e.g., "new_storage_request")
    pub fn new(metrics: MetricsLink, event_name: &'static str) -> Self {
        Self {
            metrics,
            event_name,
        }
    }
}

impl LifecycleMetricRecorder for EventMetricRecorder {
    fn record_pending(&self) {
        if let Some(m) = self.metrics.as_ref() {
            m.event_handler_total
                .with_label_values(&[self.event_name, STATUS_PENDING])
                .inc();
        }
    }

    fn record_success(&self, duration_secs: f64) {
        if let Some(m) = self.metrics.as_ref() {
            m.event_handler_total
                .with_label_values(&[self.event_name, STATUS_SUCCESS])
                .inc();
            m.event_handler_seconds
                .with_label_values(&[self.event_name, STATUS_SUCCESS])
                .observe(duration_secs);
        }
    }

    fn record_failure(&self, duration_secs: f64) {
        if let Some(m) = self.metrics.as_ref() {
            m.event_handler_total
                .with_label_values(&[self.event_name, STATUS_FAILURE])
                .inc();
            m.event_handler_seconds
                .with_label_values(&[self.event_name, STATUS_FAILURE])
                .observe(duration_secs);
        }
    }
}

/// Increments a counter metric if metrics are enabled.
///
/// # Example
///
/// ```ignore
/// // With handler (calls handler.metrics())
/// inc_counter!(handler: self.storage_hub_handler, bytes_downloaded_total, STATUS_SUCCESS);
///
/// // With direct metrics reference (Option<&StorageHubMetrics>)
/// inc_counter!(metrics: self.metrics.as_ref(), bytes_downloaded_total, STATUS_SUCCESS);
/// ```
#[macro_export]
macro_rules! inc_counter {
    // Handler pattern: calls $handler.metrics()
    (handler: $handler:expr, $metric:ident, $label:expr) => {
        if let Some(m) = $handler.metrics() {
            m.$metric.with_label_values(&[$label]).inc();
        }
    };
    // Direct pattern: accepts Option<&StorageHubMetrics> directly
    (metrics: $metrics:expr, $metric:ident, $label:expr) => {
        if let Some(m) = $metrics {
            m.$metric.with_label_values(&[$label]).inc();
        }
    };
}

/// Increments a counter metric by a specific amount if metrics are enabled.
///
/// # Example
///
/// ```ignore
/// // With handler (calls handler.metrics())
/// inc_counter_by!(handler: self.storage_hub_handler, bytes_uploaded_total, STATUS_SUCCESS, 1024);
///
/// // With direct metrics reference (Option<&StorageHubMetrics>)
/// inc_counter_by!(metrics: self.metrics.as_ref(), bytes_downloaded_total, STATUS_SUCCESS, 1024);
/// ```
#[macro_export]
macro_rules! inc_counter_by {
    // Handler pattern: calls $handler.metrics()
    (handler: $handler:expr, $metric:ident, $label:expr, $value:expr) => {
        if let Some(m) = $handler.metrics() {
            m.$metric.with_label_values(&[$label]).inc_by($value);
        }
    };
    // Direct pattern: accepts Option<&StorageHubMetrics> directly
    (metrics: $metrics:expr, $metric:ident, $label:expr, $value:expr) => {
        if let Some(m) = $metrics {
            m.$metric.with_label_values(&[$label]).inc_by($value);
        }
    };
}

/// Records an observation to a histogram metric if metrics are enabled.
///
/// # Example
///
/// ```ignore
/// // With handler (calls handler.metrics())
/// observe_histogram!(handler: self.storage_hub_handler, storage_request_setup_seconds, STATUS_SUCCESS, elapsed.as_secs_f64());
///
/// // With direct metrics reference (Option<&StorageHubMetrics>)
/// observe_histogram!(metrics: self.metrics.as_ref(), file_download_seconds, STATUS_SUCCESS, elapsed.as_secs_f64());
/// ```
#[macro_export]
macro_rules! observe_histogram {
    // Handler pattern: calls $handler.metrics()
    (handler: $handler:expr, $metric:ident, $label:expr, $value:expr) => {
        if let Some(m) = $handler.metrics() {
            m.$metric.with_label_values(&[$label]).observe($value);
        }
    };
    // Direct pattern: accepts Option<&StorageHubMetrics> directly
    (metrics: $metrics:expr, $metric:ident, $label:expr, $value:expr) => {
        if let Some(m) = $metrics {
            m.$metric.with_label_values(&[$label]).observe($value);
        }
    };
}
