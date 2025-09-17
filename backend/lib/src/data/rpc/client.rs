//! StorageHub RPC client implementation

use std::sync::Arc;

use jsonrpsee::core::traits::ToRpcParams;
use serde::de::DeserializeOwned;

use crate::data::rpc::{connection::error::RpcResult, AnyRpcConnection, RpcConnection};

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

    /// Forward a JSON-RPC call to the underlying connection
    pub async fn call<P, R>(&self, method: &str, params: P) -> RpcResult<R>
    where
        P: ToRpcParams + Send + Sync,
        R: DeserializeOwned,
    {
        self.connection.call(method, params).await
    }

    /// Get the current price per giga unit per tick
    ///
    /// Returns the price value (u128) that represents the cost per giga unit per tick
    /// in the StorageHub network.
    pub async fn get_current_price_per_unit_per_tick(&self) -> RpcResult<u128> {
        self.connection
            .call(
                "storagehubclient_getCurrentPricePerUnitPerTick",
                jsonrpsee::rpc_params![],
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
        assert_eq!(price > 0);
    }
}
