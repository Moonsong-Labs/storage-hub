//! Request handlers for StorageHub API endpoints

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::Result;
use crate::services::Services;

/// Response for counter operations
#[derive(Debug, Serialize, Deserialize)]
pub struct CounterResponse {
    pub value: i64,
}

/// Increment counter handler
pub async fn increment_counter(
    State(services): State<Services>,
) -> Result<Json<CounterResponse>> {
    let value = services.counter.increment().await?;
    Ok(Json(CounterResponse { value }))
}

/// Decrement counter handler
pub async fn decrement_counter(
    State(services): State<Services>,
) -> Result<Json<CounterResponse>> {
    let value = services.counter.decrement().await?;
    Ok(Json(CounterResponse { value }))
}

/// Get current counter value handler
pub async fn get_counter(
    State(services): State<Services>,
) -> Result<Json<CounterResponse>> {
    let value = services.counter.get().await?;
    Ok(Json(CounterResponse { value }))
}

/// Health check handler
pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "storagehub-backend"
    }))
}

// WIP: Tests commented out until PostgreSQL mock implementation is complete
#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use super::*;
    use crate::data::storage::{BoxedStorageWrapper, InMemoryStorage};
    // WIP: Mock PostgreSQL imports commented out until diesel traits are fully implemented
    // use crate::data::postgres::{AnyDbConnection, MockDbConnection, PostgresClient};
    use std::sync::Arc;

    fn create_test_services() -> Services {
        let memory_storage = InMemoryStorage::new();
        let boxed_storage = BoxedStorageWrapper::new(memory_storage);
        let _storage: Arc<dyn crate::data::storage::BoxedStorage> = Arc::new(boxed_storage);
        // WIP: Mock PostgreSQL connection commented out until diesel traits are fully implemented
        // let mock_conn = Arc::new(AnyDbConnection::Mock(MockDbConnection::new()));
        // let postgres: Arc<dyn crate::data::postgres::PostgresClientTrait> = Arc::new(PostgresClient::new(mock_conn));

        // For now, we'll panic in tests that need postgres
        panic!("Test requires PostgreSQL mock implementation - currently WIP")
    }

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await;
        assert_eq!(response.0["status"], "ok");
        assert_eq!(response.0["service"], "storagehub-backend");
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL mock implementation - currently WIP"]
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
