//! Forest root write lock coordination primitives.
//!
//! This crate provides core synchronization primitives for coordinating exclusive
//! write access to the forest root across multiple services.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                         FOREST WRITE LOCK MECHANISM                         │
//! │                                                                             │
//! │  ┌──────────────────┐         ┌──────────────────┐                          │
//! │  │    Service A     │         │    Service B     │                          │
//! │  └────────┬─────────┘         └────────┬─────────┘                          │
//! │           │                            │                                    │
//! │           │   Arc<ForestRootWriteGate> │  (shared across services)          │
//! │           └──────────┬─────────────────┘                                    │
//! │                      │                                                      │
//! │                      ▼                                                      │
//! │  ┌──────────────────────────────────────────────────────────────────────┐   │
//! │  │                      ForestRootWriteGate                             │   │
//! │  │  ┌────────────────────────────────────────────────────────────────┐  │   │
//! │  │  │ semaphore: Semaphore(1)  ◄── Single permit for mutual exclusion│  │   │
//! │  │  └────────────────────────────────────────────────────────────────┘  │   │
//! │  │  ┌────────────────────────────────────────────────────────────────┐  │   │
//! │  │  │ release_tx: broadcast::Sender  ◄── Notifies when lock released │  │   │
//! │  │  └────────────────────────────────────────────────────────────────┘  │   │
//! │  └──────────────────────────────────────────────────────────────────────┘   │
//! │                      │                                                      │
//! │                      │ try_acquire()                                        │
//! │                      ▼                                                      │
//! │  ┌──────────────────────────────────────────────────────────────────────┐   │
//! │  │                    ForestRootWriteGuard                              │   │
//! │  │  ┌────────────────────────────────────────────────────────────────┐  │   │
//! │  │  │ _permit: OwnedSemaphorePermit  ◄── Auto-releases on drop       │  │   │
//! │  │  └────────────────────────────────────────────────────────────────┘  │   │
//! │  │  ┌────────────────────────────────────────────────────────────────┐  │   │
//! │  │  │ release_tx: broadcast::Sender  ◄── Sends notification on drop  │  │   │
//! │  │  └────────────────────────────────────────────────────────────────┘  │   │
//! │  └──────────────────────────────────────────────────────────────────────┘   │
//! │                      │                                                      │
//! │                      │ Drop (RAII)                                          │
//! │                      ▼                                                      │
//! │  ┌──────────────────────────────────────────────────────────────────────┐   │
//! │  │                    Release Notification                              │   │
//! │  │                                                                      │   │
//! │  │  1. Semaphore permit released (automatic via OwnedSemaphorePermit)   │   │
//! │  │  2. Broadcast sent to all subscribers via release_tx.send(())        │   │
//! │  │                                                                      │   │
//! │  │  Waiting services receive notification and can retry acquisition     │   │
//! │  └──────────────────────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## TOCTOU-Safe Acquisition Pattern
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │  Step 1: Acquire lock FIRST (before any state changes)                      │
//! │  ┌───────────────────────────────────────────────────────────────────────┐  │
//! │  │ let Some(guard) = gate.try_acquire() else { return; };                │  │
//! │  └───────────────────────────────────────────────────────────────────────┘  │
//! │                          │                                                  │
//! │                          │ Lock acquired ✓                                  │
//! │                          ▼                                                  │
//! │  Step 2: Safely modify state (protected by lock)                            │
//! │  ┌───────────────────────────────────────────────────────────────────────┐  │
//! │  │ let request = queue.pop_front();  // Safe: lock held                  │  │
//! │  └───────────────────────────────────────────────────────────────────────┘  │
//! │                          │                                                  │
//! │                          ▼                                                  │
//! │  Step 3: Pass guard to consumer (lock moves with ownership)                 │
//! │  ┌───────────────────────────────────────────────────────────────────────┐  │
//! │  │ process_request(request, guard);  // Guard ownership transferred      │  │
//! │  └───────────────────────────────────────────────────────────────────────┘  │
//! │                                                                             │
//! │  KEY: Lock acquired BEFORE state mutation                                   │
//! │  → No TOCTOU race: if try_acquire() fails, state unchanged                  │
//! │  → No data loss: failed acquisition = queues untouched                      │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Types
//!
//! - [`ForestRootWriteGuard`]: RAII guard that releases on drop
//! - [`ForestRootWriteGuardSlot`]: Cloneable wrapper for passing guards through channels
//! - [`ForestRootWriteAccess`]: Trait for types that may carry a lock
//! - [`ForestRootWriteGate`]: Thread-safe lock manager
//!
//! ## Usage
//!
//! For actor-based event handling integration, see `shc_actors_framework::forest_write_lock`.

use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, OwnedSemaphorePermit, Semaphore};

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

/// Thread-safe forest write lock manager using Semaphore.
///
/// Uses a single-permit semaphore for mutual exclusion and broadcast
/// channel for release notifications to waiting services.
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
