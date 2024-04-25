use sc_network::{PeerId, ProtocolName};
use storage_hub_infra::actor::ActorHandle;

use crate::services::FileTransferService;

/// Messages understood by the FileTransfer service actor
#[derive(Debug)]
pub enum FileTransferServiceCommand {
    // TODO: use proper types for the proofs: FileProof.
    UploadRequest { data: String }
}

/// Allows our ActorHandle to implement
/// the specific methods for each kind of message.
pub trait FileTransferServiceInterface {
    fn upload_request(&self, data: String);
}

impl FileTransferServiceInterface for ActorHandle<FileTransferService> {
    fn upload_request(&self, data: String) {
        
    }
}
