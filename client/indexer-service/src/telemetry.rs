//! Indexer-specific telemetry module for the storage hub indexer service.
//!
//! This module provides a wrapper around the typed telemetry events from the
//! telemetry-service crate for the indexer service.

use shc_actors_framework::actor::ActorHandle;
use shc_telemetry_service::{
    create_base_event, telemetry::events::indexer_events::*, TelemetryService,
    TelemetryServiceCommandInterfaceExt,
};

/// Service-scoped telemetry wrapper for indexer service
pub struct IndexerServiceTelemetry {
    inner: Option<ActorHandle<TelemetryService>>,
    service_name: String,
}

impl IndexerServiceTelemetry {
    /// Create a new indexer service telemetry wrapper
    pub fn new(telemetry_handle: Option<ActorHandle<TelemetryService>>) -> Self {
        Self {
            inner: telemetry_handle,
            service_name: "storage-hub-indexer".to_string(),
        }
    }

    /// Send a block processed event
    pub async fn block_processed(
        &self,
        handler_name: &str,
        block_number: u64,
        block_hash: String,
        parent_hash: String,
        events_count: u32,
        processing_time_ms: u64,
        indexing_mode: String,
    ) {
        if let Some(telemetry) = &self.inner {
            let event = IndexerBlockProcessedEvent {
                base: create_base_event("indexer_block_processed", self.service_name.clone(), None),
                handler_name: handler_name.to_string(),
                block_number,
                block_hash,
                parent_hash,
                events_count,
                processing_time_ms,
                indexing_mode,
            };
            telemetry.queue_typed_event(event).await.ok();
        }
    }

    /// Send an event processed event
    pub async fn event_processed(
        &self,
        handler_name: &str,
        block_number: u64,
        event_name: String,
        event_index: u32,
        processing_time_ms: u64,
        file_key: Option<String>,
        bucket_id: Option<String>,
        provider_id: Option<String>,
        user_id: Option<String>,
        extrinsic_hash: Option<String>,
    ) {
        if let Some(telemetry) = &self.inner {
            let event = IndexerEventProcessedEvent {
                base: create_base_event("indexer_event_processed", self.service_name.clone(), None),
                handler_name: handler_name.to_string(),
                block_number,
                event_name,
                event_index,
                processing_time_ms,
                file_key,
                bucket_id,
                provider_id,
                user_id,
                extrinsic_hash,
            };
            telemetry.queue_typed_event(event).await.ok();
        }
    }

    /// Send a sync started event
    pub async fn sync_started(
        &self,
        handler_name: &str,
        start_block: u64,
        target_block: u64,
        blocks_to_sync: u64,
        sync_mode: String,
    ) {
        if let Some(telemetry) = &self.inner {
            let event = IndexerSyncStartedEvent {
                base: create_base_event("indexer_sync_started", self.service_name.clone(), None),
                handler_name: handler_name.to_string(),
                start_block,
                target_block,
                blocks_to_sync,
                sync_mode,
            };
            telemetry.queue_typed_event(event).await.ok();
        }
    }

    /// Send a sync completed event
    pub async fn sync_completed(
        &self,
        handler_name: &str,
        blocks_synced: u64,
        events_processed: u64,
        sync_duration_ms: u64,
        avg_blocks_per_second: f64,
        final_block: u64,
    ) {
        if let Some(telemetry) = &self.inner {
            let event = IndexerSyncCompletedEvent {
                base: create_base_event("indexer_sync_completed", self.service_name.clone(), None),
                handler_name: handler_name.to_string(),
                blocks_synced,
                events_processed,
                sync_duration_ms,
                avg_blocks_per_second,
                final_block,
            };
            telemetry.queue_typed_event(event).await.ok();
        }
    }

    /// Send an error event
    pub async fn indexer_error(
        &self,
        handler_name: &str,
        block_number: Option<u64>,
        event_name: Option<String>,
        error_type: String,
        error_message: String,
        will_retry: bool,
        retry_attempt: Option<u32>,
    ) {
        if let Some(telemetry) = &self.inner {
            let event = IndexerErrorEvent {
                base: create_base_event("indexer_error", self.service_name.clone(), None),
                handler_name: handler_name.to_string(),
                block_number,
                event_name,
                error_type,
                error_message,
                will_retry,
                retry_attempt,
            };
            telemetry.queue_typed_event(event).await.ok();
        }
    }

    /// Send a reorg event
    pub async fn indexer_reorg(
        &self,
        handler_name: &str,
        common_block: u64,
        blocks_reverted: u32,
        blocks_applied: u32,
        old_tip: String,
        new_tip: String,
    ) {
        if let Some(telemetry) = &self.inner {
            let event = IndexerReorgEvent {
                base: create_base_event("indexer_reorg", self.service_name.clone(), None),
                handler_name: handler_name.to_string(),
                common_block,
                blocks_reverted,
                blocks_applied,
                old_tip,
                new_tip,
            };
            telemetry.queue_typed_event(event).await.ok();
        }
    }

    /// Send a health metrics event
    pub async fn indexer_health(
        &self,
        handler_name: &str,
        current_block: u64,
        chain_tip: u64,
        blocks_behind: i64,
        queue_size: u32,
        avg_block_time_ms: f64,
        memory_usage_bytes: u64,
        database_size_bytes: u64,
    ) {
        if let Some(telemetry) = &self.inner {
            let event = IndexerHealthEvent {
                base: create_base_event("indexer_health", self.service_name.clone(), None),
                handler_name: handler_name.to_string(),
                current_block,
                chain_tip,
                blocks_behind,
                queue_size,
                avg_block_time_ms,
                memory_usage_bytes,
                database_size_bytes,
            };
            telemetry.queue_typed_event(event).await.ok();
        }
    }
}
