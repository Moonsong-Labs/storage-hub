//! StorageHub Backend Library

pub mod api;
pub mod config;
pub mod constants;
pub mod data;
pub mod error;
pub mod models;
pub mod services;

pub use api::create_app;
pub use config::Config;
pub use error::{Error, Result};

#[cfg(all(test, feature = "mocks"))]
#[allow(dead_code)]
mod tests {
    use axum::http::StatusCode;
    use axum_test::TestServer;

    use super::*;

    /// Creates a test application with mocked services
    ///
    /// This function serves as utility for other tests
    #[cfg(feature = "mocks")]
    fn create_test_app() -> axum::Router {
        // Create test services with everything mocked
        let services = services::Services::mocks();

        api::create_app(services)
    }
}
