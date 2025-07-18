//! Storage traits for backend-specific data
//!
//! This module defines the storage interfaces for data that is specific to the backend
//! and not part of the StorageHub indexer database. This includes counters, sessions,
//! caches, and other temporary or backend-specific data.

use async_trait::async_trait;
use std::error::Error;

/// Storage trait for backend-specific data operations
///
/// This trait provides an abstraction over different storage backends (in-memory, Redis, etc.)
/// for data that is specific to the backend service and not part of the indexer database.
#[async_trait]
pub trait Storage: Send + Sync {
    /// Error type for storage operations
    type Error: Error + Send + Sync + 'static;

    /// Increment a counter by the specified amount
    ///
    /// # Arguments
    /// * `key` - The counter identifier
    /// * `amount` - The amount to increment (default: 1)
    ///
    /// # Returns
    /// The new counter value after incrementing
    async fn increment_counter(&self, key: &str, amount: i64) -> Result<i64, Self::Error>;

    /// Decrement a counter by the specified amount
    ///
    /// # Arguments
    /// * `key` - The counter identifier
    /// * `amount` - The amount to decrement (default: 1)
    ///
    /// # Returns
    /// The new counter value after decrementing
    async fn decrement_counter(&self, key: &str, amount: i64) -> Result<i64, Self::Error>;

    /// Get the current value of a counter
    ///
    /// # Arguments
    /// * `key` - The counter identifier
    ///
    /// # Returns
    /// The current counter value, or 0 if the counter doesn't exist
    async fn get_counter(&self, key: &str) -> Result<i64, Self::Error>;

    /// Set a counter to a specific value
    ///
    /// # Arguments
    /// * `key` - The counter identifier
    /// * `value` - The value to set
    ///
    /// # Returns
    /// The previous counter value, or 0 if the counter didn't exist
    async fn set_counter(&self, key: &str, value: i64) -> Result<i64, Self::Error>;

    /// Delete a counter
    ///
    /// # Arguments
    /// * `key` - The counter identifier
    ///
    /// # Returns
    /// The counter value before deletion, or 0 if the counter didn't exist
    async fn delete_counter(&self, key: &str) -> Result<i64, Self::Error>;
}