//! StorageHub Prometheus metrics definitions and system metrics collection.

use substrate_prometheus_endpoint::{
    register, CounterVec, Gauge, GaugeVec, HistogramOpts, HistogramVec, Opts, PrometheusError,
    Registry, U64,
};
use sysinfo::{Pid, System};

use crate::constants::{
    FAST_OP_BUCKETS, REQUEST_BUCKETS, SYSTEM_METRICS_INTERVAL_SECS, TRANSFER_BUCKETS,
};
use crate::link::MetricsLink;

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

    // === Blockchain Service Metrics ===
    /// Block processing duration by operation type and status.
    /// Labels: `["operation", "status"]`
    /// Operations:
    /// - `"block_import"` - Blockchain service processing imported blocks (post-sync)
    /// - `"finalized_block"` - Blockchain service processing finalized blocks
    pub block_processing_seconds: HistogramVec,
    /// Command processing duration by command type and status.
    /// Labels: `["command", "status"]`
    /// Tracks time from command receipt to completion.
    /// - `success`: Command completed successfully
    /// - `failure`: Command returned an error
    pub command_processing_seconds: HistogramVec,

    // === Event Handler Lifecycle Metrics ===
    /// Currently in-flight event handlers by event type (pending gauge).
    /// Labels: `["event"]`
    /// Incremented when event received, decremented on completion.
    pub event_handler_pending: GaugeVec<U64>,
    /// Total event handler invocations by event type and status.
    /// Labels: `["event", "status"]`
    /// Automatically recorded by the event bus for all registered handlers.
    /// - `success`: Handler completed successfully
    /// - `failure`: Handler returned an error
    pub event_handler_total: CounterVec<U64>,
    /// Event handler processing duration by event type and status.
    /// Labels: `["event", "status"]`
    /// Automatically recorded by the event bus on handler completion.
    pub event_handler_seconds: HistogramVec,

    // === BSP Metrics ===
    /// Time spent generating proofs for challenge responses.
    /// Labels: `["status"]`
    /// Measures the duration of generate_key_proof operations.
    pub bsp_proof_generation_seconds: HistogramVec,

    // === General Metrics ===
    /// Time spent sending file chunks to a peer (outbound transfer).
    /// Labels: `["status"]`
    /// Measures the duration of send_chunks operations used by MSP to distribute files to BSPs.
    /// Note: This tracks outbound transfers, not receiving uploads.
    pub file_transfer_seconds: HistogramVec,

    // === Upload Metrics ===
    /// Total bytes received from upload requests (inbound).
    /// Labels: `["status"]`
    /// Tracks bytes received when users or MSPs upload file chunks to this provider.
    /// Use `rate()` in PromQL for throughput analysis (bytes/sec).
    pub bytes_uploaded_total: CounterVec<U64>,

    // === MSP Data Transfer Metrics ===
    /// Total bytes received by MSP from users (inbound).
    /// Labels: `["status"]`
    /// Tracks bytes received when users upload files to MSP.
    /// Use `rate()` in PromQL for throughput analysis (bytes/sec).
    pub msp_bytes_received_total: CounterVec<U64>,
    /// Total bytes sent by MSP to BSPs (outbound).
    /// Labels: `["status"]`
    /// Tracks bytes sent when MSP distributes files to BSPs.
    /// Use `rate()` in PromQL for throughput analysis (bytes/sec).
    pub msp_bytes_sent_total: CounterVec<U64>,
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
            // Command processing metrics (across all services)
            command_processing_seconds: register(
                HistogramVec::new(
                    HistogramOpts::new(
                        "storagehub_command_processing_seconds",
                        "Command processing duration by command type and status",
                    )
                    .buckets(FAST_OP_BUCKETS.to_vec()),
                    &["command", "status"],
                )?,
                registry,
            )?,
            block_processing_seconds: register(
                HistogramVec::new(
                    HistogramOpts::new(
                        "storagehub_block_processing_seconds",
                        "Block processing duration by operation type and status",
                    )
                    .buckets(FAST_OP_BUCKETS.to_vec()),
                    &["operation", "status"],
                )?,
                registry,
            )?,
            // Event handler lifecycle metrics
            event_handler_pending: register(
                GaugeVec::new(
                    Opts::new(
                        "storagehub_event_handler_pending",
                        "Currently in-flight event handlers by event type",
                    ),
                    &["event"],
                )?,
                registry,
            )?,
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
            // MSP data transfer metrics
            msp_bytes_received_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_msp_bytes_received_total",
                        "Bytes received by MSP from users (inbound)",
                    ),
                    &["status"],
                )?,
                registry,
            )?,
            msp_bytes_sent_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_msp_bytes_sent_total",
                        "Bytes sent by MSP to BSPs (outbound)",
                    ),
                    &["status"],
                )?,
                registry,
            )?,
        })
    }
}

/// Spawns a background task that collects system metrics (CPU, memory) every 5 seconds.
///
/// This function should be called once after metrics are successfully registered.
/// The task runs indefinitely, updating system resource gauges at regular intervals.
pub fn spawn_system_metrics_collector(metrics: MetricsLink) {
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
            tokio::time::sleep(std::time::Duration::from_secs(SYSTEM_METRICS_INTERVAL_SECS)).await;

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
