// Node management functionality

use anyhow::Result;
use std::process::{Child, Command};
use std::time::{Duration, Instant};
use subxt::{OnlineClient, PolkadotConfig};
use tracing::{info, warn};

/// Configuration for running a local node
#[derive(Debug)]
pub struct NodeConfig {
    /// The command to run
    pub binary_path: String,
    /// Arguments to pass to the node
    pub args: Vec<String>,
    /// Time to wait for node startup
    pub startup_wait: Duration,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            binary_path: "cargo".to_string(),
            args: vec![
                "run".to_string(),
                "--release".to_string(),
                "--bin".to_string(),
                "storage-hub-node".to_string(),
                "--".to_string(),
                "--dev".to_string(),
                "--tmp".to_string(),
            ],
            startup_wait: Duration::from_secs(10),
        }
    }
}

/// Represents a running Storage Hub node
pub struct Node {
    process: Child,
    #[allow(dead_code)]
    pub config: NodeConfig,
}

impl Node {
    /// Start a new node with the given configuration
    pub fn start(config: NodeConfig) -> Result<Self> {
        info!("Starting storage hub node...");
        let process = Command::new(&config.binary_path)
            .args(&config.args)
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to start node: {}", e))?;

        // Give the node time to start up
        std::thread::sleep(config.startup_wait);

        info!("Node started with PID: {}", process.id());

        Ok(Self { process, config })
    }

    /// Wait for the node to be ready by attempting to connect
    pub async fn wait_for_ready(&self, url: &str, timeout: Duration) -> Result<()> {
        let start_time = Instant::now();

        info!("Using config: {:?}", self.config);

        loop {
            if start_time.elapsed() > timeout {
                return Err(anyhow::anyhow!("Timeout waiting for node to be ready"));
            }

            match OnlineClient::<PolkadotConfig>::from_url(url).await {
                Ok(_) => {
                    info!("Node is ready and accepting connections");
                    return Ok(());
                }
                Err(e) => {
                    warn!("Not ready yet, retrying: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        info!("Stopping node...");
        if let Err(e) = self.process.kill() {
            warn!("Failed to kill node process: {}", e);
        }
    }
}
