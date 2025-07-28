//! # StorageHub Backend Library
//!
//! Core library for the StorageHub backend service.

pub mod api;
pub mod config;
pub mod data;
pub mod error;
pub mod services;

pub use api::create_app;
pub use config::Config;
pub use error::{Error, Result};

// WIP: Tests commented out until PostgreSQL mock implementation is complete
#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;
    use data::storage::{BoxedStorageWrapper, InMemoryStorage};
    use std::sync::Arc;

    // WIP: Mock PostgreSQL imports commented out until diesel traits are fully implemented
    // use crate::data::postgres::{AnyDbConnection, MockDbConnection, PostgresClient};

    /// Creates a test application with in-memory storage
    fn create_test_app() -> axum::Router {
        // Create in-memory storage
        let memory_storage = InMemoryStorage::new();
        let boxed_storage = BoxedStorageWrapper::new(memory_storage);
        let _storage: Arc<dyn data::storage::BoxedStorage> = Arc::new(boxed_storage);

        // WIP: Mock PostgreSQL connection commented out until diesel traits are fully implemented
        // let mock_conn = MockDbConnection::new();
        // let db_conn = Arc::new(AnyDbConnection::Mock(mock_conn));
        // let postgres: Arc<dyn data::postgres::PostgresClientTrait> = Arc::new(PostgresClient::new(db_conn));

        // For now, we'll panic in tests that need postgres
        panic!("Test requires PostgreSQL mock implementation - currently WIP")
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL mock implementation - currently WIP"]
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
    #[ignore = "Requires PostgreSQL mock implementation - currently WIP"]
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
