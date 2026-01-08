//! Metrics helper macros for recording telemetry.
//!
//! These macros provide a convenient way to record metrics with optional metrics support.

/// Increments a counter metric if metrics are enabled.
///
/// # Example
///
/// ```ignore
/// // With handler (calls handler.metrics())
/// inc_counter!(handler: self.storage_hub_handler, bytes_uploaded_total, STATUS_SUCCESS);
///
/// // With direct metrics reference (Option<&StorageHubMetrics>)
/// inc_counter!(metrics: self.metrics.as_ref(), bytes_uploaded_total, STATUS_SUCCESS);
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
    // Direct pattern with multiple labels: use `labels:` keyword to pass &[&str] directly
    (metrics: $metrics:expr, $metric:ident, labels: $labels:expr) => {
        if let Some(m) = $metrics {
            m.$metric.with_label_values($labels).inc();
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
/// inc_counter_by!(metrics: self.metrics.as_ref(), bytes_uploaded_total, STATUS_SUCCESS, 1024);
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
/// // With handler (calls handler.metrics()) - single label
/// observe_histogram!(handler: self.storage_hub_handler, bsp_proof_generation_seconds, STATUS_SUCCESS, elapsed.as_secs_f64());
///
/// // With direct metrics reference (Option<&StorageHubMetrics>) - single label
/// observe_histogram!(metrics: self.metrics.as_ref(), file_transfer_seconds, STATUS_SUCCESS, elapsed.as_secs_f64());
///
/// // With direct metrics reference - multiple labels (use `labels:` keyword)
/// observe_histogram!(metrics: self.metrics.as_ref(), block_processing_seconds, labels: &["block_import", STATUS_SUCCESS], elapsed.as_secs_f64());
/// ```
#[macro_export]
macro_rules! observe_histogram {
    // Handler pattern: calls $handler.metrics() - single label
    (handler: $handler:expr, $metric:ident, $label:expr, $value:expr) => {
        if let Some(m) = $handler.metrics() {
            m.$metric.with_label_values(&[$label]).observe($value);
        }
    };
    // Direct pattern: accepts Option<&StorageHubMetrics> directly - single label
    (metrics: $metrics:expr, $metric:ident, $label:expr, $value:expr) => {
        if let Some(m) = $metrics {
            m.$metric.with_label_values(&[$label]).observe($value);
        }
    };
    // Direct pattern with multiple labels: use `labels:` keyword to pass &[&str] directly
    (metrics: $metrics:expr, $metric:ident, labels: $labels:expr, $value:expr) => {
        if let Some(m) = $metrics {
            m.$metric.with_label_values($labels).observe($value);
        }
    };
}

/// Decrements a gauge metric if metrics are enabled.
///
/// # Example
///
/// ```ignore
/// // With direct metrics reference (Option<&StorageHubMetrics>)
/// dec_gauge!(metrics: self.metrics.as_ref(), command_pending, "SomeCommand");
/// ```
#[macro_export]
macro_rules! dec_gauge {
    // Direct pattern: accepts Option<&StorageHubMetrics> directly
    (metrics: $metrics:expr, $metric:ident, $label:expr) => {
        if let Some(m) = $metrics {
            m.$metric.with_label_values(&[$label]).dec();
        }
    };
}
