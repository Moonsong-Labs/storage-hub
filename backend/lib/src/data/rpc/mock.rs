//! Mock StorageHub RPC client for testing
//!
//! This module provides a mock implementation of the StorageHub RPC client
//! that simulates blockchain interactions for testing purposes.

use super::{BucketInfo, FileMetadata, ProviderInfo, StorageHubRpcTrait, TransactionReceipt};
use async_trait::async_trait;
use jsonrpsee::core::Error as RpcError;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Mock StorageHub RPC client
#[derive(Debug, Clone)]
pub struct MockStorageHubRpc {
    /// Storage for simulated blockchain data
    data: Arc<Mutex<MockBlockchainData>>,
}

/// Mock blockchain data storage
#[derive(Debug, Default)]
struct MockBlockchainData {
    /// File metadata by file key
    files: HashMap<Vec<u8>, FileMetadata>,
    /// Bucket data by bucket ID
    buckets: HashMap<Vec<u8>, BucketInfo>,
    /// Provider information
    providers: HashMap<Vec<u8>, ProviderInfo>,
    /// Current block number
    block_number: u64,
    /// Current block hash
    block_hash: Vec<u8>,
}

impl MockStorageHubRpc {
    /// Create a new mock RPC client
    pub fn new() -> Self {
        let mut data = MockBlockchainData::default();
        
        // Initialize with some test data
        data.block_number = 100;
        data.block_hash = vec![0xde, 0xad, 0xbe, 0xef];
        
        // Add test provider
        data.providers.insert(
            vec![1, 2, 3, 4],
            ProviderInfo {
                peer_id: vec![10, 20, 30, 40],
                root: vec![11, 22, 33, 44],
                capacity: 1_000_000,
                data_used: 100_000,
            },
        );
        
        // Add test bucket
        data.buckets.insert(
            vec![5, 6, 7, 8],
            BucketInfo {
                owner: vec![50, 60, 70, 80],
                msp_id: vec![1, 2, 3, 4],
                root: vec![55, 66, 77, 88],
                user_peer_ids: vec![vec![90, 91, 92], vec![93, 94, 95]],
            },
        );
        
        // Add test file
        data.files.insert(
            vec![100, 101, 102],
            FileMetadata {
                owner: vec![50, 60, 70, 80],
                bucket_id: vec![5, 6, 7, 8],
                location: vec![110, 111, 112],
                fingerprint: vec![120, 121, 122],
                size: 1024,
                peer_ids: vec![vec![130, 131], vec![132, 133]],
            },
        );
        
        Self {
            data: Arc::new(Mutex::new(data)),
        }
    }
    
    /// Add a file to the mock storage
    pub fn add_file(&self, file_key: Vec<u8>, metadata: FileMetadata) {
        let mut data = self.data.lock().unwrap();
        data.files.insert(file_key, metadata);
    }
    
    /// Add a bucket to the mock storage
    pub fn add_bucket(&self, bucket_id: Vec<u8>, info: BucketInfo) {
        let mut data = self.data.lock().unwrap();
        data.buckets.insert(bucket_id, info);
    }
    
    /// Increment block number
    pub fn increment_block(&self) {
        let mut data = self.data.lock().unwrap();
        data.block_number += 1;
        // Update block hash
        data.block_hash = vec![
            (data.block_number >> 24) as u8,
            (data.block_number >> 16) as u8,
            (data.block_number >> 8) as u8,
            data.block_number as u8,
        ];
    }
}

impl Default for MockStorageHubRpc {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageHubRpcTrait for MockStorageHubRpc {
    async fn get_file_metadata(&self, file_key: &[u8]) -> Result<Option<FileMetadata>, RpcError> {
        let data = self.data.lock().unwrap();
        Ok(data.files.get(file_key).cloned())
    }
    
    async fn get_bucket_info(&self, bucket_id: &[u8]) -> Result<Option<BucketInfo>, RpcError> {
        let data = self.data.lock().unwrap();
        Ok(data.buckets.get(bucket_id).cloned())
    }
    
    async fn get_provider_info(&self, provider_id: &[u8]) -> Result<Option<ProviderInfo>, RpcError> {
        let data = self.data.lock().unwrap();
        Ok(data.providers.get(provider_id).cloned())
    }
    
    async fn get_block_number(&self) -> Result<u64, RpcError> {
        let data = self.data.lock().unwrap();
        Ok(data.block_number)
    }
    
    async fn get_block_hash(&self) -> Result<Vec<u8>, RpcError> {
        let data = self.data.lock().unwrap();
        Ok(data.block_hash.clone())
    }
    
    async fn submit_storage_request(
        &self,
        location: Vec<u8>,
        fingerprint: Vec<u8>,
        size: u64,
        peer_ids: Vec<Vec<u8>>,
    ) -> Result<TransactionReceipt, RpcError> {
        // Simulate successful transaction
        let data = self.data.lock().unwrap();
        Ok(TransactionReceipt {
            block_hash: data.block_hash.clone(),
            block_number: data.block_number,
            extrinsic_index: 1,
            success: true,
        })
    }
    
    async fn get_storage_request_status(&self, file_key: &[u8]) -> Result<Option<String>, RpcError> {
        let data = self.data.lock().unwrap();
        if data.files.contains_key(file_key) {
            Ok(Some("confirmed".to_string()))
        } else {
            Ok(Some("pending".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mock_rpc_basic_operations() {
        let rpc = MockStorageHubRpc::new();
        
        // Test block operations
        let block_num = rpc.get_block_number().await.unwrap();
        assert_eq!(block_num, 100);
        
        let block_hash = rpc.get_block_hash().await.unwrap();
        assert_eq!(block_hash, vec![0xde, 0xad, 0xbe, 0xef]);
        
        // Test increment block
        rpc.increment_block();
        let new_block_num = rpc.get_block_number().await.unwrap();
        assert_eq!(new_block_num, 101);
    }
    
    #[tokio::test]
    async fn test_mock_rpc_file_operations() {
        let rpc = MockStorageHubRpc::new();
        
        // Test existing file
        let file_key = vec![100, 101, 102];
        let metadata = rpc.get_file_metadata(&file_key).await.unwrap();
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        assert_eq!(metadata.size, 1024);
        
        // Test non-existent file
        let missing_key = vec![200, 201, 202];
        let metadata = rpc.get_file_metadata(&missing_key).await.unwrap();
        assert!(metadata.is_none());
    }
    
    #[tokio::test]
    async fn test_mock_rpc_storage_request() {
        let rpc = MockStorageHubRpc::new();
        
        // Submit storage request
        let receipt = rpc.submit_storage_request(
            vec![1, 2, 3],
            vec![4, 5, 6],
            2048,
            vec![vec![7, 8], vec![9, 10]],
        ).await.unwrap();
        
        assert!(receipt.success);
        assert_eq!(receipt.block_number, 100);
    }
}