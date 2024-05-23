use std::{
    str::FromStr,
    time::{Duration, Instant},
};

use anyhow::anyhow;
use log::*;
use sp_core::H256;
use storage_hub_infra::actor::ActorHandle;
use tokio::sync::mpsc::Receiver;

use crate::services::blockchain::{commands::BlockchainServiceInterface, types::ExtrinsicResult};

use super::{types::ExtrinsicHash, BlockchainService};

const LOG_TARGET: &str = "blockchain-transaction";

/// A struct that handles the lifecycle of a submitted transaction.
///
/// It holds a `watcher` that is used to query the state of the transaction from
/// the blockchain node, a `hash` that is used to identify the transaction, and an
/// optional `timeout` that specifies the maximum amount of time to wait for the
/// transaction to either be successful or fail.
#[derive(Debug)]
pub struct SubmittedTransaction {
    /// The watcher used to query the state of the transaction from the blockchain node.
    watcher: Receiver<String>,
    /// The hash of the transaction.
    hash: ExtrinsicHash,
    /// The maximum amount of time to wait for the transaction to either be successful or fail.
    timeout: Option<Duration>,
}

const NO_TIMEOUT_WARN: Duration = Duration::from_secs(60);

impl SubmittedTransaction {
    pub fn new(watcher: Receiver<String>, hash: H256) -> Self {
        Self {
            watcher,
            hash,
            timeout: None,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub async fn watch_for_success(
        &mut self,
        blockchain: &ActorHandle<BlockchainService>,
    ) -> anyhow::Result<()> {
        // Wait for the transaction to be included in a block.
        let block_hash;

        let start_time = Instant::now();
        loop {
            // Get the elapsed time since submit.
            let elapsed = start_time.elapsed();
            // Calculate the remaining time to wait.
            let remaining = match self.timeout {
                Some(timeout) => {
                    // Check if the timeout has been reached.
                    if elapsed > timeout {
                        return Err(anyhow!(
                            "Timeout waiting for transaction to be included in a block"
                        ));
                    }

                    timeout - elapsed
                }
                None => NO_TIMEOUT_WARN,
            };

            // Wait for either a new message from the watcher, or the timeout to be reached.
            let result = match tokio::time::timeout(remaining, self.watcher.recv()).await {
                Ok(result) => match result {
                    Some(result) => result,
                    None => {
                        return Err(anyhow!("Transaction watcher channel closed"));
                    }
                },
                Err(_) => {
                    // Timeout reached, exit the loop.
                    match self.timeout {
                        Some(_) => {
                            return Err(anyhow!(
                                "Timeout waiting for transaction to be included in a block"
                            ));
                        }
                        None => {
                            // No timeout set, continue waiting.
                            warn!(target: LOG_TARGET, "No timeout set and {:?} elapsed, continuing to wait for transaction to be included in a block.", NO_TIMEOUT_WARN);

                            continue;
                        }
                    }
                }
            };
            // Parse the JSONRPC string. The strings sent by the RPC wacher should be valid JSONRPC strings.
            let json: serde_json::Value = serde_json::from_str(&result).map_err(|_| {
                anyhow!("The result, if not an error, can only be a JSONRPC string; qed")
            })?;

            debug!(target: LOG_TARGET, "Transaction information: {:?}", json);

            // Checking if the transaction is included in a block.
            // TODO: Consider if we might want to wait for "finalized".
            // TODO: Handle other lifetime extrinsic edge cases. See https://github.com/paritytech/polkadot-sdk/blob/master/substrate/client/transaction-pool/api/src/lib.rs#L131
            if let Some(in_block) = json["params"]["result"]["inBlock"].as_str() {
                block_hash = Some(H256::from_str(in_block)?);
                let subscription_id = json["params"]["subscription"]
                    .as_number()
                    .ok_or_else(|| anyhow!("Subscription should exist and be a number; qed"))?;

                // Unwatch extrinsic to release tx_watcher.
                blockchain
                    .unwatch_extrinsic(subscription_id.to_owned())
                    .await?;

                // Breaking while loop.
                // Even though we unwatch the transaction, and the loop should break, we still break manually
                // in case we continue to receive updates. This should not happen, but it is a safety measure,
                // and we already have what we need.
                break;
            }
        }

        // Get the extrinsic from the block, with its events.
        let block_hash = block_hash.ok_or_else(
            || anyhow!("Block hash should exist after waiting for extrinsic to be included in a block; qed")
        )?;
        let extrinsic_in_block = blockchain
            .get_extrinsic_from_block(block_hash, self.hash)
            .await?;

        // Check if the extrinsic was successful.
        let extrinsic_successful = ActorHandle::<BlockchainService>::extrinsic_result(extrinsic_in_block.clone())
            .map_err(|_| anyhow!("Extrinsic does not contain an ExtrinsicFailed nor ExtrinsicSuccess event, which is not possible; qed"))?;
        match extrinsic_successful {
            ExtrinsicResult::Success { dispatch_info } => {
                info!(target: LOG_TARGET, "Extrinsic successful with dispatch info: {:?}", dispatch_info);
            }
            ExtrinsicResult::Failure {
                dispatch_error,
                dispatch_info,
            } => {
                error!(target: LOG_TARGET, "Extrinsic failed with dispatch error: {:?}, dispatch info: {:?}", dispatch_error, dispatch_info);
                return Err(anyhow::anyhow!("Extrinsic failed"));
            }
        }

        info!(target: LOG_TARGET, "Events in extrinsic: {:?}", &extrinsic_in_block.events);

        Ok(())
    }
}
