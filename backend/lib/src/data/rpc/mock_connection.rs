//! Mock RPC connection implementation for testing
//!
//! This module provides a mock RPC connection that can be configured
//! with predefined responses and error scenarios for testing purposes.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use tokio::time::sleep;

use super::connection::{RpcConnection, RpcConnectionBuilder, RpcConnectionError, RpcResult};

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
    responses: Arc<Mutex<HashMap<String, Value>>>,
    /// Error simulation mode
    error_mode: Arc<Mutex<ErrorMode>>,
    /// Whether the connection is "connected"
    connected: Arc<Mutex<bool>>,
    /// Call counter for FailAfterNCalls mode
    call_count: Arc<Mutex<usize>>,
    /// Optional delay to simulate network latency
    latency_ms: Arc<Mutex<Option<u64>>>,
}

impl MockConnection {
    /// Create a new mock connection with default responses
    pub fn new() -> Self {
        let mut responses = HashMap::new();

        // Add default responses for common StorageHub methods
        responses.insert(
            "system_health".to_string(),
            serde_json::json!({
                "peers": 5,
                "isSyncing": false,
                "shouldHavePeers": true
            }),
        );

        responses.insert(
            "chain_getBlockHash".to_string(),
            serde_json::json!("0xdeadbeef"),
        );

        responses.insert(
            "chain_getHeader".to_string(),
            serde_json::json!({
                "number": "0x64",
                "parentHash": "0xabcdef",
                "stateRoot": "0x123456",
                "extrinsicsRoot": "0x789abc"
            }),
        );

        responses.insert("state_getStorage".to_string(), serde_json::json!(null));

        // Add default file metadata response
        responses.insert(
            "storagehub_getFileMetadata".to_string(),
            serde_json::json!({
                "owner": [50, 60, 70, 80],
                "bucket_id": [5, 6, 7, 8],
                "location": [110, 111, 112],
                "fingerprint": [120, 121, 122],
                "size": 1024,
                "peer_ids": [[130, 131], [132, 133]]
            }),
        );

        // Add default bucket info response
        responses.insert(
            "storagehub_getBucketInfo".to_string(),
            serde_json::json!({
                "owner": [50, 60, 70, 80],
                "msp_id": [1, 2, 3, 4],
                "root": [55, 66, 77, 88],
                "user_peer_ids": [[90, 91, 92], [93, 94, 95]]
            }),
        );

        // Add default provider info response
        responses.insert(
            "storagehub_getProviderInfo".to_string(),
            serde_json::json!({
                "peer_id": [10, 20, 30, 40],
                "root": [11, 22, 33, 44],
                "capacity": 1000000,
                "data_used": 100000
            }),
        );

        // Add transaction submission response
        responses.insert(
            "storagehub_submitStorageRequest".to_string(),
            serde_json::json!({
                "block_hash": [222, 173, 190, 239],
                "block_number": 100,
                "extrinsic_index": 1,
                "success": true
            }),
        );

        // Add storage request status response
        responses.insert(
            "storagehub_getStorageRequestStatus".to_string(),
            serde_json::json!("confirmed"),
        );

        // Don't add default responses for methods that might be tested for errors
        // The test_error_handling test expects no response to be set

        Self {
            responses: Arc::new(Mutex::new(responses)),
            error_mode: Arc::new(Mutex::new(ErrorMode::None)),
            connected: Arc::new(Mutex::new(true)),
            call_count: Arc::new(Mutex::new(0)),
            latency_ms: Arc::new(Mutex::new(None)),
        }
    }

    /// Set a custom response for a specific method
    pub fn set_response(&self, method: &str, response: Value) {
        let mut responses = self.responses.lock().unwrap();
        responses.insert(method.to_string(), response);
    }

    /// Set the error simulation mode
    pub fn set_error_mode(&self, mode: ErrorMode) {
        let mut error_mode = self.error_mode.lock().unwrap();
        *error_mode = mode;

        // Reset call count when changing error mode
        let mut call_count = self.call_count.lock().unwrap();
        *call_count = 0;
    }

    /// Set network latency simulation
    pub fn set_latency_ms(&self, latency: u64) {
        let mut latency_guard = self.latency_ms.lock().unwrap();
        *latency_guard = Some(latency);
    }

    /// Simulate disconnection
    pub fn disconnect(&self) {
        let mut connected = self.connected.lock().unwrap();
        *connected = false;
    }

    /// Simulate reconnection
    pub fn reconnect(&self) {
        let mut connected = self.connected.lock().unwrap();
        *connected = true;
    }

    /// Check if we should simulate an error based on current mode
    async fn check_error(&self) -> RpcResult<()> {
        let current_count = {
            let mut call_count = self.call_count.lock().unwrap();
            *call_count += 1;
            *call_count
        };

        let error_mode = self.error_mode.lock().unwrap().clone();

        match error_mode {
            ErrorMode::None => Ok(()),
            ErrorMode::Timeout => {
                // Simulate timeout by sleeping then returning error
                let latency = *self.latency_ms.lock().unwrap();
                if let Some(latency_ms) = latency {
                    sleep(Duration::from_millis(latency_ms * 10)).await;
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
        Self::new()
    }
}

#[async_trait]
impl RpcConnection for MockConnection {
    async fn call<P, R>(&self, method: &str, _params: P) -> RpcResult<R>
    where
        P: Serialize + Send + Sync,
        R: DeserializeOwned,
    {
        // Check if connected
        {
            let connected = self.connected.lock().unwrap();
            if !*connected {
                return Err(RpcConnectionError::ConnectionClosed);
            }
        }

        // Simulate network latency if configured
        let latency = *self.latency_ms.lock().unwrap();
        if let Some(latency_ms) = latency {
            sleep(Duration::from_millis(latency_ms)).await;
        }

        // Check for simulated errors
        self.check_error().await?;

        // Get response for method
        let response = {
            let responses = self.responses.lock().unwrap();
            responses
                .get(method)
                .cloned()
                .unwrap_or(serde_json::json!(null))
        };

        // Deserialize the response
        serde_json::from_value(response).map_err(|e| {
            RpcConnectionError::Serialization(format!("Failed to deserialize response: {}", e))
        })
    }

    async fn is_connected(&self) -> bool {
        let connected = self.connected.lock().unwrap();
        *connected
    }

    async fn close(&self) -> RpcResult<()> {
        self.disconnect();
        Ok(())
    }
}

/// Builder for mock RPC connections
pub struct MockConnectionBuilder {
    connection: MockConnection,
}

impl MockConnectionBuilder {
    /// Create a new mock connection builder
    pub fn new() -> Self {
        Self {
            connection: MockConnection::new(),
        }
    }

    /// Add a custom response for a method
    pub fn with_response(self, method: &str, response: Value) -> Self {
        self.connection.set_response(method, response);
        self
    }

    /// Set the error simulation mode
    pub fn with_error_mode(self, mode: ErrorMode) -> Self {
        self.connection.set_error_mode(mode);
        self
    }

    /// Set network latency simulation
    pub fn with_latency_ms(self, latency: u64) -> Self {
        self.connection.set_latency_ms(latency);
        self
    }

    /// Start in disconnected state
    pub fn disconnected(self) -> Self {
        self.connection.disconnect();
        self
    }
}

impl Default for MockConnectionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RpcConnectionBuilder for MockConnectionBuilder {
    type Connection = MockConnection;

    async fn build(self) -> RpcResult<Self::Connection> {
        Ok(self.connection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_connection_basic() {
        let conn = MockConnection::new();

        // Test system health call
        let health: Value = conn.call("system_health", ()).await.unwrap();
        assert_eq!(health["peers"], 5);
        assert_eq!(health["isSyncing"], false);

        // Test connection status
        assert!(conn.is_connected().await);

        // Test close
        conn.close().await.unwrap();
        assert!(!conn.is_connected().await);
    }

    #[tokio::test]
    async fn test_mock_connection_custom_response() {
        let conn = MockConnection::new();
        conn.set_response("custom_method", serde_json::json!({"result": "custom"}));

        let response: Value = conn.call("custom_method", ()).await.unwrap();
        assert_eq!(response["result"], "custom");
    }

    #[tokio::test]
    async fn test_mock_connection_error_modes() {
        // Test timeout error
        let conn = MockConnection::new();
        conn.set_error_mode(ErrorMode::Timeout);
        let result: Result<Value, _> = conn.call("any_method", ()).await;
        assert!(matches!(result, Err(RpcConnectionError::Timeout)));

        // Test connection closed error
        let conn = MockConnection::new();
        conn.set_error_mode(ErrorMode::ConnectionClosed);
        let result: Result<Value, _> = conn.call("any_method", ()).await;
        assert!(matches!(result, Err(RpcConnectionError::ConnectionClosed)));

        // Test fail after N calls
        let conn = MockConnection::new();
        conn.set_error_mode(ErrorMode::FailAfterNCalls(2));

        // First two calls should succeed
        let _: Value = conn.call("system_health", ()).await.unwrap();
        let _: Value = conn.call("system_health", ()).await.unwrap();

        // Third call should fail
        let result: Result<Value, _> = conn.call("system_health", ()).await;
        assert!(matches!(result, Err(RpcConnectionError::Rpc(_))));
    }

    #[tokio::test]
    async fn test_mock_connection_builder() {
        let conn = MockConnectionBuilder::new()
            .with_response("test", serde_json::json!(42))
            .with_latency_ms(10)
            .with_error_mode(ErrorMode::None)
            .build()
            .await
            .unwrap();

        let response: i32 = conn.call("test", ()).await.unwrap();
        assert_eq!(response, 42);
    }

    #[tokio::test]
    async fn test_mock_connection_disconnect_reconnect() {
        let conn = MockConnection::new();

        // Initially connected
        assert!(conn.is_connected().await);

        // Disconnect
        conn.disconnect();
        assert!(!conn.is_connected().await);

        // Try to call - should fail
        let result: Result<Value, _> = conn.call("any_method", ()).await;
        assert!(matches!(result, Err(RpcConnectionError::ConnectionClosed)));

        // Reconnect
        conn.reconnect();
        assert!(conn.is_connected().await);

        // Call should work now
        let _: Value = conn.call("system_health", ()).await.unwrap();
    }
}
