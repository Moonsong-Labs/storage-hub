//! StorageHub RPC client implementation
//!
//! This module provides a concrete implementation of the StorageHubRpcTrait
//! that uses the RpcConnection abstraction for making RPC calls to the
//! StorageHub blockchain.

use async_trait::async_trait;
use jsonrpsee::core::Error as RpcError;
use serde_json::json;
use std::sync::Arc;

use super::{
    FileMetadata, BucketInfo, ProviderInfo, TransactionReceipt,
    StorageHubRpcTrait, RpcConnection, IntoRpcError,
};

/// StorageHub RPC client that uses an RpcConnection
pub struct StorageHubRpcClient {
    connection: Arc<dyn RpcConnection>,
}

impl StorageHubRpcClient {
    /// Create a new StorageHubRpcClient with the given connection
    pub fn new(connection: Arc<dyn RpcConnection>) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl StorageHubRpcTrait for StorageHubRpcClient {
    async fn get_file_metadata(&self, file_key: &[u8]) -> Result<Option<FileMetadata>, RpcError> {
        let params = json!([file_key]);
        
        self.connection
            .call("storagehub_getFileMetadata", params)
            .await
            .map_err(|e| RpcError::Custom(e.to_string()))
    }
    
    async fn get_bucket_info(&self, bucket_id: &[u8]) -> Result<Option<BucketInfo>, RpcError> {
        let params = json!([bucket_id]);
        
        self.connection
            .call("storagehub_getBucketInfo", params)
            .await
            .map_err(|e| RpcError::Custom(e.to_string()))
    }
    
    async fn get_provider_info(&self, provider_id: &[u8]) -> Result<Option<ProviderInfo>, RpcError> {
        let params = json!([provider_id]);
        
        self.connection
            .call("storagehub_getProviderInfo", params)
            .await
            .map_err(|e| RpcError::Custom(e.to_string()))
    }
    
    async fn get_block_number(&self) -> Result<u64, RpcError> {
        self.connection
            .call_no_params("chain_getBlockNumber")
            .await
            .map_err(|e| RpcError::Custom(e.to_string()))
    }
    
    async fn get_block_hash(&self) -> Result<Vec<u8>, RpcError> {
        // Get the latest block hash
        let block_number = self.get_block_number().await?;
        let params = json!([block_number]);
        
        let hash: Option<String> = self.connection
            .call("chain_getBlockHash", params)
            .await
            .map_err(|e| RpcError::Custom(e.to_string()))?;
        
        // Convert hex string to bytes
        hash.ok_or_else(|| RpcError::Custom("Block hash not found".to_string()))
            .and_then(|h| {
                hex::decode(h.trim_start_matches("0x"))
                    .map_err(|e| RpcError::Custom(format!("Invalid hex: {}", e)))
            })
    }
    
    async fn submit_storage_request(
        &self,
        location: Vec<u8>,
        fingerprint: Vec<u8>,
        size: u64,
        peer_ids: Vec<Vec<u8>>,
    ) -> Result<TransactionReceipt, RpcError> {
        // Create the storage request parameters
        let params = json!({
            "location": location,
            "fingerprint": fingerprint,
            "size": size,
            "peer_ids": peer_ids,
        });
        
        // Submit the extrinsic
        let tx_hash: String = self.connection
            .call("author_submitStorageRequest", params)
            .await
            .map_err(|e| RpcError::Custom(e.to_string()))?;
        
        // Wait for transaction finalization and get receipt
        let receipt_params = json!([tx_hash]);
        let receipt: TransactionReceipt = self.connection
            .call("storagehub_getTransactionReceipt", receipt_params)
            .await
            .map_err(|e| RpcError::Custom(e.to_string()))?;
        
        Ok(receipt)
    }
    
    async fn get_storage_request_status(&self, file_key: &[u8]) -> Result<Option<String>, RpcError> {
        let params = json!([file_key]);
        
        self.connection
            .call("storagehub_getStorageRequestStatus", params)
            .await
            .map_err(|e| RpcError::Custom(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::rpc::{MockConnection, MockConnectionBuilder, ErrorMode};
    use serde_json::Value;
    
    #[tokio::test]
    async fn test_get_file_metadata() {
        // Create mock connection
        let mut builder = MockConnectionBuilder::new();
        builder.add_response(
            "storagehub_getFileMetadata",
            json!([[1, 2, 3]]),
            json!({
                "owner": [4, 5, 6],
                "bucket_id": [7, 8, 9],
                "location": [10, 11, 12],
                "fingerprint": [13, 14, 15],
                "size": 1024,
                "peer_ids": [[16, 17], [18, 19]]
            }),
        );
        
        let connection = Arc::new(builder.build().await.unwrap());
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
        let mut builder = MockConnectionBuilder::new();
        builder.add_response(
            "chain_getBlockNumber",
            json!([]),
            json!(12345),
        );
        
        let connection = Arc::new(builder.build().await.unwrap());
        let client = StorageHubRpcClient::new(connection);
        
        let block_number = client.get_block_number().await.unwrap();
        assert_eq!(block_number, 12345);
    }
    
    #[tokio::test]
    async fn test_submit_storage_request() {
        let mut builder = MockConnectionBuilder::new();
        
        // Mock the submission response
        builder.add_response(
            "author_submitStorageRequest",
            json!({
                "location": [1, 2, 3],
                "fingerprint": [4, 5, 6],
                "size": 2048,
                "peer_ids": [[7, 8], [9, 10]]
            }),
            json!("0x1234567890abcdef"),
        );
        
        // Mock the receipt response
        builder.add_response(
            "storagehub_getTransactionReceipt",
            json!(["0x1234567890abcdef"]),
            json!({
                "block_hash": [11, 12, 13],
                "block_number": 100,
                "extrinsic_index": 5,
                "success": true
            }),
        );
        
        let connection = Arc::new(builder.build().await.unwrap());
        let client = StorageHubRpcClient::new(connection);
        
        let receipt = client.submit_storage_request(
            vec![1, 2, 3],
            vec![4, 5, 6],
            2048,
            vec![vec![7, 8], vec![9, 10]],
        ).await.unwrap();
        
        assert_eq!(receipt.block_number, 100);
        assert!(receipt.success);
    }
    
    #[tokio::test]
    async fn test_error_handling() {
        let mut builder = MockConnectionBuilder::new();
        builder.set_error_mode(ErrorMode::AllErrors);
        
        let connection = Arc::new(builder.build().await.unwrap());
        let client = StorageHubRpcClient::new(connection);
        
        // Test that errors are properly propagated
        let result = client.get_file_metadata(&[1, 2, 3]).await;
        assert!(result.is_err());
    }
}