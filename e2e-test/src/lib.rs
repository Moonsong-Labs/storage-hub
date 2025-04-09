// Storage Hub E2E Testing Framework
//! This crate provides end-to-end testing utilities for Storage Hub.

use anyhow::Result;
use std::time::Duration;
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::sr25519::{dev, Keypair};

pub mod node;
pub mod tests;
pub mod utils;

/// Default timeout for operations
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// Default node URL for local testing
pub const DEFAULT_NODE_URL: &str = "ws://127.0.0.1:9944";

/// Configuration for E2E tests
pub struct TestConfig {
    /// Node URL to connect to
    pub node_url: String,
    /// Timeout for operations
    pub timeout: Duration,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            node_url: DEFAULT_NODE_URL.to_string(),
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

/// Create a new subxt client connected to the specified node
pub async fn create_subxt_api() -> Result<OnlineClient<PolkadotConfig>> {
    let config = TestConfig::default();
    let client = OnlineClient::<PolkadotConfig>::from_url(&config.node_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to node: {}", e))?;

    Ok(client)
}

/// Get a keypair from a seed or use a predefined one
pub fn get_keypair(seed: Option<&str>) -> Result<Keypair> {
    if let Some(seed_str) = seed {
        // Convert seed string to secret key
        let seed_bytes = seed_str.as_bytes();
        let secret = sp_core::blake2_256(seed_bytes);

        // Use from_secret_key with the 32-byte array
        Keypair::from_secret_key(secret)
            .map_err(|e| anyhow::anyhow!("Failed to create keypair from seed: {}", e))
    } else {
        // Use a development account - simplest approach
        Ok(dev::alice())
    }
}
