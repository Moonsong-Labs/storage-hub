//! MetricsLink - Wrapper for optional metrics.

use substrate_prometheus_endpoint::Registry;

use crate::constants::LOG_TARGET;
use crate::metrics::{spawn_system_metrics_collector, StorageHubMetrics};

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
                    spawn_system_metrics_collector(metrics_link.clone());

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
}
