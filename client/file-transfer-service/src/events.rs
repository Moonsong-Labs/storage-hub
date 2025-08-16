use sc_network::PeerId;
use shc_actors_derive::{ActorEvent, ActorEventBus};
use shc_common::{
    traits::StorageEnableRuntime,
    types::{BucketId, ChunkId, DownloadRequestId, FileKey, FileKeyProof, UploadRequestId},
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
#[derive(Clone, ActorEvent)]
#[actor(actor = "file_transfer_service", generics(Runtime: StorageEnableRuntime))]
pub struct RemoteUploadRequest<Runtime: StorageEnableRuntime> {
    /// The peer ID of the receiver node.
    pub peer: PeerId,
    /// File key of the file which is being uploaded.
    pub file_key: FileKey,
    /// Proof containing the file chunk(s) which are being uploaded.
    pub file_key_proof: FileKeyProof,
    /// Optional bucket identifier for file organization only required based on the receiver's implementation.
    pub bucket_id: Option<BucketId<Runtime>>,
    /// Unique identifier for tracking the upload request and its response.
    pub request_id: UploadRequestId,
}

/// A request to download chunks from a remote peer
#[derive(Clone, ActorEvent)]
#[actor(actor = "file_transfer_service", generics(Runtime: StorageEnableRuntime))]
pub struct RemoteDownloadRequest<Runtime: StorageEnableRuntime> {
    /// The key of the file to download chunks from
    pub file_key: FileKey,
    /// Set of unique chunk IDs to download. Using HashSet to enforce uniqueness
    pub chunk_ids: HashSet<ChunkId>,
    /// Optional bucket ID for bucket operations
    pub bucket_id: Option<BucketId<Runtime>>,
    /// Unique identifier for this download request
    pub request_id: DownloadRequestId,
}

/// Event triggered to retry pending bucket move downloads.
/// This is emitted on startup and will be periodically emitted later to ensure
/// any interrupted downloads can be resumed.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "file_transfer_service")]
pub struct RetryBucketMoveDownload;

#[ActorEventBus("file_transfer_service")]
pub struct FileTransferServiceEventBusProvider<Runtime: StorageEnableRuntime>;
