//! API module for StorageHub backend

pub mod handlers;
pub mod routes;
pub mod validation;

use axum::{
    http::{
        header::{ACCEPT, AUTHORIZATION, CONTENT_RANGE, CONTENT_TYPE},
        Method,
    },
    Router,
};
use tower_http::cors::CorsLayer;

use crate::services::Services;

/// Creates the axum application with all routes and middleware
pub fn create_app(services: Services) -> Router {
    let router = routes::routes(services);

    // Add CORS layer for permissive access
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE, ACCEPT, CONTENT_RANGE])
        .allow_credentials(false);

    router.layer(cors)
}

#[cfg(all(test, feature = "mocks"))]
/// Create a test application
///
/// This function creates a test application with mock services.
pub async fn mock_app() -> Router {
    let services = Services::mocks().await;
    create_app(services)
}
