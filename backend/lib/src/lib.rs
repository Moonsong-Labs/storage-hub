//! StorageHub Backend Library

pub mod api;
pub mod config;
pub mod constants;
pub mod data;
pub mod error;
pub mod services;

pub use api::create_app;
pub use config::Config;
pub use error::{Error, Result};

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use axum::http::StatusCode;
    use axum_test::TestServer;

    use super::*;

    /// Creates a test application with mocked services
    ///
    /// This function serves as utility for other tests
    #[cfg(feature = "mocks")]
    fn create_test_app() -> axum::Router {
        // Create test services with everything mocked
        let services = services::Services::mocks();

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
}
