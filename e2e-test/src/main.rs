// Storage Hub E2E Testing CLI

use anyhow::Result;
use clap::{Parser, Subcommand};
use parity_scale_codec::Encode;
use sh_e2e::{
    create_subxt_api,
    node::{Node, NodeConfig},
    TestConfig, DEFAULT_NODE_URL,
};
use std::{fs, time::Duration};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "sh-e2e")]
#[command(about = "Storage Hub E2E Testing Tool")]
struct Cli {
    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: Level,

    /// Node URL to connect to (WebSocket)
    #[arg(long, default_value = DEFAULT_NODE_URL)]
    node_url: String,

    /// Timeout in seconds for operations
    #[arg(long, default_value = "60")]
    timeout: u64,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a node and run tests
    Run {
        /// Tests to run (all if not specified)
        #[arg(long)]
        test: Option<String>,

        /// Don't start a node, connect to existing one
        #[arg(long)]
        no_node: bool,
    },

    /// Start a node for manual testing
    StartNode {
        /// Wait for the node to be ready before exiting
        #[arg(long)]
        wait_ready: bool,
    },

    /// Download runtime metadata from a node
    MetadataDownload {
        /// Output file path
        #[arg(long, default_value = "metadata.scale")]
        output: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize the tracing subscriber
    let subscriber = FmtSubscriber::builder()
        .with_max_level(cli.log_level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Create test config
    let test_config = TestConfig {
        node_url: cli.node_url.clone(),
        timeout: Duration::from_secs(cli.timeout),
    };

    match cli.command {
        Commands::Run { test, no_node } => {
            // Start node if requested
            let _node = if !no_node {
                info!("Starting local node for testing...");
                let node_config = NodeConfig::default();
                let node = Node::start(node_config)?;

                // Wait for node to be ready
                node.wait_for_ready(&test_config.node_url, test_config.timeout)
                    .await?;

                Some(node)
            } else {
                None
            };

            // Run the specified tests
            if let Some(test_name) = test {
                info!("Running test: {}", test_name);
                // Here we would select specific tests to run
                // This would require a test registry or pattern matching
                info!("Test selection not yet implemented, please run via cargo test");
            } else {
                info!("Running all tests");
                // Here we would run all tests
                info!("Please run tests via cargo test");
            }

            info!("Tests completed");
        }

        Commands::StartNode { wait_ready } => {
            info!("Starting local node...");
            let node_config = NodeConfig::default();
            let node = Node::start(node_config)?;

            if wait_ready {
                info!("Waiting for node to be ready...");
                node.wait_for_ready(&test_config.node_url, test_config.timeout)
                    .await?;
                info!(
                    "Node is ready and accepting connections at {}",
                    test_config.node_url
                );
            }

            info!("Node started. Press Ctrl+C to stop.");

            // Keep the node running until interrupted
            tokio::signal::ctrl_c().await?;
            info!("Shutting down node...");
        }

        Commands::MetadataDownload { output } => {
            info!("Downloading metadata from {}", test_config.node_url);

            // Create a client connection to the node
            let client = create_subxt_api().await?;

            // Fetch the metadata and convert to bytes
            let metadata = client.metadata();
            let metadata_bytes = metadata.encode();

            // Save to file
            fs::write(&output, &metadata_bytes)?;
            info!(
                "Metadata saved to {} ({} bytes)",
                output,
                metadata_bytes.len()
            );

            // Display additional information
            info!("This file can be used for offline code generation with #[subxt::subxt(runtime_metadata_path = \"path/to/{}\")]", output);
        }
    }

    Ok(())
}
