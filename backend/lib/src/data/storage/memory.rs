//! In-memory storage implementation
//!
//! This module provides a thread-safe in-memory implementation of the Storage trait,
//! suitable for development and testing environments.

use super::traits::Storage;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// Errors that can occur during in-memory storage operations
#[derive(Debug, Error)]
pub enum InMemoryStorageError {
    /// Lock poisoned error
    #[error("Lock poisoned: {0}")]
    LockPoisoned(String),
}

/// In-memory storage implementation using Arc<RwLock<HashMap>>
///
/// This implementation is thread-safe and suitable for development environments.
/// All data is lost when the process terminates.
#[derive(Clone)]
pub struct InMemoryStorage {
    /// Thread-safe map of counters
    counters: Arc<RwLock<HashMap<String, i64>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Storage for InMemoryStorage {
    type Error = InMemoryStorageError;

    async fn increment_counter(&self, key: &str, amount: i64) -> Result<i64, Self::Error> {
        let mut counters = self
            .counters
            .write()
            .map_err(|e| InMemoryStorageError::LockPoisoned(e.to_string()))?;

        let value = counters.entry(key.to_string()).or_insert(0);
        *value = value.saturating_add(amount);
        Ok(*value)
    }

    async fn decrement_counter(&self, key: &str, amount: i64) -> Result<i64, Self::Error> {
        let mut counters = self
            .counters
            .write()
            .map_err(|e| InMemoryStorageError::LockPoisoned(e.to_string()))?;

        let value = counters.entry(key.to_string()).or_insert(0);
        *value = value.saturating_sub(amount);
        Ok(*value)
    }

    async fn get_counter(&self, key: &str) -> Result<i64, Self::Error> {
        let counters = self
            .counters
            .read()
            .map_err(|e| InMemoryStorageError::LockPoisoned(e.to_string()))?;

        Ok(counters.get(key).copied().unwrap_or(0))
    }

    async fn set_counter(&self, key: &str, value: i64) -> Result<i64, Self::Error> {
        let mut counters = self
            .counters
            .write()
            .map_err(|e| InMemoryStorageError::LockPoisoned(e.to_string()))?;

        let previous = counters.insert(key.to_string(), value);
        Ok(previous.unwrap_or(0))
    }

    async fn delete_counter(&self, key: &str) -> Result<i64, Self::Error> {
        let mut counters = self
            .counters
            .write()
            .map_err(|e| InMemoryStorageError::LockPoisoned(e.to_string()))?;

        Ok(counters.remove(key).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_increment_counter() {
        let storage = InMemoryStorage::new();

        // First increment should return 1
        let result = storage.increment_counter("test", 1).await.unwrap();
        assert_eq!(result, 1);

        // Second increment should return 2
        let result = storage.increment_counter("test", 1).await.unwrap();
        assert_eq!(result, 2);

        // Increment by 5 should return 7
        let result = storage.increment_counter("test", 5).await.unwrap();
        assert_eq!(result, 7);
    }

    #[tokio::test]
    async fn test_decrement_counter() {
        let storage = InMemoryStorage::new();

        // Set initial value
        storage.set_counter("test", 10).await.unwrap();

        // Decrement by 1 should return 9
        let result = storage.decrement_counter("test", 1).await.unwrap();
        assert_eq!(result, 9);

        // Decrement by 5 should return 4
        let result = storage.decrement_counter("test", 5).await.unwrap();
        assert_eq!(result, 4);
    }

    #[tokio::test]
    async fn test_get_counter() {
        let storage = InMemoryStorage::new();

        // Non-existent counter should return 0
        let result = storage.get_counter("test").await.unwrap();
        assert_eq!(result, 0);

        // Set value and verify
        storage.set_counter("test", 42).await.unwrap();
        let result = storage.get_counter("test").await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_set_counter() {
        let storage = InMemoryStorage::new();

        // First set should return 0 (no previous value)
        let result = storage.set_counter("test", 10).await.unwrap();
        assert_eq!(result, 0);

        // Second set should return previous value
        let result = storage.set_counter("test", 20).await.unwrap();
        assert_eq!(result, 10);
    }

    #[tokio::test]
    async fn test_delete_counter() {
        let storage = InMemoryStorage::new();

        // Delete non-existent counter should return 0
        let result = storage.delete_counter("test").await.unwrap();
        assert_eq!(result, 0);

        // Set value, delete, and verify return value
        storage.set_counter("test", 42).await.unwrap();
        let result = storage.delete_counter("test").await.unwrap();
        assert_eq!(result, 42);

        // Verify counter is deleted
        let result = storage.get_counter("test").await.unwrap();
        assert_eq!(result, 0);
    }

    #[tokio::test]
    async fn test_saturation_arithmetic() {
        let storage = InMemoryStorage::new();

        // Test overflow protection
        storage.set_counter("test", i64::MAX - 1).await.unwrap();
        let result = storage.increment_counter("test", 2).await.unwrap();
        assert_eq!(result, i64::MAX);

        // Test underflow protection
        storage.set_counter("test", i64::MIN + 1).await.unwrap();
        let result = storage.decrement_counter("test", 2).await.unwrap();
        assert_eq!(result, i64::MIN);
    }
}
