use shc_actors_derive::{ActorEvent, ActorEventBus};
use shc_common::types::{BackupStorageProviderId, BucketId, FileKey};

/// Represent where a file should be deleted from for the deletion process
#[derive(Clone, Debug)]
pub enum FileDeletionTarget {
    BspId(BackupStorageProviderId),
    BucketId(BucketId),
}

/// Event triggered when fisherman detects a file deletion request
///
/// Should contain all the data required to construct a proof of inclusion for a file key
/// to be deleted from the [`FileDeletionTarget`].
#[derive(Clone, ActorEvent)]
#[actor(actor = "fisherman_service")]
pub struct ProcessFileDeletionRequest {
    /// The file key for which deletion proof is requested
    pub file_key: FileKey,
    /// The deletion target containing provider and bucket information
    pub deletion_target: FileDeletionTarget,
    // TODO: Add user signed message
}

#[ActorEventBus("fisherman_service")]
pub struct FishermanServiceEventBusProvider;
