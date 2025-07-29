//! API module for StorageHub backend

pub mod handlers;
pub mod routes;

use axum::http::header::CONTENT_TYPE;
use axum::http::Method;
use axum::Router;
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

    router.layer(cors)
}

#[cfg(test)]
/// Create a test application
pub fn test_app() -> Router {
    let services = Services::test();
    create_app(services)
}
