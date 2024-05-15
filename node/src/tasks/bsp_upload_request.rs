use std::path::Path;

use sc_tracing::tracing::{error, info, warn};
use shc_common::types::HasherOutT;

use file_manager::traits::{FileStorage, FileStorageWriteError, FileStorageWriteOutcome};
use storage_hub_infra::event_bus::EventHandler;
use tokio::{fs::File, io::AsyncWriteExt};

use crate::services::{
    file_transfer::{commands::FileTransferServiceInterface, events::RemoteUploadRequest},
    handler::{StorageHubHandler, StorageHubHandlerConfig},
};

const LOG_TARGET: &str = "bsp-upload-request-handler";

pub struct BspUploadRequestHandler<SHC: StorageHubHandlerConfig> {
    storage_hub_handler: StorageHubHandler<SHC>,
}

impl<SHC: StorageHubHandlerConfig> Clone for BspUploadRequestHandler<SHC> {
    fn clone(&self) -> BspUploadRequestHandler<SHC> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<SHC: StorageHubHandlerConfig> BspUploadRequestHandler<SHC> {
    pub fn new(storage_hub_handler: StorageHubHandler<SHC>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<SHC: StorageHubHandlerConfig> EventHandler<RemoteUploadRequest>
    for BspUploadRequestHandler<SHC>
where
    HasherOutT<SHC::TrieLayout>: TryFrom<[u8; 32]>,
{
    async fn handle_event(&self, event: RemoteUploadRequest) -> anyhow::Result<()> {
        if !event.chunk_with_proof.verify() {
            error!(
                target: LOG_TARGET,
                "Received invalid proof for chunk: {} (file: {:?}))", event.chunk_with_proof.proven.key, event.file_key
            );
            // TODO: Record this for further reputation actions
            return Ok(());
        }

        let file_key: HasherOutT<SHC::TrieLayout> = TryFrom::try_from(*event.file_key.as_ref())
            .map_err(|_| anyhow::anyhow!("File key and HasherOutT mismatch!"))?;

        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;
        let write_chunk_result = write_file_storage.write_chunk(
            &file_key,
            &event.chunk_with_proof.proven.key,
            &event.chunk_with_proof.proven.data,
        );
        drop(write_file_storage);

        match write_chunk_result {
            Ok(outcome) => match outcome {
                FileStorageWriteOutcome::FileComplete => self.on_file_complete(&file_key).await,
                FileStorageWriteOutcome::FileIncomplete => {}
            },
            Err(error) => match error {
                FileStorageWriteError::FileChunkAlreadyExists => {
                    warn!(
                        target: LOG_TARGET,
                        "Received duplicate chunk with key: {}",
                        event.chunk_with_proof.proven.key
                    );
                    // TODO: Record this for further reputation actions
                }
                FileStorageWriteError::FailedToGetFileChunk
                | FileStorageWriteError::FailedToInsertFileChunk => {
                    error!(
                        target: LOG_TARGET,
                        "Internal trie read/write error {:?}:{}",
                        event.file_key,
                        event.chunk_with_proof.proven.key
                    );
                }
                FileStorageWriteError::FileDoesNotExist => {
                    error!(
                        target: LOG_TARGET,
                        "File does not exist for key {:?}. Maybe we forgot to unregister before deleting?", event.file_key
                    );
                }
                FileStorageWriteError::FingerprintAndStoredFileMismatch => {
                    error!(
                        target: LOG_TARGET,
                        "Invariant broken! Fingerprint and stored file mismatch for key {:?}.", event.file_key
                    );
                }
            },
        }

        Ok(())
    }
}

impl<SHC: StorageHubHandlerConfig> BspUploadRequestHandler<SHC> {
    async fn on_file_complete(&self, file_key: &HasherOutT<SHC::TrieLayout>) {
        info!(target: LOG_TARGET, "File upload complete ({:?})", file_key);

        // Unregister the file from the file transfer service.
        self.storage_hub_handler
            .file_transfer
            .unregister_file(file_key.as_ref().into())
            .await
            .expect("File is not registered. This should not happen!");

        // Get the metadata for the file.
        let read_file_storage = self.storage_hub_handler.file_storage.read().await;
        let metadata = read_file_storage
            .get_metadata(file_key)
            .expect("File metadata not found");
        drop(read_file_storage);

        // TODO: update the forest storage with the new file metadata & send the proof to runtime
        // Save the newly stored file metadata in the forest storage.
        let write_forest_storage = self.storage_hub_handler.forest_storage.write().await;
        // write_forest_storage.insert_file_key(file_key.as_bytes().into(), metadata);
        let read_forest_storage = write_forest_storage.downgrade();
        // read_forest_storage.generate_proof(file_key);
        drop(read_forest_storage);

        // TODO: put this under an RPC call
        let file_path = Path::new("./storage/").join(
            String::from_utf8(metadata.location.clone())
                .expect("File location should be an utf8 string"),
        );
        info!("Saving file to: {:?}", file_path);
        let mut file = File::create(file_path)
            .await
            .expect("Failed to open file for writing.");

        let read_file_storage = self.storage_hub_handler.file_storage.read().await;
        for chunk_id in 0..metadata.chunk_count() {
            let chunk = read_file_storage
                .get_chunk(file_key, &chunk_id)
                .expect("Chunk not found in storage.");
            file.write_all(&chunk)
                .await
                .expect("Failed to write file chunk.");
        }
        drop(read_file_storage);
    }
}
