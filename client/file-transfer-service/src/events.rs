use sc_network::PeerId;
use shc_actors_framework::event_bus::{EventBus, EventBusMessage, ProvidesEventBus};
use shc_common::types::{BucketId, ChunkId, DownloadRequestId, FileKey, FileKeyProof};

#[derive(Clone)]
pub struct RemoteUploadRequest {
    pub peer: PeerId,
    pub file_key: FileKey,
    pub file_key_proof: FileKeyProof,
    pub bucket_id: Option<BucketId>,
    /// A channel to notify the user that the file is stored completely
    /// and there is no need to continue uploading chunks.
    pub file_complete_channel: tokio::sync::mpsc::Sender<bool>,
}

impl EventBusMessage for RemoteUploadRequest {}

#[derive(Clone)]
pub struct RemoteDownloadRequest {
    pub file_key: FileKey,
    pub chunk_id: ChunkId,
    pub bucket_id: Option<BucketId>,
    pub request_id: DownloadRequestId,
}

impl EventBusMessage for RemoteDownloadRequest {}

#[derive(Clone, Default)]
pub struct FileTransferServiceEventBusProvider {
    remote_upload_request_event_bus: EventBus<RemoteUploadRequest>,
    remote_download_request_event_bus: EventBus<RemoteDownloadRequest>,
}

impl FileTransferServiceEventBusProvider {
    pub fn new() -> Self {
        Self {
            remote_upload_request_event_bus: EventBus::new(),
            remote_download_request_event_bus: EventBus::new(),
        }
    }
}

impl ProvidesEventBus<RemoteUploadRequest> for FileTransferServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<RemoteUploadRequest> {
        &self.remote_upload_request_event_bus
    }
}

impl ProvidesEventBus<RemoteDownloadRequest> for FileTransferServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<RemoteDownloadRequest> {
        &self.remote_download_request_event_bus
    }
}
