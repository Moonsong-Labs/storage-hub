//! WebSocket RPC connection implementation
//!
//! This module provides a WebSocket-based RPC connection implementation
//! using jsonrpsee for communication with StorageHub nodes.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use jsonrpsee::core::traits::ToRpcParams;
use jsonrpsee::{
    core::{client::ClientT, traits::ToRpcParams},
    ws_client::{WsClient, WsClientBuilder},
};
use serde::de::DeserializeOwned;
use tokio::sync::RwLock;

use crate::data::rpc::{
    connection::error::{IntoRpcError, RpcConnectionError, RpcResult},
    RpcConnection,
};

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

    /// Get a reference to the client, ensuring it's connected
    async fn get_client(&self) -> RpcResult<Arc<WsClient>> {
        let mut client_guard = self.client.write().await;
        match client_guard.as_ref() {
            None => {
                let new_client = Self::build_client(&self.config).await?;
                let new_client = Arc::new(new_client);
                *client_guard = Some(Arc::clone(&new_client));
                Ok(new_client)
            }
            Some(client) => Ok(Arc::clone(client)),
        }
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

        let result = client
            .request(method, params)
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
        self.client.write().await.take();

        Ok(())
    }
}

// TODO: add some tests here for the RPC using testcontainers
// to ensure our RPC connection actually behaves like we expect it to
