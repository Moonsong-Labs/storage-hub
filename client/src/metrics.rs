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

pub const LOG_TARGET: &str = "storagehub::metrics";

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
                    log::info!(target: LOG_TARGET, "StorageHub Prometheus metrics registered successfully");
                    Self(Arc::new(Some(metrics)))
                }
                Err(e) => {
                    log::error!(target: LOG_TARGET, "Failed to register StorageHub Prometheus metrics: {}", e);
                    Self(Arc::new(None))
                }
            },
            None => {
                log::warn!(target: LOG_TARGET, "No Prometheus registry provided, StorageHub metrics disabled");
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
    /// Total BSP storage request confirmations, labeled by status.
    /// Tracks the full lifecycle from receiving NewStorageRequest through submitting ConfirmStoring extrinsic.
    /// - `pending`: NewStorageRequest event received
    /// - `success`: ConfirmStoring extrinsic submitted successfully
    /// - `failure`: Any error during volunteer or confirmation process
    pub bsp_storage_requests_total: CounterVec<U64>,

    /// Total proof submission attempts by BSP, labeled by status.
    /// Tracks SubmitProof extrinsic submissions in response to challenges.
    pub bsp_proofs_submitted_total: CounterVec<U64>,

    /// Total fee charge extrinsic submissions by BSP, labeled by status.
    /// Tracks attempts to submit ChargePaymentStreams extrinsic for users with debt.
    /// Note: Success means extrinsic was submitted, not necessarily that fees were collected.
    pub bsp_fees_charged_total: CounterVec<U64>,

    /// Total file deletion events processed by BSP, labeled by status.
    /// Tracks TrieRemoveMutation events where files are removed from BSP's forest storage.
    pub bsp_files_deleted_total: CounterVec<U64>,

    /// Total bucket move events processed by BSP, labeled by status.
    /// Tracks MoveBucketRequested/Accepted/Rejected/Expired events.
    /// - `pending`: MoveBucketRequested received
    /// - `success`: MoveBucketAccepted processed
    /// - `failure`: MoveBucketRejected or MoveBucketExpired processed
    pub bsp_bucket_moves_total: CounterVec<U64>,

    /// Total file download requests handled by BSP, labeled by status.
    /// Tracks FileDownloadRequest events where BSP serves files to requesters.
    pub bsp_download_requests_total: CounterVec<U64>,

    /// Total chunk upload batches received by BSP, labeled by status.
    /// Tracks RemoteUploadRequest events (each event contains a batch of chunks).
    /// Note: This counts batches, not individual chunks.
    pub bsp_upload_chunks_received_total: CounterVec<U64>,

    /// Time spent generating proofs for challenge responses, labeled by status.
    /// Measures the duration of generate_key_proof operations.
    pub bsp_proof_generation_seconds: HistogramVec,

    // === MSP Metrics ===
    /// Total MSP storage request responses, labeled by status.
    /// Tracks the lifecycle from receiving NewStorageRequest through submitting RespondStorageRequest extrinsic.
    /// - `pending`: NewStorageRequest event received
    /// - `success`: RespondStorageRequest extrinsic submitted successfully
    /// - `failure`: Any error during setup or response process
    pub msp_storage_requests_total: CounterVec<U64>,

    /// Total file distribution attempts by MSP to BSPs, labeled by status.
    /// Tracks DistributeFileToBsp events where MSP sends files to volunteered BSPs.
    pub msp_files_distributed_total: CounterVec<U64>,

    /// Total file deletion events processed by MSP, labeled by status.
    /// Tracks removal of files from MSP's file storage after forest mutations are finalized.
    pub msp_files_deleted_total: CounterVec<U64>,

    /// Total bucket deletion events processed by MSP, labeled by status.
    /// Tracks bucket deletions after move, stop storing, or insolvent user events.
    pub msp_buckets_deleted_total: CounterVec<U64>,

    /// Total fee charge extrinsic submissions by MSP, labeled by status.
    /// Tracks attempts to submit ChargePaymentStreams extrinsic for users with debt.
    /// Note: Success means extrinsic was submitted, not necessarily that fees were collected.
    pub msp_fees_charged_total: CounterVec<U64>,

    /// Total bucket move events processed by MSP, labeled by status.
    /// Tracks the bucket download process when taking over from another MSP.
    /// - `pending`: MoveBucketRequested received
    /// - `success`: All files downloaded and bucket move complete
    /// - `failure`: Download failed or bucket move rejected
    pub msp_bucket_moves_total: CounterVec<U64>,

    /// Total bucket move retry attempts by MSP, labeled by status.
    /// Tracks retry attempts for incomplete bucket downloads during MSP move.
    pub msp_bucket_move_retries_total: CounterVec<U64>,

    /// Total bucket forest verification runs by MSP, labeled by status.
    /// Tracks periodic verification that local forest storage exists for all managed buckets.
    pub msp_forest_verifications_total: CounterVec<U64>,

    /// Time spent verifying all bucket forests exist, labeled by status.
    /// Measures the duration of the periodic bucket forest verification task.
    pub msp_forest_verification_seconds: HistogramVec,

    // === SP Metrics ===
    /// Total slash extrinsic submissions by any SP, labeled by status.
    /// Tracks attempts to submit slash extrinsics for slashable providers.
    pub sp_slash_submissions_total: CounterVec<U64>,

    // === General Metrics ===
    /// Time spent handling the initial NewStorageRequest event in seconds.
    /// For BSP: Includes validation, waiting for volunteer tick eligibility, and sending volunteer extrinsic.
    /// For MSP: Includes validation, capacity checks, and file storage setup.
    /// Note: This does NOT include file transfer time or the actual accept/confirm response.
    pub storage_request_setup_seconds: HistogramVec,

    /// Time spent sending file chunks to a peer (outbound transfer), labeled by status.
    /// Measures the duration of send_chunks operations used by MSP to distribute files to BSPs.
    /// Note: This tracks outbound transfers, not receiving uploads.
    pub file_transfer_seconds: HistogramVec,

    /// Total insolvent user processing events, labeled by status.
    /// Tracks UserWithoutFunds and FinalisedMspStopStoringBucketInsolventUser events.
    /// Used by both BSP and MSP when handling users who can no longer pay.
    pub insolvent_users_processed_total: CounterVec<U64>,

    /// Total batch deletion processing results by fisherman, labeled by status.
    /// Tracks the outcome of processing batches of pending file deletions.
    /// Success means all deletions in the batch were processed successfully.
    pub fisherman_batch_deletions_total: CounterVec<U64>,

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
            // BSP metrics
            bsp_storage_requests_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_bsp_storage_requests_total",
                        "BSP storage request confirmations (volunteer to confirm lifecycle)",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            bsp_proofs_submitted_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_bsp_proofs_submitted_total",
                        "BSP proof submission attempts in response to challenges",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            bsp_fees_charged_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_bsp_fees_charged_total",
                        "BSP fee charge extrinsic submissions",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            bsp_files_deleted_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_bsp_files_deleted_total",
                        "BSP file deletion events (TrieRemoveMutation)",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            bsp_bucket_moves_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_bsp_bucket_moves_total",
                        "BSP bucket move events (requested/accepted/rejected/expired)",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            bsp_download_requests_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_bsp_download_requests_total",
                        "BSP file download requests served to requesters",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            bsp_upload_chunks_received_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_bsp_upload_chunks_received_total",
                        "BSP chunk upload batches received (per batch, not per chunk)",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

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

            // MSP metrics
            msp_storage_requests_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_msp_storage_requests_total",
                        "MSP storage request responses (setup to respond lifecycle)",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            msp_files_distributed_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_msp_files_distributed_total",
                        "MSP file distribution attempts to BSPs",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            msp_files_deleted_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_msp_files_deleted_total",
                        "MSP file deletions from file storage",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            msp_buckets_deleted_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_msp_buckets_deleted_total",
                        "MSP bucket deletions (move/stop-storing/insolvent events)",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            msp_fees_charged_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_msp_fees_charged_total",
                        "MSP fee charge extrinsic submissions",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            msp_bucket_moves_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_msp_bucket_moves_total",
                        "MSP bucket move downloads (taking over from another MSP)",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            msp_bucket_move_retries_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_msp_bucket_move_retries_total",
                        "MSP bucket move download retry attempts",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            msp_forest_verifications_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_msp_forest_verifications_total",
                        "MSP bucket forest verification runs",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            msp_forest_verification_seconds: register(
                HistogramVec::new(
                    HistogramOpts::new(
                        "storagehub_msp_forest_verification_seconds",
                        "MSP bucket forest verification duration",
                    )
                    .buckets(FAST_OP_BUCKETS.to_vec()),
                    &["status"],
                )?,
                registry,
            )?,

            // SP metrics
            sp_slash_submissions_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_sp_slash_submissions_total",
                        "SP slash extrinsic submissions for slashable providers",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            // General metrics
            storage_request_setup_seconds: register(
                HistogramVec::new(
                    HistogramOpts::new(
                        "storagehub_storage_request_setup_seconds",
                        "Initial NewStorageRequest handling (validation, setup, not file transfer)",
                    )
                    .buckets(REQUEST_BUCKETS.to_vec()),
                    &["status"],
                )?,
                registry,
            )?,

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

            insolvent_users_processed_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_insolvent_users_processed_total",
                        "Insolvent user processing events (BSP and MSP)",
                    ),
                    &["status"],
                )?,
                registry,
            )?,

            fisherman_batch_deletions_total: register(
                CounterVec::new(
                    Opts::new(
                        "storagehub_fisherman_batch_deletions_total",
                        "Fisherman batch deletion processing results",
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

/// Increments a counter metric if metrics are enabled.
///
/// # Example
///
/// ```ignore
/// // With handler (calls handler.metrics())
/// inc_counter!(handler: self.storage_hub_handler, bsp_storage_requests_total, STATUS_SUCCESS);
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
/// inc_counter_by!(handler: self.storage_hub_handler, bsp_storage_requests_total, STATUS_SUCCESS, 5);
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
