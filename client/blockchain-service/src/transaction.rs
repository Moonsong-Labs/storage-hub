use std::{
    str::FromStr,
    time::{Duration, Instant},
};

use log::{debug, error, info};
use shc_actors_framework::actor::ActorHandle;
use shc_common::traits::StorageEnableRuntime;
use shc_common::types::StorageHubEventsVec;
use shc_forest_manager::traits::ForestStorageHandler;
use tokio::sync::mpsc::Receiver;

use crate::{
    commands::{BlockchainServiceCommandInterface, BlockchainServiceCommandInterfaceExt},
    types::{Extrinsic, ExtrinsicResult, WatchTransactionError},
    BlockchainService,
};

const LOG_TARGET: &str = "blockchain-transaction";

/// A struct that handles the lifecycle of a submitted transaction.
///
/// It holds a `watcher` that is used to query the state of the transaction from
/// the blockchain node, a `hash` that is used to identify the transaction, and an
/// optional `timeout` that specifies the maximum amount of time to wait for the
/// transaction to either be successful or fail.
#[derive(Debug)]
pub struct SubmittedTransaction<Runtime: StorageEnableRuntime> {
    /// The watcher used to query the state of the transaction from the blockchain node.
    watcher: Receiver<String>,
    /// The hash of the transaction.
    hash: Runtime::Hash,
    /// The nonce of the transaction.
    nonce: u32,
    /// The maximum amount of time to wait for the transaction to either be successful or fail.
    timeout: Duration,
}

impl<Runtime: StorageEnableRuntime> SubmittedTransaction<Runtime> {
    pub fn new(
        watcher: Receiver<String>,
        hash: Runtime::Hash,
        nonce: u32,
        timeout: Duration,
    ) -> Self {
        Self {
            watcher,
            hash,
            timeout,
            nonce,
        }
    }

    /// Getter for the transaction hash.
    pub fn hash(&self) -> Runtime::Hash {
        self.hash
    }

    /// Getter for the transaction nonce.
    pub fn nonce(&self) -> u32 {
        self.nonce
    }

    /// Handles the lifecycle of a submitted transaction.
    ///
    /// Waits for the transaction to be included in a block AND the checks the transaction is successful.
    /// If the transaction is not included in a block within the specified timeout, it will be
    /// considered failed and an error will be returned.
    pub async fn watch_for_success<FSH>(
        &mut self,
        blockchain: &ActorHandle<BlockchainService<FSH, Runtime>>,
    ) -> Result<(), WatchTransactionError>
    where
        FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
        Runtime: StorageEnableRuntime,
    {
        let extrinsic_in_block = self.watch_transaction(blockchain).await?;

        // Check if the extrinsic was successful.
        let extrinsic_result = ActorHandle::<BlockchainService<FSH, Runtime>>::extrinsic_result(extrinsic_in_block.clone())
            .map_err(|_| {
              let err_msg = "Extrinsic does not contain an ExtrinsicFailed nor ExtrinsicSuccess event, which is not possible; qed";
              error!(target: LOG_TARGET, "{}", err_msg);
              WatchTransactionError::Internal(err_msg.to_string())
            })?;
        match extrinsic_result {
            ExtrinsicResult::Success { dispatch_info } => {
                info!(target: LOG_TARGET, "Extrinsic successful with dispatch info: {:?}", dispatch_info);
            }
            ExtrinsicResult::Failure {
                dispatch_error,
                dispatch_info,
            } => {
                error!(target: LOG_TARGET, "Extrinsic failed with dispatch error: {:?}, dispatch info: {:?}", dispatch_error, dispatch_info);
                return Err(WatchTransactionError::TransactionFailed {
                    dispatch_info: format!("{:?}", dispatch_info),
                    dispatch_error: format!("{:?}", dispatch_error),
                });
            }
        }

        debug!(target: LOG_TARGET, "Events in extrinsic: {:?}", &extrinsic_in_block.events);

        Ok(())
    }

    /// Handles the lifecycle of a submitted transaction.
    ///
    /// Waits for the transaction to be included in a block AND the checks the transaction is successful.
    /// If the transaction is not included in a block within the specified timeout, it will be
    /// considered failed and an error will be returned.
    ///
    /// Returns the events emitted by the transaction.
    pub async fn watch_for_success_with_events<FSH>(
        &mut self,
        blockchain: &ActorHandle<BlockchainService<FSH, Runtime>>,
    ) -> Result<StorageHubEventsVec<Runtime>, WatchTransactionError>
    where
        FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
        Runtime: StorageEnableRuntime,
    {
        let extrinsic_in_block = self.watch_transaction(blockchain).await?;

        // Check if the extrinsic was successful.
        let extrinsic_result = ActorHandle::<BlockchainService<FSH, Runtime>>::extrinsic_result(extrinsic_in_block.clone())
            .map_err(|_| {
              let err_msg = "Extrinsic does not contain an ExtrinsicFailed nor ExtrinsicSuccess event, which is not possible; qed";
              error!(target: LOG_TARGET, "{}", err_msg);
              WatchTransactionError::Internal(err_msg.to_string())
            })?;

        match extrinsic_result {
            ExtrinsicResult::Success { dispatch_info } => {
                info!(target: LOG_TARGET, "Extrinsic successful with dispatch info: {:?}", dispatch_info);
            }
            ExtrinsicResult::Failure {
                dispatch_error,
                dispatch_info,
            } => {
                error!(target: LOG_TARGET, "Extrinsic failed with dispatch error: {:?}, dispatch info: {:?}", dispatch_error, dispatch_info);
                return Err(WatchTransactionError::TransactionFailed {
                    dispatch_info: format!("{:?}", dispatch_info),
                    dispatch_error: format!("{:?}", dispatch_error),
                });
            }
        }

        debug!(target: LOG_TARGET, "Events in extrinsic: {:?}", &extrinsic_in_block.events);

        Ok(extrinsic_in_block.events)
    }

    async fn watch_transaction<FSH>(
        &mut self,
        blockchain: &ActorHandle<BlockchainService<FSH, Runtime>>,
    ) -> Result<Extrinsic<Runtime>, WatchTransactionError>
    where
        FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
        Runtime: StorageEnableRuntime,
    {
        let block_hash;
        let start_time = Instant::now();
        loop {
            // Get the elapsed time since submit.
            let elapsed = start_time.elapsed();
            // Calculate the remaining time to wait.

            // Check if the timeout has been reached.
            if elapsed > self.timeout {
                error!(target: LOG_TARGET, "Timeout waiting for transaction {} to be included in a block", self.hash);
                return Err(WatchTransactionError::Timeout);
            }

            let remaining = self.timeout - elapsed;

            // Wait for either a new message from the watcher, or the timeout to be reached.
            let result = match tokio::time::timeout(remaining, self.watcher.recv()).await {
                Ok(result) => match result {
                    Some(result) => result,
                    None => {
                        error!(target: LOG_TARGET, "Transaction watcher channel closed");
                        return Err(WatchTransactionError::WatcherChannelClosed);
                    }
                },
                Err(_) => {
                    // Timeout reached, exit the loop.
                    error!(target: LOG_TARGET, "Timeout waiting for transaction to be included in a block");
                    return Err(WatchTransactionError::Timeout);
                }
            };

            // Parse the JSONRPC string. The strings sent by the RPC wacher should be valid JSONRPC strings.
            let json: serde_json::Value = serde_json::from_str(&result).map_err(|_| {
                let err_msg = format!(
                    "The result, if not an error, can only be a JSONRPC string: {:?}",
                    result
                );
                error!(target: LOG_TARGET, "{}", err_msg);
                WatchTransactionError::Internal(err_msg)
            })?;

            debug!(target: LOG_TARGET, "Transaction information: {:?}", json);

            // Checking if the transaction is included in a block.
            // TODO: Consider if we might want to wait for "finalized".
            // TODO: Handle other lifetime extrinsic edge cases. See https://github.com/paritytech/polkadot-sdk/blob/master/substrate/client/transaction-pool/api/src/lib.rs#L131
            if let Some(in_block) = json["params"]["result"]["inBlock"].as_str() {
                block_hash = Some(Runtime::Hash::from_str(in_block).map_err(|_| {
                    error!(target: LOG_TARGET, "Block hash should be a valid H256; qed");
                    WatchTransactionError::Internal("Block hash should be a valid H256".to_string())
                })?);
                let subscription_id =
                    json["params"]["subscription"].as_number().ok_or_else(|| {
                        let err_msg = "Subscription should exist and be a number; qed";
                        error!(target: LOG_TARGET, "{}", err_msg);
                        WatchTransactionError::Internal(err_msg.to_string())
                    })?;

                // Unwatch extrinsic to release tx_watcher.
                blockchain
                    .unwatch_extrinsic(subscription_id.to_owned())
                    .await
                    .map_err(|e| {
                        let err_msg = format!("Error unwatching extrinsic: {:?}", e);
                        error!(target: LOG_TARGET, "{}", err_msg);
                        WatchTransactionError::Internal(err_msg)
                    })?;

                // Breaking while loop.
                // Even though we unwatch the transaction, and the loop should break, we still break manually
                // in case we continue to receive updates. This should not happen, but it is a safety measure,
                // and we already have what we need.
                break;
            }
        }

        // Get the extrinsic from the block, with its events.
        let block_hash = block_hash.ok_or_else(
            || {
                let err_msg = "Block hash should exist after waiting for extrinsic to be included in a block; qed";
                error!(target: LOG_TARGET, "{}", err_msg);
                WatchTransactionError::Internal(err_msg.to_string())
            })?;
        let extrinsic_in_block = blockchain
            .get_extrinsic_from_block(block_hash, self.hash)
            .await
            .map_err(|e| {
                let err_msg = format!("Error getting extrinsic from block: {:?}", e);
                error!(target: LOG_TARGET, "{}", err_msg);
                WatchTransactionError::Internal(err_msg)
            })?;
        Ok(extrinsic_in_block)
    }
}
