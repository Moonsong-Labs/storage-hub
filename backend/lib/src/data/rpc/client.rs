//! StorageHub RPC client implementation

use std::sync::Arc;

use jsonrpsee::core::traits::ToRpcParams;
use serde::de::DeserializeOwned;

use shc_rpc::SaveFileToDisk;

use crate::data::rpc::{connection::error::RpcResult, methods, AnyRpcConnection, RpcConnection};

/// StorageHub RPC client that uses an RpcConnection
pub struct StorageHubRpcClient {
    connection: Arc<AnyRpcConnection>,
}

impl StorageHubRpcClient {
    /// Create a new StorageHubRpcClient with the given connection
    pub fn new(connection: Arc<AnyRpcConnection>) -> Self {
        Self { connection }
    }

    pub async fn is_connected(&self) -> bool {
        self.connection.is_connected().await
    }

    /// Call a JSON-RPC method on the connected node
    pub async fn call<P, R>(&self, method: &str, params: P) -> RpcResult<R>
    where
        P: ToRpcParams + Send,
        R: DeserializeOwned,
    {
        self.connection.call(method, params).await
    }

    // TODO: use the storagehubrpc client trait directly

    /// Get the current price per giga unit per tick
    ///
    /// Returns the price value (u128) that represents the cost per giga unit per tick
    /// in the StorageHub network.
    pub async fn get_current_price_per_unit_per_tick(&self) -> RpcResult<u128> {
        self.connection
            .call(methods::CURRENT_PRICE, jsonrpsee::rpc_params![])
            .await
    }

    /// Returns whether the given file key is expected to be received by the MSP node
    pub async fn is_file_key_expected(&self, file_key: &str) -> RpcResult<bool> {
        self.connection
            .call(methods::FILE_KEY_EXPECTED, jsonrpsee::rpc_params![file_key])
            .await
    }

    pub async fn save_file_to_disk(&self, file_key: &str, path: &str) -> RpcResult<SaveFileToDisk> {
        self.connection
            .call(
                methods::SAVE_FILE_TO_DISK,
                jsonrpsee::rpc_params![file_key, path],
            )
            .await
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::*;
    use crate::data::rpc::{AnyRpcConnection, MockConnection};

    // TODO(SCAFFOLDING): this will contain proper tests when we have defined
    // what RPC methods to make use of
    #[tokio::test]
    async fn use_mock_connection() {
        let mock_conn = MockConnection::new();
        mock_conn.disconnect().await;

        let connection = Arc::new(AnyRpcConnection::Mock(mock_conn));
        let client = StorageHubRpcClient::new(connection);

        let connected = client.is_connected().await;
        assert!(!connected);
    }

    #[tokio::test]
    async fn test_get_current_price_per_unit_per_tick() {
        let mock_conn = MockConnection::new();
        let connection = Arc::new(AnyRpcConnection::Mock(mock_conn));
        let client = StorageHubRpcClient::new(connection);

        // Test that the mock returns the expected price
        let price = client
            .get_current_price_per_unit_per_tick()
            .await
            .expect("able to retrieve current price per giga unit");
        assert!(price > 0);
    }
}
