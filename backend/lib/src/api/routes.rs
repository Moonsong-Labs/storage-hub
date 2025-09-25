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
        .route("/payment_stream", get(handlers::payment_stream))
        // Add state to all routes
        .with_state(services)
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use crate::{constants::mocks::DOWNLOAD_FILE_CONTENT, services::health::HealthService};

    use std::path::Path;

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

    #[tokio::test]
    async fn test_download_by_key_streams_and_cleans_temp() {
        let app = crate::api::mock_app().await;
        let server = TestServer::new(app).unwrap();

        let file_key = "0xde4a17999bc1482ba71737367e5d858a133ed1e13327a29c495ab976004a138f";
        let temp_path = format!("/tmp/uploads/{}", file_key);

        let response = server.get(&format!("/download/{}", file_key)).await;

        assert_eq!(response.status_code(), StatusCode::OK);

        // Assert: body bytes match the mocked content written by RPC mock
        let body = response.as_bytes();
        assert_eq!(body.as_ref(), DOWNLOAD_FILE_CONTENT.as_bytes());

        // Assert: temp file was removed
        assert!(!Path::new(&temp_path).exists());
    }
}
