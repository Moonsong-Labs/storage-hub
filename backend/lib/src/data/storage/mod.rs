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
}
