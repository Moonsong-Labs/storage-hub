//! Type-erased storage implementation

use std::error::Error as StdError;

use async_trait::async_trait;

use super::Storage;

/// A boxed storage error that can wrap any storage implementation's error type
pub type BoxedStorageError = Box<dyn StdError + Send + Sync>;

/// Type-erased storage trait for use across service boundaries
#[async_trait]
pub trait BoxedStorage: Send + Sync {
    async fn increment_counter(&self, key: &str, amount: i64) -> Result<i64, BoxedStorageError>;
    async fn decrement_counter(&self, key: &str, amount: i64) -> Result<i64, BoxedStorageError>;
    async fn get_counter(&self, key: &str) -> Result<i64, BoxedStorageError>;
    async fn set_counter(&self, key: &str, value: i64) -> Result<i64, BoxedStorageError>;
    async fn delete_counter(&self, key: &str) -> Result<i64, BoxedStorageError>;
}

/// Wrapper struct that implements BoxedStorage for any Storage implementation
pub struct BoxedStorageWrapper<S: Storage> {
    inner: S,
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
    async fn increment_counter(&self, key: &str, amount: i64) -> Result<i64, BoxedStorageError> {
        self.inner
            .increment_counter(key, amount)
            .await
            .map_err(|e| Box::new(e) as BoxedStorageError)
    }

    async fn decrement_counter(&self, key: &str, amount: i64) -> Result<i64, BoxedStorageError> {
        self.inner
            .decrement_counter(key, amount)
            .await
            .map_err(|e| Box::new(e) as BoxedStorageError)
    }

    async fn get_counter(&self, key: &str) -> Result<i64, BoxedStorageError> {
        self.inner
            .get_counter(key)
            .await
            .map_err(|e| Box::new(e) as BoxedStorageError)
    }

    async fn set_counter(&self, key: &str, value: i64) -> Result<i64, BoxedStorageError> {
        self.inner
            .set_counter(key, value)
            .await
            .map_err(|e| Box::new(e) as BoxedStorageError)
    }

    async fn delete_counter(&self, key: &str) -> Result<i64, BoxedStorageError> {
        self.inner
            .delete_counter(key)
            .await
            .map_err(|e| Box::new(e) as BoxedStorageError)
    }
}
