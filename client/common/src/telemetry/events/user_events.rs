//! User telemetry events.
//!
//! This module defines all telemetry events related to user operations including
//! file sending, bucket management, and payment operations.

use crate::telemetry::{BaseTelemetryEvent, TelemetryEvent, TelemetryStrategy};
use serde::{Deserialize, Serialize};

/// Event sent when a user initiates sending a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSendFileStartedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub task_name: String,
    /// User account ID
    pub user_id: String,
    pub file_key: String,
    pub file_size_bytes: u64,
    /// Target bucket ID
    pub bucket_id: String,
    /// Selected MSP for storage
    pub msp_id: String,
    /// Number of BSPs for redundancy
    pub bsp_count: u32,
}

impl TelemetryEvent for UserSendFileStartedEvent {
    fn event_type(&self) -> &str {
        "user_send_file_started"
    }
}

/// Event sent when user file send completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSendFileCompletedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub task_name: String,
    pub user_id: String,
    pub file_key: String,
    pub file_size_bytes: u64,
    pub duration_ms: u64,
    pub average_transfer_rate_mbps: f64,
    /// Storage cost for this file
    pub storage_cost: u128,
}

impl TelemetryEvent for UserSendFileCompletedEvent {
    fn event_type(&self) -> &str {
        "user_send_file_completed"
    }
}

/// Event sent when user file send fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSendFileFailedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub task_name: String,
    pub user_id: String,
    pub file_key: String,
    pub file_size_bytes: u64,
    pub duration_ms: u64,
    pub error_type: String,
    pub error_message: String,
    /// How far the upload progressed (percentage)
    pub progress_percent: f32,
}

impl TelemetryEvent for UserSendFileFailedEvent {
    fn event_type(&self) -> &str {
        "user_send_file_failed"
    }
    
    fn strategy(&self) -> TelemetryStrategy {
        TelemetryStrategy::Guaranteed
    }
}

/// Event sent when user requests file download.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDownloadFileStartedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub user_id: String,
    pub file_key: String,
    pub file_size_bytes: u64,
    /// Provider serving the file
    pub provider_id: String,
}

impl TelemetryEvent for UserDownloadFileStartedEvent {
    fn event_type(&self) -> &str {
        "user_download_file_started"
    }
}

/// Event sent when user file download completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDownloadFileCompletedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub user_id: String,
    pub file_key: String,
    pub file_size_bytes: u64,
    pub duration_ms: u64,
    pub average_transfer_rate_mbps: f64,
}

impl TelemetryEvent for UserDownloadFileCompletedEvent {
    fn event_type(&self) -> &str {
        "user_download_file_completed"
    }
}

/// Event sent when user creates a bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBucketCreatedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub user_id: String,
    pub bucket_id: String,
    /// Bucket name if provided
    pub bucket_name: Option<String>,
    /// Initial capacity in bytes
    pub initial_capacity_bytes: u64,
    /// Selected MSP for the bucket
    pub msp_id: String,
}

impl TelemetryEvent for UserBucketCreatedEvent {
    fn event_type(&self) -> &str {
        "user_bucket_created"
    }
}

/// Event sent when user deletes a bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBucketDeletedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub user_id: String,
    pub bucket_id: String,
    /// Number of files deleted with bucket
    pub files_deleted: u32,
    /// Total size freed in bytes
    pub size_freed_bytes: u64,
}

impl TelemetryEvent for UserBucketDeletedEvent {
    fn event_type(&self) -> &str {
        "user_bucket_deleted"
    }
}

/// Event sent when user payment is processed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPaymentProcessedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub user_id: String,
    /// Payment amount
    pub amount: u128,
    /// Currency
    pub currency: String,
    /// Payment type (e.g., "storage_fee", "deposit")
    pub payment_type: String,
    /// Whether payment succeeded
    pub success: bool,
    /// Failure reason if failed
    pub failure_reason: Option<String>,
}

impl TelemetryEvent for UserPaymentProcessedEvent {
    fn event_type(&self) -> &str {
        "user_payment_processed"
    }
}