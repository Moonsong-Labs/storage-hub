use storage_hub_infra::actor::ActorHandle;

use crate::services::FileTransferService;

#[derive(Debug)]
pub enum FileTransferServiceCommand {
    EstablishConnection {
        multiaddresses: Vec<String>,
    },
    SendFile { file: Vec<u8> }
}

pub trait FileTransferServiceInterface {
    fn establish_connection(&self, multiaddresses: Vec<String>) -> anyhow::Result<()>;

    fn send_file(&self, file: Vec<u8>) -> anyhow::Result<()>;
}

impl FileTransferServiceInterface for ActorHandle<FileTransferService> {
    fn establish_connection(&self, multiaddresses: Vec<String>) -> anyhow::Result<()> {
        Ok(())
    }

    fn send_file(&self, file: Vec<u8>) -> anyhow::Result<()> {
        Ok(())
    }
}