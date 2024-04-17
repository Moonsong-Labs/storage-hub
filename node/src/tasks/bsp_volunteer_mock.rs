use log::{debug, error, info};
use sp_runtime::BoundedVec;
use storage_hub_infra::event_bus::EventHandler;

use crate::services::{
    blockchain::{commands::BlockchainServiceInterface, events::NewStorageRequest},
    StorageHubHandler,
};

const LOG_TARGET: &str = "bsp-volunteer-mock-task";

#[derive(Clone)]
pub struct BspVolunteerMockTask {
    storage_hub_handler: StorageHubHandler,
}

impl BspVolunteerMockTask {
    pub fn new(storage_hub_handler: StorageHubHandler) -> Self {
        Self {
            storage_hub_handler: storage_hub_handler,
        }
    }
}

impl EventHandler<NewStorageRequest> for BspVolunteerMockTask {
    async fn handle_event(&self, event: NewStorageRequest) -> anyhow::Result<()> {
        // TODO: Here we would send the actual multiaddresses of this BSP.
        let multiaddresses = BoundedVec::default();

        // Build extrinsic.
        let call =
            storage_hub_runtime::RuntimeCall::FileSystem(pallet_file_system::Call::bsp_volunteer {
                location: event.location,
                fingerprint: event.fingerprint,
                multiaddresses,
            });

        let mut tx_watcher = self
            .storage_hub_handler
            .blockchain
            .send_extrinsic(call)
            .await;

        // Wait for the transaction to be included in a block.
        // TODO: unwatch transaction to release tx_watcher.
        let mut block_hash = None;
        // TODO: Consider adding a timeout.
        while let Some(tx_result) = tx_watcher.recv().await {
            // Checking if there is an update with an error in our transaction.
            if tx_result.starts_with("Error") {
                error!(target: LOG_TARGET, "Error in transaction: {:?}", tx_result);
                return Err(anyhow::anyhow!("Error in transaction: {:?}", tx_result));
            }

            // Parse the JSONRPC string, now that we know it is not an error.
            let json: serde_json::Value = serde_json::from_str(&tx_result)
                .expect("The result, if not an error, can only be a JSONRPC string; qed");

            debug!(target: LOG_TARGET, "Transaction information: {:?}", json);

            // Checking if the transaction is included in a block.
            // TODO: Consider if we might want to wait for "finalized".
            if let Some(in_block) = json["params"]["result"]["inBlock"].as_str() {
                block_hash = Some(in_block.to_owned());
                let subscription_id = json["params"]["subscription"]
                    .as_number()
                    .expect("Subscription should exist and be a number; qed");

                // Unwatch extrinsic to release tx_watcher.
                self.storage_hub_handler
                    .blockchain
                    .unwatch_extrinsic(subscription_id.to_owned())
                    .await?;

                // Breaking while loop.
                // Even though we unwatch the transaction, and the loop should break, we still break manually
                // in case we continue to receive updates. This should not happen, but it is a safety measure,
                // and we already have what we need.
                break;
            }
        }

        info!(target: LOG_TARGET, "No more transaction information");

        Ok(())
    }
}
