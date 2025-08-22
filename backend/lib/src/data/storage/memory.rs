//! In-memory storage implementation
//!
//! TODO(SCAFFOLDING): This in-memory storage is for development/testing only.
//! Production MSP should use persistent storage (PostgreSQL, Redis, etc.)
//!
//! This module provides a thread-safe in-memory implementation of the Storage trait,
//! suitable for development and testing environments.

use std::{collections::HashMap, sync::Arc};

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

/// In-memory storage implementation using Arc<RwLock<HashMap>>
///
/// TODO(SCAFFOLDING): Example storage implementation for demonstration.
/// Replace with actual persistent storage in production.
///
/// This implementation is thread-safe and suitable for development environments.
/// All data is lost when the process terminates.
#[derive(Default, Clone)]
pub struct InMemoryStorage {
    map: Arc<RwLock<HashMap<String, String>>>,
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
        // just to "use" `map`
        Ok(self.map.read().capacity() >= 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let storage = InMemoryStorage::new();

        assert_eq!(storage.health_check().await.unwrap(), true)
    }
}
