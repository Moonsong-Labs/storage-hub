//! StorageHub RPC client implementation

use std::sync::Arc;

use serde_json::json;

use super::{
    AnyRpcConnection, BucketInfo, FileMetadata, ProviderInfo, RpcConnection, TransactionReceipt,
};
use crate::error::{Error, Result};

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
    pub async fn get_provider_info(&self, provider_id: &[u8]) -> Result<Option<ProviderInfo>> {
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
                hex::decode(h.trim_start_matches("0x")).map_err(|e| {
                    Error::Rpc(jsonrpsee::core::client::Error::Custom(format!(
                        "Invalid hex: {}",
                        e
                    )))
                })
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
    pub async fn get_storage_request_status(&self, file_key: &[u8]) -> Result<Option<String>> {
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
    use crate::{
        constants::test::{
            accounts, blockchain, file_keys, file_metadata, helpers,
        },
        data::rpc::{AnyRpcConnection, ErrorMode, MockConnection},
    };

    #[tokio::test]
    async fn test_get_file_metadata() {
        // Create mock connection
        let mock_conn = MockConnection::new();
        mock_conn.set_response(
            "storagehub_getFileMetadata",
            helpers::create_test_file_metadata(),
        );

        let connection = Arc::new(AnyRpcConnection::Mock(mock_conn));
        let client = StorageHubRpcClient::new(connection);

        // Test the method with a clearly defined test file key
        let result = client
            .get_file_metadata(file_keys::TEST_FILE_KEY)
            .await
            .unwrap();
        assert!(result.is_some());

        let metadata = result.unwrap();
        assert_eq!(metadata.owner, accounts::TEST_OWNER.to_vec());
        assert_eq!(metadata.size, file_metadata::TEST_FILE_SIZE);
    }

    #[tokio::test]
    async fn test_get_block_number() {
        let mock_conn = MockConnection::new();
        mock_conn.set_response("chain_getBlockNumber", json!(blockchain::TEST_BLOCK_NUMBER));

        let connection = Arc::new(AnyRpcConnection::Mock(mock_conn));
        let client = StorageHubRpcClient::new(connection);

        let block_number = client.get_block_number().await.unwrap();
        assert_eq!(block_number, blockchain::TEST_BLOCK_NUMBER);
    }

    #[tokio::test]
    async fn test_submit_storage_request() {
        let mock_conn = MockConnection::new();
        // Mock the submission response
        mock_conn.set_response(
            "author_submitStorageRequest",
            json!(blockchain::TEST_TX_HASH),
        );
        // Mock the receipt response
        mock_conn.set_response(
            "storagehub_getTransactionReceipt",
            helpers::create_test_transaction_receipt(),
        );

        let connection = Arc::new(AnyRpcConnection::Mock(mock_conn));
        let client = StorageHubRpcClient::new(connection);

        let receipt = client
            .submit_storage_request(
                file_metadata::ALTERNATIVE_LOCATION.to_vec(),
                file_metadata::ALTERNATIVE_FINGERPRINT.to_vec(),
                file_metadata::LARGE_FILE_SIZE,
                helpers::create_test_peer_ids(),
            )
            .await
            .unwrap();

        assert_eq!(receipt.block_number, blockchain::ALTERNATIVE_BLOCK_NUMBER);
        assert!(receipt.success);
    }

    #[tokio::test]
    async fn test_error_handling() {
        let mock_conn = MockConnection::new();
        // Set error mode to simulate connection errors
        mock_conn.set_error_mode(ErrorMode::ConnectionClosed);

        let connection = Arc::new(AnyRpcConnection::Mock(mock_conn));
        let client = StorageHubRpcClient::new(connection);

        // Test that errors are properly propagated with a well-defined test file key
        let result = client.get_file_metadata(file_keys::TEST_FILE_KEY).await;
        assert!(result.is_err());
    }
}
