use sc_network::PeerId;
use shc_actors_framework::event_bus::{EventBus, EventBusMessage, ProvidesEventBus};
use shc_common::types::{
    BucketId, ChunkId, DownloadRequestId, FileKey, FileKeyProof, UploadRequestId,
};
use std::collections::HashSet;

/// A request to upload file chunks to a remote peer with verifiable proof.
///
/// This request contains a file key proof that allows the receiver to verify and extract
/// the chunks being uploaded. While the request itself doesn't impose limits on the number
/// of chunks in the proof, specific implementations (like `MspUploadFileTask` or
/// `BspUploadFileTask`) enforce their own chunk count restrictions.
///
/// The proof must contain at least one chunk to be considered valid.
#[derive(Clone)]
pub struct RemoteUploadRequest {
    /// The peer ID of the receiver node.
    pub peer: PeerId,
    /// File key of the file which is being uploaded.
    pub file_key: FileKey,
    /// Proof containing the file chunk(s) which are being uploaded.
    pub file_key_proof: FileKeyProof,
    /// Optional bucket identifier for file organization only required based on the receiver's implementation.
    pub bucket_id: Option<BucketId>,
    /// Unique identifier for tracking the upload request and its response.
    pub request_id: UploadRequestId,
}

impl EventBusMessage for RemoteUploadRequest {}

/// A request to download chunks from a remote peer
#[derive(Clone)]
pub struct RemoteDownloadRequest {
    /// The key of the file to download chunks from
    pub file_key: FileKey,
    /// Set of unique chunk IDs to download. Using HashSet to enforce uniqueness
    pub chunk_ids: HashSet<ChunkId>,
    /// Optional bucket ID for bucket operations
    pub bucket_id: Option<BucketId>,
    /// Unique identifier for this download request
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
