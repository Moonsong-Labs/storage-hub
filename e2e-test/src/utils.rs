// Utility functions for E2E testing

use anyhow::Result;
use futures::StreamExt;
use std::time::{Duration, Instant};
use subxt::{blocks::Block, events::EventDetails, OnlineClient, PolkadotConfig};
use tracing::info;

/// Wait for a specific number of blocks to be produced
pub async fn wait_for_blocks(
    client: &OnlineClient<PolkadotConfig>,
    count: u32,
    timeout: Duration,
) -> Result<Block<PolkadotConfig, OnlineClient<PolkadotConfig>>> {
    let start = Instant::now();
    let mut block_stream = client.blocks().subscribe_finalized().await?;

    let mut blocks_seen = 0;
    let mut last_block = None;

    while let Some(block_result) = block_stream.next().await {
        if start.elapsed() > timeout {
            return Err(anyhow::anyhow!("Timeout waiting for blocks"));
        }

        let block = block_result?;

        // Log block info before moving the block
        let block_number = block.header().number;
        let block_hash = block.hash();
        info!("Block #{} with hash {}", block_number, block_hash);

        // Now we can safely move the block
        last_block = Some(block);
        blocks_seen += 1;

        if blocks_seen >= count {
            break;
        }
    }

    Ok(last_block.ok_or_else(|| anyhow::anyhow!("No blocks received"))?)
}

/// Wait for a specific event to occur in the blockchain
pub async fn wait_for_event<F, R>(
    client: &OnlineClient<PolkadotConfig>,
    predicate: F,
    timeout: Duration,
) -> Result<R>
where
    F: Fn(&EventDetails<PolkadotConfig>) -> Option<R>,
{
    let start = Instant::now();
    let mut block_stream = client.blocks().subscribe_finalized().await?;

    while let Some(block_result) = block_stream.next().await {
        if start.elapsed() > timeout {
            return Err(anyhow::anyhow!("Timeout waiting for event"));
        }

        let block = block_result?;
        info!(
            "Checking block #{} with hash {}",
            block.header().number,
            block.hash()
        );

        let events = block.events().await?;

        for event_result in events.iter() {
            let event = event_result?;
            if let Some(result) = predicate(&event) {
                return Ok(result);
            }
        }
    }

    Err(anyhow::anyhow!("Stream ended without finding the event"))
}

/// Utility for tracking balances before and after operations
pub struct BalanceTracker {
    pub address: String,
    pub initial_balance: Option<u128>,
}

impl BalanceTracker {
    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
            initial_balance: None,
        }
    }

    // Additional methods for balance tracking can be added here
}
