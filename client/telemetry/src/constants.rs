//! Status constants and histogram bucket definitions for telemetry.

/// Log target for telemetry-related logging.
pub const LOG_TARGET: &str = "metrics";

/// Metric status label for successful operations.
pub const STATUS_SUCCESS: &str = "success";
/// Metric status label for failed operations.
pub const STATUS_FAILURE: &str = "failure";
/// Metric status label for pending operations.
pub const STATUS_PENDING: &str = "pending";

/// System metrics collection interval in seconds.
pub const SYSTEM_METRICS_INTERVAL_SECS: u64 = 5;

/// Fast CPU-bound operations (proof generation): 1ms to 5min.
/// Provides finer granularity for sub-10ms operations while also capturing longer operations.
pub const FAST_OP_BUCKETS: &[f64] = &[
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0,
];

/// Network I/O operations (file transfers/downloads): 100ms to 30min.
/// Extended range for large file operations that can take several minutes.
pub const TRANSFER_BUCKETS: &[f64] = &[
    0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1800.0,
];

/// General request processing: 10ms to 5min.
/// Balanced buckets for typical request-response patterns.
pub const REQUEST_BUCKETS: &[f64] = &[
    0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 300.0,
];
