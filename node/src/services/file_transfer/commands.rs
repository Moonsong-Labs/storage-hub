use anyhow::Result;
use sc_network::PeerId;
use storage_hub_infra::actor::ActorHandle;

use crate::services::FileTransferService;

/// Messages understood by the FileTransfer service actor
#[derive(Debug)]
pub enum FileTransferServiceCommand {
    // TODO: use proper types for the proofs: FileProof.
    UploadRequest { peer_id: PeerId, data: Vec<u8> },
}

/// Allows our ActorHandle to implement
/// the specific methods for each kind of message.
pub trait FileTransferServiceInterface {
    async fn upload_request(&self, peer_id: PeerId, data: Vec<u8>) -> Result<()>;
}

impl FileTransferServiceInterface for ActorHandle<FileTransferService> {
    async fn upload_request(&self, peer_id: PeerId, data: Vec<u8>) -> Result<()> {
        let (_callback, rx) = tokio::sync::oneshot::channel();
        let command = FileTransferServiceCommand::UploadRequest { peer_id, data };
        self.send(command).await;
        rx.await.expect("Failed to received response from FileTransferService. Probably means FileTransferService has crashed.")
    }
}
