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
        let mut counters = self.counters.write();

        let value = counters.entry(key.to_string()).or_insert(0);
        *value = value.saturating_add(amount);
        Ok(*value)
    }

    async fn decrement_counter(&self, key: &str, amount: i64) -> Result<i64, Self::Error> {
        let mut counters = self.counters.write();

        let value = counters.entry(key.to_string()).or_insert(0);
        *value = value.saturating_sub(amount);
        Ok(*value)
    }

    async fn get_counter(&self, key: &str) -> Result<i64, Self::Error> {
        let counters = self.counters.read();

        Ok(counters.get(key).copied().unwrap_or(0))
    }

    async fn set_counter(&self, key: &str, value: i64) -> Result<i64, Self::Error> {
        let mut counters = self.counters.write();

        let previous = counters.insert(key.to_string(), value);
        Ok(previous.unwrap_or(0))
    }

    async fn delete_counter(&self, key: &str) -> Result<i64, Self::Error> {
        let mut counters = self.counters.write();

        Ok(counters.remove(key).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::test::counter::*;

    #[tokio::test]
    async fn test_increment_counter() {
        let storage = InMemoryStorage::new();

        let result = storage
            .increment_counter(TEST_COUNTER_KEY, DEFAULT_INCREMENT)
            .await
            .unwrap();
        assert_eq!(result, DEFAULT_INCREMENT);

        let result = storage
            .increment_counter(TEST_COUNTER_KEY, DEFAULT_INCREMENT)
            .await
            .unwrap();
        assert_eq!(result, DEFAULT_INCREMENT * 2);

        let result = storage
            .increment_counter(TEST_COUNTER_KEY, LARGE_INCREMENT)
            .await
            .unwrap();
        assert_eq!(result, DEFAULT_INCREMENT * 2 + LARGE_INCREMENT);
    }

    #[tokio::test]
    async fn test_decrement_counter() {
        let storage = InMemoryStorage::new();

        storage
            .set_counter(TEST_COUNTER_KEY, SET_VALUE)
            .await
            .unwrap();

        let result = storage
            .decrement_counter(TEST_COUNTER_KEY, DEFAULT_INCREMENT)
            .await
            .unwrap();
        assert_eq!(result, SET_VALUE - DEFAULT_INCREMENT);

        let result = storage
            .decrement_counter(TEST_COUNTER_KEY, LARGE_INCREMENT)
            .await
            .unwrap();
        assert_eq!(result, SET_VALUE - DEFAULT_INCREMENT - LARGE_INCREMENT);
    }

    #[tokio::test]
    async fn test_get_counter() {
        let storage = InMemoryStorage::new();

        let result = storage.get_counter(TEST_COUNTER_KEY).await.unwrap();
        assert_eq!(result, INITIAL_VALUE);

        storage
            .set_counter(TEST_COUNTER_KEY, EXPECTED_VALUE)
            .await
            .unwrap();
        let result = storage.get_counter(TEST_COUNTER_KEY).await.unwrap();
        assert_eq!(result, EXPECTED_VALUE);
    }

    #[tokio::test]
    async fn test_set_counter() {
        let storage = InMemoryStorage::new();

        let result = storage
            .set_counter(TEST_COUNTER_KEY, SET_VALUE)
            .await
            .unwrap();
        assert_eq!(result, INITIAL_VALUE);

        let result = storage
            .set_counter(TEST_COUNTER_KEY, SET_VALUE * 2)
            .await
            .unwrap();
        assert_eq!(result, SET_VALUE);
    }

    #[tokio::test]
    async fn test_delete_counter() {
        let storage = InMemoryStorage::new();

        let result = storage.delete_counter(TEST_COUNTER_KEY).await.unwrap();
        assert_eq!(result, INITIAL_VALUE);

        storage
            .set_counter(TEST_COUNTER_KEY, EXPECTED_VALUE)
            .await
            .unwrap();
        let result = storage.delete_counter(TEST_COUNTER_KEY).await.unwrap();
        assert_eq!(result, EXPECTED_VALUE);

        let result = storage.get_counter(TEST_COUNTER_KEY).await.unwrap();
        assert_eq!(result, INITIAL_VALUE);
    }

    #[tokio::test]
    async fn test_saturation_arithmetic() {
        let storage = InMemoryStorage::new();

        storage
            .set_counter(TEST_COUNTER_KEY, i64::MAX - 1)
            .await
            .unwrap();
        let result = storage
            .increment_counter(TEST_COUNTER_KEY, 2)
            .await
            .unwrap();
        assert_eq!(result, i64::MAX);

        storage
            .set_counter(TEST_COUNTER_KEY, i64::MIN + 1)
            .await
            .unwrap();
        let result = storage
            .decrement_counter(TEST_COUNTER_KEY, 2)
            .await
            .unwrap();
        assert_eq!(result, i64::MIN);
    }
}
