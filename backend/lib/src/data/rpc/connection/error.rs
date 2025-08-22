use thiserror::Error;

/// Error type for RPC operations
#[derive(Debug, Error)]
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

/// Trait for types that can be converted to RPC errors
pub trait IntoRpcError {
    /// Convert this error into an `RpcConnectionError`
    fn into_rpc_error(self) -> RpcConnectionError;
}

impl IntoRpcError for jsonrpsee::core::client::Error {
    fn into_rpc_error(self) -> RpcConnectionError {
        match self {
            Self::Call(e) => RpcConnectionError::Rpc(e.to_string()),
            Self::Transport(e) => RpcConnectionError::Transport(e.to_string()),
            Self::RestartNeeded(_) => RpcConnectionError::ConnectionClosed,
            Self::ParseError(e) => RpcConnectionError::Serialization(e.to_string()),
            Self::InvalidSubscriptionId => {
                RpcConnectionError::Rpc("Invalid subscription ID".to_string())
            }
            Self::InvalidRequestId(e) => {
                RpcConnectionError::Rpc(format!("Invalid request ID: {}", e))
            }
            Self::RequestTimeout => RpcConnectionError::Timeout,
            Self::HttpNotImplemented => {
                RpcConnectionError::Other("HTTP not implemented".to_string())
            }
            Self::EmptyBatchRequest(_) => {
                RpcConnectionError::Rpc("Empty batch request".to_string())
            }
            Self::RegisterMethod(e) => {
                RpcConnectionError::Rpc(format!("Failed to register method: {}", e))
            }
            other => RpcConnectionError::Other(other.to_string()),
        }
    }
}
