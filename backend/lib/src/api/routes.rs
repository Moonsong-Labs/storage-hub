//! Route definitions for StorageHub API

use axum::{
    routing::{get, post},
    Router,
};

use crate::services::Services;
use super::handlers;

/// Creates the router with all API routes
pub fn routes(services: Services) -> Router {
    Router::new()
        // Health check endpoint
        .route("/health", get(handlers::health_check))
        
        // Counter endpoints
        .route("/counter", get(handlers::get_counter))
        .route("/counter/inc", post(handlers::increment_counter))
        .route("/counter/dec", post(handlers::decrement_counter))
        
        // Add state to all routes
        .with_state(services)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;
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

    fn create_test_app() -> Router {
        let memory_storage = InMemoryStorage::new();
        let boxed_storage = BoxedStorageWrapper::new(memory_storage);
        let storage: Arc<dyn crate::data::storage::BoxedStorage> = Arc::new(boxed_storage);
        let postgres: Arc<dyn crate::data::postgres::PostgresClientTrait> = Arc::new(TestPostgresClient);
        let services = Services::new(storage, postgres);
        routes(services)
    }

    #[tokio::test]
    async fn test_health_route() {
        let app = create_test_app();
        let server = TestServer::new(app).unwrap();
        
        let response = server.get("/health").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        
        let json: serde_json::Value = response.json();
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn test_counter_routes() {
        let app = create_test_app();
        let server = TestServer::new(app).unwrap();
        
        // Get initial counter
        let response = server.get("/counter").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let json: serde_json::Value = response.json();
        assert_eq!(json["value"], 0);
        
        // Increment counter
        let response = server.post("/counter/inc").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let json: serde_json::Value = response.json();
        assert_eq!(json["value"], 1);
        
        // Decrement counter
        let response = server.post("/counter/dec").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let json: serde_json::Value = response.json();
        assert_eq!(json["value"], 0);
    }
}