//! Route definitions for StorageHub API

use axum::{routing::get, Router};

use crate::{api::handlers, services::Services};

/// Creates the router with all API routes
pub fn routes(services: Services) -> Router {
    Router::new()
        // TODO(SCAFFOLDING): These are example endpoints for demonstration purposes only.
        .route("/health", get(handlers::health_check_detailed))
        .with_state(services)
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use axum::http::StatusCode;
    use axum_test::TestServer;

    use crate::services::health::HealthService;

    #[tokio::test]
    async fn test_health_route() {
        let app = crate::api::mock_app();
        let server = TestServer::new(app).unwrap();

        let response = server.get("/health").await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let json: serde_json::Value = response.json();
        assert_eq!(json["status"], HealthService::HEALTHY);
    }
}
