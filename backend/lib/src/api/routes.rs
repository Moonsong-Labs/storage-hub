//! Route definitions for StorageHub API

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post, put},
    Router,
};

use crate::{api::handlers, services::Services};

/// Creates the router with all API routes
pub fn routes(services: Services) -> Router {
    // we use a separate router for the upload path
    // so we can disable the request body limit
    let file_upload = Router::new()
        .route(
            "/buckets/:bucket_id/upload/:file_key",
            put(handlers::upload_file),
        )
        .route_layer(DefaultBodyLimit::disable());

    Router::new()
        // Auth routes
        .route("/auth/nonce", post(handlers::nonce))
        .route("/auth/verify", post(handlers::verify))
        .route("/auth/refresh", post(handlers::refresh))
        .route("/auth/logout", post(handlers::logout))
        .route("/auth/profile", get(handlers::profile))
        // MSP info routes
        .route("/info", get(handlers::info))
        .route("/stats", get(handlers::stats))
        .route("/value-props", get(handlers::value_props))
        .route("/health", get(handlers::msp_health))
        // Bucket routes
        .route("/buckets", get(handlers::list_buckets))
        .route("/buckets/:bucket_id", get(handlers::get_bucket))
        .route("/buckets/:bucket_id/files", get(handlers::get_files))
        // File routes
        .route(
            "/buckets/:bucket_id/info/:file_key",
            get(handlers::get_file_info),
        )
        .merge(file_upload)
        .route(
            "/buckets/:bucket_id/distribute/:file_key",
            post(handlers::distribute_file),
        )
        .route(
            "/buckets/:bucket_id/download/:file_key",
            get(handlers::download_by_key),
        )
        .route(
            "/buckets/:bucket_id/download/path/*file_location",
            get(handlers::download_by_location),
        )
        // Payment streams routes
        .route("/payment_stream", get(handlers::payment_stream))
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
