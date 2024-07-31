use sc_tracing::tracing::{error, info};
use shc_actors_framework::event_bus::EventHandler;
use shc_common::types::HasherOutT;
use shc_file_manager::traits::FileStorage;
use shc_file_transfer_service::{
    commands::FileTransferServiceInterface, events::RemoteDownloadRequest,
};
use shc_forest_manager::traits::ForestStorage;
use shp_constants::H_LENGTH;
use sp_trie::TrieLayout;

use crate::services::handler::StorageHubHandler;

const LOG_TARGET: &str = "bsp-download-file-task";

pub struct BspDownloadFileTask<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    storage_hub_handler: StorageHubHandler<T, FL, FS>,
}

impl<T, FL, FS> Clone for BspDownloadFileTask<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    fn clone(&self) -> BspDownloadFileTask<T, FL, FS> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<T, FL, FS> BspDownloadFileTask<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    pub fn new(storage_hub_handler: StorageHubHandler<T, FL, FS>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<T, FL, FS> EventHandler<RemoteDownloadRequest> for BspDownloadFileTask<T, FL, FS>
where
    T: TrieLayout + Send + Sync + 'static,
    FL: FileStorage<T> + Send + Sync,
    FS: ForestStorage<T> + Send + Sync + 'static,
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    async fn handle_event(&mut self, event: RemoteDownloadRequest) -> anyhow::Result<()> {
        info!(target: LOG_TARGET, "Received remote download request with id {:?} for file {:?}", event.request_id, event.file_key);

        let chunk_id = event.chunk_id;
        let request_id = event.request_id;
        // TODO: use helper method defined in RocksDbFileStorage
        let file_key: HasherOutT<T> = TryFrom::try_from(*event.file_key.as_ref())
            .map_err(|_| anyhow::anyhow!("File key and HasherOutT mismatch!"))?;

        let file_storage_read_lock = self.storage_hub_handler.file_storage.read().await;
        let generate_proof_result =
            file_storage_read_lock.generate_proof(&file_key, &vec![chunk_id]);

        match generate_proof_result {
            Ok(file_key_proof) => {
                self.storage_hub_handler
                    .file_transfer
                    .download_response(file_key_proof, request_id)
                    .await?;
            }
            Err(error) => {
                error!(target: LOG_TARGET, "Failed to generate proof for chunk id {:?} of file {:?}", chunk_id, file_key);
                return Err(anyhow::anyhow!("{:?}", error));
            }
        }

        Ok(())
    }
}
