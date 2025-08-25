//! BSP (Backup Storage Provider) telemetry events.
//!
//! This module defines all telemetry events related to BSP operations including
//! file uploads, downloads, proof generation, and fee collection.

use crate::{BaseTelemetryEvent, TelemetryEvent, TelemetryStrategy};
use serde::{Deserialize, Serialize};

/// Event sent when BSP starts receiving a file upload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BspUploadStartedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    /// Unique task instance ID
    pub task_id: String,
    /// Task name (always "bsp_upload_file")
    pub task_name: String,
    
    /// Hex-encoded file key
    pub file_key: String,
    /// Total file size in bytes
    pub file_size_bytes: u64,
    /// Storage location
    pub location: String,
    /// File fingerprint
    pub fingerprint: String,
    /// Peer ID sending the file
    pub peer_id: String,
}

impl TelemetryEvent for BspUploadStartedEvent {
    fn event_type(&self) -> &str {
        "bsp_upload_started"
    }
}

/// Event sent for each chunk received during upload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BspUploadChunkReceivedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    /// Links to BspUploadStartedEvent
    pub task_id: String,
    /// File key being uploaded
    pub file_key: String,
    /// Current chunk number
    pub chunk_index: u32,
    /// Size of this chunk in bytes
    pub chunk_size_bytes: u64,
    /// Total expected chunks
    pub total_chunks: u32,
    /// Chunks received so far
    pub chunks_received: u32,
    /// Upload progress percentage (0-100)
    pub progress_percent: f32,
    /// Current transfer rate in Mbps
    pub transfer_rate_mbps: f64,
}

impl TelemetryEvent for BspUploadChunkReceivedEvent {
    fn event_type(&self) -> &str {
        "bsp_upload_chunk_received"
    }
}

/// Event sent when upload completes successfully.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BspUploadCompletedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub task_name: String,
    pub file_key: String,
    pub file_size_bytes: u64,
    /// Total upload time in milliseconds
    pub duration_ms: u64,
    /// Average transfer rate in Mbps
    pub average_transfer_rate_mbps: f64,
    /// Time to generate proof in milliseconds
    pub proof_generation_time_ms: u64,
    /// New forest root after upload
    pub forest_root: String,
    /// File merkle root
    pub merkle_root: String,
}

impl TelemetryEvent for BspUploadCompletedEvent {
    fn event_type(&self) -> &str {
        "bsp_upload_completed"
    }
}

/// Event sent when upload fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BspUploadFailedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub task_name: String,
    pub file_key: String,
    /// Duration before failure in milliseconds
    pub duration_ms: u64,
    /// Type of error (network_error, storage_error, etc.)
    pub error_type: String,
    /// Error message details
    pub error_message: String,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Chunks successfully received before failure
    pub chunks_received: u32,
    /// Total chunks expected
    pub total_chunks: u32,
}

impl TelemetryEvent for BspUploadFailedEvent {
    fn event_type(&self) -> &str {
        "bsp_upload_failed"
    }
    
    fn strategy(&self) -> TelemetryStrategy {
        TelemetryStrategy::Guaranteed
    }
}

/// Event sent when BSP starts downloading a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BspDownloadStartedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub task_name: String,
    pub file_key: String,
    pub file_size_bytes: u64,
    /// Peer requesting the file
    pub requester_peer_id: String,
}

impl TelemetryEvent for BspDownloadStartedEvent {
    fn event_type(&self) -> &str {
        "bsp_download_started"
    }
}

/// Event sent when download completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BspDownloadCompletedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub task_name: String,
    pub file_key: String,
    pub file_size_bytes: u64,
    pub duration_ms: u64,
    pub average_transfer_rate_mbps: f64,
}

impl TelemetryEvent for BspDownloadCompletedEvent {
    fn event_type(&self) -> &str {
        "bsp_download_completed"
    }
}

/// Event sent when BSP starts generating a proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BspProofGenerationStartedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    /// Type of proof: "storage" or "challenge"
    pub proof_type: String,
    /// Number of challenges to answer
    pub challenges_count: u32,
    /// Current forest root
    pub forest_root: String,
}

impl TelemetryEvent for BspProofGenerationStartedEvent {
    fn event_type(&self) -> &str {
        "bsp_proof_generation_started"
    }
}

/// Event sent when BSP submits a proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BspProofSubmittedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub proof_type: String,
    /// Challenges successfully answered
    pub challenges_answered: u32,
    pub merkle_root: String,
    pub forest_root: String,
    /// Proof generation time in milliseconds
    pub generation_time_ms: u64,
    /// Number of submission attempts
    pub submission_attempts: u32,
    /// Blockchain transaction hash
    pub extrinsic_hash: String,
}

impl TelemetryEvent for BspProofSubmittedEvent {
    fn event_type(&self) -> &str {
        "bsp_proof_submitted"
    }
}

/// Event sent when BSP proof submission fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BspProofFailedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub proof_type: String,
    pub error_type: String,
    pub error_message: String,
    pub generation_time_ms: Option<u64>,
    pub submission_attempts: u32,
}

impl TelemetryEvent for BspProofFailedEvent {
    fn event_type(&self) -> &str {
        "bsp_proof_failed"
    }
    
    fn strategy(&self) -> TelemetryStrategy {
        TelemetryStrategy::Guaranteed
    }
}

/// Event sent when BSP charges storage fees.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BspFeesChargedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    /// Total fee amount charged
    pub fee_amount: u128,
    /// Currency (e.g., "DOT")
    pub currency: String,
    /// Number of files being charged for
    pub file_count: u32,
    /// Total storage size in bytes
    pub total_storage_bytes: u64,
    /// Billing period
    pub billing_period: String,
    /// Successful charge count
    pub successful_charges: u32,
    /// Failed charge count
    pub failed_charges: u32,
}

impl TelemetryEvent for BspFeesChargedEvent {
    fn event_type(&self) -> &str {
        "bsp_fees_charged"
    }
}

/// Event sent when BSP deletes a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BspFileDeletedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub file_key: String,
    pub file_size_bytes: u64,
    /// Reason for deletion
    pub deletion_reason: String,
    /// Whether forest was updated
    pub forest_updated: bool,
    /// New forest root after deletion
    pub new_forest_root: Option<String>,
}

impl TelemetryEvent for BspFileDeletedEvent {
    fn event_type(&self) -> &str {
        "bsp_file_deleted"
    }
}

/// Event sent when BSP moves a bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BspBucketMovedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    pub task_id: String,
    pub bucket_id: String,
    /// Previous MSP ID
    pub from_msp: String,
    /// New MSP ID
    pub to_msp: String,
    /// Number of files in bucket
    pub file_count: u32,
    /// Total bucket size in bytes
    pub bucket_size_bytes: u64,
    /// Migration duration in milliseconds
    pub migration_duration_ms: u64,
}

impl TelemetryEvent for BspBucketMovedEvent {
    fn event_type(&self) -> &str {
        "bsp_bucket_moved"
    }
}