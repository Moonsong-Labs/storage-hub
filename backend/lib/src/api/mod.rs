//! API module for StorageHub backend

pub mod handlers;
pub mod routes;
pub mod validation;

use axum::Router;
use tower_http::cors::CorsLayer;

use crate::services::Services;

/// Creates the axum application with all routes and middleware
pub fn create_app(services: Services) -> Router {
    let router = routes::routes(services);

    // Add CORS layer for permissive access
    let cors = CorsLayer::permissive();

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
