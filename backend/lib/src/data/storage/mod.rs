//! Storage data access module

use std::error::Error;

use alloy_core::primitives::Address;
use async_trait::async_trait;

pub mod boxed;
pub mod memory;

pub use boxed::{BoxedStorage, BoxedStorageWrapper};
pub use memory::InMemoryStorage;

/// Represents the various possibilities of retrieval for an expirable value
#[derive(Debug, PartialEq, Eq)]
pub enum WithExpiry<T> {
    /// The value was found and is still valid
    Valid(T),
    /// The value was found but has expired
    Expired,
    /// The value was not found
    NotFound,
}

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
    async fn get_nonce(&self, message: &str) -> Result<WithExpiry<Address>, Self::Error>;
}
