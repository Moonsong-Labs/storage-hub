//! Request handlers for StorageHub API endpoints

use axum::{
    extract::State,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::Result;
use crate::services::Services;

/// Response for counter operations
#[derive(Debug, Serialize, Deserialize)]
pub struct CounterResponse {
    pub value: i64,
}

/// Increment counter handler
pub async fn increment_counter(
    State(services): State<Services>,
) -> Result<Json<CounterResponse>> {
    let value = services.counter.increment().await?;
    Ok(Json(CounterResponse { value }))
}

/// Decrement counter handler
pub async fn decrement_counter(
    State(services): State<Services>,
) -> Result<Json<CounterResponse>> {
    let value = services.counter.decrement().await?;
    Ok(Json(CounterResponse { value }))
}

/// Get current counter value handler
pub async fn get_counter(
    State(services): State<Services>,
) -> Result<Json<CounterResponse>> {
    let value = services.counter.get().await?;
    Ok(Json(CounterResponse { value }))
}

/// Health check handler
pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "storagehub-backend"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::storage::{BoxedStorageWrapper, InMemoryStorage};
    use std::sync::Arc;

    // Simple test postgres client
    use async_trait::async_trait;
    use crate::data::postgres::{PostgresClientTrait, PaginationParams};
    
    struct TestPostgresClient;
    
    #[async_trait]
    impl PostgresClientTrait for TestPostgresClient {
        async fn test_connection(&self) -> crate::error::Result<()> {
            Ok(())
        }

        async fn get_file_by_key(&self, _file_key: &[u8]) -> crate::error::Result<shc_indexer_db::models::File> {
            Err(crate::error::Error::NotFound("Test file not found".to_string()))
        }

        async fn get_files_by_user(
            &self,
            _user_account: &[u8],
            _pagination: Option<PaginationParams>,
        ) -> crate::error::Result<Vec<shc_indexer_db::models::File>> {
            Ok(vec![])
        }

        async fn get_files_by_user_and_msp(
            &self,
            _user_account: &[u8],
            _msp_id: i64,
            _pagination: Option<PaginationParams>,
        ) -> crate::error::Result<Vec<shc_indexer_db::models::File>> {
            Ok(vec![])
        }

        async fn get_files_by_bucket_id(
            &self,
            _bucket_id: i64,
            _pagination: Option<PaginationParams>,
        ) -> crate::error::Result<Vec<shc_indexer_db::models::File>> {
            Ok(vec![])
        }

        async fn create_file(&self, _file: shc_indexer_db::models::File) -> crate::error::Result<shc_indexer_db::models::File> {
            Err(crate::error::Error::Database("Cannot create files in test".to_string()))
        }

        async fn update_file_step(
            &self,
            _file_key: &[u8],
            _step: shc_indexer_db::models::FileStorageRequestStep,
        ) -> crate::error::Result<()> {
            Err(crate::error::Error::Database("Cannot update files in test".to_string()))
        }

        async fn delete_file(&self, _file_key: &[u8]) -> crate::error::Result<()> {
            Err(crate::error::Error::Database("Cannot delete files in test".to_string()))
        }

        async fn get_bucket_by_id(&self, _bucket_id: i64) -> crate::error::Result<shc_indexer_db::models::Bucket> {
            Err(crate::error::Error::NotFound("Test bucket not found".to_string()))
        }

        async fn get_buckets_by_user(
            &self,
            _user_account: &[u8],
            _pagination: Option<PaginationParams>,
        ) -> crate::error::Result<Vec<shc_indexer_db::models::Bucket>> {
            Ok(vec![])
        }

        async fn get_msp_by_id(&self, _msp_id: i64) -> crate::error::Result<shc_indexer_db::models::Msp> {
            Err(crate::error::Error::NotFound("Test MSP not found".to_string()))
        }

        async fn get_all_msps(&self, _pagination: Option<PaginationParams>) -> crate::error::Result<Vec<shc_indexer_db::models::Msp>> {
            Ok(vec![])
        }

        async fn execute_raw_query(&self, _query: &str) -> crate::error::Result<Vec<serde_json::Value>> {
            Ok(vec![])
        }
    }

    fn create_test_services() -> Services {
        let memory_storage = InMemoryStorage::new();
        let boxed_storage = BoxedStorageWrapper::new(memory_storage);
        let storage: Arc<dyn crate::data::storage::BoxedStorage> = Arc::new(boxed_storage);
        let postgres: Arc<dyn crate::data::postgres::PostgresClientTrait> = Arc::new(TestPostgresClient);
        Services::new(storage, postgres)
    }

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await;
        assert_eq!(response.0["status"], "ok");
        assert_eq!(response.0["service"], "storagehub-backend");
    }

    #[tokio::test]
    async fn test_counter_handlers() {
        let services = create_test_services();
        
        // Test get initial value
        let response = get_counter(State(services.clone())).await.unwrap();
        assert_eq!(response.0.value, 0);
        
        // Test increment
        let response = increment_counter(State(services.clone())).await.unwrap();
        assert_eq!(response.0.value, 1);
        
        // Test decrement
        let response = decrement_counter(State(services.clone())).await.unwrap();
        assert_eq!(response.0.value, 0);
    }
}