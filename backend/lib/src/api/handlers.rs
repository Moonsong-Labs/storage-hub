//! Request handlers for StorageHub API endpoints

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    error::Result,
    services::{health, Services},
};

// TODO(SCAFFOLDING): Counter handlers are example endpoints for demonstration.
// Replace with actual MSP API endpoints when implementing real features.

/// Response for counter operations
#[derive(Debug, Serialize, Deserialize)]
pub struct CounterResponse {
    pub value: i64,
}

/// Increment counter handler
pub async fn increment_counter(State(services): State<Services>) -> Result<Json<CounterResponse>> {
    let value = services.counter.increment().await?;
    Ok(Json(CounterResponse { value }))
}

/// Decrement counter handler
pub async fn decrement_counter(State(services): State<Services>) -> Result<Json<CounterResponse>> {
    let value = services.counter.decrement().await?;
    Ok(Json(CounterResponse { value }))
}

/// Get current counter value handler
pub async fn get_counter(State(services): State<Services>) -> Result<Json<CounterResponse>> {
    let value = services.counter.get().await?;
    Ok(Json(CounterResponse { value }))
}

/// Health check handler
pub async fn health_check_detailed(
    State(services): State<Services>,
) -> Json<health::DetailedHealthStatus> {
    Json(services.health.check_health().await)
}

// WIP: Tests commented out until PostgreSQL mock implementation is complete
#[cfg(all(test, feature = "mocks"))]
#[allow(dead_code)]
mod tests {
    use super::*;

    fn create_test_services() -> Services {
        // Use consolidated test utilities
        Services::test()
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL mock implementation - currently WIP"]
    // TODO
    async fn test_health_check() {
        let services = create_test_services();
        let response = health_check_detailed(State(services)).await;
        assert_eq!(response.0.status, "healthy");
        assert!(!response.0.version.is_empty());
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL mock implementation - currently WIP"]
    // TODO
    async fn test_counter_handlers() {
        let services = create_test_services();

        // Test get initial value
        let response = get_counter(State(services.clone())).await.unwrap();
        assert_eq!(response.0.value, 0);

        // Test increment
        let response = increment_counter(State(services.clone())).await.unwrap();
        assert_eq!(response.0.value, 1);

        // Test decrement
        let response = decrement_counter(State(services.clone())).await.unwrap();
        assert_eq!(response.0.value, 0);
    }
}
