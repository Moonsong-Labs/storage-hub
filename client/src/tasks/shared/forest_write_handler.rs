//! Forest Write Handler wrapper for automatic lock management.
//!
//! This module provides the `ForestWriteHandler` wrapper which automatically
//! extracts and holds the forest root write lock guard for the duration of
//! event handling, ensuring the lock is always released when the handler
//! completes (success, error, or panic).

use std::marker::PhantomData;

use anyhow::Result;
use shc_actors_framework::event_bus::{EventBusMessage, EventHandler};
use shc_blockchain_service::{ForestRootWriteLockGuard, TakeForestWriteLock};
use shc_common::traits::StorageEnableRuntime;

/// A wrapper handler that automatically manages the forest root write lock.
///
/// This wrapper extracts the `ForestRootWriteLockGuard` from events that implement
/// `TakeForestWriteLock` before calling the inner handler. The guard is held
/// for the duration of the inner handler's execution and automatically released
/// on drop, regardless of the handler's outcome.
///
/// # Type Parameters
///
/// * `H` - The inner event handler type
/// * `Runtime` - The runtime type parameter
///
/// # Example
///
/// ```ignore
/// // Wrap an existing handler with ForestWriteHandler:
/// let wrapped = ForestWriteHandler::new(my_bsp_submit_proof_task);
///
/// // When subscribed, the wrapper will:
/// // 1. Extract the guard from incoming events
/// // 2. Call the inner handler
/// // 3. Automatically release the lock when done
/// ```
#[derive(Clone)]
pub struct ForestWriteHandler<H, Runtime> {
    inner: H,
    _marker: PhantomData<Runtime>,
}

impl<H, Runtime> ForestWriteHandler<H, Runtime> {
    /// Creates a new `ForestWriteHandler` wrapping the given handler.
    pub fn new(inner: H) -> Self {
        Self {
            inner,
            _marker: PhantomData,
        }
    }
}

impl<E, H, Runtime> EventHandler<E> for ForestWriteHandler<H, Runtime>
where
    E: EventBusMessage + TakeForestWriteLock<Runtime>,
    H: EventHandler<E>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: E) -> Result<String> {
        // Extract the guard from the event. This will be held for the duration
        // of the inner handler's execution and released automatically on drop.
        let _guard: ForestRootWriteLockGuard<Runtime> = event.take_forest_root_write_lock();

        // Call the inner handler. The guard stays in scope until this completes.
        self.inner.handle_event(event).await

        // Guard is dropped here, triggering automatic lock release
    }
}
