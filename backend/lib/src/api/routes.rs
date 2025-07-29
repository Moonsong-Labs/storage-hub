//! Route definitions for StorageHub API

use axum::routing::{get, post};
use axum::Router;

use super::handlers;
use crate::services::Services;

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

// WIP: Tests commented out until PostgreSQL mock implementation is complete
#[cfg(test)]
#[allow(dead_code)]
mod tests {
    // WIP: Mock PostgreSQL imports commented out until diesel traits are fully implemented
    // use crate::data::postgres::{AnyDbConnection, MockDbConnection, PostgresClient};
    use std::sync::Arc;

    use axum::http::StatusCode;
    use axum_test::TestServer;

    use super::*;

    fn create_test_app() -> Router {
        // Use consolidated test utilities
        let services = Services::test();
        create_routes(services)
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL mock implementation - currently WIP"]
    async fn test_health_route() {
        let app = create_test_app();
        let server = TestServer::new(app).unwrap();

        let response = server.get("/health").await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let json: serde_json::Value = response.json();
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL mock implementation - currently WIP"]
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
