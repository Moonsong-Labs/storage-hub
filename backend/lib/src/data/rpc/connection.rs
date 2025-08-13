//! RPC connection abstraction for StorageHub

use std::fmt::Debug;

use async_trait::async_trait;
use jsonrpsee::core::client::Error;
use serde::{de::DeserializeOwned, Serialize};

use crate::constants::rpc::{DEFAULT_MAX_CONCURRENT_REQUESTS, DEFAULT_TIMEOUT_SECS};
#[cfg(feature = "mocks")]
use super::mock_connection::MockConnection;
use super::{ws_connection::WsConnection, RpcConnection};

/// Error type for RPC operations
#[derive(Debug, thiserror::Error)]
pub enum RpcConnectionError {
    /// Network or transport-related errors
    #[error("Transport error: {0}")]
    Transport(String),

    /// JSON-RPC protocol errors
    #[error("RPC error: {0}")]
    Rpc(String),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Request timeout errors
    #[error("Request timeout")]
    Timeout,

    /// Connection closed or unavailable
    #[error("Connection closed")]
    ConnectionClosed,

    /// Other errors
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for RPC operations
pub type RpcResult<T> = Result<T, RpcConnectionError>;

/// Configuration for RPC connections
#[derive(Debug, Clone)]
pub struct RpcConfig {
    /// The RPC endpoint URL
    pub url: String,

    /// Request timeout in seconds
    pub timeout_secs: Option<u64>,

    /// Maximum number of concurrent requests
    pub max_concurrent_requests: Option<usize>,

    /// Whether to verify TLS certificates (for HTTPS/WSS)
    pub verify_tls: bool,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            timeout_secs: Some(DEFAULT_TIMEOUT_SECS),
            max_concurrent_requests: Some(DEFAULT_MAX_CONCURRENT_REQUESTS),
            verify_tls: true,
        }
    }
}

/// Trait for types that can be converted to RPC errors
pub trait IntoRpcError {
    /// Convert this error into an `RpcConnectionError`
    fn into_rpc_error(self) -> RpcConnectionError;
}

impl IntoRpcError for jsonrpsee::core::client::Error {
    fn into_rpc_error(self) -> RpcConnectionError {
        match self {
            Error::Call(e) => RpcConnectionError::Rpc(e.to_string()),
            Error::Transport(e) => RpcConnectionError::Transport(e.to_string()),
            Error::RestartNeeded(_) => RpcConnectionError::ConnectionClosed,
            Error::ParseError(e) => RpcConnectionError::Serialization(e.to_string()),
            Error::InvalidSubscriptionId => {
                RpcConnectionError::Rpc("Invalid subscription ID".to_string())
            }
            Error::InvalidRequestId(e) => {
                RpcConnectionError::Rpc(format!("Invalid request ID: {}", e))
            }
            Error::RequestTimeout => RpcConnectionError::Timeout,
            Error::HttpNotImplemented => {
                RpcConnectionError::Other("HTTP not implemented".to_string())
            }
            Error::EmptyBatchRequest(_) => {
                RpcConnectionError::Rpc("Empty batch request".to_string())
            }
            Error::RegisterMethod(e) => {
                RpcConnectionError::Rpc(format!("Failed to register method: {}", e))
            }
            other => RpcConnectionError::Other(other.to_string()),
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_config_default() {
        let config = RpcConfig::default();
        assert_eq!(config.url, "");
        assert_eq!(config.timeout_secs, Some(DEFAULT_TIMEOUT_SECS));
        assert_eq!(
            config.max_concurrent_requests,
            Some(DEFAULT_MAX_CONCURRENT_REQUESTS)
        );
        assert!(config.verify_tls);
    }

    #[test]
    fn test_rpc_connection_error_display() {
        let errors = vec![
            RpcConnectionError::Transport("Network error".to_string()),
            RpcConnectionError::Rpc("Method not found".to_string()),
            RpcConnectionError::Serialization("Invalid JSON".to_string()),
            RpcConnectionError::Timeout,
            RpcConnectionError::ConnectionClosed,
            RpcConnectionError::Other("Unknown error".to_string()),
        ];

        for error in errors {
            // Just ensure Display is implemented
            let _ = format!("{}", error);
        }
    }
}
