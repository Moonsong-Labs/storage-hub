use shc_actors_derive::{ActorEvent, ActorEventBus};
use shc_common::{
    traits::StorageEnableRuntime,
    types::{BackupStorageProviderId, BucketId},
};
use std::sync::Arc;

/// Represent where a file should be deleted from for the deletion process
#[derive(Clone, Debug)]
pub enum FileDeletionTarget<Runtime: StorageEnableRuntime> {
    BspId(BackupStorageProviderId<Runtime>),
    BucketId(BucketId<Runtime>),
}

/// Event triggered every time interval ([`batch_interval_duration`](`crate::handler::FishermanService::batch_interval_duration`)) to process batched file deletions.
///
/// Contains the deletion type to process in this cycle. FishermanService alternates between
/// User and Incomplete deletion types across batch cycles.
///
/// The semaphore permit is automatically released when the event handler completes or fails,
/// ensuring only one batch deletion cycle runs at a time.
#[derive(Clone, Debug, ActorEvent)]
#[actor(actor = "fisherman_service")]
pub struct BatchFileDeletions {
    /// Type of deletion to process in this batch cycle (User or Incomplete)
    pub deletion_type: shc_indexer_db::models::FileDeletionType,
    /// Maximum number of files to process in this batch cycle
    pub batch_deletion_limit: u64,
    /// Semaphore permit wrapped in Arc to satisfy Clone requirement for events.
    /// The permit is held by the event handler for its lifetime,
    /// automatically releasing when the handler completes or fails.
    pub permit: Arc<tokio::sync::OwnedSemaphorePermit>,
}

/// Event bus provider for fisherman service events
#[ActorEventBus("fisherman_service")]
pub struct FishermanServiceEventBusProvider;
