use sc_network::{config::OutgoingResponse, PeerId};
use std::sync::Arc;
use tokio::sync::Mutex;

use storage_hub_infra::{
    event_bus::{EventBus, EventBusMessage, ProvidesEventBus},
    types::{ChunkId, FileProof, Key},
};

#[derive(Clone)]
pub struct RemoteUploadRequest {
    pub peer: PeerId,
    pub file_key: Key,
    pub chunk_with_proof: FileProof,
    // TODO: Confirm whether this is needed and should be here in the first place.
    pub maybe_pending_response:
        Arc<Mutex<Option<futures::channel::oneshot::Sender<OutgoingResponse>>>>,
}

impl EventBusMessage for RemoteUploadRequest {}

#[derive(Clone)]
pub struct RemoteDownloadRequest {
    pub file_key: Key,
    pub chunk_id: ChunkId,
    // TODO: Confirm whether this is needed and should be here in the first place.
    pub maybe_pending_response:
        Arc<Mutex<Option<futures::channel::oneshot::Sender<OutgoingResponse>>>>,
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
