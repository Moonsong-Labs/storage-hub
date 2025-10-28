//! API module for StorageHub backend

pub mod handlers;
pub mod routes;
pub mod validation;

use axum::Router;
use tower_http::cors::CorsLayer;

use crate::{log, services::Services};

/// Creates the axum application with all routes and middleware
pub fn create_app(services: Services) -> Router {
    let router = routes::routes(services);

    // Add CORS layer for permissive access
    let cors = CorsLayer::permissive();

    // Add tracing layer to attach endpoint information to all logs within a request
    let trace_layer = log::create_http_trace_layer();

    router.layer(cors).layer(trace_layer)
}

#[cfg(all(test, feature = "mocks"))]
/// Create a test application
///
/// This function creates a test application with mock services.
pub async fn mock_app() -> Router {
    let services = Services::mocks().await;
    create_app(services)
}
