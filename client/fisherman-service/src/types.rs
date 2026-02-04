use log::warn;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{mpsc::UnboundedSender, OwnedSemaphorePermit};

use crate::handler::LOG_TARGET;

/// Notification sent to the Fisherman service event loop when a batch deletion
/// semaphore permit is released (dropped).
#[derive(Debug, Clone, Copy)]
pub struct BatchDeletionPermitReleased {
    /// Whether the completed batch attempted at least one deletion target.
    ///
    /// If `false`, the batch found no work and the scheduler should back off using the idle
    /// poll interval rather than the cooldown.
    pub did_work: bool,
}

/// RAII wrapper for batch deletion permits that notifies the fisherman service
/// when the permit is dropped via the `release_notifier` channel.
///
/// This mirrors the `ForestWritePermitGuard` pattern used by the blockchain service.
#[derive(Debug)]
pub struct BatchDeletionPermitGuard {
    _permit: OwnedSemaphorePermit,
    release_notifier: UnboundedSender<BatchDeletionPermitReleased>,
    did_work: AtomicBool,
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
            did_work: AtomicBool::new(false),
        }
    }

    /// Marks that this batch attempted at least one deletion target.
    pub fn mark_did_work(&self) {
        self.did_work.store(true, Ordering::Relaxed);
    }
}

impl Drop for BatchDeletionPermitGuard {
    fn drop(&mut self) {
        // Ignore errors as the receiver may be dropped during shutdown.
        let did_work = self.did_work.load(Ordering::Relaxed);
        if let Err(e) = self
            .release_notifier
            .send(BatchDeletionPermitReleased { did_work })
        {
            warn!(
                target: LOG_TARGET,
                "Failed to send batch deletion permit release notification: {}",
                e
            );
        }
    }
}
