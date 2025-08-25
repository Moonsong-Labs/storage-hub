//! RPC connection abstraction for StorageHub

use std::fmt::Debug;

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

#[cfg(feature = "mocks")]
use crate::data::rpc::mock_connection::MockConnection;
use crate::data::rpc::{ws_connection::WsConnection, RpcConnection};

pub mod error;

use error::RpcResult;

/// Enum wrapper for different RPC connection implementations
///
/// This enum allows using concrete types instead of trait objects,
/// solving trait object safety issues while maintaining flexibility
/// between real and mock connections.
pub enum AnyRpcConnection {
    /// Real WebSocket connection
    Real(WsConnection),

    /// Mock connection for testing
    #[cfg(feature = "mocks")]
    Mock(MockConnection),
}

impl Debug for AnyRpcConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnyRpcConnection::Real(_) => write!(f, "AnyRpcConnection::Real(WsConnection)"),
            #[cfg(feature = "mocks")]
            AnyRpcConnection::Mock(_) => write!(f, "AnyRpcConnection::Mock(MockConnection)"),
        }
    }
}

#[async_trait]
impl RpcConnection for AnyRpcConnection {
    async fn call<P, R>(&self, method: &str, params: P) -> RpcResult<R>
    where
        P: Serialize + Send + Sync,
        R: DeserializeOwned,
    {
        match self {
            AnyRpcConnection::Real(conn) => conn.call(method, params).await,
            #[cfg(feature = "mocks")]
            AnyRpcConnection::Mock(conn) => conn.call(method, params).await,
        }
    }

    async fn call_no_params<R>(&self, method: &str) -> RpcResult<R>
    where
        R: DeserializeOwned,
    {
        match self {
            AnyRpcConnection::Real(conn) => conn.call_no_params(method).await,
            #[cfg(feature = "mocks")]
            AnyRpcConnection::Mock(conn) => conn.call_no_params(method).await,
        }
    }

    async fn is_connected(&self) -> bool {
        match self {
            AnyRpcConnection::Real(conn) => conn.is_connected().await,
            #[cfg(feature = "mocks")]
            AnyRpcConnection::Mock(conn) => conn.is_connected().await,
        }
    }

    async fn close(&self) -> RpcResult<()> {
        match self {
            AnyRpcConnection::Real(conn) => conn.close().await,
            #[cfg(feature = "mocks")]
            AnyRpcConnection::Mock(conn) => conn.close().await,
        }
    }
}
