use anyhow::anyhow;
use sc_tracing::tracing::*;
use shc_blockchain_service::events::FinalisedMspStoppedStoringBucket;

use crate::services::handler::StorageHubHandler;
use crate::tasks::{FileStorageT, MspForestStorageHandlerT};
use shc_actors_framework::event_bus::EventHandler;

const LOG_TARGET: &str = "msp-stopped-storing-task";

const MAX_CONFIRM_STORING_REQUEST_TRY_COUNT: u32 = 3;

pub struct BspDeleteFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<FL, FSH>,
}

impl<FL, FSH> Clone for BspDeleteFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    fn clone(&self) -> MspStoppedStoringTask<FL, FSH> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<FL, FSH> BspDeleteFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    pub fn new(storage_hub_handler: StorageHubHandler<FL, FSH>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

/// Handles the [`MspStoppedStoringBucket`] event.
///
/// This event is triggered by an on-chain event which is part of a finalized anchored relay block.
///
/// This task will:
/// - Delete the bucket from the MSP's storage.
/// - Delete all the files in the bucket.
/// upload requests.
impl<FL, FSH> EventHandler<FinalisedMspStoppedStoringBucket> for BspDeleteFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    async fn handle_event(
        &mut self,
        event: FinalisedMspStoppedStoringBucket,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Deleting file {:?} for BSP {:?}",
            event.bucket_id,
            event.bsp_id
        );

        Ok(())
    }
}
