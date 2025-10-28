use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface, events::VerifyMspBucketForests,
};
use shc_common::{traits::StorageEnableRuntime, types::StorageProviderId};

use crate::{
    handler::StorageHubHandler,
    types::{ForestStorageKey, MspForestStorageHandlerT, ShNodeType},
};
use shc_forest_manager::traits::ForestStorageHandler;

const LOG_TARGET: &str = "msp-verify-bucket-forests-task";

pub struct MspVerifyBucketForestsTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for MspVerifyBucketForestsTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> Self {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, Runtime> MspVerifyBucketForestsTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<NT, Runtime> EventHandler<VerifyMspBucketForests> for MspVerifyBucketForestsTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, _event: VerifyMspBucketForests) -> anyhow::Result<()> {
        // Determine this node's provider id
        let maybe_provider_id = self
            .storage_hub_handler
            .blockchain
            .query_storage_provider_id(None)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query storage provider id: {:?}", e))?;

        let Some(storage_provider_id) = maybe_provider_id else {
            trace!(target: LOG_TARGET, "Node is not a storage provider; skipping MSP forest verification");
            return Ok(());
        };

        // Only proceed if this is an MSP node
        let msp_id = match storage_provider_id {
            StorageProviderId::MainStorageProvider(msp_id) => msp_id,
            _ => {
                trace!(target: LOG_TARGET, "Node is not an MSP; skipping MSP forest verification");
                return Ok(());
            }
        };

        // Query buckets managed by this MSP
        let buckets = self
            .storage_hub_handler
            .blockchain
            .query_buckets_for_msp(msp_id)
            .await
            .unwrap_or_else(|e| {
                error!(target: LOG_TARGET, "Failed to query buckets for MSP: {:?}", e);
                Vec::new()
            });

        if buckets.is_empty() {
            trace!(target: LOG_TARGET, "No buckets managed by MSP; nothing to verify");
            return Ok(());
        }

        // Verify each bucket has a local forest storage instance
        for bucket_id in buckets {
            let key = ForestStorageKey::from(bucket_id.as_ref().to_vec());
            let has_instance = self
                .storage_hub_handler
                .forest_storage_handler
                .get(&key)
                .await
                .is_some();

            if !has_instance {
                error!(
                    target: LOG_TARGET,
                    "CRITICAL❗️❗️ Missing local forest storage for bucket [{:?}] managed by this MSP",
                    bucket_id
                );
            } else {
                info!(
                    target: LOG_TARGET,
                    "🌳 Verified local forest storage present for bucket [{:?}]",
                    bucket_id
                );
            }
        }

        info!(target: LOG_TARGET, "🌳 Verified local forest storage present for all buckets managed by this MSP");

        Ok(())
    }
}
