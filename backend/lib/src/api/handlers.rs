//! Request handlers for StorageHub API endpoints

use axum::{extract::State, Json};

use crate::services::{health, Services};

// TODO(SCAFFOLDING): These are example endpoints for demonstration.
// Replace with actual MSP API endpoints when implementing real features.

/// Health check handler
pub async fn health_check_detailed(
    State(services): State<Services>,
) -> Json<health::DetailedHealthStatus> {
    Json(services.health.check_health().await)
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let services = Services::mocks();
        let response = health_check_detailed(State(services)).await;
        assert_eq!(response.0.status, "healthy");
        assert!(!response.0.version.is_empty());
    }
}
