//! Storage data access module

use std::error::Error;

use alloy_core::primitives::Address;
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

    async fn health_check(&self) -> Result<bool, Self::Error>;

    /// Store a nonce with associated data (message as key, address and expiration as value)
    ///
    /// Returns the raw result from the storage operation
    async fn store_nonce(
        &self,
        message: String,
        address: &Address,
        expiration_seconds: u64,
    ) -> Result<(), Self::Error>;

    /// Retrieve nonce data by message. Will remove the nonce from storage.
    ///
    /// Returns None if not found or expired
    async fn get_nonce(&self, message: &str) -> Result<Option<Address>, Self::Error>;
}
