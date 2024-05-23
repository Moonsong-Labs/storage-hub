use std::str::FromStr;

use anyhow::anyhow;
use log::*;
use sp_core::H256;
use storage_hub_infra::actor::ActorHandle;
use tokio::sync::mpsc::Receiver;

use crate::services::blockchain::{commands::BlockchainServiceInterface, types::ExtrinsicResult};

use super::{types::ExtrinsicHash, BlockchainService};

const LOG_TARGET: &str = "blockchain-transaction";

#[derive(Debug)]
pub struct SubmittedTransaction {
    watcher: Receiver<String>,
    hash: ExtrinsicHash,
}

impl SubmittedTransaction {
    pub fn new(watcher: Receiver<String>, hash: H256) -> Self {
        Self { watcher, hash }
    }

    pub async fn watch_for_success(
        &mut self,
        blockchain: &ActorHandle<BlockchainService>,
    ) -> anyhow::Result<()> {
        // Wait for the transaction to be included in a block.
        let mut block_hash = None;
        // TODO: Consider adding a timeout.
        while let Some(result) = self.watcher.recv().await {
            // Parse the JSONRPC string, now that we know it is not an error.
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

        // Check if the extrinsic was successful. In this mocked task we know this should fail if Alice is
        // not a registered BSP.
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
