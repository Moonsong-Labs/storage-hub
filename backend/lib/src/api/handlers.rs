//! Request handlers for StorageHub API endpoints

use axum::{
    extract::State,
    Json,
};
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
/// 
/// POST /counter/inc
pub async fn increment_counter(
    State(services): State<Services>,
) -> Result<Json<CounterResponse>> {
    let value = services.counter.increment().await?;
    Ok(Json(CounterResponse { value }))
}

/// Decrement counter handler
/// 
/// POST /counter/dec
pub async fn decrement_counter(
    State(services): State<Services>,
) -> Result<Json<CounterResponse>> {
    let value = services.counter.decrement().await?;
    Ok(Json(CounterResponse { value }))
}

/// Get current counter value handler
/// 
/// GET /counter
pub async fn get_counter(
    State(services): State<Services>,
) -> Result<Json<CounterResponse>> {
    let value = services.counter.get().await?;
    Ok(Json(CounterResponse { value }))
}

/// Health check handler
/// 
/// GET /health
pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "storagehub-backend"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::storage::{BoxedStorageWrapper, InMemoryStorage};
    use crate::data::postgres::{AnyDbConnection, MockDbConnection, PostgresClient};
    use crate::data::rpc::{AnyRpcConnection, MockConnection, StorageHubRpcClient};
    use std::sync::Arc;

    fn create_test_services() -> Services {
        let memory_storage = InMemoryStorage::new();
        let boxed_storage = BoxedStorageWrapper::new(memory_storage);
        let storage: Arc<dyn crate::data::storage::BoxedStorage> = Arc::new(boxed_storage);
        let mock_conn = Arc::new(AnyDbConnection::Mock(MockDbConnection::new()));
        let postgres: Arc<dyn crate::data::postgres::PostgresClientTrait> = Arc::new(PostgresClient::new(mock_conn));
        
        // Create mock RPC client
        let mock_rpc_conn = Arc::new(AnyRpcConnection::Mock(MockConnection::new()));
        let rpc: Arc<dyn crate::data::rpc::StorageHubRpcTrait> = Arc::new(StorageHubRpcClient::new(mock_rpc_conn));
        
        Services::new(storage, postgres, rpc)
    }

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await;
        assert_eq!(response.0["status"], "ok");
        assert_eq!(response.0["service"], "storagehub-backend");
    }

    #[tokio::test]
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