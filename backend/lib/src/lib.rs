//! # StorageHub Backend Library
//!
//! Core library for the StorageHub backend service.

pub mod api;
pub mod config;
pub mod data;
pub mod error;
pub mod services;

#[cfg(feature = "mocks")]
pub mod mocks;

pub use api::create_app;
pub use config::Config;
pub use error::{Error, Result};

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;
    use data::storage::{BoxedStorageWrapper, InMemoryStorage};
    use services::Services;
    use std::sync::Arc;

    // Simple mock PostgreSQL client for testing
    use async_trait::async_trait;
    use data::postgres::{PostgresClientTrait, PaginationParams};
    
    struct TestPostgresClient;
    
    #[async_trait]
    impl PostgresClientTrait for TestPostgresClient {
        async fn test_connection(&self) -> Result<()> {
            Ok(())
        }

        async fn get_file_by_key(&self, _file_key: &[u8]) -> Result<shc_indexer_db::models::File> {
            Err(Error::NotFound("Test file not found".to_string()))
        }

        async fn get_files_by_user(
            &self,
            _user_account: &[u8],
            _pagination: Option<PaginationParams>,
        ) -> Result<Vec<shc_indexer_db::models::File>> {
            Ok(vec![])
        }

        async fn get_files_by_user_and_msp(
            &self,
            _user_account: &[u8],
            _msp_id: i64,
            _pagination: Option<PaginationParams>,
        ) -> Result<Vec<shc_indexer_db::models::File>> {
            Ok(vec![])
        }

        async fn get_files_by_bucket_id(
            &self,
            _bucket_id: i64,
            _pagination: Option<PaginationParams>,
        ) -> Result<Vec<shc_indexer_db::models::File>> {
            Ok(vec![])
        }

        async fn create_file(&self, _file: shc_indexer_db::models::File) -> Result<shc_indexer_db::models::File> {
            Err(Error::Database("Cannot create files in test".to_string()))
        }

        async fn update_file_step(
            &self,
            _file_key: &[u8],
            _step: shc_indexer_db::models::FileStorageRequestStep,
        ) -> Result<()> {
            Err(Error::Database("Cannot update files in test".to_string()))
        }

        async fn delete_file(&self, _file_key: &[u8]) -> Result<()> {
            Err(Error::Database("Cannot delete files in test".to_string()))
        }

        async fn get_bucket_by_id(&self, _bucket_id: i64) -> Result<shc_indexer_db::models::Bucket> {
            Err(Error::NotFound("Test bucket not found".to_string()))
        }

        async fn get_buckets_by_user(
            &self,
            _user_account: &[u8],
            _pagination: Option<PaginationParams>,
        ) -> Result<Vec<shc_indexer_db::models::Bucket>> {
            Ok(vec![])
        }

        async fn get_msp_by_id(&self, _msp_id: i64) -> Result<shc_indexer_db::models::Msp> {
            Err(Error::NotFound("Test MSP not found".to_string()))
        }

        async fn get_all_msps(&self, _pagination: Option<PaginationParams>) -> Result<Vec<shc_indexer_db::models::Msp>> {
            Ok(vec![])
        }

        async fn execute_raw_query(&self, _query: &str) -> Result<Vec<serde_json::Value>> {
            Ok(vec![])
        }
    }

    /// Creates a test application with in-memory storage
    fn create_test_app() -> axum::Router {
        // Create in-memory storage
        let memory_storage = InMemoryStorage::new();
        let boxed_storage = BoxedStorageWrapper::new(memory_storage);
        let storage: Arc<dyn data::storage::BoxedStorage> = Arc::new(boxed_storage);
        
        // Create test postgres client
        let postgres: Arc<dyn data::postgres::PostgresClientTrait> = Arc::new(TestPostgresClient);
        
        // Create services and app
        let services = Services::new(storage, postgres);
        create_app(services)
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        // Create test server
        let app = create_test_app();
        let server = TestServer::new(app).unwrap();
        
        // Test health endpoint
        let response = server.get("/health").await;
        
        // Assert status code
        assert_eq!(response.status_code(), StatusCode::OK);
        
        // Assert response body
        let json: serde_json::Value = response.json();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["service"], "storagehub-backend");
    }

    #[tokio::test]
    async fn test_counter_endpoints() {
        // Create test server
        let app = create_test_app();
        let server = TestServer::new(app).unwrap();
        
        // Test GET /counter - should return 0 initially
        let response = server.get("/counter").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let json: serde_json::Value = response.json();
        assert_eq!(json["value"], 0);
        
        // Test POST /counter/inc - should increment to 1
        let response = server.post("/counter/inc").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let json: serde_json::Value = response.json();
        assert_eq!(json["value"], 1);
        
        // Test GET /counter again - should return 1
        let response = server.get("/counter").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let json: serde_json::Value = response.json();
        assert_eq!(json["value"], 1);
        
        // Test POST /counter/inc again - should increment to 2
        let response = server.post("/counter/inc").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let json: serde_json::Value = response.json();
        assert_eq!(json["value"], 2);
        
        // Test multiple increments persist correctly
        let response = server.get("/counter").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let json: serde_json::Value = response.json();
        assert_eq!(json["value"], 2);
    }
}
