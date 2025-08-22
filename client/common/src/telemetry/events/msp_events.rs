//! MSP (Main Storage Provider) telemetry events.
//!
//! This module defines all telemetry events related to MSP operations including
//! file acceptance, bucket management, capacity management, and fee collection.

use crate::telemetry::{BaseTelemetryEvent, TelemetryEvent, TelemetryStrategy};
use serde::{Deserialize, Serialize};

/// Event sent when MSP accepts a file upload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MspUploadAcceptedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub file_key: String,
    /// Bucket storing the file
    pub bucket_id: String,
    pub file_size_bytes: u64,
    /// Current capacity used in bytes
    pub capacity_used_bytes: u64,
    /// Remaining capacity available in bytes
    pub capacity_available_bytes: u64,
    /// Whether capacity was auto-increased
    pub auto_capacity_increased: bool,
    /// Amount of capacity increase if auto-increased
    pub capacity_increase_amount: Option<u64>,
}

impl TelemetryEvent for MspUploadAcceptedEvent {
    fn event_type(&self) -> &str {
        "msp_upload_accepted"
    }
}

/// Event sent when MSP rejects a file upload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MspUploadRejectedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub file_key: String,
    pub bucket_id: String,
    pub file_size_bytes: u64,
    /// Reason for rejection
    pub rejection_reason: String,
    /// Current capacity status
    pub capacity_used_bytes: u64,
    pub capacity_available_bytes: u64,
}

impl TelemetryEvent for MspUploadRejectedEvent {
    fn event_type(&self) -> &str {
        "msp_upload_rejected"
    }
}

/// Event sent when MSP charges storage fees.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MspFeesChargedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    /// Total fee amount charged
    pub fee_amount: u128,
    /// Currency (e.g., "DOT")
    pub currency: String,
    /// Number of buckets being charged
    pub bucket_count: u32,
    /// Number of users charged
    pub user_count: u32,
    /// Total storage being charged for in bytes
    pub total_storage_used_bytes: u64,
    /// Billing period (e.g., "monthly", "weekly")
    pub billing_period: String,
    /// Successful fee collections
    pub successful_charges: u32,
    /// Failed fee collections
    pub failed_charges: u32,
}

impl TelemetryEvent for MspFeesChargedEvent {
    fn event_type(&self) -> &str {
        "msp_fees_charged"
    }
}

/// Event sent when MSP automatically increases capacity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MspCapacityAutoIncreasedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    /// Previous capacity in bytes
    pub previous_capacity_bytes: u64,
    /// New capacity in bytes
    pub new_capacity_bytes: u64,
    /// Increase amount in bytes
    pub increase_amount_bytes: u64,
    /// Utilization percentage that triggered increase
    pub trigger_utilization_percent: f32,
    /// Number of pending uploads that triggered increase
    pub pending_uploads_count: u32,
}

impl TelemetryEvent for MspCapacityAutoIncreasedEvent {
    fn event_type(&self) -> &str {
        "msp_capacity_auto_increased"
    }
}

/// Event sent when MSP deletes a bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MspBucketDeletedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub bucket_id: String,
    /// User who owned the bucket
    pub user_id: String,
    /// Number of files deleted
    pub files_deleted: u32,
    /// Total size freed in bytes
    pub size_freed_bytes: u64,
    /// Reason for deletion
    pub deletion_reason: String,
}

impl TelemetryEvent for MspBucketDeletedEvent {
    fn event_type(&self) -> &str {
        "msp_bucket_deleted"
    }
}

/// Event sent when MSP responds to a bucket move request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MspBucketMoveRespondedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub bucket_id: String,
    /// MSP requesting the move
    pub requesting_msp: String,
    /// Whether the move was accepted
    pub accepted: bool,
    /// Reason if rejected
    pub rejection_reason: Option<String>,
    /// Number of files in bucket
    pub file_count: u32,
    /// Total bucket size in bytes
    pub bucket_size_bytes: u64,
}

impl TelemetryEvent for MspBucketMoveRespondedEvent {
    fn event_type(&self) -> &str {
        "msp_bucket_move_responded"
    }
}

/// Event sent when MSP retries a bucket move.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MspBucketMoveRetriedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub bucket_id: String,
    /// Target MSP for the move
    pub target_msp: String,
    /// Retry attempt number
    pub retry_attempt: u32,
    /// Previous failure reason
    pub previous_failure_reason: String,
}

impl TelemetryEvent for MspBucketMoveRetriedEvent {
    fn event_type(&self) -> &str {
        "msp_bucket_move_retried"
    }
}

/// Event sent when MSP stops storing for an insolvent user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MspStopStoringInsolventUserEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    /// User being stopped
    pub user_id: String,
    /// Number of buckets affected
    pub buckets_affected: u32,
    /// Total storage freed in bytes
    pub storage_freed_bytes: u64,
    /// Outstanding debt amount
    pub outstanding_debt: u128,
    /// Number of payment failures before stopping
    pub payment_failures: u32,
}

impl TelemetryEvent for MspStopStoringInsolventUserEvent {
    fn event_type(&self) -> &str {
        "msp_stop_storing_insolvent_user"
    }
}

/// Event sent when MSP storage operation fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MspStorageFailedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub operation: String,
    pub error_type: String,
    pub error_message: String,
    /// Affected bucket if applicable
    pub bucket_id: Option<String>,
    /// Affected file if applicable
    pub file_key: Option<String>,
}

impl TelemetryEvent for MspStorageFailedEvent {
    fn event_type(&self) -> &str {
        "msp_storage_failed"
    }
    
    fn strategy(&self) -> TelemetryStrategy {
        TelemetryStrategy::Guaranteed
    }
}