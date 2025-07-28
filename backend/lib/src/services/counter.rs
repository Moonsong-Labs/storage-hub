use crate::data::storage::BoxedStorage;
use crate::error::Result;
use std::sync::Arc;

pub struct CounterService {
    storage: Arc<dyn BoxedStorage>,
}

impl CounterService {
    pub fn new(storage: Arc<dyn BoxedStorage>) -> Self {
        Self { storage }
    }

    pub async fn increment(&self) -> Result<i64> {
        self.storage
            .increment_counter("default", 1)
            .await
            .map_err(|e| crate::error::Error::Storage(e.to_string()))
    }

    pub async fn decrement(&self) -> Result<i64> {
        self.storage
            .decrement_counter("default", 1)
            .await
            .map_err(|e| crate::error::Error::Storage(e.to_string()))
    }

    pub async fn get(&self) -> Result<i64> {
        self.storage
            .get_counter("default")
            .await
            .map_err(|e| crate::error::Error::Storage(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::storage::{BoxedStorageWrapper, InMemoryStorage};

    #[tokio::test]
    async fn test_counter_service() {
        // Create in-memory storage
        let memory_storage = InMemoryStorage::new();
        let boxed_storage = BoxedStorageWrapper::new(memory_storage);
        let storage: Arc<dyn BoxedStorage> = Arc::new(boxed_storage);

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
