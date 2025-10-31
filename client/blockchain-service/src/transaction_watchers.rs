//! Transaction watcher module for monitoring transaction lifecycle events.
//!
//! This module provides functionality to watch transactions submitted to the blockchain
//! and track their lifecycle through various states (Future, Ready, InBlock, Finalized, etc.).
//!
//! ## Usage
//!
//! Watchers are spawned automatically by `send_extrinsic()` in the BlockchainService.
//! They monitor transactions via RPC subscriptions and send status updates to the
//! transaction pool through an unbounded channel.
//!
//! ## Transaction Lifecycle
//!
//! Watchers track transactions through these states:
//! - **Future**: Transaction nonce is ahead, waiting in the future queue
//! - **Ready**: Transaction is ready for inclusion in a block
//! - **Broadcast**: Transaction has been propagated to peers
//! - **InBlock**: Transaction included in a block (NOT final - can be retracted)
//! - **Retracted**: Block containing tx was reverted (tx stays in pool)
//! - **Finalized**: Transaction was finalized by consensus (terminal success)
//! - **Invalid**: Transaction is no longer valid (terminal failure, retriable)
//! - **Dropped**: Transaction was removed due to pool limits (terminal failure, retriable)
//! - **Usurped**: Transaction was replaced by another with same nonce (terminal)
//! - **FinalityTimeout**: Finality unwatched after 512 blocks (terminal)

use codec::Decode;
use log::{debug, error, info, warn};
use sc_transaction_pool_api::TransactionStatus;
use shc_common::traits::StorageEnableRuntime;

const LOG_TARGET: &str = "blockchain-transaction-watcher";

/// Watch and log a transaction's lifecycle.
///
/// This spawns a background task that monitors the transaction status via RPC subscription
/// (`author_submitAndWatchExtrinsic`), logs all state changes, and sends TransactionStatus
/// updates to the transaction pool via the status channel.
///
/// The watcher will run until the transaction reaches a terminal state or the receiver channel closes.
pub fn spawn_transaction_watcher<Runtime>(
    nonce: u32,
    tx_hash: Runtime::Hash,
    mut receiver: tokio::sync::mpsc::Receiver<String>,
    status_tx: tokio::sync::mpsc::UnboundedSender<(
        u32,
        Runtime::Hash,
        TransactionStatus<Runtime::Hash, Runtime::Hash>,
    )>,
) where
    Runtime: StorageEnableRuntime,
{
    tokio::spawn(async move {
        info!(
            target: LOG_TARGET,
            "📡 Watching transaction with nonce {} (hash: {:?})",
            nonce,
            tx_hash
        );

        while let Some(status_update) = receiver.recv().await {
            match serde_json::from_str::<serde_json::Value>(&status_update) {
                Ok(json) => {
                    if let Some(params) = json.get("params") {
                        if let Some(result) = params.get("result") {
                            // Handle all TransactionStatus variants according to the API
                            if result.as_str() == Some("future") {
                                warn!(
                                    target: LOG_TARGET,
                                    "⏭ Transaction with nonce {} is future",
                                    nonce
                                );
                                let _ = status_tx.send((nonce, tx_hash, TransactionStatus::Future));
                            } else if result.as_str() == Some("ready") {
                                debug!(
                                    target: LOG_TARGET,
                                    "✓ Transaction with nonce {} is ready (in transaction pool)",
                                    nonce
                                );
                                let _ = status_tx.send((nonce, tx_hash, TransactionStatus::Ready));
                            } else if let Some(broadcast) = result.get("broadcast") {
                                // Parse peer IDs from the broadcast array
                                let peer_ids: Vec<String> = broadcast
                                    .as_array()
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|v| v.as_str().map(String::from))
                                            .collect()
                                    })
                                    .unwrap_or_default();

                                debug!(
                                    target: LOG_TARGET,
                                    "📡 Transaction with nonce {} was broadcast to {} peers",
                                    nonce,
                                    peer_ids.len()
                                );
                                let _ =
                                    status_tx.send((nonce, tx_hash, TransactionStatus::Broadcast(peer_ids)));
                            } else if let Some(block_hash_json) = result.get("inBlock") {
                                let block_hash =
                                    parse_block_hash_from_json::<Runtime>(block_hash_json);
                                info!(
                                    target: LOG_TARGET,
                                    "✓ Transaction with nonce {} was included in block: {:?}",
                                    nonce,
                                    block_hash
                                );
                                // Note: TxIndex is not present in the RPC JSON response, and since the pool doesn't need it
                                // for state tracking, we use 0 as a placeholder
                                let _ = status_tx
                                    .send((nonce, tx_hash, TransactionStatus::InBlock((block_hash, 0))));
                            } else if let Some(block_hash_json) = result.get("retracted") {
                                let block_hash =
                                    parse_block_hash_from_json::<Runtime>(block_hash_json);
                                warn!(
                                    target: LOG_TARGET,
                                    "🔄 Transaction with nonce {} was retracted from block: {:?}. Block was reverted in reorg. \
                                    Transaction stays in pool and may be included in another block.",
                                    nonce,
                                    block_hash
                                );
                                let _ = status_tx
                                    .send((nonce, tx_hash, TransactionStatus::Retracted(block_hash)));
                            } else if let Some(block_hash_json) = result.get("finalized") {
                                let block_hash =
                                    parse_block_hash_from_json::<Runtime>(block_hash_json);
                                info!(
                                    target: LOG_TARGET,
                                    "✓ Transaction with nonce {} was finalized in block: {:?}",
                                    nonce,
                                    block_hash
                                );
                                // Note: TxIndex is not present in the RPC JSON response, and since the pool doesn't need it
                                // for state tracking, we use 0 as a placeholder
                                let _ = status_tx
                                    .send((nonce, tx_hash, TransactionStatus::Finalized((block_hash, 0))));
                                // Finalized is a terminal state, stop watching
                                break;
                            } else if let Some(block_hash_json) = result.get("finalityTimeout") {
                                let block_hash =
                                    parse_block_hash_from_json::<Runtime>(block_hash_json);
                                warn!(
                                    target: LOG_TARGET,
                                    "⏱️ Transaction with nonce {} had finality timeout after 512 blocks in block: {:?}",
                                    nonce,
                                    block_hash
                                );
                                let _ = status_tx
                                    .send((nonce, tx_hash, TransactionStatus::FinalityTimeout(block_hash)));
                                // FinalityTimeout is a terminal state, stop watching
                                break;
                            } else if result.as_str() == Some("invalid") {
                                error!(
                                    target: LOG_TARGET,
                                    "✗ Transaction with nonce {} is invalid (hash: {:?})",
                                    nonce,
                                    tx_hash
                                );
                                let _ = status_tx.send((nonce, tx_hash, TransactionStatus::Invalid));
                                // Invalid is a terminal state, stop watching
                                break;
                            } else if let Some(usurped_by_json) = result.get("usurped") {
                                let usurped_by_hash =
                                    parse_tx_hash_from_json::<Runtime>(usurped_by_json);
                                warn!(
                                    target: LOG_TARGET,
                                    "⚠ Transaction with nonce {} (hash: {:?}) was usurped by transaction {:?}",
                                    nonce,
                                    tx_hash,
                                    usurped_by_hash
                                );
                                let _ = status_tx
                                    .send((nonce, tx_hash, TransactionStatus::Usurped(usurped_by_hash)));
                                // Usurped is a terminal state, stop watching
                                break;
                            } else if result.as_str() == Some("dropped") {
                                warn!(
                                    target: LOG_TARGET,
                                    "⚠ Transaction with nonce {} was dropped (hash: {:?})",
                                    nonce,
                                    tx_hash
                                );
                                let _ = status_tx.send((nonce, tx_hash, TransactionStatus::Dropped));
                                // Dropped is a terminal state, stop watching
                                break;
                            } else {
                                debug!(
                                    target: LOG_TARGET,
                                    "Transaction with nonce {} status update: {:?}",
                                    nonce,
                                    result
                                );
                            }
                        }
                    } else if let Some(error) = json.get("error") {
                        error!(
                            target: LOG_TARGET,
                            "✗ Transaction with nonce {} error: {:?}",
                            nonce,
                            error
                        );
                        break;
                    }
                }
                Err(e) => {
                    warn!(
                        target: LOG_TARGET,
                        "Failed to parse transaction status for nonce {}: {:?}",
                        nonce,
                        e
                    );
                }
            }
        }

        debug!(
            target: LOG_TARGET,
            "📡 Stopped watching transaction with nonce {}",
            nonce
        );
    });
}

/// Parse a block hash from a JSON value containing a hex-encoded hash string.
///
/// Returns `Default::default()` if parsing fails (hex decoding error or wrong length).
fn parse_block_hash_from_json<Runtime>(json_value: &serde_json::Value) -> Runtime::Hash
where
    Runtime: StorageEnableRuntime,
{
    json_value
        .as_str()
        .and_then(|hex_str| {
            // Remove 0x prefix if present
            let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
            // Decode hex to bytes
            array_bytes::hex2bytes(hex_str).ok()
        })
        .and_then(|bytes| {
            // Try to decode as Runtime::Hash
            Decode::decode(&mut &bytes[..]).ok()
        })
        .unwrap_or_default()
}

/// Parse a transaction hash from a JSON value containing a hex-encoded hash string.
///
/// Returns `Default::default()` if parsing fails.
fn parse_tx_hash_from_json<Runtime>(json_value: &serde_json::Value) -> Runtime::Hash
where
    Runtime: StorageEnableRuntime,
{
    // Same implementation as block hash since they're both Runtime::Hash
    parse_block_hash_from_json::<Runtime>(json_value)
}
