//! WebSocket RPC connection implementation
//!
//! This module provides a WebSocket-based RPC connection implementation
//! using jsonrpsee for communication with StorageHub nodes.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use jsonrpsee::{
    core::client::ClientT,
    ws_client::{WsClient, WsClientBuilder},
};
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::RwLock;

use super::{
    connection::{IntoRpcError, RpcConfig, RpcConnectionError, RpcResult},
    RpcConnection,
};

/// WebSocket RPC connection implementation
pub struct WsConnection {
    /// The underlying jsonrpsee WebSocket client wrapped in Arc for sharing
    client: Arc<RwLock<Option<Arc<WsClient>>>>,
    /// Configuration for the connection
    config: RpcConfig,
}

impl std::fmt::Debug for WsConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WsConnection")
            .field("config", &self.config)
            .finish()
    }
}

impl WsConnection {
    /// Create a new WebSocket connection
    pub async fn new(config: RpcConfig) -> RpcResult<Self> {
        let client = Self::build_client(&config).await?;
        Ok(Self {
            client: Arc::new(RwLock::new(Some(Arc::new(client)))),
            config,
        })
    }

    /// Build a new WebSocket client with the given configuration
    async fn build_client(config: &RpcConfig) -> RpcResult<WsClient> {
        let mut builder = WsClientBuilder::default();

        // Configure request timeout
        if let Some(timeout_secs) = config.timeout_secs {
            builder = builder.request_timeout(Duration::from_secs(timeout_secs));
        }

        // Configure max concurrent requests
        if let Some(max_concurrent) = config.max_concurrent_requests {
            builder = builder.max_concurrent_requests(max_concurrent);
        }

        // Build and connect the client
        builder.build(&config.url).await.map_err(|e| {
            RpcConnectionError::Transport(format!("Failed to connect to {}: {}", config.url, e))
        })
    }

    /// Attempt to reconnect if the connection is closed
    async fn ensure_connected(&self) -> RpcResult<()> {
        let mut client_guard = self.client.write().await;

        // Check if we need to reconnect
        if client_guard.is_none() {
            // Attempt to reconnect
            let new_client = Self::build_client(&self.config).await?;
            *client_guard = Some(Arc::new(new_client));
        }

        Ok(())
    }

    /// Get a reference to the client, ensuring it's connected
    async fn get_client(&self) -> RpcResult<Arc<WsClient>> {
        self.ensure_connected().await?;

        let client_guard = self.client.read().await;
        client_guard
            .as_ref()
            .cloned()
            .ok_or(RpcConnectionError::ConnectionClosed)
    }
}

#[async_trait]
impl RpcConnection for WsConnection {
    async fn call<P, R>(&self, method: &str, params: P) -> RpcResult<R>
    where
        P: Serialize + Send + Sync,
        R: DeserializeOwned,
    {
        let client = self.get_client().await?;

        // Use rpc_params! macro to properly format parameters
        let result = client
            .request(method, jsonrpsee::rpc_params![params])
            .await
            .map_err(|e| e.into_rpc_error())?;

        Ok(result)
    }

    async fn is_connected(&self) -> bool {
        let client_guard = self.client.read().await;

        if let Some(client) = client_guard.as_ref() {
            // Try a simple ping-like operation to check connection health
            // We'll use system_health as it's a common RPC method
            // Use rpc_params! macro for empty params
            client
                .request::<serde_json::Value, _>("system_health", jsonrpsee::rpc_params![])
                .await
                .is_ok()
        } else {
            false
        }
    }

    async fn close(&self) -> RpcResult<()> {
        let mut client_guard = self.client.write().await;

        // Drop the client to close the connection
        if let Some(_client) = client_guard.take() {
            // Client is dropped here, closing the connection
        }

        Ok(())
    }
}

/// Builder for WebSocket RPC connections
pub struct WsConnectionBuilder {
    config: RpcConfig,
}

impl WsConnectionBuilder {
    /// Create a new WebSocket connection builder
    pub fn new(url: impl Into<String>) -> Self {
        let config = RpcConfig {
            url: url.into(),
            ..Default::default()
        };
        Self { config }
    }

    /// Set the request timeout
    pub fn timeout_secs(mut self, timeout: u64) -> Self {
        self.config.timeout_secs = Some(timeout);
        self
    }

    /// Set the maximum number of concurrent requests
    pub fn max_concurrent_requests(mut self, max: usize) -> Self {
        self.config.max_concurrent_requests = Some(max);
        self
    }

    /// Set whether to verify TLS certificates
    pub fn verify_tls(mut self, verify: bool) -> Self {
        self.config.verify_tls = verify;
        self
    }
}

impl WsConnectionBuilder {
    /// Build and establish the WebSocket connection
    pub async fn build(self) -> RpcResult<WsConnection> {
        WsConnection::new(self.config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::test::{MAX_CONCURRENT_REQUESTS, RPC_TIMEOUT_SECS};

    #[test]
    fn test_ws_connection_builder() {
        let builder = WsConnectionBuilder::new("ws://localhost:9944")
            .timeout_secs(RPC_TIMEOUT_SECS)
            .max_concurrent_requests(MAX_CONCURRENT_REQUESTS)
            .verify_tls(false);

        assert_eq!(builder.config.url, "ws://localhost:9944");
        assert_eq!(builder.config.timeout_secs, Some(RPC_TIMEOUT_SECS));
        assert_eq!(
            builder.config.max_concurrent_requests,
            Some(MAX_CONCURRENT_REQUESTS)
        );
        assert!(!builder.config.verify_tls);
    }
}
