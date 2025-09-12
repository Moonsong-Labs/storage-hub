//! Storage data access module

use std::error::Error;

use async_trait::async_trait;

pub mod boxed;
pub mod memory;

pub use boxed::{BoxedStorage, BoxedStorageWrapper};
pub use memory::InMemoryStorage;

/// Storage trait for backend-specific data operations
#[async_trait]
pub trait Storage: Send + Sync {
    /// Error type for storage operations
    type Error: Error + Send + Sync + 'static;

    // TODO(SCAFFOLDING): The methods here are for demonstration.
    // Should be replaced with appropriate methods for what needs to be stored
    // in the backend's memory

    async fn health_check(&self) -> Result<bool, Self::Error>;

    /// Store a nonce with associated data (message as key, address and expiration as value)
    /// Returns the raw result from the storage operation
    async fn store_nonce(
        &self,
        message: String,
        address: String,
        expiration_seconds: u64,
    ) -> Result<(), Self::Error>;

    /// Retrieve nonce data by message
    /// Returns None if not found or expired
    async fn get_nonce(&self, message: &str) -> Result<Option<String>, Self::Error>;

    /// Remove a nonce entry
    async fn remove_nonce(&self, message: &str) -> Result<(), Self::Error>;
}
