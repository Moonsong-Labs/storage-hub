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
//!     m.bsp_storage_requests_total.with_label_values(&["success"]).inc();
//! }
//! ```

use std::sync::Arc;

use substrate_prometheus_endpoint::{
    register, CounterVec, HistogramOpts, HistogramVec, Opts, PrometheusError, Registry, U64,
};

/// Duration buckets for histograms (in seconds): 10ms to 5min.
const DURATION_BUCKETS: &[f64] = &[
    0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 300.0,
];

/// Metric status label for successful operations.
pub const STATUS_SUCCESS: &str = "success";
/// Metric status label for failed operations.
pub const STATUS_FAILURE: &str = "failure";
/// Metric status label for pending operations.
pub const STATUS_PENDING: &str = "pending";

/// Increments a counter metric if metrics are enabled.
///
/// # Example
///
/// ```ignore
/// inc_counter!(self.storage_hub_handler, bsp_storage_requests_total, STATUS_SUCCESS);
/// ```
#[macro_export]
macro_rules! inc_counter {
    ($handler:expr, $metric:ident, $label:expr) => {
        if let Some(m) = $handler.metrics() {
            m.$metric.with_label_values(&[$label]).inc();
        }
    };
}

/// Increments a counter metric by a specific amount if metrics are enabled.
///
/// # Example
///
/// ```ignore
/// inc_counter_by!(self.storage_hub_handler, bsp_storage_requests_total, STATUS_SUCCESS, 5);
/// ```
#[macro_export]
macro_rules! inc_counter_by {
    ($handler:expr, $metric:ident, $label:expr, $value:expr) => {
        if let Some(m) = $handler.metrics() {
            m.$metric.with_label_values(&[$label]).inc_by($value);
        }
    };
}

/// Records an observation to a histogram metric if metrics are enabled.
///
/// # Example
///
/// ```ignore
/// observe_histogram!(self.storage_hub_handler, storage_request_seconds, STATUS_SUCCESS, elapsed.as_secs_f64());
/// ```
#[macro_export]
macro_rules! observe_histogram {
    ($handler:expr, $metric:ident, $label:expr, $value:expr) => {
        if let Some(m) = $handler.metrics() {
            m.$metric.with_label_values(&[$label]).observe($value);
        }
    };
}

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
pub struct MetricsLink(Arc<Option<StorageHubMetrics>>);

impl MetricsLink {
    /// Creates a new [`MetricsLink`] from an optional [`Registry`].
    ///
    /// If the registry is `Some`, metrics are registered. If registration fails,
    /// metrics will be `None` and a warning is logged.
    pub fn new(registry: Option<&Registry>) -> Self {
        match registry {
            Some(r) => match StorageHubMetrics::register(r) {
                Ok(metrics) => {
                    log::info!("StorageHub Prometheus metrics registered successfully");
                    Self(Arc::new(Some(metrics)))
                }
                Err(e) => {
                    log::warn!("Failed to register StorageHub Prometheus metrics: {}", e);
                    Self(Arc::new(None))
                }
            },
            None => {
                log::warn!("No Prometheus registry provided, StorageHub metrics disabled");
                Self(Arc::new(None))
            }
        }
    }

    /// Returns a reference to the metrics if available.
    #[must_use]
    pub fn as_ref(&self) -> Option<&StorageHubMetrics> {
        (*self.0).as_ref()
    }
}

/// StorageHub Prometheus metrics.
///
/// All metrics are registered upfront to avoid duplicate registration errors.
/// Metrics follow Prometheus naming conventions:
/// - Prefix: `storagehub_`
/// - Counters: suffix `_total`
/// - Histograms: suffix `_seconds` or `_bytes`
pub struct StorageHubMetrics {
    // === BSP Metrics ===
    /// Total storage requests processed by BSP, labeled by status.
    pub bsp_storage_requests_total: CounterVec<U64>,

    /// Total proofs submitted by BSP, labeled by status.
    pub bsp_proofs_submitted_total: CounterVec<U64>,

    /// Total fees charged by BSP, labeled by status.
    pub bsp_fees_charged_total: CounterVec<U64>,

    /// Total files deleted by BSP, labeled by status.
    pub bsp_files_deleted_total: CounterVec<U64>,

    /// Total bucket moves processed by BSP, labeled by status.
    pub bsp_bucket_moves_total: CounterVec<U64>,

    /// BSP proof generation duration in seconds, labeled by status.
    pub bsp_proof_generation_seconds: HistogramVec,

    // === MSP Metrics ===
    /// Total storage requests processed by MSP, labeled by status.
    pub msp_storage_requests_total: CounterVec<U64>,

    /// Total files distributed by MSP, labeled by status.
    pub msp_files_distributed_total: CounterVec<U64>,

    /// Total files deleted by MSP, labeled by status.
    pub msp_files_deleted_total: CounterVec<U64>,

    /// Total buckets deleted by MSP, labeled by status.
    pub msp_buckets_deleted_total: CounterVec<U64>,

    /// Total fees charged by MSP, labeled by status.
    pub msp_fees_charged_total: CounterVec<U64>,

    /// Total bucket moves processed by MSP, labeled by status.
    pub msp_bucket_moves_total: CounterVec<U64>,

    // === General Metrics ===
    /// Storage request processing duration in seconds, labeled by status.
    pub storage_request_seconds: HistogramVec,

    /// File transfer duration in seconds, labeled by status.
    pub file_transfer_seconds: HistogramVec,

    /// Total insolvent users processed, labeled by status.
    pub insolvent_users_processed_total: CounterVec<U64>,

    /// Total batch deletions processed by fisherman, labeled by status.
    pub fisherman_batch_deletions_total: CounterVec<U64>,

    // === Download Metrics ===
    /// Total bytes downloaded from peers, labeled by status.
    pub bytes_downloaded_total: CounterVec<U64>,

    /// Total chunks downloaded from peers, labeled by status.
    pub chunks_downloaded_total: CounterVec<U64>,

    /// File download duration in seconds, labeled by status.
    pub file_download_seconds: HistogramVec,
}

impl StorageHubMetrics {
    /// Registers all metrics with the given Prometheus [`Registry`].
    pub fn register(registry: &Registry) -> Result<Self, PrometheusError> {
        Ok(Self {
            // BSP metrics
            bsp_storage_requests_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_bsp_storage_requests_total",
                        "Total number of storage requests processed by BSP",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            bsp_proofs_submitted_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_bsp_proofs_submitted_total",
                        "Total number of proofs submitted by BSP",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            bsp_fees_charged_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_bsp_fees_charged_total",
                        "Total number of fee charges processed by BSP",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            bsp_files_deleted_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_bsp_files_deleted_total",
                        "Total number of files deleted by BSP",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            bsp_bucket_moves_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_bsp_bucket_moves_total",
                        "Total number of bucket moves processed by BSP",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            bsp_proof_generation_seconds: register(
                HistogramVec::new(
                    HistogramOpts::new(
                        "storagehub_bsp_proof_generation_seconds",
                        "Time spent generating proofs by BSP",
                    )
                    .buckets(DURATION_BUCKETS.to_vec()),
                    &["status"],
                )?,
                registry,
            )?,

            // MSP metrics
            msp_storage_requests_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_msp_storage_requests_total",
                        "Total number of storage requests processed by MSP",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            msp_files_distributed_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_msp_files_distributed_total",
                        "Total number of files distributed by MSP to BSPs",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            msp_files_deleted_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_msp_files_deleted_total",
                        "Total number of files deleted by MSP",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            msp_buckets_deleted_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_msp_buckets_deleted_total",
                        "Total number of buckets deleted by MSP",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            msp_fees_charged_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_msp_fees_charged_total",
                        "Total number of fee charges processed by MSP",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            msp_bucket_moves_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_msp_bucket_moves_total",
                        "Total number of bucket moves processed by MSP",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            // General metrics
            storage_request_seconds: register(
                HistogramVec::new(
                    HistogramOpts::new(
                        "storagehub_storage_request_seconds",
                        "Time spent processing storage requests",
                    )
                    .buckets(DURATION_BUCKETS.to_vec()),
                    &["status"],
                )?,
                registry,
            )?,

            file_transfer_seconds: register(
                HistogramVec::new(
                    HistogramOpts::new(
                        "storagehub_file_transfer_seconds",
                        "Time spent transferring files",
                    )
                    .buckets(DURATION_BUCKETS.to_vec()),
                    &["status"],
                )?,
                registry,
            )?,

            insolvent_users_processed_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_insolvent_users_processed_total",
                        "Total number of insolvent users processed",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            fisherman_batch_deletions_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_fisherman_batch_deletions_total",
                        "Total number of batch deletions processed by fisherman",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            // Download metrics
            bytes_downloaded_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_bytes_downloaded_total",
                        "Total bytes downloaded from peers",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            chunks_downloaded_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_chunks_downloaded_total",
                        "Total chunks downloaded from peers",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            file_download_seconds: register(
                HistogramVec::new(
                    HistogramOpts::new(
                        "storagehub_file_download_seconds",
                        "Time spent downloading files from peers",
                    )
                    .buckets(DURATION_BUCKETS.to_vec()),
                    &["status"],
                )?,
                registry,
            )?,
        })
    }
}
