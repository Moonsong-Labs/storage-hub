//! StorageHub RPC client module

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub mod client;
pub mod connection;
pub mod ws_connection;

#[cfg(feature = "mocks")]
pub mod mock_connection;

pub use client::StorageHubRpcClient;
pub use connection::{AnyRpcConnection, IntoRpcError, RpcConfig, RpcConnectionError, RpcResult};
#[cfg(feature = "mocks")]
pub use mock_connection::{ErrorMode, MockConnection, MockConnectionBuilder};
pub use ws_connection::{WsConnection, WsConnectionBuilder};

/// Trait for RPC connections
#[async_trait]
pub trait RpcConnection: Send + Sync {
    /// Execute a JSON-RPC method call
    async fn call<P, R>(&self, method: &str, params: P) -> RpcResult<R>
    where
        P: Serialize + Send + Sync,
        R: DeserializeOwned;

    /// Execute a JSON-RPC method call without parameters
    async fn call_no_params<R>(&self, method: &str) -> RpcResult<R>
    where
        R: DeserializeOwned,
    {
        // Default implementation using empty tuple as params
        self.call::<_, R>(method, ()).await
    }

    /// Check if the connection is currently active
    async fn is_connected(&self) -> bool;

    /// Close the connection gracefully
    async fn close(&self) -> RpcResult<()>;
}

// TODO: These types could be imported from the client crate if they were in a separate
// types-only crate. Currently, importing from shc-rpc would bring in too many dependencies
// and unrelated functionality (substrate node RPC implementation, file storage, etc.).
// Consider extracting common types to a shared crate in the future.

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
