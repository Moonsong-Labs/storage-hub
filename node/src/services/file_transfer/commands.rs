use anyhow::Result;
use sc_network::PeerId;
use storage_hub_infra::{
    actor::ActorHandle,
    types::{Chunk, FileProof, Key},
};

use crate::services::FileTransferService;

// Placeholder for the actual types
type ChunkId = u64;

/// Messages understood by the FileTransfer service actor
pub enum FileTransferServiceCommand {
    // TODO: use proper types for the proofs: FileProof.
    UploadRequest {
        peer_id: PeerId,
        file_key: Key,
        chunk_with_proof: FileProof<Chunk>,
        callback: tokio::sync::oneshot::Sender<Result<()>>,
    },
    DownloadRequest {
        peer_id: PeerId,
        file_key: Key,
        chunk_id: ChunkId,
        callback: tokio::sync::oneshot::Sender<Result<()>>,
    },
}

/// Allows our ActorHandle to implement
/// the specific methods for each kind of message.
pub trait FileTransferServiceInterface {
    async fn upload_request(
        &self,
        peer_id: PeerId,
        file_key: Key,
        data: FileProof<Chunk>,
    ) -> Result<()>;

    async fn download_request(
        &self,
        peer_id: PeerId,
        file_key: Key,
        chunk_id: ChunkId,
    ) -> Result<()>;
}

impl FileTransferServiceInterface for ActorHandle<FileTransferService> {
    async fn upload_request(
        &self,
        peer_id: PeerId,
        file_key: Key,
        chunk_with_proof: FileProof<Chunk>,
    ) -> Result<()> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let command = FileTransferServiceCommand::UploadRequest {
            peer_id,
            file_key,
            chunk_with_proof,
            callback,
        };
        self.send(command).await;
        rx.await.expect("Failed to received response from FileTransferService. Probably means FileTransferService has crashed.")
    }

    async fn download_request(
        &self,
        peer_id: PeerId,
        file_key: Key,
        chunk_id: ChunkId,
    ) -> Result<()> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let command = FileTransferServiceCommand::DownloadRequest {
            peer_id,
            file_key,
            chunk_id,
            callback,
        };
        self.send(command).await;
        rx.await.expect("Failed to received response from FileTransferService. Probably means FileTransferService has crashed.")
    }
}
