//! In-memory storage implementation
//!
//! TODO(SCAFFOLDING): This in-memory storage is for development/testing only.
//! Production MSP should use persistent storage (PostgreSQL, Redis, etc.)
//!
//! This module provides a thread-safe in-memory implementation of the Storage trait,
//! suitable for development and testing environments.

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use alloy_core::primitives::Address;
use async_trait::async_trait;
use parking_lot::RwLock;
use thiserror::Error;
use tokio::{
    task::JoinHandle,
    time::{interval, Instant},
};
use tracing::warn;

use super::Storage;

/// Errors that can occur during in-memory storage operations
#[derive(Debug, Error)]
pub enum InMemoryStorageError {
    // Currently no errors are possible with parking_lot RwLock
    // This enum is kept for future extensibility
}

/// Nonce entry with address and expiration timestamp
#[derive(Clone, Debug)]
struct NonceEntry {
    /// The user address associated with the nonce key
    address: Address,
    /// Timestamp when the nonce will expire from storage
    expires_at: Instant,
}

/// In-memory storage implementation
///
/// This implementation is thread-safe and suitable for development environments.
/// All data is lost when the process terminates.
#[derive(Clone)]
pub struct InMemoryStorage {
    /// Contains the authentication nonces<->user address relations, mapping a given nonce to the corresponding user address that requested the nonce
    nonces: Arc<RwLock<HashMap<String, NonceEntry>>>,

    /// Handle for the nonce cleanup task
    ///
    /// The cleanup task is in charge of finding expired nonces and removing them from the map
    cleanup_task: Arc<RwLock<Option<JoinHandle<()>>>>,

    /// Signal for the cleanup task to terminate
    shutdown: Arc<AtomicBool>,
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryStorage {
    pub fn new() -> Self {
        let storage = Self {
            nonces: Arc::new(RwLock::new(HashMap::new())),
            cleanup_task: Arc::new(RwLock::new(None)),
            shutdown: Arc::new(AtomicBool::new(false)),
        };

        // Start the cleanup task
        storage.start_cleanup_task();
        storage
    }

    fn start_cleanup_task(&self) {
        let nonces = self.nonces.clone();
        let shutdown = self.shutdown.clone();

        let handle = tokio::spawn(async move {
            let mut cleanup_interval = interval(Duration::from_secs(10)); // Check every 10 seconds

            loop {
                cleanup_interval.tick().await;

                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                // Clean up expired nonces
                let now = Instant::now();

                let mut nonces_guard = nonces.write();
                nonces_guard.retain(|_, entry| entry.expires_at > now);
            }
        });

        *self.cleanup_task.write() = Some(handle);
    }
}

impl Drop for InMemoryStorage {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.cleanup_task.write().take() {
            // This doesn't ensure the task is shut down before the end of the drop impl
            tokio::task::spawn(async move {
                match tokio::time::timeout(Duration::from_secs(5), handle).await {
                    Ok(result) => {
                        if let Err(e) = result {
                            warn!(error = ?e, "Cleanup task failed during shutdown");
                        }
                    }
                    Err(_) => warn!("Cleanup task did not complete within timeout"),
                }
            });
        }
    }
}

#[async_trait]
impl Storage for InMemoryStorage {
    type Error = InMemoryStorageError;

    async fn health_check(&self) -> Result<bool, Self::Error> {
        Ok(true)
    }

    async fn store_nonce(
        &self,
        message: String,
        address: &Address,
        expiration_seconds: u64,
    ) -> Result<(), Self::Error> {
        let now = Instant::now();
        let expires_at = now + Duration::from_secs(expiration_seconds);

        let entry = NonceEntry {
            address: *address,
            expires_at,
        };

        self.nonces.write().insert(message, entry);
        Ok(())
    }

    async fn get_nonce(&self, message: &str) -> Result<Option<Address>, Self::Error> {
        let mut nonces = self.nonces.write();

        if let Some(entry) = nonces.remove(message) {
            let now = Instant::now();

            if now < entry.expires_at {
                return Ok(Some(entry.address));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use tokio::time::advance;

    use super::*;

    use crate::constants::mocks::MOCK_ADDRESS;

    #[tokio::test]
    async fn test_health_check() {
        let storage = InMemoryStorage::new();

        assert!(storage.health_check().await.unwrap())
    }

    #[tokio::test]
    async fn can_store_and_retrieve_nonces() {
        let storage = InMemoryStorage::new();
        let message = "test_nonce_123";
        let address = MOCK_ADDRESS;
        let expiration_seconds = 300; // 5 minutes

        // Store nonce
        storage
            .store_nonce(message.to_string(), &address, expiration_seconds)
            .await
            .unwrap();

        // Retrieve nonce
        let retrieved = storage.get_nonce(message).await.unwrap();
        assert_eq!(retrieved, Some(address));

        // Verify it can't be retrieved twice
        let retrieved_again = storage.get_nonce(message).await.unwrap();
        assert_eq!(retrieved_again, None);
    }

    #[tokio::test]
    async fn cannot_retrieve_expired_nonces() {
        let storage = InMemoryStorage::new();
        let message = "expired_nonce";
        let address = MOCK_ADDRESS;
        let expiration_seconds = 0; // Expire immediately

        // Store nonce with 0 expiration
        storage
            .store_nonce(message.to_string(), &address, expiration_seconds)
            .await
            .unwrap();

        // Try to retrieve - should be None since it's expired
        let retrieved = storage.get_nonce(message).await.unwrap();
        assert_eq!(retrieved, None);
    }

    #[tokio::test(start_paused = true)]
    async fn nonce_cleaned_up_after_expiry() {
        let storage = InMemoryStorage::new();
        let message = "auto_cleanup_nonce";
        let address = MOCK_ADDRESS;
        let expiration_seconds = 1; // Expire after 1 second

        // Store nonce with 1 second expiration
        storage
            .store_nonce(message.to_string(), &address, expiration_seconds)
            .await
            .unwrap();

        // Should be retrievable immediately
        let retrieved = storage.get_nonce(message).await.unwrap();
        assert_eq!(retrieved, Some(address));

        // Advance time by 2 seconds to expire the nonce
        advance(Duration::from_secs(2)).await;

        // Should return None since it's expired
        let retrieved_after_expiry = storage.get_nonce(message).await.unwrap();
        assert_eq!(retrieved_after_expiry, None);

        // Advance time to trigger cleanup task (runs every 10 seconds)
        advance(Duration::from_secs(10)).await;

        // Should be gone from storage after cleanup task runs
        assert!(storage.nonces.read().get(message).is_none());
    }
}
