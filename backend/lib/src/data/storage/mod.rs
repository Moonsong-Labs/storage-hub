//! Storage data access module

use std::error::Error;

use async_trait::async_trait;

pub mod boxed;
pub mod memory;

pub use boxed::{BoxedStorage, BoxedStorageWrapper};
pub use memory::InMemoryStorage;

#[cfg(test)]
pub fn test_storage() -> std::sync::Arc<dyn BoxedStorage> {
    use std::sync::Arc;
    let memory_storage = InMemoryStorage::new();
    let boxed_storage = BoxedStorageWrapper::new(memory_storage);
    Arc::new(boxed_storage)
}

/// Storage trait for backend-specific data operations
#[async_trait]
pub trait Storage: Send + Sync {
    /// Error type for storage operations
    type Error: Error + Send + Sync + 'static;

    /// Increment a counter by the specified amount
    async fn increment_counter(&self, key: &str, amount: i64) -> Result<i64, Self::Error>;

    /// Decrement a counter by the specified amount
    async fn decrement_counter(&self, key: &str, amount: i64) -> Result<i64, Self::Error>;

    /// Get the current value of a counter (returns 0 if not found)
    async fn get_counter(&self, key: &str) -> Result<i64, Self::Error>;

    /// Set a counter to a specific value (returns previous value)
    async fn set_counter(&self, key: &str, value: i64) -> Result<i64, Self::Error>;

    /// Delete a counter (returns value before deletion)
    async fn delete_counter(&self, key: &str) -> Result<i64, Self::Error>;
}
