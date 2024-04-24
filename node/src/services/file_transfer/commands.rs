use storage_hub_infra::actor::ActorHandle;

use crate::services::FileTransferService;

/// Messages understood by the FileTransfer service actor
#[derive(Debug)]
pub enum FileTransferServiceCommand {
    EstablishConnection { multiaddresses: Vec<String> },
    SendFile { file: Vec<u8> },
}

/// Allows our ActorHandle to implement
/// the specific methods for each kind of message.
pub trait FileTransferServiceInterface {
    fn establish_connection(&self, multiaddresses: Vec<String>) -> anyhow::Result<()>;

    fn send_file(&self, file: Vec<u8>) -> anyhow::Result<()>;
}

impl FileTransferServiceInterface for ActorHandle<FileTransferService> {
    fn establish_connection(&self, _multiaddresses: Vec<String>) -> anyhow::Result<()> {
        Ok(())
    }

    fn send_file(&self, _file: Vec<u8>) -> anyhow::Result<()> {
        Ok(())
    }
}
