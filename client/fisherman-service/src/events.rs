use shc_actors_derive::{ActorEvent, ActorEventBus};
use shc_common::{
    traits::StorageEnableRuntime,
    types::{BackupStorageProviderId, BucketId},
};

/// Represent where a file should be deleted from for the deletion process
#[derive(Clone, Debug)]
pub enum FileDeletionTarget<Runtime: StorageEnableRuntime> {
    BspId(BackupStorageProviderId<Runtime>),
    BucketId(BucketId<Runtime>),
}

/// Event triggered every 5 blocks (when lock is available) to process batched file deletions.
///
/// Contains the deletion type to process in this cycle. FishermanService alternates between
/// User and Incomplete deletion types across batch cycles.
#[derive(Clone, ActorEvent)]
#[actor(actor = "fisherman_service")]
pub struct BatchFileDeletions {
    /// Type of deletion to process in this batch cycle (User or Incomplete)
    pub deletion_type: shc_indexer_db::models::FileDeletionType,
}

/// Event bus provider for fisherman service events
#[ActorEventBus("fisherman_service")]
pub struct FishermanServiceEventBusProvider;
