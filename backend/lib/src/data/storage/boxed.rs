//! Type-erased storage implementation

use std::error::Error as StdError;

use alloy_core::primitives::Address;
use async_trait::async_trait;

use super::Storage;

/// A boxed storage error that can wrap any storage implementation's error type
pub type BoxedStorageError = Box<dyn StdError + Send + Sync>;

/// Type-erased storage trait for use across service boundaries
#[async_trait]
pub trait BoxedStorage: Send + Sync {
    async fn health_check(&self) -> Result<bool, BoxedStorageError>;

    async fn store_nonce(
        &self,
        message: String,
        address: &Address,
        expiration_seconds: u64,
    ) -> Result<(), BoxedStorageError>;

    async fn get_nonce(&self, message: &str) -> Result<Option<Address>, BoxedStorageError>;
}

/// Wrapper struct that implements BoxedStorage for any Storage implementation
pub struct BoxedStorageWrapper<S: Storage> {
    inner: S,
}

impl<S: Storage> BoxedStorageWrapper<S> {
    fn wrap_err<E: StdError + Send + Sync + 'static>(err: E) -> BoxedStorageError {
        Box::new(err) as BoxedStorageError
    }
}

impl<S: Storage> BoxedStorageWrapper<S> {
    pub fn new(storage: S) -> Self {
        Self { inner: storage }
    }
}

#[async_trait]
impl<S> BoxedStorage for BoxedStorageWrapper<S>
where
    S: Storage + Send + Sync,
    S::Error: StdError + Send + Sync + 'static,
{
    async fn health_check(&self) -> Result<bool, BoxedStorageError> {
        self.inner.health_check().await.map_err(Self::wrap_err)
    }

    async fn store_nonce(
        &self,
        message: String,
        address: &Address,
        expiration_seconds: u64,
    ) -> Result<(), BoxedStorageError> {
        self.inner
            .store_nonce(message, address, expiration_seconds)
            .await
            .map_err(Self::wrap_err)
    }

    async fn get_nonce(&self, message: &str) -> Result<Option<Address>, BoxedStorageError> {
        self.inner.get_nonce(message).await.map_err(Self::wrap_err)
    }
}
