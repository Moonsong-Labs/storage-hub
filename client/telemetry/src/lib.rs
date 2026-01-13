//! Prometheus telemetry for StorageHub client.
//!
//! This crate provides metrics infrastructure that can be shared across all StorageHub
//! client crates, enabling direct macro usage without trait abstractions.
//!
//! # Architecture
//!
//! - [`constants`] - Status labels, histogram buckets, and configuration values
//! - [`macros`] - Helper macros for recording metrics (`inc_counter!`, `observe_histogram!`, etc.)
//! - [`link`] - [`MetricsLink`] wrapper for optional metrics
//! - [`metrics`] - [`StorageHubMetrics`] definitions and system metrics collection
//!
//! # Usage
//!
//! ```ignore
//! use shc_telemetry::{MetricsLink, inc_counter, observe_histogram, STATUS_SUCCESS};
//!
//! // Record a counter
//! inc_counter!(metrics: metrics_link.as_ref(), bytes_uploaded_total, STATUS_SUCCESS);
//!
//! // Record a histogram observation
//! observe_histogram!(metrics: metrics_link.as_ref(), file_transfer_seconds, STATUS_SUCCESS, elapsed.as_secs_f64());
//! ```

pub mod constants;
pub mod link;
pub mod metrics;

// Macros are defined here with #[macro_export] so they're available at crate root
#[macro_use]
mod macros;

// Re-export public API for convenience
pub use constants::{LOG_TARGET, STATUS_FAILURE, STATUS_PENDING, STATUS_SUCCESS};
pub use link::MetricsLink;
pub use metrics::StorageHubMetrics;
