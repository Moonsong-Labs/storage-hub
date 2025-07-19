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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;
    use data::storage::{BoxedStorageWrapper, InMemoryStorage};
    use services::Services;
    use std::sync::Arc;

    use crate::data::postgres::MockPostgresClient;
    













    /// Creates a test application with in-memory storage
    fn create_test_app() -> axum::Router {
        // Create in-memory storage
        let memory_storage = InMemoryStorage::new();
        let boxed_storage = BoxedStorageWrapper::new(memory_storage);
        let storage: Arc<dyn data::storage::BoxedStorage> = Arc::new(boxed_storage);
        
        // Create test postgres client
        let postgres: Arc<dyn data::postgres::PostgresClientTrait> = Arc::new(MockPostgresClient::new());
        
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
