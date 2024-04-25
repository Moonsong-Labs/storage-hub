use sc_network::{PeerId, ProtocolName};
use storage_hub_infra::actor::ActorHandle;

use crate::services::FileTransferService;

/// Messages understood by the FileTransfer service actor
#[derive(Debug)]
pub enum FileTransferServiceCommand {
    SendRequest { target: PeerId, protocol_name: ProtocolName, Request: Vec<u8> },
    // TODO: use proper types for the proofs: FileProof.
    UploadRequest { data: String }
}

/// Allows our ActorHandle to implement
/// the specific methods for each kind of message.
pub trait FileTransferServiceInterface {
    fn send_request(&self, target: PeerId, protocol_name: ProtocolName, request: Vec<u8>);

    fn upload_request(&self, data: String);
}

impl FileTransferServiceInterface for ActorHandle<FileTransferService> {
    fn send_request(&self, target: PeerId, protocol_name: ProtocolName, request: Vec<u8>) {
    }

    fn upload_request(&self, data: String) {
        
    }
}
