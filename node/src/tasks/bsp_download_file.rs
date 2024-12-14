use sc_tracing::tracing::{error, trace};
use shc_actors_framework::event_bus::EventHandler;
use shc_file_transfer_service::{
    commands::FileTransferServiceInterface, events::RemoteDownloadRequest,
};

use crate::{
    services::handler::StorageHubHandler,
    tasks::{BspForestStorageHandlerT, FileStorageT},
};

const LOG_TARGET: &str = "bsp-download-file-task";

pub struct BspDownloadFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<FL, FSH>,
}

impl<FL, FSH> Clone for BspDownloadFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    fn clone(&self) -> BspDownloadFileTask<FL, FSH> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<FL, FSH> BspDownloadFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    pub fn new(storage_hub_handler: StorageHubHandler<FL, FSH>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<FL, FSH> EventHandler<RemoteDownloadRequest> for BspDownloadFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: RemoteDownloadRequest) -> anyhow::Result<()> {
        trace!(target: LOG_TARGET, "Received remote download request with id {:?} for file {:?}", event.request_id, event.file_key);

        let chunk_id = event.chunk_id;
        let request_id = event.request_id;

        let file_storage_read_lock = self.storage_hub_handler.file_storage.read().await;
        let generate_proof_result =
            file_storage_read_lock.generate_proof(&event.file_key.into(), &vec![chunk_id]);

        match generate_proof_result {
            Ok(file_key_proof) => {
                self.storage_hub_handler
                    .file_transfer
                    .download_response(file_key_proof, request_id)
                    .await?;
            }
            Err(error) => {
                error!(target: LOG_TARGET, "Failed to generate proof for chunk id {:?} of file {:?}", chunk_id, event.file_key);
                return Err(anyhow::anyhow!("{:?}", error));
            }
        }

        Ok(())
    }
}
