//! Mock RPC connection implementation for testing
//!
//! This module provides a mock RPC connection that can be configured
//! with predefined responses and error scenarios for testing purposes.

use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use codec::Encode;
use jsonrpsee::core::traits::ToRpcParams;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::{
    sync::{Mutex, RwLock},
    time::sleep,
};

use shc_rpc::{GetValuePropositionsResult, RpcProviderId};
use sp_core::H256;

use crate::{
    constants::{mocks::MOCK_PRICE_PER_GIGA_UNIT, rpc::DUMMY_MSP_ID, rpc::TIMEOUT_MULTIPLIER},
    data::rpc::{
        connection::error::{RpcConnectionError, RpcResult},
        methods, RpcConnection,
    },
    models::msp_info::{ValueProposition, ValuePropositionWithId},
    test_utils::random_bytes_32,
};

/// Error simulation modes for testing
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorMode {
    /// No errors - all calls succeed
    None,
    /// Simulate connection timeout
    Timeout,
    /// Simulate connection closed
    ConnectionClosed,
    /// Simulate transport error
    TransportError(String),
    /// Simulate RPC error
    RpcError(String),
    /// Fail after N successful calls
    FailAfterNCalls(usize),
}

/// Mock RPC connection for testing
pub struct MockConnection {
    /// Predefined responses for specific methods
    responses: Arc<RwLock<HashMap<String, Value>>>,
    /// Error simulation mode
    error_mode: Arc<RwLock<ErrorMode>>,
    /// Whether the connection is "connected"
    connected: Arc<RwLock<bool>>,
    /// Call counter for FailAfterNCalls mode
    call_count: Arc<Mutex<usize>>,
    /// Optional delay to simulate network latency
    latency_ms: Arc<RwLock<Option<u64>>>,
}

impl MockConnection {
    /// Create a new mock connection
    ///
    /// The mock connection will have no faults configured
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a custom response for a specific method
    pub async fn set_response(&self, method: &str, response: Value) {
        let mut responses = self.responses.write().await;
        responses.insert(method.to_string(), response);
    }

    /// Set the error simulation mode
    pub async fn set_error_mode(&self, mode: ErrorMode) {
        let mut error_mode = self.error_mode.write().await;
        *error_mode = mode;

        // Reset call count when changing error mode
        let mut call_count = self.call_count.lock().await;
        *call_count = 0;
    }

    /// Set network latency simulation
    pub async fn set_latency_ms(&self, latency: u64) {
        let mut latency_guard = self.latency_ms.write().await;
        *latency_guard = Some(latency);
    }

    /// Simulate disconnection
    pub async fn disconnect(&self) {
        let mut connected = self.connected.write().await;
        *connected = false;
    }

    /// Simulate reconnection
    pub async fn reconnect(&self) {
        let mut connected = self.connected.write().await;
        *connected = true;
    }

    /// Check if we should simulate an error based on current mode
    async fn check_error(&self) -> RpcResult<()> {
        let current_count = {
            let mut call_count = self.call_count.lock().await;
            *call_count += 1;
            *call_count
        };

        let error_mode = self.error_mode.read().await.clone();

        match error_mode {
            ErrorMode::None => Ok(()),
            ErrorMode::Timeout => {
                // Simulate timeout by sleeping then returning error
                let latency = *self.latency_ms.read().await;
                if let Some(latency_ms) = latency {
                    sleep(Duration::from_millis(latency_ms * TIMEOUT_MULTIPLIER)).await;
                }
                Err(RpcConnectionError::Timeout)
            }
            ErrorMode::ConnectionClosed => Err(RpcConnectionError::ConnectionClosed),
            ErrorMode::TransportError(msg) => Err(RpcConnectionError::Transport(msg)),
            ErrorMode::RpcError(msg) => Err(RpcConnectionError::Rpc(msg)),
            ErrorMode::FailAfterNCalls(n) => {
                if current_count > n {
                    Err(RpcConnectionError::Rpc(
                        "Simulated failure after N calls".to_string(),
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl Default for MockConnection {
    fn default() -> Self {
        Self {
            responses: Arc::new(RwLock::new(HashMap::new())),
            error_mode: Arc::new(RwLock::new(ErrorMode::None)),
            connected: Arc::new(RwLock::new(true)),
            call_count: Arc::new(Mutex::new(0)),
            latency_ms: Arc::new(RwLock::new(None)),
        }
    }
}

#[async_trait]
impl RpcConnection for MockConnection {
    async fn call<P, R>(&self, method: &str, _params: P) -> RpcResult<R>
    where
        P: ToRpcParams + Send,
        R: DeserializeOwned,
    {
        // Global checks
        {
            let connected = self.connected.read().await;
            if !*connected {
                return Err(RpcConnectionError::ConnectionClosed);
            }
        }

        let latency = *self.latency_ms.read().await;
        if let Some(latency_ms) = latency {
            sleep(Duration::from_millis(latency_ms)).await;
        }

        self.check_error().await?;

        // Build JSON response by method
        let response: Value = match method {
            methods::FILE_KEY_EXPECTED => serde_json::json!(true),
            methods::SAVE_FILE_TO_DISK => {
                let location: Vec<u8> = b"my_file.jpg".to_vec();
                let fingerprint: Vec<u8> = random_bytes_32().to_vec();
                serde_json::json!({
                    "Success": {
                        "owner": [1],
                        "bucket_id": [1],
                        "location": location,
                        "file_size": 1u64,
                        "fingerprint": fingerprint
                    }
                })
            }
            methods::PROVIDER_ID => serde_json::json!(RpcProviderId::Msp(
                shp_types::Hash::from_slice(DUMMY_MSP_ID.as_slice())
            )),
            methods::VALUE_PROPS => {
                serde_json::json!(GetValuePropositionsResult::Success(vec![
                    {
                        let mut value_prop_with_id = ValuePropositionWithId::default();
                        value_prop_with_id.id = H256::from_slice(&random_bytes_32());
                        value_prop_with_id.value_prop = ValueProposition::default();
                        value_prop_with_id
                            .value_prop
                            .price_per_giga_unit_of_data_per_block = 100;
                        value_prop_with_id.value_prop.bucket_data_limit = 100;
                        value_prop_with_id.value_prop.available = true;
                        value_prop_with_id.encode()
                    },
                    {
                        let mut value_prop_with_id = ValuePropositionWithId::default();
                        value_prop_with_id.id = H256::from_slice(&random_bytes_32());
                        value_prop_with_id.value_prop = ValueProposition::default();
                        value_prop_with_id
                            .value_prop
                            .price_per_giga_unit_of_data_per_block = 200;
                        value_prop_with_id.value_prop.bucket_data_limit = 300;
                        value_prop_with_id.value_prop.available = false;
                        value_prop_with_id.encode()
                    }
                ]))
            },
            methods::PEER_IDS => serde_json::json!(vec![
                "/ip4/192.168.0.10/tcp/30333/p2p/12D3KooWSUvz8QM5X4tfAaSLErAZjR2puojo16pULBHyqTMGKtNV"
            ]),
            methods::CURRENT_PRICE => {
                // Return a mock price value (e.g., 100 units)
                serde_json::json!(MOCK_PRICE_PER_GIGA_UNIT)
            },
            methods::RECEIVE_FILE_CHUNKS => {
                serde_json::json!([])
            }
            _ => {
                let responses = self.responses.read().await;
                responses
                    .get(method)
                    .cloned()
                    .unwrap_or(serde_json::json!(null))
            }
        };

        // Deserialize to expected type
        serde_json::from_value(response).map_err(|e| {
            RpcConnectionError::Serialization(format!("Failed to deserialize response: {}", e))
        })
    }

    async fn is_connected(&self) -> bool {
        let connected = self.connected.read().await;
        *connected
    }

    async fn close(&self) -> RpcResult<()> {
        self.disconnect().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::test::mock_rpc::*;

    #[tokio::test]
    async fn test_connection_basic() {
        let conn = MockConnection::new();

        // Set up test response
        conn.set_response(
            SAMPLE_METHOD,
            serde_json::json!({
                SAMPLE_FIELD: SAMPLE_VALUE
            }),
        )
        .await;

        // Test system health call
        let health: Value = conn
            .call(SAMPLE_METHOD, jsonrpsee::rpc_params![])
            .await
            .unwrap();
        assert_eq!(health[SAMPLE_FIELD], SAMPLE_VALUE);

        // Test connection status
        assert!(conn.is_connected().await);

        // Test close
        conn.close().await.unwrap();
        assert!(!conn.is_connected().await);
    }

    #[tokio::test]
    async fn test_connection_custom_response() {
        let conn = MockConnection::new();
        conn.set_response(
            SAMPLE_METHOD,
            serde_json::json!({
                SAMPLE_FIELD: SAMPLE_VALUE
            }),
        )
        .await;

        let response: Value = conn
            .call(SAMPLE_METHOD, jsonrpsee::rpc_params![])
            .await
            .unwrap();
        assert_eq!(response[SAMPLE_FIELD], SAMPLE_VALUE);
    }

    #[tokio::test]
    async fn test_error_mode_timeout() {
        let conn = MockConnection::new();
        conn.set_error_mode(ErrorMode::Timeout).await;

        let result: Result<Value, _> = conn.call(SAMPLE_METHOD, jsonrpsee::rpc_params![]).await;
        assert!(matches!(result, Err(RpcConnectionError::Timeout)));
    }

    #[tokio::test]
    async fn test_error_mode_connection_closed() {
        let conn = MockConnection::new();
        conn.set_error_mode(ErrorMode::ConnectionClosed).await;

        let result: Result<Value, _> = conn.call(SAMPLE_METHOD, jsonrpsee::rpc_params![]).await;
        assert!(matches!(result, Err(RpcConnectionError::ConnectionClosed)));
    }

    #[tokio::test]
    async fn test_error_mode_transport_error() {
        let conn = MockConnection::new();
        conn.set_error_mode(ErrorMode::TransportError(
            TEST_TRANSPORT_ERROR_MSG.to_string(),
        ))
        .await;

        let result: Result<Value, _> = conn.call(SAMPLE_METHOD, jsonrpsee::rpc_params![]).await;
        match result {
            Err(RpcConnectionError::Transport(msg)) => {
                assert_eq!(msg, TEST_TRANSPORT_ERROR_MSG);
            }
            _ => panic!("Expected transport error"),
        }
    }

    #[tokio::test]
    async fn test_error_mode_rpc_error() {
        let conn = MockConnection::new();
        conn.set_error_mode(ErrorMode::RpcError(TEST_RPC_ERROR_MSG.to_string()))
            .await;

        let result: Result<Value, _> = conn.call(SAMPLE_METHOD, jsonrpsee::rpc_params![]).await;
        match result {
            Err(RpcConnectionError::Rpc(msg)) => {
                assert_eq!(msg, TEST_RPC_ERROR_MSG);
            }
            _ => panic!("Expected RPC error"),
        }
    }

    #[tokio::test]
    async fn test_error_mode_fail_after_n_calls() {
        let conn = MockConnection::new();
        conn.set_response(
            SAMPLE_METHOD,
            serde_json::json!({
                SAMPLE_FIELD: SAMPLE_VALUE
            }),
        )
        .await;
        conn.set_error_mode(ErrorMode::FailAfterNCalls(FAIL_AFTER_N_CALLS_THRESHOLD))
            .await;

        // First N calls should succeed
        for _ in 0..FAIL_AFTER_N_CALLS_THRESHOLD {
            let result: Value = conn
                .call(SAMPLE_METHOD, jsonrpsee::rpc_params![])
                .await
                .unwrap();
            assert_eq!(result[SAMPLE_FIELD], SAMPLE_VALUE);
        }

        // Next call should fail
        let result: Result<Value, _> = conn.call(SAMPLE_METHOD, jsonrpsee::rpc_params![]).await;
        match result {
            Err(RpcConnectionError::Rpc(msg)) => {
                assert!(msg.contains(ERROR_MESSAGE_FAIL_AFTER_N));
            }
            _ => panic!("Expected RPC error after N calls"),
        }
    }

    #[tokio::test]
    async fn test_connection_disconnect_reconnect() {
        let conn = MockConnection::new();

        // Initially connected
        assert!(conn.is_connected().await);

        // Disconnect
        conn.disconnect().await;
        assert!(!conn.is_connected().await);

        // Try to call - should fail
        let result: Result<Value, _> = conn.call(SAMPLE_METHOD, jsonrpsee::rpc_params![]).await;
        assert!(matches!(result, Err(RpcConnectionError::ConnectionClosed)));

        // Reconnect
        conn.reconnect().await;
        assert!(conn.is_connected().await);

        // Call should work now
        conn.set_response(
            SAMPLE_METHOD,
            serde_json::json!({
                SAMPLE_FIELD: SAMPLE_VALUE
            }),
        )
        .await;
        let response: Value = conn
            .call(SAMPLE_METHOD, jsonrpsee::rpc_params![])
            .await
            .unwrap();
        assert_eq!(response[SAMPLE_FIELD], SAMPLE_VALUE);
    }
}
