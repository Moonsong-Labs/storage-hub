//! Route definitions for StorageHub API

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post, put},
    Router,
};

use crate::{api::handlers, constants::auth::AUTH_NONCE_ENDPOINT, services::Services};

/// Creates the router with all API routes
pub fn routes(services: Services) -> Router {
    // we use a separate router for the upload path
    // so we can disable the request body limit
    let file_upload = Router::new()
        .route(
            "/buckets/{bucket_id}/upload/{file_key}",
            put(handlers::files::upload_file),
        )
        .route_layer(DefaultBodyLimit::disable());

    let internal_file_upload = Router::new()
        .route(
            "/internal/uploads/{file_key}",
            put(handlers::files::internal_upload_by_key),
        )
        .route_layer(DefaultBodyLimit::disable());

    Router::new()
        // Auth routes
        .route(AUTH_NONCE_ENDPOINT, post(handlers::auth::nonce))
        .route("/auth/verify", post(handlers::auth::verify))
        .route("/auth/refresh", post(handlers::auth::refresh))
        .route("/auth/logout", post(handlers::auth::logout))
        .route("/auth/profile", get(handlers::auth::profile))
        // MSP info routes
        .route("/info", get(handlers::info))
        .route("/stats", get(handlers::stats))
        .route("/value-props", get(handlers::value_props))
        .route("/health", get(handlers::msp_health))
        // Bucket routes
        .route("/buckets", get(handlers::buckets::list_buckets))
        .route("/buckets/{bucket_id}", get(handlers::buckets::get_bucket))
        .route(
            "/buckets/{bucket_id}/files",
            get(handlers::buckets::get_files),
        )
        // File routes
        .route(
            "/buckets/{bucket_id}/info/{file_key}",
            get(handlers::files::get_file_info),
        )
        .merge(file_upload)
        .merge(internal_file_upload)
        .route(
            "/buckets/{bucket_id}/distribute/{file_key}",
            post(handlers::files::distribute_file),
        )
        .route(
            "/download/{file_key}",
            get(handlers::files::download_by_key),
        )
        // Payment streams routes
        .route("/payment_streams", get(handlers::payment_streams))
        // Add state to all routes
        .with_state(services)
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use crate::services::health::HealthService;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_health_route() {
        let app = crate::api::mock_app().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/health").await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let json: serde_json::Value = response.json();
        assert_eq!(json["status"], HealthService::HEALTHY);
    }
}
