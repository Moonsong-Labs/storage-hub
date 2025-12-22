use anyhow::anyhow;
use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::events::{FinalisedBucketMovedAway, FinalisedMspStoppedStoringBucket};
use shc_common::traits::StorageEnableRuntime;
use shc_common::types::BucketId;
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::ForestStorageHandler;

use crate::{
    handler::StorageHubHandler,
    types::{ForestStorageKey, MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "msp-stopped-storing-task";

/// Task that handles bucket deletion for an MSP in two scenarios:
/// 1. When a bucket is moved away to another MSP ([`BucketMovedAway`])
/// 2. When the MSP stops storing a bucket ([`FinalisedMspStoppedStoringBucket`])
///
/// The task will:
/// 1. Delete all files with the bucket prefix from [`FileStorage`]
/// 2. Remove the bucket's [`ForestStorageHandler`] instance
///
/// # Note
/// The cleanup happens immediately after the events are confirmed in a finalized block.
///
/// [`FileStorage`]: shc_file_manager::traits::FileStorage
/// [`ForestStorageHandler`]: shc_forest_manager::traits::ForestStorageHandler
pub struct MspDeleteBucketTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for MspDeleteBucketTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> MspDeleteBucketTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, Runtime> MspDeleteBucketTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<NT, Runtime> EventHandler<FinalisedBucketMovedAway<Runtime>>
    for MspDeleteBucketTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: FinalisedBucketMovedAway<Runtime>,
    ) -> anyhow::Result<String> {
        info!(
            target: LOG_TARGET,
            "MSP: bucket {:?} moved to MSP {:?}, starting cleanup",
            event.bucket_id,
            event.new_msp_id,
        );

        if let Err(e) = self.delete_bucket(&event.bucket_id).await {
            error!(
                target: LOG_TARGET,
                "Failed to delete bucket {:?} after move: {:?}",
                event.bucket_id,
                e
            );
            return Err(e);
        }

        Ok(format!(
            "MSP: successfully deleted bucket [{:x}] after move",
            event.bucket_id,
        ))
    }
}

impl<NT, Runtime> EventHandler<FinalisedMspStoppedStoringBucket<Runtime>>
    for MspDeleteBucketTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: FinalisedMspStoppedStoringBucket<Runtime>,
    ) -> anyhow::Result<String> {
        info!(
            target: LOG_TARGET,
            "MSP: deleting bucket {:?} for MSP {:?}",
            event.bucket_id,
            event.msp_id
        );

        if let Err(e) = self.delete_bucket(&event.bucket_id).await {
            error!(
                target: LOG_TARGET,
                "Failed to delete bucket {:?} after stop storing: {:?}",
                event.bucket_id,
                e
            );
            return Err(e);
        }

        Ok(format!(
            "MSP: successfully deleted bucket [{:x}] after stop storing",
            event.bucket_id,
        ))
    }
}

impl<NT, Runtime> MspDeleteBucketTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    /// Deletes all files in a bucket and removes the bucket's forest storage
    async fn delete_bucket(&mut self, bucket_id: &BucketId<Runtime>) -> anyhow::Result<()> {
        self.storage_hub_handler
            .file_storage
            .write()
            .await
            .delete_files_with_prefix(
                &bucket_id
                    .as_ref()
                    .try_into()
                    .map_err(|_| anyhow!("Invalid bucket id"))?,
            )
            .map_err(|e| anyhow!("Failed to delete files with prefix: {:?}", e))?;

        self.storage_hub_handler
            .forest_storage_handler
            .remove_forest_storage(&ForestStorageKey::from(bucket_id.as_ref().to_vec()))
            .await;

        Ok(())
    }
}
