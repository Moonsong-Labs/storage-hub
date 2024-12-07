use sc_tracing::tracing::{error, info};
use shc_actors_framework::event_bus::EventHandler;
use shc_file_transfer_service::{
    commands::FileTransferServiceInterface, events::RemoteDownloadRequest,
};

use crate::services::handler::StorageHubHandler;
use crate::tasks::{BspForestStorageHandlerT, FileStorageT};

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
        info!(target: LOG_TARGET, "Received remote download request with id {:?} for file {:?}", event.request_id, event.file_key);

        let chunk_id = event.chunk_id;
        let request_id = event.request_id;
        let bucket_id = event.bucket_id;

        let file_storage_read_lock = self.storage_hub_handler.file_storage.read().await;
        let file_metadata = file_storage_read_lock
            .get_metadata(&event.file_key.into())
            .map_err(|_| anyhow::anyhow!("Failed to get file metadata"))?;

        let file_metadata = if let Some(file_metadata) = file_metadata {
            file_metadata
        } else {
            error!(target: LOG_TARGET, "File not found in file storage");
            return Err(anyhow::anyhow!("File not found in file storage"));
        };

        if let Some(bucket_id) = bucket_id {
            if file_metadata.bucket_id != bucket_id.as_ref().to_vec() {
                error!(
                    target: LOG_TARGET,
                    "File bucket mismatch for file {:?}: expected {:?}, got {:?}",
                    event.file_key, file_metadata.bucket_id, bucket_id
                );
                return Err(anyhow::anyhow!("File bucket mismatch"));
            }
        }

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
