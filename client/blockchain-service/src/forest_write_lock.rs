//! Forest Write Lock Manager
//!
//! This module provides the [`ForestWriteLockManager`] for coordinating exclusive access
//! to the runtime Forest root.
//!
//! ## Design Overview
//!
//! The lock system ensures only one task can write to the runtime Forest root at a time.
//!
//! Components:
//! - A boolean to track lock state
//! - An unbounded channel to notify the BlockchainService when the lock is released
//! - An RAII guard that automatically releases the lock on drop
//!
//! ## Usage
//!
//! ```ignore
//! // Create manager (returns receiver for the BlockchainService event loop)
//! let (manager, release_rx) = ForestWriteLockManager::new();
//!
//! // Try to acquire the lock
//! if let Some(guard) = manager.try_acquire() {
//!     // Lock acquired, emit event with guard
//!     emit_event(ProcessRequest { data, forest_root_write_lock: guard.into() });
//! }
//!
//! // When guard drops, () is sent to release_rx
//! // BlockchainService receives it and calls manager.mark_released()
//! ```

use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use log::warn;
use shc_common::traits::StorageEnableRuntime;
use tokio::sync::mpsc;

const LOG_TARGET: &str = "blockchain-service-forest-write-lock";

/// Sender type for forest root write lock release notifications.
///
/// This is a lightweight channel that only sends `()` to notify
/// the BlockchainService that a lock has been released.
pub type LockReleaseSender = mpsc::UnboundedSender<()>;

/// Receiver type for forest root write lock release notifications.
pub type LockReleaseReceiver = mpsc::UnboundedReceiver<()>;

/// Manager for forest root write locks.
///
/// Provides an API for acquiring and releasing forest root write locks
/// using boolean state tracking and channel-based release notifications.
///
/// ## Thread Safety
///
/// The manager is designed to be owned by a single thread.
/// The guard can be dropped from any thread, and the release notification will
/// be sent to the BlockchainService event loop.
pub struct ForestWriteLockManager<Runtime: StorageEnableRuntime> {
    /// Whether the lock is currently held.
    locked: bool,
    /// Channel sender for release notifications.
    /// Guards hold a clone of this and send `()` when dropped.
    release_tx: LockReleaseSender,
    _marker: PhantomData<Runtime>,
}

/// Creates a new lock release channel.
///
/// Returns `(sender, receiver)` where:
/// - The sender should be passed to `ForestWriteLockManager::with_sender()`
/// - The receiver should be polled in the BlockchainService event loop
pub fn lock_release_channel() -> (LockReleaseSender, LockReleaseReceiver) {
    mpsc::unbounded_channel()
}

impl<Runtime: StorageEnableRuntime> ForestWriteLockManager<Runtime> {
    /// Creates a new `ForestWriteLockManager` and returns the release notification receiver.
    ///
    /// The receiver should be stored in the BlockchainService and polled in the event loop.
    /// When `()` is received, call `mark_released()` and then the assign function.
    ///
    /// Use this constructor when you want the manager to create its own channel.
    /// For cases where the channel is created externally (e.g., at event loop startup),
    /// use [`Self::with_sender()`] instead.
    pub fn new() -> (Self, LockReleaseReceiver) {
        let (release_tx, release_rx) = mpsc::unbounded_channel();
        (
            Self {
                locked: false,
                release_tx,
                _marker: PhantomData,
            },
            release_rx,
        )
    }

    /// Creates a new `ForestWriteLockManager` with an existing release sender.
    ///
    /// This is useful when the lock release channel is created at event loop startup
    /// and the receiver needs to be integrated into the event loop's stream processing.
    ///
    /// The manager will use the provided sender for release notifications.
    pub fn with_sender(release_tx: LockReleaseSender) -> Self {
        Self {
            locked: false,
            release_tx,
            _marker: PhantomData,
        }
    }

    /// Tries to acquire the forest root write lock.
    ///
    /// Returns `Some(guard)` if the lock was acquired, `None` if the lock is already held.
    /// The guard will automatically release the lock when dropped.
    pub fn try_acquire(&mut self) -> Option<ForestRootWriteLockGuard<Runtime>> {
        if self.locked {
            None
        } else {
            self.locked = true;
            Some(ForestRootWriteLockGuard::new(self.release_tx.clone()))
        }
    }

    /// Marks the lock as released.
    ///
    /// This should be called by the BlockchainService when it receives a release
    /// notification on the release channel.
    pub fn mark_released(&mut self) {
        if !self.locked {
            warn!(
                target: LOG_TARGET,
                "Received lock release while not locked - possible spurious release"
            );
        }
        self.locked = false;
    }

    /// Returns whether the lock is currently held.
    pub fn is_locked(&self) -> bool {
        self.locked
    }
}

impl<Runtime: StorageEnableRuntime> std::fmt::Debug for ForestWriteLockManager<Runtime> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForestWriteLockManager")
            .field("locked", &self.locked)
            .finish()
    }
}

/// RAII guard for the forest root write lock.
///
/// This guard automatically releases the forest root write lock when dropped,
/// ensuring that the lock is always released regardless of how the event handler
/// exits (success, error, or panic).
///
/// On drop, the guard sends `()` through the release channel to notify the
/// BlockchainService, which then marks the lock as released and assigns it
/// to the next pending task.
pub struct ForestRootWriteLockGuard<Runtime: StorageEnableRuntime> {
    /// Channel sender to notify the BlockchainService of lock release.
    release_tx: LockReleaseSender,
    _marker: PhantomData<Runtime>,
}

impl<Runtime: StorageEnableRuntime> ForestRootWriteLockGuard<Runtime> {
    /// Creates a new `ForestRootWriteLockGuard`.
    ///
    /// This is called internally by `ForestWriteLockManager::try_acquire()`.
    pub(crate) fn new(release_tx: LockReleaseSender) -> Self {
        Self {
            release_tx,
            _marker: PhantomData,
        }
    }
}

impl<Runtime: StorageEnableRuntime> From<ForestRootWriteLockGuard<Runtime>>
    for Arc<Mutex<Option<ForestRootWriteLockGuard<Runtime>>>>
{
    /// Converts a guard into the event field type for use in events.
    ///
    /// This wraps the guard in `Arc<Mutex<Option<...>>>` which is required because:
    /// - Events need to implement `Clone` for the event bus
    /// - The lock guard should only be taken once
    /// - Multiple subscribers might receive the same event
    fn from(guard: ForestRootWriteLockGuard<Runtime>) -> Self {
        Arc::new(Mutex::new(Some(guard)))
    }
}

impl<Runtime: StorageEnableRuntime> std::fmt::Debug for ForestRootWriteLockGuard<Runtime> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForestRootWriteLockGuard").finish()
    }
}

impl<Runtime: StorageEnableRuntime> Drop for ForestRootWriteLockGuard<Runtime> {
    fn drop(&mut self) {
        // Send () to notify the BlockchainService that the lock is released.
        // We use unbounded_send since Drop is sync and we can't await.
        // If the channel is closed (BlockchainService shut down), the send fails silently.
        if let Err(e) = self.release_tx.send(()) {
            log::error!(target: LOG_TARGET, "Failed to send release signal: {}", e);
        }
    }
}

/// Trait for events that carry a forest root write lock.
///
/// Events implementing this trait can have their lock guard automatically
/// extracted and managed by the `ForestWriteHandler` wrapper, ensuring
/// automatic lock release on any exit path.
pub trait TakeForestWriteLock<Runtime: StorageEnableRuntime>: Send + 'static {
    /// Takes ownership of the forest root write lock guard from this event.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The mutex is poisoned (a thread panicked while holding the lock)
    /// - The guard has already been taken
    fn take_forest_root_write_lock(&self) -> anyhow::Result<ForestRootWriteLockGuard<Runtime>>;
}

/// Type alias for the forest root write lock field in events.
///
/// This is wrapped in `Arc<Mutex<Option<...>>>` because:
/// - Events need to implement `Clone` for the event bus
/// - The lock guard should only be taken once
/// - Multiple subscribers might receive the same event
pub type ForestRootWriteLock<Runtime> = Arc<Mutex<Option<ForestRootWriteLockGuard<Runtime>>>>;
