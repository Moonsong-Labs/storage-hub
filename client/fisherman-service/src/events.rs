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
#[ActorEventBus("fisherman_service")]
pub struct FishermanServiceEventBusProvider;
