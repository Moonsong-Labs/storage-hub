use anyhow::anyhow;
use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::events::{FinalisedBucketMovedAway, FinalisedMspStoppedStoringBucket};
use shc_common::traits::{StorageEnableApiCollection, StorageEnableRuntimeApi};
use shc_common::types::BucketId;
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::ForestStorageHandler;

use crate::{
    handler::StorageHubHandler,
    types::{MspForestStorageHandlerT, ShNodeType},
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
pub struct MspDeleteBucketTask<NT, RuntimeApi>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    storage_hub_handler: StorageHubHandler<NT, RuntimeApi>,
}

impl<NT, RuntimeApi> Clone for MspDeleteBucketTask<NT, RuntimeApi>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    fn clone(&self) -> MspDeleteBucketTask<NT, RuntimeApi> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, RuntimeApi> MspDeleteBucketTask<NT, RuntimeApi>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, RuntimeApi>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<NT, RuntimeApi> EventHandler<FinalisedBucketMovedAway> for MspDeleteBucketTask<NT, RuntimeApi>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    async fn handle_event(&mut self, event: FinalisedBucketMovedAway) -> anyhow::Result<()> {
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

        info!(
            target: LOG_TARGET,
            "MSP: successfully deleted bucket {:?} after move",
            event.bucket_id,
        );

        Ok(())
    }
}

impl<NT, RuntimeApi> EventHandler<FinalisedMspStoppedStoringBucket>
    for MspDeleteBucketTask<NT, RuntimeApi>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    async fn handle_event(
        &mut self,
        event: FinalisedMspStoppedStoringBucket,
    ) -> anyhow::Result<()> {
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

        info!(
            target: LOG_TARGET,
            "MSP: successfully deleted bucket {:?} after stop storing",
            event.bucket_id,
        );

        Ok(())
    }
}

impl<NT, RuntimeApi> MspDeleteBucketTask<NT, RuntimeApi>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    /// Deletes all files in a bucket and removes the bucket's forest storage
    async fn delete_bucket(&mut self, bucket_id: &BucketId) -> anyhow::Result<()> {
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
            .remove_forest_storage(&bucket_id.as_ref().to_vec())
            .await;

        Ok(())
    }
}
