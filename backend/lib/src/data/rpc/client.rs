//! StorageHub RPC client implementation
//!
//! This module provides a client implementation that uses the RpcConnection 
//! abstraction for making RPC calls to the StorageHub blockchain.

use std::sync::Arc;

use crate::error::{Error, Result};
use serde_json::json;

use super::{
    AnyRpcConnection, BucketInfo, FileMetadata, ProviderInfo, RpcConnection,
    TransactionReceipt,
};

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

    /// Get file metadata from the blockchain
    pub async fn get_file_metadata(&self, file_key: &[u8]) -> Result<Option<FileMetadata>> {
        let params = json!([file_key]);

        self.connection
            .call("storagehub_getFileMetadata", params)
            .await
            .map_err(|e| Error::Rpc(jsonrpsee::core::client::Error::Custom(e.to_string())))
    }

    /// Get bucket information from the blockchain
    pub async fn get_bucket_info(&self, bucket_id: &[u8]) -> Result<Option<BucketInfo>> {
        let params = json!([bucket_id]);

        self.connection
            .call("storagehub_getBucketInfo", params)
            .await
            .map_err(|e| Error::Rpc(jsonrpsee::core::client::Error::Custom(e.to_string())))
    }

    /// Get provider information
    pub async fn get_provider_info(
        &self,
        provider_id: &[u8],
    ) -> Result<Option<ProviderInfo>> {
        let params = json!([provider_id]);

        self.connection
            .call("storagehub_getProviderInfo", params)
            .await
            .map_err(|e| Error::Rpc(jsonrpsee::core::client::Error::Custom(e.to_string())))
    }

    /// Get current block number
    pub async fn get_block_number(&self) -> Result<u64> {
        self.connection
            .call_no_params("chain_getBlockNumber")
            .await
            .map_err(|e| Error::Rpc(jsonrpsee::core::client::Error::Custom(e.to_string())))
    }

    /// Get current block hash
    pub async fn get_block_hash(&self) -> Result<Vec<u8>> {
        // Get the latest block hash
        let block_number = self.get_block_number().await?;
        let params = json!([block_number]);

        let hash: Option<String> = self
            .connection
            .call("chain_getBlockHash", params)
            .await
            .map_err(|e| Error::Rpc(jsonrpsee::core::client::Error::Custom(e.to_string())))?;

        // Convert hex string to bytes
        hash.ok_or_else(|| Error::NotFound("Block hash not found".to_string()))
            .and_then(|h| {
                hex::decode(h.trim_start_matches("0x"))
                    .map_err(|e| Error::Rpc(jsonrpsee::core::client::Error::Custom(format!("Invalid hex: {}", e))))
            })
    }

    /// Submit a storage request transaction
    pub async fn submit_storage_request(
        &self,
        location: Vec<u8>,
        fingerprint: Vec<u8>,
        size: u64,
        peer_ids: Vec<Vec<u8>>,
    ) -> Result<TransactionReceipt> {
        // Create the storage request parameters
        let params = json!({
            "location": location,
            "fingerprint": fingerprint,
            "size": size,
            "peer_ids": peer_ids,
        });

        // Submit the extrinsic
        let tx_hash: String = self
            .connection
            .call("author_submitStorageRequest", params)
            .await
            .map_err(|e| Error::Rpc(jsonrpsee::core::client::Error::Custom(e.to_string())))?;

        // Wait for transaction finalization and get receipt
        let receipt_params = json!([tx_hash]);
        let receipt: TransactionReceipt = self
            .connection
            .call("storagehub_getTransactionReceipt", receipt_params)
            .await
            .map_err(|e| Error::Rpc(jsonrpsee::core::client::Error::Custom(e.to_string())))?;

        Ok(receipt)
    }

    /// Get storage request status
    pub async fn get_storage_request_status(
        &self,
        file_key: &[u8],
    ) -> Result<Option<String>> {
        let params = json!([file_key]);

        self.connection
            .call("storagehub_getStorageRequestStatus", params)
            .await
            .map_err(|e| Error::Rpc(jsonrpsee::core::client::Error::Custom(e.to_string())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::rpc::{AnyRpcConnection, ErrorMode, MockConnection};

    #[tokio::test]
    async fn test_get_file_metadata() {
        // Create mock connection
        let mock_conn = MockConnection::new();
        mock_conn.set_response(
            "storagehub_getFileMetadata",
            json!({
                "owner": [4, 5, 6],
                "bucket_id": [7, 8, 9],
                "location": [10, 11, 12],
                "fingerprint": [13, 14, 15],
                "size": 1024,
                "peer_ids": [[16, 17], [18, 19]]
            }),
        );

        let connection = Arc::new(AnyRpcConnection::Mock(mock_conn));
        let client = StorageHubRpcClient::new(connection);

        // Test the method
        let result = client.get_file_metadata(&[1, 2, 3]).await.unwrap();
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert_eq!(metadata.owner, vec![4, 5, 6]);
        assert_eq!(metadata.size, 1024);
    }

    #[tokio::test]
    async fn test_get_block_number() {
        let mock_conn = MockConnection::new();
        mock_conn.set_response("chain_getBlockNumber", json!(12345));

        let connection = Arc::new(AnyRpcConnection::Mock(mock_conn));
        let client = StorageHubRpcClient::new(connection);

        let block_number = client.get_block_number().await.unwrap();
        assert_eq!(block_number, 12345);
    }

    #[tokio::test]
    async fn test_submit_storage_request() {
        let mock_conn = MockConnection::new();
        // Mock the submission response
        mock_conn.set_response("author_submitStorageRequest", json!("0x1234567890abcdef"));
        // Mock the receipt response
        mock_conn.set_response(
            "storagehub_getTransactionReceipt",
            json!({
                "block_hash": [11, 12, 13],
                "block_number": 100,
                "extrinsic_index": 5,
                "success": true
            }),
        );

        let connection = Arc::new(AnyRpcConnection::Mock(mock_conn));
        let client = StorageHubRpcClient::new(connection);

        let receipt = client
            .submit_storage_request(
                vec![1, 2, 3],
                vec![4, 5, 6],
                2048,
                vec![vec![7, 8], vec![9, 10]],
            )
            .await
            .unwrap();

        assert_eq!(receipt.block_number, 100);
        assert!(receipt.success);
    }

    #[tokio::test]
    async fn test_error_handling() {
        let mock_conn = MockConnection::new();
        // Set error mode to simulate connection errors
        mock_conn.set_error_mode(ErrorMode::ConnectionClosed);

        let connection = Arc::new(AnyRpcConnection::Mock(mock_conn));
        let client = StorageHubRpcClient::new(connection);

        // Test that errors are properly propagated
        let result = client.get_file_metadata(&[1, 2, 3]).await;
        assert!(result.is_err());
    }
}
