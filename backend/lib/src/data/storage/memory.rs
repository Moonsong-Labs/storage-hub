//! In-memory storage implementation
//!
//! TODO(SCAFFOLDING): This in-memory storage is for development/testing only.
//! Production MSP should use persistent storage (PostgreSQL, Redis, etc.)
//!
//! This module provides a thread-safe in-memory implementation of the Storage trait,
//! suitable for development and testing environments.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use parking_lot::RwLock;
use thiserror::Error;

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
#[derive(Default, Clone)]
pub struct InMemoryStorage {
    nonces: Arc<RwLock<HashMap<String, NonceEntry>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self::default()
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
        // TODO: use duration
        expiration_seconds: u64,
    ) -> Result<(), Self::Error> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // TODO: spawn task to cleanup storage after expiration
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

    #[tokio::test]
    async fn test_health_check() {
        let storage = InMemoryStorage::new();

        assert!(storage.health_check().await.unwrap())
    }

    // TODO: add tests for nonces
    // * can store/retrieve
    // * can't retrieve expired
}
