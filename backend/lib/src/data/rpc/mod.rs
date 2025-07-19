//! StorageHub RPC client module
//!
//! This module provides RPC client functionality for interacting with
//! the StorageHub blockchain runtime.

#[cfg(feature = "mocks")]
pub mod mock;

use async_trait::async_trait;
use jsonrpsee::core::Error as RpcError;
use serde::{Deserialize, Serialize};

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
    pub user_peer_ids: Vec<Vec<u8>>,
}

/// Provider information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub peer_id: Vec<u8>,
    pub root: Vec<u8>,
    pub capacity: u64,
    pub data_used: u64,
}

/// Transaction receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionReceipt {
    pub block_hash: Vec<u8>,
    pub block_number: u64,
    pub extrinsic_index: u32,
    pub success: bool,
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
    
    /// Submit a storage request transaction
    async fn submit_storage_request(
        &self,
        location: Vec<u8>,
        fingerprint: Vec<u8>,
        size: u64,
        peer_ids: Vec<Vec<u8>>,
    ) -> Result<TransactionReceipt, RpcError>;
    
    /// Get storage request status
    async fn get_storage_request_status(&self, file_key: &[u8]) -> Result<Option<String>, RpcError>;
}

// Re-export mock implementation when mocks feature is enabled
#[cfg(feature = "mocks")]
pub use mock::MockStorageHubRpc;