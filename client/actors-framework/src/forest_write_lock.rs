//! Forest write lock types for the actors framework.
//!
//!
//! ## Architecture Overview
//!
//! The forest root write lock system coordinates write access to the forest root across
//! multiple services (BlockchainService, FishermanService, etc.) using a single-permit
//! semaphore and RAII guards. Lock lifecycle is tied to event processing, ensuring
//! automatic release on both success and failure paths.
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────────────────┐
//! │                         FOREST ROOT WRITE LOCK SYSTEM                         │
//! │                                                                               │
//! │  ┌──────────────────┐         ┌──────────────────┐                            │
//! │  │ Service A        │         │ Service B        │                            │
//! │  │ (BSP/Fisherman)  │         │ (Blockchain)     │                            │
//! │  └────────┬─────────┘         └────────┬─────────┘                            │
//! │           │                            │                                      │
//! │           │  Arc<ForestRootWriteGate>  │  (Shared manager)                    │
//! │           └──────────┬─────────────────┘                                      │
//! │                      │                                                        │
//! │                      ▼                                                        │
//! │           ┌──────────────────────┐                                            │
//! │           │ ForestRootWriteGate  │                                            │
//! │           │ ┌──────────────────┐ │                                            │
//! │           │ │semaphore: Semaphore│ ◄─── Single-permit for mutual exclusion    │
//! │           │ └──────────────────┘ │                                            │
//! │           │ ┌──────────────────┐ │                                            │
//! │           │ │ release_tx: Sender │ ◄─── Broadcast channel for notifications   │
//! │           │ └──────────────────┘ │                                            │
//! │           └──────────┬───────────┘                                            │
//! │                      │                                                        │
//! └──────────────────────┼────────────────────────────────────────────────────────┘
//!                        │
//!                        │ try_acquire()
//!                        │ (try_acquire_owned on semaphore)
//!                        ▼
//!         ┌──────────────────────────────┐
//!         │ Some(ForestRootWriteGuard)   │  None if already locked
//!         │  ┌─────────────────────────┐ │
//!         │  │ _permit: OwnedPermit    │ │  ◄─── Auto-releases on drop
//!         │  │ release_tx: Sender      │ │  ◄─── Broadcast sender clone
//!         │  └─────────────────────────┘ │
//!         └───────────────┬──────────────┘
//!                         │
//!                         │ .into()
//!                         ▼
//!         ┌─────────────────────────────────────────┐
//!         │ ForestRootWriteGuardSlot                │
//!         │ Arc<Mutex<Option<ForestRootWriteGuard>>>│
//!         └───────────────┬─────────────────────────┘
//!                         │
//!                         │ Embedded in event
//!                         ▼
//! ┌──────────────────────────────────────────────────────────────────────────────┐
//! │                              EVENT EMISSION                                  │
//! │                                                                              │
//! │  #[actor(actor = "blockchain_service", forest_root_write_lock)]              │
//! │  pub struct ProcessSubmitProofRequest<Runtime> {                             │
//! │      pub data: ProcessSubmitProofRequestData<Runtime>,                       │
//! │      pub forest_root_write_lock: ForestRootWriteGuardSlot,  ◄─── Auto-added  │
//! │  }                                                                           │
//! │                                                                              │
//! │  emit_event(ProcessSubmitProofRequest {                                      │
//! │      data: proof_data,                                                       │
//! │      forest_root_write_lock: guard.into(),  ◄─── Lock travels with event     │
//! │  });                                                                         │
//! └───────────────────────┬──────────────────────────────────────────────────────┘
//!                         │
//!                         │ Event bus (Clone propagation)
//!                         ▼
//! ┌──────────────────────────────────────────────────────────────────────────────┐
//! │                         EVENT HANDLER PROCESSING                             │
//! │                                                                              │
//! │  ForestRootWriteGuardedHandler<ActualHandler>                                │
//! │                                                                              │
//! │  async fn handle_event(&mut self, event: E) {                                │
//! │      let _guard = event.take_lock()?;  ◄─── Extract guard from slot          │
//! │      │                                      (Mutex::lock + Option::take)     │
//! │      │                                                                       │
//! │      │  Lock held during handler execution                                   │
//! │      ├─► self.inner.handle_event(event).await  ◄─── Process with lock held   │
//! │      │                                                                       │
//! │      │  ┌─────────────────────────────────────┐                              │
//! │      │  │ Handler completes (success/error)   │                              │
//! │      │  │ OR panic occurs                     │                              │
//! │      │  └───────────────┬─────────────────────┘                              │
//! │      │                  │                                                    │
//! │      ▼                  ▼                                                    │
//! │  }  _guard dropped (RAII)  ◄─── Guaranteed cleanup                           │
//! └──────────┬───────────────────────────────────────────────────────────────────┘
//!            │
//!            │ Drop implementation
//!            ▼
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                         LOCK RELEASE (RAII)                                 │
//! │                                                                             │
//! │  impl Drop for ForestRootWriteGuard {                                       │
//! │      fn drop(&mut self) {                                                   │
//! │          // _permit auto-releases semaphore  ◄─── RAII permit release       │
//! │          let _ = self.release_tx.send(());   ◄─── Notify all subscribers    │
//! │      }                                                                      │
//! │  }                                                                          │
//! └──────────┬──────────────────────────────────────────────────────────────────┘
//!            │
//!            │ Broadcast notification
//!            ▼
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                      WAITING SERVICES (EVENT LOOPS)                         │
//! │                                                                             │
//! │  let mut lock_release_rx = forest_lock_manager.subscribe();                 │
//! │                                                                             │
//! │  loop {                                                                     │
//! │      select! {                                                              │
//! │          _ = lock_release_rx.recv() => {                                    │
//! │              // Lock released! Atomically try to acquire and process        │
//! │              bsp_assign_forest_root_write_lock();  // Calls try_acquire()   │
//! │          }                                                                  │
//! │          msg = command_rx.recv() => { /* ... */ }                           │
//! │      }                                                                      │
//! │  }                                                                          │
//! └─────────────────────────────────────────────────────────────────────────────┘
//!
//! ## Lock Acquisition Flow (TOCTOU-Safe)
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │  bsp_assign_forest_root_write_lock() / msp_assign_forest_root_write_lock()  │
//! │                                                                             │
//! │  Step 1: Atomically try to acquire lock FIRST                               │
//! │  ┌───────────────────────────────────────────────────────────────┐          │
//! │  │ let Some(guard) = lock_manager.try_acquire() else { return; } │          │
//! │  └───────────────────────┬───────────────────────────────────────┘          │
//! │                          │                                                  │
//! │                          │ Lock acquired ✓                                  │
//! │                          ▼                                                  │
//! │  Step 2: Now safely dequeue state (protected by lock)                       │
//! │  ┌─────────────────────────────────────────────────────────────┐            │
//! │  │ let request = queue.pop_front();  // Safe: lock held        │            │
//! │  │ msp_handler.pending_requests.remove(...);  // Safe          │            │
//! │  └───────────────────────┬─────────────────────────────────────┘            │
//! │                          │                                                  │
//! │                          ▼                                                  │
//! │  Step 3: Emit event with guard                                              │
//! │  ┌─────────────────────────────────────────────────────────────┐            │
//! │  │ bsp_emit_forest_write_event(event_data, guard);             │            │
//! │  │    // Guard passed as parameter (NOT acquired inside)       │            │
//! │  └───────────────────────┬─────────────────────────────────────┘            │
//! │                          │                                                  │
//! │                          ▼                                                  │
//! │  ┌─────────────────────────────────────────────────────────────┐            │
//! │  │ emit(ProcessSubmitProofRequest {                            │            │
//! │  │     data,                                                   │            │
//! │  │     forest_root_write_lock: guard.into(),  // Guard moves   │            │
//! │  │ });                                                         │            │
//! │  └─────────────────────────────────────────────────────────────┘            │
//! │                                                                             │
//! │  KEY: Lock acquired BEFORE any state mutation                               │
//! │  → No TOCTOU race: if try_acquire() fails, nothing is dequeued              │
//! │  → No data loss: failed acquisition = queues untouched                      │
//! └─────────────────────────────────────────────────────────────────────────────┘
//!
//! This module provides the core types for forest root write lock management:
//! - [`ForestRootWriteGuard`]: RAII guard that releases on drop
//! - [`ForestRootWriteGuardSlot`]: Cloneable wrapper for use in events
//! - [`ForestRootWriteAccess`]: Trait for uniform lock detection/extraction
//! - [`ForestRootWriteGuardedHandler`]: Wrapper that auto-extracts locks during event handling
//! - [`ForestRootWriteGate`]: Thread-safe lock manager for shared access across services

use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, OwnedSemaphorePermit, Semaphore};

use crate::event_bus::{EventBusMessage, EventHandler};

const LOG_TARGET: &str = "forest-write-lock";

/// RAII guard for the forest root write lock.
///
/// When dropped, the semaphore permit is automatically released and
/// a broadcast notification is sent to all subscribers.
pub struct ForestRootWriteGuard {
    /// Owned permit - automatically releases on drop.
    _permit: OwnedSemaphorePermit,
    /// Broadcast sender for release notification.
    release_tx: broadcast::Sender<()>,
}

impl ForestRootWriteGuard {
    fn new(permit: OwnedSemaphorePermit, release_tx: broadcast::Sender<()>) -> Self {
        Self {
            _permit: permit,
            release_tx,
        }
    }
}

impl Drop for ForestRootWriteGuard {
    fn drop(&mut self) {
        log::debug!(target: LOG_TARGET, "🔓 Guard DROP: Permit releasing, sending notification");
        // Permit is automatically released when _permit is dropped.
        // We only need to send the broadcast notification.
        let _ = self.release_tx.send(());
    }
}

impl std::fmt::Debug for ForestRootWriteGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForestRootWriteGuard").finish()
    }
}

/// Type alias for the forest root write lock field in events.
///
/// Wrapped in `Arc<Mutex<Option<...>>>` because:
/// - Events need to implement `Clone` for the event bus
/// - The lock guard should only be taken once
/// - Multiple subscribers might receive the same event
pub type ForestRootWriteGuardSlot = Arc<Mutex<Option<ForestRootWriteGuard>>>;

impl From<ForestRootWriteGuard> for ForestRootWriteGuardSlot {
    fn from(guard: ForestRootWriteGuard) -> Self {
        Arc::new(Mutex::new(Some(guard)))
    }
}

/// Trait for events that may carry a forest root write lock.
pub trait ForestRootWriteAccess: Send + 'static {
    /// Whether the event requires a forest root write lock to be present.
    const REQUIRES_LOCK: bool;

    /// Attempts to take the forest root write lock guard from the event.
    fn take_lock(&self) -> Result<ForestRootWriteGuard, ForestRootWriteError>;
}

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

/// Thread-safe forest write lock manager using Semaphore.
///
/// Uses a single-permit semaphore for mutual exclusion and broadcast
/// channel for release notifications to waiting services.
///
/// ## Usage
///
/// ```ignore
/// // Create shared manager (typically in StorageHubBuilder)
/// let manager = Arc::new(ForestRootWriteGate::new());
///
/// // TOCTOU-safe pattern: acquire lock BEFORE dequeuing state
/// fn assign_forest_root_write_lock(&mut self) {
///     // Step 1: Try to acquire lock FIRST
///     let Some(guard) = manager.try_acquire() else {
///         return; // Lock busy, queues untouched
///     };
///
///     // Step 2: Now safely dequeue (lock held)
///     let request = queue.pop_front()?;
///
///     // Step 3: Emit with guard
///     emit_event(ProcessRequest {
///         data: request,
///         forest_root_write_lock: guard.into()
///     });
/// }
///
/// // Subscribe to release notifications in event loops
/// let mut rx = manager.subscribe();
/// loop {
///     rx.recv().await;  // Notified when any guard is dropped
///     assign_forest_root_write_lock(); // Retry
/// }
/// ```
pub struct ForestRootWriteGate {
    /// Single-permit semaphore for mutual exclusion.
    semaphore: Arc<Semaphore>,
    /// Broadcast sender for release notifications.
    release_tx: broadcast::Sender<()>,
}

impl ForestRootWriteGate {
    /// Creates a new shared forest write gate.
    pub fn new() -> Self {
        // Buffer of 16 is sufficient - release notifications are transient signals
        let (release_tx, _) = broadcast::channel(16);
        Self {
            semaphore: Arc::new(Semaphore::new(1)),
            release_tx,
        }
    }

    /// Tries to acquire the forest root write lock (non-blocking).
    ///
    /// Returns `Some(guard)` if the lock was acquired, `None` if already held.
    pub fn try_acquire(&self) -> Option<ForestRootWriteGuard> {
        self.semaphore
            .clone()
            .try_acquire_owned()
            .ok()
            .map(|permit| {
                log::debug!(target: LOG_TARGET, "🔓 ForestRootWriteGate: acquired lock");
                ForestRootWriteGuard::new(permit, self.release_tx.clone())
            })
    }

    /// Creates a new subscriber to lock release notifications.
    ///
    /// Subscribers receive `()` whenever any guard is dropped.
    /// Useful for event loops that need to process queued requests
    /// when the lock becomes available.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.release_tx.subscribe()
    }
}

impl Default for ForestRootWriteGate {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ForestRootWriteGate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForestRootWriteGate")
            .field("available_permits", &self.semaphore.available_permits())
            .finish()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ForestRootWriteError {
    #[error("forest root write lock not present on event")]
    LockNotPresent,
    #[error("forest root write lock guard already taken")]
    GuardAlreadyTaken,
}
