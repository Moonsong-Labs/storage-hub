//! Route definitions for StorageHub API

use axum::{
    routing::{get, post},
    Router,
};

use crate::{handlers, services::Services};

/// Creates the router with all API routes
pub fn routes(services: Services) -> Router {
    Router::new()
        // Health check endpoint
        .route("/health", get(handlers::health_check_detailed))
        // TODO(SCAFFOLDING): Remove counter routes when real MSP endpoints are implemented.
        // These are example endpoints for demonstration purposes only.
        // Counter endpoints
        .route("/counter", get(handlers::get_counter))
        .route("/counter/inc", post(handlers::increment_counter))
        .route("/counter/dec", post(handlers::decrement_counter))
        // Add state to all routes
        .with_state(services)
}

// WIP: Tests commented out until PostgreSQL mock implementation is complete
#[cfg(all(test, feature = "mocks"))]
#[allow(dead_code)]
mod tests {
    use axum::http::StatusCode;
    use axum_test::TestServer;

    #[tokio::test]
    #[ignore = "Requires PostgreSQL mock implementation - currently WIP"]
    // TODO
    async fn test_health_route() {
        let app = crate::api::test_app();
        let server = TestServer::new(app).unwrap();

        let response = server.get("/health").await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let json: serde_json::Value = response.json();
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL mock implementation - currently WIP"]
    // TODO
    async fn test_counter_routes() {
        let app = crate::api::test_app();
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
