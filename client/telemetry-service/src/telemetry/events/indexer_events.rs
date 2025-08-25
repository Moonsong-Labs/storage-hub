//! Indexer telemetry events.
//!
//! This module defines all telemetry events related to the indexer service including
//! block processing, event handling, and synchronization.

use crate::{BaseTelemetryEvent, TelemetryEvent, TelemetryStrategy};
use serde::{Deserialize, Serialize};

/// Event sent when indexer processes a block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerBlockProcessedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    /// Handler name ("fishing" or "lite")
    pub handler_name: String,
    /// Block number
    pub block_number: u64,
    /// Block hash
    pub block_hash: String,
    /// Parent block hash
    pub parent_hash: String,
    /// Number of events in this block
    pub events_count: u32,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// Current indexing mode
    pub indexing_mode: String,
}

impl TelemetryEvent for IndexerBlockProcessedEvent {
    fn event_type(&self) -> &str {
        "indexer_block_processed"
    }
}

/// Event sent when indexer processes a specific blockchain event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerEventProcessedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    /// Handler name
    pub handler_name: String,
    /// Block number containing the event
    pub block_number: u64,
    /// Event name (e.g., "FileSystem.NewStorageRequest")
    pub event_name: String,
    /// Index of event within block
    pub event_index: u32,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    
    // Event-specific optional fields (filled based on event type)
    /// File key if this is a file-related event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_key: Option<String>,
    /// Bucket ID if this is a bucket-related event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket_id: Option<String>,
    /// Provider ID if this is a provider-related event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    /// User ID if this is a user-related event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Transaction hash that triggered this event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extrinsic_hash: Option<String>,
}

impl TelemetryEvent for IndexerEventProcessedEvent {
    fn event_type(&self) -> &str {
        "indexer_event_processed"
    }
}

/// Event sent when indexer starts synchronization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerSyncStartedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    /// Handler name
    pub handler_name: String,
    /// Starting block number
    pub start_block: u64,
    /// Target block number
    pub target_block: u64,
    /// Number of blocks to sync
    pub blocks_to_sync: u64,
    /// Sync mode (e.g., "fast", "full")
    pub sync_mode: String,
}

impl TelemetryEvent for IndexerSyncStartedEvent {
    fn event_type(&self) -> &str {
        "indexer_sync_started"
    }
}

/// Event sent when indexer completes synchronization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerSyncCompletedEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    /// Handler name
    pub handler_name: String,
    /// Number of blocks synced
    pub blocks_synced: u64,
    /// Total events processed
    pub events_processed: u64,
    /// Sync duration in milliseconds
    pub sync_duration_ms: u64,
    /// Average blocks per second
    pub avg_blocks_per_second: f64,
    /// Final block number after sync
    pub final_block: u64,
}

impl TelemetryEvent for IndexerSyncCompletedEvent {
    fn event_type(&self) -> &str {
        "indexer_sync_completed"
    }
}

/// Event sent when indexer encounters an error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerErrorEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    /// Handler name
    pub handler_name: String,
    /// Block number where error occurred
    pub block_number: Option<u64>,
    /// Event that caused the error
    pub event_name: Option<String>,
    /// Error type
    pub error_type: String,
    /// Error message
    pub error_message: String,
    /// Whether indexer will retry
    pub will_retry: bool,
    /// Retry attempt number
    pub retry_attempt: Option<u32>,
}

impl TelemetryEvent for IndexerErrorEvent {
    fn event_type(&self) -> &str {
        "indexer_error"
    }
    
    fn strategy(&self) -> TelemetryStrategy {
        TelemetryStrategy::Guaranteed
    }
}

/// Event sent when indexer reorganizes blocks due to chain reorg.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerReorgEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    /// Handler name
    pub handler_name: String,
    /// Common ancestor block
    pub common_block: u64,
    /// Number of blocks reverted
    pub blocks_reverted: u32,
    /// Number of blocks applied
    pub blocks_applied: u32,
    /// Old chain tip
    pub old_tip: String,
    /// New chain tip
    pub new_tip: String,
}

impl TelemetryEvent for IndexerReorgEvent {
    fn event_type(&self) -> &str {
        "indexer_reorg"
    }
    
    fn strategy(&self) -> TelemetryStrategy {
        TelemetryStrategy::Guaranteed
    }
}

/// Event sent periodically with indexer health metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerHealthEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    
    /// Handler name
    pub handler_name: String,
    /// Current block being indexed
    pub current_block: u64,
    /// Latest chain block
    pub chain_tip: u64,
    /// Blocks behind chain tip
    pub blocks_behind: i64,
    /// Events in processing queue
    pub queue_size: u32,
    /// Average processing time per block (ms)
    pub avg_block_time_ms: f64,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// Database size in bytes
    pub database_size_bytes: u64,
}

impl TelemetryEvent for IndexerHealthEvent {
    fn event_type(&self) -> &str {
        "indexer_health"
    }
}