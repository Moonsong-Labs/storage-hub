use std::sync::Arc;

use crate::constants::counter::{DEFAULT_INCREMENT, DEFAULT_COUNTER_KEY};
use crate::data::storage::BoxedStorage;
use crate::error::Result;

pub struct CounterService {
    storage: Arc<dyn BoxedStorage>,
}

impl CounterService {
    pub fn new(storage: Arc<dyn BoxedStorage>) -> Self {
        Self { storage }
    }

    pub async fn increment(&self) -> Result<i64> {
        self.storage
            .increment_counter(DEFAULT_COUNTER_KEY, DEFAULT_INCREMENT)
            .await
            .map_err(|e| crate::error::Error::Storage(e))
    }

    pub async fn decrement(&self) -> Result<i64> {
        self.storage
            .decrement_counter(DEFAULT_COUNTER_KEY, DEFAULT_INCREMENT)
            .await
            .map_err(|e| crate::error::Error::Storage(e))
    }

    pub async fn get(&self) -> Result<i64> {
        self.storage
            .get_counter(DEFAULT_COUNTER_KEY)
            .await
            .map_err(|e| crate::error::Error::Storage(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_counter_service() {
        // Create test storage
        let storage = crate::data::storage::test_storage();

        // Create counter service
        let counter_service = CounterService::new(storage);

        let result = counter_service.increment().await.unwrap();
        assert_eq!(result, 1);

        let result = counter_service.get().await.unwrap();
        assert_eq!(result, 1);

        let result = counter_service.decrement().await.unwrap();
        assert_eq!(result, 0);
    }
}
