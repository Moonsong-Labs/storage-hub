//! API module for StorageHub backend

pub mod handlers;
pub mod msp_handlers;
pub mod routes;
pub mod validation;

use axum::{
    extract::DefaultBodyLimit,
    http::{header::CONTENT_TYPE, Method},
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
        .allow_headers([CONTENT_TYPE])
        .allow_credentials(false);

    router
        .layer(cors)
        .layer(DefaultBodyLimit::max(200 * 1024 * 1024))
}

#[cfg(all(test, feature = "mocks"))]
/// Create a test application
///
/// This function creates a test application with mock services.
pub fn mock_app() -> Router {
    let services = Services::mocks();
    create_app(services)
}
