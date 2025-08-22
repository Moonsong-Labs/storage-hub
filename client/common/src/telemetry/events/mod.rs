//! Typed telemetry event definitions for StorageHub.
//!
//! This module contains all typed event definitions for different components of the system.
//! Each event has strongly-typed, queryable fields - no JSON blobs for metrics.

pub mod bsp_events;
pub mod fisherman_events;
pub mod indexer_events;
pub mod msp_events;
pub mod user_events;

// Re-export all event types for convenience
pub use bsp_events::*;
pub use fisherman_events::*;
pub use indexer_events::*;
pub use msp_events::*;
pub use user_events::*;