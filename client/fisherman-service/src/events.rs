use shc_actors_derive::{ActorEvent, ActorEventBus};
use shc_common::{
    traits::StorageEnableRuntime,
    types::{BackupStorageProviderId, BucketId, FileOperationIntention},
};
use sp_runtime::MultiSignature;
/// Represent where a file should be deleted from for the deletion process
#[derive(Clone, Debug)]
pub enum FileDeletionTarget<Runtime: StorageEnableRuntime> {
    BspId(BackupStorageProviderId<Runtime>),
    BucketId(BucketId<Runtime>),
}
/// Event triggered when fisherman detects a file deletion request
///
/// Contains the signed deletion intention data to be processed by the task.
#[derive(Clone, ActorEvent)]
#[actor(actor = "fisherman_service", generics(Runtime: StorageEnableRuntime))]
pub struct ProcessFileDeletionRequest<Runtime: StorageEnableRuntime> {
    /// The file key from the signed intention
    pub signed_file_operation_intention: FileOperationIntention<Runtime>,
    /// The signed intention
    pub signature: MultiSignature,
}
#[ActorEventBus("fisherman_service")]
pub struct FishermanServiceEventBusProvider<Runtime: StorageEnableRuntime>;
