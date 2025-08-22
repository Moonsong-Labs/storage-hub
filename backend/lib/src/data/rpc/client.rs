//! StorageHub RPC client implementation

use std::sync::Arc;

use crate::data::rpc::{AnyRpcConnection, RpcConnection};

/// StorageHub RPC client that uses an RpcConnection
pub struct StorageHubRpcClient {
    connection: Arc<AnyRpcConnection>,
}

impl StorageHubRpcClient {
    /// Create a new StorageHubRpcClient with the given connection
    pub fn new(connection: Arc<AnyRpcConnection>) -> Self {
        Self { connection }
    }
}

impl StorageHubRpcClient {
    /// Check if the RPC connection is active
    pub async fn is_connected(&self) -> bool {
        self.connection.is_connected().await
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
}
