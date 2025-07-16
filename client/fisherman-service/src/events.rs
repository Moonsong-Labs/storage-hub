use shc_actors_derive::{ActorEvent, ActorEventBus};
use shc_common::types::{BackupStorageProviderId, BucketId, FileKey};

/// Enum to represent the file deletion target
/// Contains either a BSP with its bucket ID or just a bucket ID (for MSP)
#[derive(Clone, Debug)]
pub enum FileDeletionTarget {
    /// BSP storage with associated bucket ID
    BspId(BackupStorageProviderId),
    /// MSP storage (bucket ID only)
    BucketId(BucketId),
}

/// Event triggered when fisherman detects a file deletion request
/// This event will be handled by a task to construct and submit proof of inclusion
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
