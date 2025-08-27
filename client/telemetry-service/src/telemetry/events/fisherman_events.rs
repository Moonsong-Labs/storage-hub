//! Fisherman telemetry events.
//!
//! This module defines all telemetry events related to Fisherman operations including
//! challenge verification, slashing, and file deletion processing.

use crate::{BaseTelemetryEvent, TelemetryEvent, TelemetryStrategy};
use serde::{Deserialize, Serialize};

/// Event sent when fisherman starts a challenge verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FishermanChallengeStartedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,

    pub task_id: String,
    /// Provider being challenged
    pub provider_id: String,
    /// Type of provider (BSP or MSP)
    pub provider_type: String,
    /// Challenge ID
    pub challenge_id: String,
    /// File key being challenged
    pub file_key: String,
    /// Challenge type
    pub challenge_type: String,
}

impl TelemetryEvent for FishermanChallengeStartedEvent {
    fn event_type(&self) -> &str {
        "fisherman_challenge_started"
    }
}

/// Event sent when fisherman completes a challenge verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FishermanChallengeCompletedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,

    pub task_id: String,
    pub provider_id: String,
    pub provider_type: String,
    pub challenge_id: String,
    pub file_key: String,
    /// Whether the challenge was successful
    pub challenge_passed: bool,
    /// Verification duration in milliseconds
    pub verification_duration_ms: u64,
    /// Proof verification result
    pub proof_valid: bool,
}

impl TelemetryEvent for FishermanChallengeCompletedEvent {
    fn event_type(&self) -> &str {
        "fisherman_challenge_completed"
    }
}

/// Event sent when fisherman initiates provider slashing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FishermanSlashingInitiatedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,

    pub task_id: String,
    /// Provider being slashed
    pub provider_id: String,
    pub provider_type: String,
    /// Reason for slashing
    pub slashing_reason: String,
    /// Evidence provided
    pub evidence_type: String,
    /// Challenge ID that led to slashing
    pub challenge_id: Option<String>,
    /// Amount to be slashed
    pub slash_amount: u128,
}

impl TelemetryEvent for FishermanSlashingInitiatedEvent {
    fn event_type(&self) -> &str {
        "fisherman_slashing_initiated"
    }

    fn strategy(&self) -> TelemetryStrategy {
        TelemetryStrategy::Guaranteed
    }
}

/// Event sent when fisherman processes a file deletion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FishermanFileDeletionProcessedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,

    pub task_id: String,
    pub file_key: String,
    /// Provider storing the file
    pub provider_id: String,
    /// Reason for deletion
    pub deletion_reason: String,
    /// Whether deletion was verified
    pub deletion_verified: bool,
    /// Verification method used
    pub verification_method: String,
}

impl TelemetryEvent for FishermanFileDeletionProcessedEvent {
    fn event_type(&self) -> &str {
        "fisherman_file_deletion_processed"
    }
}

/// Event sent when fisherman detects suspicious activity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FishermanSuspiciousActivityDetectedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,

    pub task_id: String,
    /// Type of suspicious activity
    pub activity_type: String,
    /// Provider involved
    pub provider_id: String,
    pub provider_type: String,
    /// Severity level (low, medium, high, critical)
    pub severity: String,
    /// Details of the suspicious activity
    pub activity_details: String,
    /// Whether automatic action was taken
    pub action_taken: bool,
    /// Action details if taken
    pub action_details: Option<String>,
}

impl TelemetryEvent for FishermanSuspiciousActivityDetectedEvent {
    fn event_type(&self) -> &str {
        "fisherman_suspicious_activity_detected"
    }

    fn strategy(&self) -> TelemetryStrategy {
        TelemetryStrategy::Guaranteed
    }
}

/// Event sent when fisherman completes a periodic audit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FishermanAuditCompletedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,

    pub task_id: String,
    /// Number of providers audited
    pub providers_audited: u32,
    /// Number of files verified
    pub files_verified: u32,
    /// Number of issues found
    pub issues_found: u32,
    /// Audit duration in milliseconds
    pub audit_duration_ms: u64,
    /// Next scheduled audit time
    pub next_audit_timestamp: Option<i64>,
}

impl TelemetryEvent for FishermanAuditCompletedEvent {
    fn event_type(&self) -> &str {
        "fisherman_audit_completed"
    }
}
