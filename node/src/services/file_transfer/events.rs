use storage_hub_infra::event_bus::{EventBus, EventBusMessage, ProvidesEventBus};

#[derive(Clone, Debug, Default)]
pub struct FileTransferServiceEventBusProvider {
    remote_upload_request_event_bus: EventBus<RemoteUploadRequest>,
}

impl FileTransferServiceEventBusProvider {
    pub fn new() -> Self {
        Self {
            remote_upload_request_event_bus: EventBus::new(),
        }
    }
}

impl ProvidesEventBus<RemoteUploadRequest> for FileTransferServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<RemoteUploadRequest> {
        &self.remote_upload_request_event_bus
    }
}

#[derive(Debug, Clone)]
pub struct RemoteUploadRequest {
    pub location: String,
}

impl EventBusMessage for RemoteUploadRequest {}
