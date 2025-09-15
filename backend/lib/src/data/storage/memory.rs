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
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use parking_lot::RwLock;
use thiserror::Error;
use tokio::{task::JoinHandle, time::interval};

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
    address: String,
    expiration_timestamp: u64,
}

/// In-memory storage implementation
///
/// This implementation is thread-safe and suitable for development environments.
/// All data is lost when the process terminates.
#[derive(Clone)]
pub struct InMemoryStorage {
    nonces: Arc<RwLock<HashMap<String, NonceEntry>>>,
    cleanup_task: Arc<RwLock<Option<JoinHandle<()>>>>,
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
                let current_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let mut nonces_guard = nonces.write();
                nonces_guard.retain(|_, entry| entry.expiration_timestamp > current_time);
            }
        });

        *self.cleanup_task.write() = Some(handle);
    }

    pub async fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        let handle = self.cleanup_task.write().take();

        if let Some(handle) = handle {
            let _ = handle.await;
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
        address: String,
        expiration_seconds: u64,
    ) -> Result<(), Self::Error> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expiration_timestamp = current_time + expiration_seconds;

        let entry = NonceEntry {
            address,
            expiration_timestamp,
        };

        self.nonces.write().insert(message, entry);
        Ok(())
    }

    async fn get_nonce(&self, message: &str) -> Result<Option<String>, Self::Error> {
        let nonces = self.nonces.read();

        if let Some(entry) = nonces.get(message) {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            if current_time < entry.expiration_timestamp {
                return Ok(Some(entry.address.clone()));
            }
            // Entry is expired but will be cleaned up by the background task
        }

        Ok(None)
    }

    async fn remove_nonce(&self, message: &str) -> Result<(), Self::Error> {
        self.nonces.write().remove(message);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::advance;

    #[tokio::test]
    async fn test_health_check() {
        let storage = InMemoryStorage::new();

        assert!(storage.health_check().await.unwrap())
    }

    #[tokio::test]
    async fn can_store_and_retrieve_nonces() {
        let storage = InMemoryStorage::new();
        let message = "test_nonce_123";
        let address = "0x1234567890abcdef";
        let expiration_seconds = 300; // 5 minutes

        // Store nonce
        storage
            .store_nonce(message.to_string(), address.to_string(), expiration_seconds)
            .await
            .unwrap();

        // Retrieve nonce
        let retrieved = storage.get_nonce(message).await.unwrap();
        assert_eq!(retrieved, Some(address.to_string()));

        // Remove nonce
        storage.remove_nonce(message).await.unwrap();

        // Verify it's gone
        let retrieved_after_remove = storage.get_nonce(message).await.unwrap();
        assert_eq!(retrieved_after_remove, None);
    }

    #[tokio::test]
    async fn cannot_retrieve_expired_nonces() {
        let storage = InMemoryStorage::new();
        let message = "expired_nonce";
        let address = "0xdeadbeef";
        let expiration_seconds = 0; // Expire immediately

        // Store nonce with 0 expiration
        storage
            .store_nonce(message.to_string(), address.to_string(), expiration_seconds)
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
        let address = "0xcafebabe";
        let expiration_seconds = 1; // Expire after 1 second

        // Store nonce with 1 second expiration
        storage
            .store_nonce(message.to_string(), address.to_string(), expiration_seconds)
            .await
            .unwrap();

        // Should be retrievable immediately
        let retrieved = storage.get_nonce(message).await.unwrap();
        assert_eq!(retrieved, Some(address.to_string()));

        // Advance time by 2 seconds to expire the nonce
        advance(Duration::from_secs(2)).await;

        // Should return None since it's expired
        let retrieved_after_expiry = storage.get_nonce(message).await.unwrap();
        assert_eq!(retrieved_after_expiry, None);

        // Advance time to trigger cleanup task (runs every 10 seconds)
        advance(Duration::from_secs(10)).await;

        // Should be gone from storage after cleanup task runs
        assert!(storage.nonces.read().get(message).is_none());

        // Shutdown cleanup task
        storage.shutdown().await;
    }
}
