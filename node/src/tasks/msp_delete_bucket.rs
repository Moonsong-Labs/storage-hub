use anyhow::anyhow;
use sc_tracing::tracing::*;
use shc_blockchain_service::events::FinalisedMspStoppedStoringBucket;

use crate::services::handler::StorageHubHandler;
use crate::tasks::{FileStorageT, MspForestStorageHandlerT};
use shc_actors_framework::event_bus::EventHandler;

const LOG_TARGET: &str = "msp-stopped-storing-task";

const MAX_CONFIRM_STORING_REQUEST_TRY_COUNT: u32 = 3;

/// [`MspStoppedStoringTask`]: Handles the event of the MSP stopping storing a bucket.
///
/// - [`FinalisedMspStoppedStoringBucket`]: Handles the event of the MSP stopping storing a bucket.
/// This should only be triggered when the anchor relay chain block is finalized to avoid
/// deleting the bucket prematurely in the event there is a reorg.
pub struct MspStoppedStoringTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: MspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<FL, FSH>,
}

impl<FL, FSH> Clone for MspStoppedStoringTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: MspForestStorageHandlerT,
{
    fn clone(&self) -> MspStoppedStoringTask<FL, FSH> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<FL, FSH> MspStoppedStoringTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: MspForestStorageHandlerT,
{
    pub fn new(storage_hub_handler: StorageHubHandler<FL, FSH>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

/// Handles the [`FinalisedMspStoppedStoringBucket`] event.
///
/// This event is triggered by an on-chain event which is part of a finalized anchored relay block.
///
/// This task will:
/// - Delete the bucket from the MSP's storage.
/// - Delete all the files in the bucket.
/// upload requests.
impl<FL, FSH> EventHandler<FinalisedMspStoppedStoringBucket> for MspStoppedStoringTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: MspForestStorageHandlerT,
{
    async fn handle_event(
        &mut self,
        event: FinalisedMspStoppedStoringBucket,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Deleting bucket {:?} for MSP {:?}",
            event.bucket_id,
            event.msp_id
        );

        let file_storage = self.storage_hub_handler.file_storage.clone();
        let mut file_storage_write = file_storage.write().await;

        file_storage_write
            .delete_files_with_prefix(
                &event
                    .bucket_id
                    .as_ref()
                    .try_into()
                    .map_err(|_| anyhow!("Invalid bucket id"))?,
            )
            .map_err(|e| anyhow!("Failed to delete files with prefix: {:?}", e))?;

        self.storage_hub_handler
            .forest_storage_handler
            .remove_forest_storage(&event.bucket_id.as_ref().to_vec())
            .await;

        Ok(())
    }
}
