use sc_network::PeerId;
use shc_actors_framework::event_bus::{EventBusMessage, define_event_bus};
use shc_common::types::{ChunkId, DownloadRequestId, FileKey, FileKeyProof};

#[derive(Clone, EventBusMessage)]
pub struct RemoteUploadRequest {
    pub peer: PeerId,
    pub file_key: FileKey,
    pub file_key_proof: FileKeyProof,
}

#[derive(Clone, EventBusMessage)]
pub struct RemoteDownloadRequest {
    pub file_key: FileKey,
    pub chunk_id: ChunkId,
    pub request_id: DownloadRequestId,
}

define_event_bus!(
    FileTransferServiceEventBusProvider,
    RemoteUploadRequest,
    RemoteDownloadRequest
);
