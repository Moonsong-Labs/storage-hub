use sc_tracing::tracing::{error, info};
use shc_actors_framework::event_bus::EventHandler;
use shc_common::types::StorageProofsMerkleTrieLayout;
use shc_file_manager::traits::FileStorage;
use shc_file_transfer_service::{
    commands::FileTransferServiceInterface, events::RemoteDownloadRequest,
};
use shc_forest_manager::traits::ForestStorage;

use crate::services::handler::StorageHubHandler;

const LOG_TARGET: &str = "bsp-download-file-task";

pub struct BspDownloadFileTask<FL, FS>
where
    FL: Send + Sync + FileStorage<StorageProofsMerkleTrieLayout>,
    FS: Send + Sync + ForestStorage<StorageProofsMerkleTrieLayout>,
{
    storage_hub_handler: StorageHubHandler<FL, FS>,
}

impl<FL, FS> Clone for BspDownloadFileTask<FL, FS>
where
    FL: Send + Sync + FileStorage<StorageProofsMerkleTrieLayout>,
    FS: Send + Sync + ForestStorage<StorageProofsMerkleTrieLayout>,
{
    fn clone(&self) -> BspDownloadFileTask<FL, FS> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<FL, FS> BspDownloadFileTask<FL, FS>
where
    FL: Send + Sync + FileStorage<StorageProofsMerkleTrieLayout>,
    FS: Send + Sync + ForestStorage<StorageProofsMerkleTrieLayout>,
{
    pub fn new(storage_hub_handler: StorageHubHandler<FL, FS>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<FL, FS> EventHandler<RemoteDownloadRequest> for BspDownloadFileTask<FL, FS>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync + 'static,
{
    async fn handle_event(&mut self, event: RemoteDownloadRequest) -> anyhow::Result<()> {
        info!(target: LOG_TARGET, "Received remote download request with id {:?} for file {:?}", event.request_id, event.file_key);

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
