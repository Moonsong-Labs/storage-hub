use shc_actors_derive::{ActorEvent, ActorEventBus};
use shc_common::types::{BackupStorageProviderId, BucketId, FileOperationIntention};
use sp_runtime::MultiSignature;

/// Represent where a file should be deleted from for the deletion process
#[derive(Clone, Debug)]
pub enum FileDeletionTarget {
    BspId(BackupStorageProviderId),
    BucketId(BucketId),
}
/// Event triggered when fisherman detects a file deletion request
///
/// Contains the signed deletion intention data to be processed by the task.
#[derive(Clone, ActorEvent)]
#[actor(actor = "fisherman_service")]
pub struct ProcessFileDeletionRequest {
    /// The file key from the signed intention
    pub signed_file_operation_intention: FileOperationIntention,
    /// The signed intention
    pub signature: MultiSignature,
}

/// Event triggered when fisherman detects an incomplete storage request
///
/// These events indicate storage requests that failed (expired, revoked, or rejected)
/// and require file deletion without user signature validation.
#[derive(Clone, ActorEvent)]
#[actor(actor = "fisherman_service")]
pub struct ProcessIncompleteStorageRequest {
    /// The file key that needs to be deleted
    pub file_key: sp_core::H256,
}

#[ActorEventBus("fisherman_service")]
pub struct FishermanServiceEventBusProvider;
