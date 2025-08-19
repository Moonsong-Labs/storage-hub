//! StorageHub Backend Library

pub mod api;
pub mod config;
pub mod constants;
pub mod data;
pub mod error;
pub mod repository;
pub mod services;

pub use api::create_app;
pub use config::Config;
pub use error::{Error, Result};

// WIP: Tests commented out until PostgreSQL mock implementation is complete
#[cfg(all(test, feature = "mocks"))]
#[allow(dead_code)]
mod tests {

    use axum::http::StatusCode;
    use axum_test::TestServer;

    use super::*;

    /// Creates a test application with mocked services
    #[cfg(feature = "mocks")]
    fn create_test_app() -> axum::Router {
        // Create test services with all mocks
        let services = services::Services::test();

        // Create the app with test services
        api::create_app(services)
    }

    #[cfg(feature = "mocks")]
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
        assert_eq!(json["status"], "healthy");
        assert_eq!(json["service"], "storagehub-backend");
    }

    #[cfg(feature = "mocks")]
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
