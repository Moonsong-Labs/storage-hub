use sc_tracing::tracing::*;
use shc_blockchain_service::events::MoveBucketRequestedForNewMsp;

use crate::services::handler::StorageHubHandler;
use crate::tasks::{FileStorageT, MspForestStorageHandlerT};
use shc_actors_framework::event_bus::EventHandler;

const LOG_TARGET: &str = "msp-move-bucket-task";

/// [`MspMoveBucketTask`]: Handles the [`MoveBucketRequestedForNewMsp`] event.
pub struct MspMoveBucketTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: MspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<FL, FSH>,
}

impl<FL, FSH> Clone for MspMoveBucketTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: MspForestStorageHandlerT,
{
    fn clone(&self) -> MspMoveBucketTask<FL, FSH> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<FL, FSH> MspMoveBucketTask<FL, FSH>
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

/// Handles the [`MoveBucketRequestedForNewMsp`] event.
///
/// TODO DOCS
impl<FL, FSH> EventHandler<MoveBucketRequestedForNewMsp> for MspMoveBucketTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: MspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: MoveBucketRequestedForNewMsp) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "MSP: user requested to move bucket {:?} to us",
            event.bucket_id,
        );

        Ok(())
    }
}
