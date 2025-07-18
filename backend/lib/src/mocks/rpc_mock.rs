//! Mock StorageHub RPC client for testing
//!
//! This module provides a mock implementation of the StorageHub RPC client
//! that simulates blockchain interactions for testing purposes.

use async_trait::async_trait;
use jsonrpsee::core::Error as RpcError;
use serde::{Deserialize, Serialize};
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

/// File metadata on the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub owner: Vec<u8>,
    pub bucket_id: Vec<u8>,
    pub location: Vec<u8>,
    pub fingerprint: Vec<u8>,
    pub size: u64,
    pub peer_ids: Vec<Vec<u8>>,
}

/// Bucket information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketInfo {
    pub owner: Vec<u8>,
    pub msp_id: Vec<u8>,
    pub root: Vec<u8>,
    pub capacity: u64,
    pub used_capacity: u64,
}

/// Provider information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub account: Vec<u8>,
    pub peer_id: Vec<u8>,
    pub multiaddresses: Vec<String>,
    pub capacity: u64,
    pub used_capacity: u64,
}

impl MockStorageHubRpc {
    /// Create a new mock RPC client with default test data
    pub fn new() -> Self {
        let mut data = MockBlockchainData::default();
        
        // Set initial block data
        data.block_number = 1000;
        data.block_hash = vec![1, 2, 3, 4, 5, 6, 7, 8];

        // Add test bucket
        data.buckets.insert(
            vec![30, 31, 32, 33], // bucket ID
            BucketInfo {
                owner: vec![50, 51, 52, 53],
                msp_id: vec![1, 2, 3, 4],
                root: vec![40, 41, 42, 43],
                capacity: 1_000_000_000, // 1GB
                used_capacity: 3072, // 3KB used
            },
        );

        // Add test file
        data.files.insert(
            vec![70, 71, 72, 73], // file key
            FileMetadata {
                owner: vec![50, 51, 52, 53],
                bucket_id: vec![30, 31, 32, 33],
                location: vec![80, 81, 82, 83],
                fingerprint: vec![90, 91, 92, 93],
                size: 1024,
                peer_ids: vec![vec![60, 61, 62, 63]],
            },
        );

        // Add test provider (MSP)
        data.providers.insert(
            vec![1, 2, 3, 4], // MSP ID
            ProviderInfo {
                account: vec![10, 11, 12, 13],
                peer_id: vec![60, 61, 62, 63],
                multiaddresses: vec![
                    "/ip4/127.0.0.1/tcp/30333".to_string(),
                    "/ip4/192.168.1.100/tcp/30333".to_string(),
                ],
                capacity: 10_000_000_000, // 10GB
                used_capacity: 1_000_000_000, // 1GB used
            },
        );

        Self {
            data: Arc::new(Mutex::new(data)),
        }
    }

    /// Add a test file to the mock blockchain
    pub fn add_test_file(&self, file_key: Vec<u8>, metadata: FileMetadata) {
        let mut data = self.data.lock().unwrap();
        data.files.insert(file_key, metadata);
    }

    /// Add a test bucket to the mock blockchain
    pub fn add_test_bucket(&self, bucket_id: Vec<u8>, info: BucketInfo) {
        let mut data = self.data.lock().unwrap();
        data.buckets.insert(bucket_id, info);
    }

    /// Simulate advancing the blockchain
    pub fn advance_block(&self, blocks: u64) {
        let mut data = self.data.lock().unwrap();
        data.block_number += blocks;
        // Update block hash (simple simulation)
        data.block_hash = data.block_number.to_be_bytes().to_vec();
    }

    /// Clear all mock data
    pub fn clear_data(&self) {
        let mut data = self.data.lock().unwrap();
        data.files.clear();
        data.buckets.clear();
        data.providers.clear();
    }
}

impl Default for MockStorageHubRpc {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for StorageHub RPC operations
#[async_trait]
pub trait StorageHubRpcTrait: Send + Sync {
    /// Get file metadata from the blockchain
    async fn get_file_metadata(&self, file_key: &[u8]) -> Result<Option<FileMetadata>, RpcError>;
    
    /// Get bucket information from the blockchain
    async fn get_bucket_info(&self, bucket_id: &[u8]) -> Result<Option<BucketInfo>, RpcError>;
    
    /// Get provider information
    async fn get_provider_info(&self, provider_id: &[u8]) -> Result<Option<ProviderInfo>, RpcError>;
    
    /// Get current block number
    async fn get_block_number(&self) -> Result<u64, RpcError>;
    
    /// Get current block hash
    async fn get_block_hash(&self) -> Result<Vec<u8>, RpcError>;
    
    /// Submit a storage request (returns transaction hash)
    async fn submit_storage_request(
        &self,
        file_key: &[u8],
        bucket_id: &[u8],
        location: &[u8],
        fingerprint: &[u8],
        size: u64,
        peer_ids: Vec<Vec<u8>>,
    ) -> Result<Vec<u8>, RpcError>;
    
    /// Confirm storage (BSP confirms storing a file)
    async fn confirm_storage(
        &self,
        file_key: &[u8],
        proof: &[u8],
    ) -> Result<Vec<u8>, RpcError>;
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
        file_key: &[u8],
        bucket_id: &[u8],
        location: &[u8],
        fingerprint: &[u8],
        size: u64,
        peer_ids: Vec<Vec<u8>>,
    ) -> Result<Vec<u8>, RpcError> {
        let mut data = self.data.lock().unwrap();
        
        // Check if bucket exists
        if !data.buckets.contains_key(bucket_id) {
            return Err(RpcError::Custom("Bucket not found".to_string()));
        }
        
        // Check if file already exists
        if data.files.contains_key(file_key) {
            return Err(RpcError::Custom("File already exists".to_string()));
        }
        
        // Get bucket info to find owner
        let bucket_info = data.buckets.get(bucket_id).unwrap();
        
        // Add file to storage
        data.files.insert(
            file_key.to_vec(),
            FileMetadata {
                owner: bucket_info.owner.clone(),
                bucket_id: bucket_id.to_vec(),
                location: location.to_vec(),
                fingerprint: fingerprint.to_vec(),
                size,
                peer_ids,
            },
        );
        
        // Update bucket used capacity
        if let Some(bucket) = data.buckets.get_mut(bucket_id) {
            bucket.used_capacity += size;
        }
        
        // Return mock transaction hash
        Ok(vec![100, 101, 102, 103, 104, 105, 106, 107])
    }

    async fn confirm_storage(
        &self,
        file_key: &[u8],
        _proof: &[u8],
    ) -> Result<Vec<u8>, RpcError> {
        let data = self.data.lock().unwrap();
        
        // Check if file exists
        if !data.files.contains_key(file_key) {
            return Err(RpcError::Custom("File not found".to_string()));
        }
        
        // Return mock transaction hash
        Ok(vec![110, 111, 112, 113, 114, 115, 116, 117])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_rpc_file_operations() {
        let rpc = MockStorageHubRpc::new();
        
        // Test getting existing file
        let file_key = vec![70, 71, 72, 73];
        let metadata = rpc.get_file_metadata(&file_key).await.unwrap();
        assert!(metadata.is_some());
        
        // Test getting non-existent file
        let missing_key = vec![99, 99, 99, 99];
        let metadata = rpc.get_file_metadata(&missing_key).await.unwrap();
        assert!(metadata.is_none());
        
        // Test submitting new storage request
        let new_key = vec![80, 81, 82, 83];
        let bucket_id = vec![30, 31, 32, 33];
        let tx_hash = rpc
            .submit_storage_request(
                &new_key,
                &bucket_id,
                &[90, 91, 92, 93],
                &[100, 101, 102, 103],
                2048,
                vec![vec![70, 71, 72, 73]],
            )
            .await
            .unwrap();
        assert!(!tx_hash.is_empty());
        
        // Verify file was added
        let metadata = rpc.get_file_metadata(&new_key).await.unwrap();
        assert!(metadata.is_some());
    }

    #[tokio::test]
    async fn test_mock_rpc_block_operations() {
        let rpc = MockStorageHubRpc::new();
        
        // Test initial block number
        let block_num = rpc.get_block_number().await.unwrap();
        assert_eq!(block_num, 1000);
        
        // Test advancing blocks
        rpc.advance_block(10);
        let block_num = rpc.get_block_number().await.unwrap();
        assert_eq!(block_num, 1010);
        
        // Test block hash changes
        let block_hash = rpc.get_block_hash().await.unwrap();
        assert!(!block_hash.is_empty());
    }
}