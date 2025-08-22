//! Task context module for tracking task lifecycle and metrics.
//!
//! This module provides a context structure that tracks the lifecycle of tasks,
//! including task IDs, timing information, and correlation IDs for distributed tracing.

use std::time::{Duration, Instant};
use uuid::Uuid;

/// Context for tracking task execution lifecycle and metrics.
///
/// The [`TaskContext`] provides a standardized way to track task execution,
/// generate unique identifiers, and measure performance metrics across all
/// storage hub tasks.
#[derive(Debug, Clone)]
pub struct TaskContext {
    /// Unique identifier for this task instance
    pub task_id: String,
    /// Name of the task type (e.g., "bsp_upload_file")
    pub task_name: String,
    /// Correlation ID for tracing related operations
    pub correlation_id: Option<String>,
    /// Task start time for duration tracking
    start_time: Instant,
}

impl TaskContext {
    /// Create a new task context with the given task name.
    ///
    /// Generates a unique task ID and captures the start time.
    pub fn new(task_name: impl Into<String>) -> Self {
        Self {
            task_id: Uuid::new_v4().to_string(),
            task_name: task_name.into(),
            correlation_id: None,
            start_time: Instant::now(),
        }
    }

    /// Create a new task context with a correlation ID.
    ///
    /// Use this when the task is part of a larger operation that should be traced together.
    pub fn with_correlation(task_name: impl Into<String>, correlation_id: String) -> Self {
        let mut ctx = Self::new(task_name);
        ctx.correlation_id = Some(correlation_id);
        ctx
    }

    /// Get the elapsed time since the task started in milliseconds.
    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    /// Get the elapsed duration since the task started.
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Generate a new correlation ID for tracking related operations.
    pub fn generate_correlation_id() -> String {
        Uuid::new_v4().to_string()
    }

    /// Set or update the correlation ID.
    pub fn set_correlation_id(&mut self, correlation_id: String) {
        self.correlation_id = Some(correlation_id);
    }
}

/// Helper function to classify errors into categories for telemetry.
///
/// This provides consistent error categorization across all tasks.
pub fn classify_error(error: &anyhow::Error) -> String {
    let error_str = error.to_string().to_lowercase();
    
    if error_str.contains("network") || error_str.contains("connection") {
        "network_error".to_string()
    } else if error_str.contains("timeout") {
        "timeout_error".to_string()
    } else if error_str.contains("permission") || error_str.contains("unauthorized") {
        "permission_error".to_string()
    } else if error_str.contains("storage") || error_str.contains("disk") {
        "storage_error".to_string()
    } else if error_str.contains("proof") || error_str.contains("verification") {
        "proof_error".to_string()
    } else if error_str.contains("blockchain") || error_str.contains("extrinsic") {
        "blockchain_error".to_string()
    } else if error_str.contains("capacity") || error_str.contains("full") {
        "capacity_error".to_string()
    } else {
        "unknown_error".to_string()
    }
}

/// Calculate transfer rate in Mbps given bytes and duration.
pub fn calculate_transfer_rate_mbps(bytes: u64, duration: Duration) -> f64 {
    if duration.as_secs_f64() == 0.0 {
        return 0.0;
    }
    
    let bits = (bytes * 8) as f64;
    let megabits = bits / 1_000_000.0;
    let seconds = duration.as_secs_f64();
    
    megabits / seconds
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_task_context_creation() {
        let ctx = TaskContext::new("test_task");
        
        assert_eq!(ctx.task_name, "test_task");
        assert!(!ctx.task_id.is_empty());
        assert!(ctx.correlation_id.is_none());
    }

    #[test]
    fn test_task_context_with_correlation() {
        let correlation_id = TaskContext::generate_correlation_id();
        let ctx = TaskContext::with_correlation("test_task", correlation_id.clone());
        
        assert_eq!(ctx.task_name, "test_task");
        assert_eq!(ctx.correlation_id, Some(correlation_id));
    }

    #[test]
    fn test_elapsed_time() {
        let ctx = TaskContext::new("test_task");
        thread::sleep(Duration::from_millis(100));
        
        let elapsed_ms = ctx.elapsed_ms();
        assert!(elapsed_ms >= 100);
        assert!(elapsed_ms < 200); // Allow some margin
    }

    #[test]
    fn test_error_classification() {
        let network_error = anyhow::anyhow!("Network connection failed");
        assert_eq!(classify_error(&network_error), "network_error");
        
        let timeout_error = anyhow::anyhow!("Operation timeout exceeded");
        assert_eq!(classify_error(&timeout_error), "timeout_error");
        
        let storage_error = anyhow::anyhow!("Disk storage full");
        assert_eq!(classify_error(&storage_error), "storage_error");
        
        let unknown_error = anyhow::anyhow!("Something went wrong");
        assert_eq!(classify_error(&unknown_error), "unknown_error");
    }

    #[test]
    fn test_transfer_rate_calculation() {
        let bytes = 10_000_000; // 10 MB
        let duration = Duration::from_secs(2);
        
        let rate = calculate_transfer_rate_mbps(bytes, duration);
        // 10 MB * 8 = 80 Mb / 2s = 40 Mbps
        assert!((rate - 40.0).abs() < 0.01);
    }
}