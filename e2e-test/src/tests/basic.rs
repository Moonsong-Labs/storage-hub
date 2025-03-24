// Basic connectivity and functionality tests
use crate::{create_client, utils::wait_for_blocks, TestConfig, DEFAULT_TIMEOUT};
use anyhow::Result;
use subxt_signer::sr25519::dev;
use tracing::info;

// Generate a minimal interface for testing basic functionality
#[subxt::subxt(runtime_metadata_path = "./storage-hub.scale")]
pub mod storage_hub {}

/// Test that we can connect to a running node
#[tokio::test]
async fn test_can_connect() -> Result<()> {
    let config = TestConfig::default();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // This will fail if we can't connect
    let client = create_client(&config).await?;
    info!("Successfully connected to node at {}", config.node_url);

    // Get chain info (updated to current API)
    let chain_info = client.rpc().chain_type().await?;
    info!("Chain type: {:?}", chain_info);

    Ok(())
}

/// Test that the chain is producing blocks
#[tokio::test]
async fn test_block_production() -> Result<()> {
    let config = TestConfig::default();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    let client = create_client(&config).await?;

    // Wait for 3 blocks
    let block = wait_for_blocks(&client, 3, DEFAULT_TIMEOUT).await?;

    info!("Observed 3 blocks, latest: {}", block.hash());

    Ok(())
}

/// Test a basic balance transfer
#[tokio::test]
async fn test_balance_transfer() -> Result<()> {
    let config = TestConfig::default();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    let client = create_client(&config).await?;

    // Alice will send funds to Bob
    let signer = dev::alice();
    let bob = dev::bob().public_key().into();

    // Create a balance transfer extrinsic
    let tx = storage_hub::tx().balances().transfer_allow_death(
        bob,
        1_000_000_000_000, // 1 token with 12 decimals
    );

    // Sign and submit the transfer, then wait for finalization
    let events = client
        .tx()
        .sign_and_submit_then_watch_default(&tx, &signer)
        .await?
        .wait_for_finalized_success()
        .await?;

    info!("Transaction finalized in block: {}", events.block_hash());

    // Verify transfer event exists
    let transfer_event = events.find_first::<storage_hub::balances::events::Transfer>()?;

    if let Some(event) = transfer_event {
        info!(
            "Balance transfer successful: {} transferred from {} to {}",
            event.amount, event.from, event.to
        );
    } else {
        anyhow::bail!("Transfer event not found");
    }

    Ok(())
}
