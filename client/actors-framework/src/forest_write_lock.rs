//! Forest write lock types for the actors framework.
//!
//! This module provides the core types for forest root write lock management:
//! - [`ForestRootWriteGuard`]: RAII guard that releases on drop
//! - [`ForestRootWriteGuardSlot`]: Cloneable wrapper for use in events
//! - [`ForestRootWriteAccess`]: Trait for uniform lock detection/extraction
//! - [`ForestRootWriteGuardedHandler`]: Wrapper that auto-extracts locks during event handling
//! - [`ForestRootWriteGate`]: Thread-safe lock manager for shared access across services

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tokio::sync::broadcast;

use crate::event_bus::{EventBusMessage, EventHandler};

const LOG_TARGET: &str = "forest-write-lock";

/// RAII guard for the forest root write lock.
///
/// When dropped, atomically releases the lock and broadcasts a notification
/// to all subscribers waiting for the lock to become available.
pub struct ForestRootWriteGuard {
    is_held: Arc<AtomicBool>,
    release_tx: broadcast::Sender<()>,
}

impl ForestRootWriteGuard {
    /// Creates a new guard.
    pub fn new(is_held: Arc<AtomicBool>, release_tx: broadcast::Sender<()>) -> Self {
        Self {
            is_held,
            release_tx,
        }
    }
}

impl Drop for ForestRootWriteGuard {
    fn drop(&mut self) {
        log::debug!(target: LOG_TARGET, "🔓 Guard DROP: Releasing lock and sending notification");
        // Atomically mark as released
        self.is_held.store(false, Ordering::Release);
        // Notify all subscribers (ignore if no receivers)
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

/// Thread-safe forest write lock manager for shared access across services.
///
/// Uses atomic operations for lock state, allowing it to be shared via `Arc`
/// across multiple services (BlockchainService, FishermanService, etc.).
///
/// ## Usage
///
/// ```ignore
/// // Create shared manager (typically in StorageHubBuilder)
/// let manager = Arc::new(ForestRootWriteGate::new());
///
/// // Any service can acquire the lock via shared reference
/// if let Some(guard) = manager.try_acquire() {
///     // Lock held until guard is dropped
///     emit_event(ProcessRequest { data, forest_root_write_lock: guard.into() });
/// }
///
/// // Subscribe to release notifications in event loops
/// let mut rx = manager.subscribe();
/// loop {
///     rx.recv().await;  // Notified when any guard is dropped
/// }
/// ```
pub struct ForestRootWriteGate {
    /// Atomic lock state - true if held, false if available.
    is_held: Arc<AtomicBool>,
    /// Broadcast sender for release notifications.
    release_tx: broadcast::Sender<()>,
}

impl ForestRootWriteGate {
    /// Creates a new shared forest write gate.
    pub fn new() -> Self {
        // Buffer of 16 is sufficient - release notifications are transient signals
        let (release_tx, _) = broadcast::channel(16);
        Self {
            is_held: Arc::new(AtomicBool::new(false)),
            release_tx,
        }
    }

    /// Tries to acquire the forest root write lock.
    ///
    /// Returns `Some(guard)` if the lock was acquired, `None` if already held.
    /// The returned guard will atomically release the lock when dropped.
    pub fn try_acquire(&self) -> Option<ForestRootWriteGuard> {
        // Atomic compare-and-swap: if is_held==false, set to true and return success
        if self
            .is_held
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            log::debug!(target: LOG_TARGET, "🔓 ForestRootWriteGate: acquired lock");
            Some(ForestRootWriteGuard::new(
                Arc::clone(&self.is_held),
                self.release_tx.clone(),
            ))
        } else {
            log::debug!(target: LOG_TARGET, "🔒 ForestRootWriteGate: lock already held");
            None
        }
    }

    /// Creates a new subscriber to lock release notifications.
    ///
    /// Subscribers receive `()` whenever any guard is dropped.
    /// Useful for event loops that need to process queued requests
    /// when the lock becomes available.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.release_tx.subscribe()
    }

    /// Returns whether the lock is currently held.
    pub fn is_locked(&self) -> bool {
        self.is_held.load(Ordering::Acquire)
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
            .field("is_held", &self.is_locked())
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
