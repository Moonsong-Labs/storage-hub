use shc_actors_derive::{ActorEvent, ActorEventBus};
use shc_common::{
    traits::StorageEnableRuntime,
    types::{BackupStorageProviderId, BucketId},
};
use std::sync::Arc;

use crate::types::BatchDeletionPermitGuard;

/// Represent where a file should be deleted from for the deletion process
#[derive(Clone, Debug)]
pub enum FileDeletionTarget<Runtime: StorageEnableRuntime> {
    BspId(BackupStorageProviderId<Runtime>),
    BucketId(BucketId<Runtime>),
}

/// Event triggered by the Fisherman scheduler (see `FishermanServiceEventLoop::run`) to process batched file deletions.
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
    /// Semaphore permit guard wrapped in Arc to satisfy Clone requirement for events.
    ///
    /// The guard is held by the event handler for its lifetime, automatically releasing the
    /// semaphore permit when the handler completes or fails. Additionally, when dropped it
    /// notifies the fisherman service event loop so it can promptly schedule the next batch.
    pub permit: Arc<BatchDeletionPermitGuard>,
}

/// Event bus provider for fisherman service events
#[ActorEventBus("fisherman_service")]
pub struct FishermanServiceEventBusProvider;
