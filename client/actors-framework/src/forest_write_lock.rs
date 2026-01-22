//! Forest write lock integration for the actors framework.
//!
//! This module re-exports the core lock primitives from [`shc_forest_lock`] and provides
//! event handler integration via [`ForestRootWriteGuardedHandler`].
//!
//! ## Architecture: Event-Based Lock Coordination
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                    ACTOR EVENT LOCK COORDINATION                            │
//! │                                                                             │
//! │  ┌─────────────────────────────────────────────────────────────────────┐    │
//! │  │                     EVENT EMISSION                                  │    │
//! │  │                                                                     │    │
//! │  │  // Macro-generated event struct with lock field                    │    │
//! │  │  #[actor(actor = "blockchain_service", forest_root_write_lock)]     │    │
//! │  │  pub struct ProcessSubmitProofRequest {                             │    │
//! │  │      pub data: ProofData,                                           │    │
//! │  │      pub forest_root_write_lock: ForestRootWriteGuardSlot, // auto  │    │
//! │  │  }                                                                  │    │
//! │  │                                                                     │    │
//! │  │  // Service acquires lock and emits event                           │    │
//! │  │  let guard = gate.try_acquire()?;                                   │    │
//! │  │  emit(ProcessSubmitProofRequest {                                   │    │
//! │  │      data: proof_data,                                              │    │
//! │  │      forest_root_write_lock: guard.into(),  // lock travels here    │    │
//! │  │  });                                                                │    │
//! │  └───────────────────────────┬─────────────────────────────────────────┘    │
//! │                              │                                              │
//! │                              │ Event bus (broadcast)                        │
//! │                              ▼                                              │
//! │  ┌─────────────────────────────────────────────────────────────────────┐    │
//! │  │                    EVENT HANDLER PROCESSING                         │    │
//! │  │                                                                     │    │
//! │  │  ForestRootWriteGuardedHandler<ActualHandler>                       │    │
//! │  │                                                                     │    │
//! │  │  async fn handle_event(&mut self, event: E) {                       │    │
//! │  │      // Extract guard from event (first handler to receive wins)    │    │
//! │  │      let _guard = event.take_lock()?;                               │    │
//! │  │      │                                                              │    │
//! │  │      │  Lock held during entire handler execution                   │    │
//! │  │      ├─► self.inner.handle_event(event).await                       │    │
//! │  │      │                                                              │    │
//! │  │      │  Handler completes (success, error, or panic)                │    │
//! │  │      ▼                                                              │    │
//! │  │  }  // _guard dropped here → lock released (RAII)                   │    │
//! │  └───────────────────────────┬─────────────────────────────────────────┘    │
//! │                              │                                              │
//! │                              │ Guard dropped                                │
//! │                              ▼                                              │
//! │  ┌─────────────────────────────────────────────────────────────────────┐    │
//! │  │                    WAITING SERVICE EVENT LOOP                       │    │
//! │  │                                                                     │    │
//! │  │  let mut release_rx = gate.subscribe();                             │    │
//! │  │                                                                     │    │
//! │  │  loop {                                                             │    │
//! │  │      select! {                                                      │    │
//! │  │          _ = release_rx.recv() => {                                 │    │
//! │  │              // Lock released! Try to process next queued request   │    │
//! │  │              try_process_next_request();                            │    │
//! │  │          }                                                          │    │
//! │  │          cmd = command_rx.recv() => { /* handle command */ }        │    │
//! │  │      }                                                              │    │
//! │  │  }                                                                  │    │
//! │  └─────────────────────────────────────────────────────────────────────┘    │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## ForestRootWriteGuardSlot: Why Arc<Mutex<Option>>?
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │  Events must be Clone for broadcast (multiple subscribers)                  │
//! │  But ForestRootWriteGuard is NOT Clone (only one owner)                     │
//! │                                                                             │
//! │  Solution: Arc<Mutex<Option<ForestRootWriteGuard>>>                         │
//! │                                                                             │
//! │                      ┌─────────────────────────────┐                        │
//! │                      │ Arc<Mutex<Option<Guard>>>   │  (single instance)     │
//! │                      └──────────────┬──────────────┘                        │
//! │                                     │                                       │
//! │                 ┌───────────────────┴───────────────────┐                   │
//! │                 │ (Arc cloned = shared reference)       │                   │
//! │                 ▼                                       ▼                   │
//! │       ┌─────────────────┐                     ┌─────────────────┐           │
//! │       │  Event (clone)  │                     │  Event (clone)  │           │
//! │       └────────┬────────┘                     └────────┬────────┘           │
//! │                │                                       │                    │
//! │                ▼                                       ▼                    │
//! │       ┌─────────────────┐                     ┌─────────────────┐           │
//! │       │   Subscriber1   │                     │   Subscriber2   │           │
//! │       └────────┬────────┘                     └────────┬────────┘           │
//! │                │                                       │                    │
//! │                │ take_lock()                take_lock()│                    │
//! │                │                                       │                    │
//! │                └───────────────────┬───────────────────┘                    │
//! │                                    │                                        │
//! │                                    ▼                                        │
//! │                     ┌─────────────────────────┐                             │
//! │                     │    First caller wins    │                             │
//! │                     │     (Option::take)      │                             │
//! │                     │    Others get error     │                             │
//! │                     └─────────────────────────┘                             │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Types
//!
//! Core primitives (re-exported from [`shc_forest_lock`]):
//! - [`ForestRootWriteGuard`]: RAII guard that releases on drop
//! - [`ForestRootWriteGuardSlot`]: Cloneable wrapper for use in events
//! - [`ForestRootWriteAccess`]: Trait for uniform lock detection/extraction
//! - [`ForestRootWriteGate`]: Thread-safe lock manager
//!
//! Event handler integration:
//! - [`ForestRootWriteGuardedHandler`]: Wrapper that auto-extracts locks during event handling

// Re-export core primitives from shc-forest-lock for convenience.
// Actors can import from here; non-actor code can import directly from shc_forest_lock.
pub use shc_forest_lock::*;

use crate::event_bus::{EventBusMessage, EventHandler};

/// Wrapper handler that automatically manages the forest root write lock.
///
/// Extracts the lock guard from the event before handling, ensuring the lock
/// is held for the duration of event processing and released when done.
#[derive(Clone)]
pub struct ForestRootWriteGuardedHandler<H> {
    inner: H,
}

impl<H> ForestRootWriteGuardedHandler<H> {
    pub fn new(inner: H) -> Self {
        Self { inner }
    }
}

impl<E, H> EventHandler<E> for ForestRootWriteGuardedHandler<H>
where
    E: EventBusMessage + ForestRootWriteAccess,
    H: EventHandler<E>,
{
    async fn handle_event(&mut self, event: E) -> anyhow::Result<String> {
        let _guard = event.take_lock().map_err(anyhow::Error::new)?;
        self.inner.handle_event(event).await
    }
}
