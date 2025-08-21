//! Route definitions for StorageHub API

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post, put},
    Router,
};

use super::{handlers, msp_handlers};
use crate::services::Services;

/// Creates the router with all API routes
pub fn routes(services: Services) -> Router {
    // we use a separate router for the upload path
    // so we can disable the request body limit
    let file_upload = Router::new()
        .route(
            "/buckets/:bucket_id/:file_key/upload",
            put(msp_handlers::upload_file),
        )
        .route_layer(DefaultBodyLimit::disable());

    Router::new()
        // TODO(SCAFFOLDING): These are example endpoints for demonstration purposes only.
        .route("/health", get(handlers::health_check_detailed))
        // TODO(SCAFFOLDING): Remove counter routes when real MSP endpoints are implemented.
        // These are example endpoints for demonstration purposes only.
        // Counter endpoints
        .route("/counter", get(handlers::get_counter))
        .route("/counter/inc", post(handlers::increment_counter))
        .route("/counter/dec", post(handlers::decrement_counter))
        // Auth routes
        .route("/auth/nonce", post(msp_handlers::nonce))
        .route("/auth/verify", post(msp_handlers::verify))
        .route("/auth/refresh", post(msp_handlers::refresh))
        .route("/auth/logout", post(msp_handlers::logout))
        .route("/auth/profile", get(msp_handlers::profile))
        // MSP info routes
        .route("/info", get(msp_handlers::info))
        .route("/stats", get(msp_handlers::stats))
        .route("/value-props", get(msp_handlers::value_props))
        .route("/msp/health", get(msp_handlers::msp_health))
        // Bucket routes
        .route("/buckets", get(msp_handlers::list_buckets))
        .route("/buckets/:bucket_id", get(msp_handlers::get_bucket))
        .route("/buckets/:bucket_id/files", get(msp_handlers::get_files))
        // File routes - note the order matters for path matching
        .route(
            "/buckets/:bucket_id/:file_key/info",
            get(msp_handlers::get_file_info),
        )
        .merge(file_upload)
        .route(
            "/buckets/:bucket_id/:file_key/distribute",
            post(msp_handlers::distribute_file),
        )
        .route(
            "/buckets/:bucket_id/:file_key",
            get(msp_handlers::download_by_key),
        )
        .route(
            "/buckets/:bucket_id/*file_location",
            get(msp_handlers::download_by_location),
        )
        // Payment route
        .route("/payment_stream", get(msp_handlers::payment_stream))
        // Add state to all routes
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
