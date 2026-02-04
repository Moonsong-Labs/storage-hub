use log::warn;
use tokio::sync::{mpsc::UnboundedSender, OwnedSemaphorePermit};

use crate::handler::LOG_TARGET;

/// Notification sent to the Fisherman service event loop when a batch deletion
/// semaphore permit is released (dropped).
#[derive(Debug, Clone, Copy)]
pub struct BatchDeletionPermitReleased;

/// RAII wrapper for batch deletion permits that notifies the fisherman service
/// when the permit is dropped via the `release_notifier` channel.
///
/// This mirrors the `ForestWritePermitGuard` pattern used by the blockchain service.
#[derive(Debug)]
pub struct BatchDeletionPermitGuard {
    _permit: OwnedSemaphorePermit,
    release_notifier: UnboundedSender<BatchDeletionPermitReleased>,
}

impl BatchDeletionPermitGuard {
    /// Creates a new `BatchDeletionPermitGuard` wrapping the given permit.
    ///
    /// When this guard is dropped, a notification will be sent through
    /// `release_notifier` to the fisherman service event loop.
    pub fn new(
        permit: OwnedSemaphorePermit,
        release_notifier: UnboundedSender<BatchDeletionPermitReleased>,
    ) -> Self {
        Self {
            _permit: permit,
            release_notifier,
        }
    }
}

impl Drop for BatchDeletionPermitGuard {
    fn drop(&mut self) {
        // Ignore errors as the receiver may be dropped during shutdown.
        if let Err(e) = self.release_notifier.send(BatchDeletionPermitReleased) {
            warn!(
                target: LOG_TARGET,
                "Failed to send batch deletion permit release notification: {}",
                e
            );
        }
    }
}
