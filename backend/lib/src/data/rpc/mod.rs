//! StorageHub RPC client module

use async_trait::async_trait;
use jsonrpsee::core::traits::ToRpcParams;
use serde::de::DeserializeOwned;

pub mod client;
pub mod connection;
pub mod ws_connection;

#[cfg(feature = "mocks")]
pub mod mock_connection;

pub use client::StorageHubRpcClient;
pub use connection::{
    error::{IntoRpcError, RpcConnectionError, RpcResult},
    AnyRpcConnection,
};
#[cfg(feature = "mocks")]
pub use mock_connection::{ErrorMode, MockConnection};
pub use ws_connection::{RpcConfig, WsConnection};

/// Trait for RPC connections
#[async_trait]
pub trait RpcConnection: Send + Sync {
    /// Execute a JSON-RPC method call
    async fn call<P, R>(&self, method: &str, params: P) -> RpcResult<R>
    where
        P: ToRpcParams + Send + Sync,
        R: DeserializeOwned;

    /// Execute a JSON-RPC method call without parameters
    async fn call_no_params<R>(&self, method: &str) -> RpcResult<R>
    where
        R: DeserializeOwned,
    {
        // Default implementation using empty params
        self.call::<_, R>(method, jsonrpsee::rpc_params![]).await
    }

    /// Check if the connection is currently active
    async fn is_connected(&self) -> bool;

    /// Close the connection gracefully
    async fn close(&self) -> RpcResult<()>;
}
