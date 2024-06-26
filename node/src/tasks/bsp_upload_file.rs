use std::{fs::create_dir_all, path::Path, str::FromStr, time::Duration};

use anyhow::anyhow;
use sc_network::PeerId;
use sc_tracing::tracing::*;
use shp_file_key_verifier::consts::H_LENGTH;
use shp_file_key_verifier::types::ChunkId;
use sp_core::H256;
use sp_runtime::AccountId32;
use sp_trie::TrieLayout;
use tokio::{fs::File, io::AsyncWriteExt};

use shc_actors_framework::event_bus::EventHandler;
use shc_common::types::{FileKey, FileMetadata, HasherOutT};
use shc_file_manager::traits::{FileStorage, FileStorageWriteError, FileStorageWriteOutcome};
use shc_forest_manager::traits::ForestStorage;

use crate::services::{
    blockchain::{commands::BlockchainServiceInterface, events::NewStorageRequest},
    file_transfer::{commands::FileTransferServiceInterface, events::RemoteUploadRequest},
    handler::StorageHubHandler,
};

const LOG_TARGET: &str = "bsp-upload-file-task";

/// BSP Upload File Task: Handles the whole flow of a file being uploaded to a BSP, from
/// the BSP's perspective.
///
/// The flow is split into two parts, which are represented here as two handlers for two
/// different events:
/// - `NewStorageRequest` event: The first part of the flow. It is triggered by an
///   on-chain event of a user submitting a storage request to StorageHub. It responds
///   by sending a volunteer transaction and registering the interest of this BSP in
///   receiving the file.
/// - `RemoteUploadRequest` event: The second part of the flow. It is triggered by a
///   user sending a chunk of the file to the BSP. It checks the proof for the chunk
///   and if it is valid, stores it, until the whole file is stored.
pub struct BspUploadFileTask<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    storage_hub_handler: StorageHubHandler<T, FL, FS>,
    file_key_cleanup: Option<HasherOutT<T>>,
}

impl<T, FL, FS> Clone for BspUploadFileTask<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    fn clone(&self) -> BspUploadFileTask<T, FL, FS> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            file_key_cleanup: self.file_key_cleanup,
        }
    }
}

impl<T, FL, FS> BspUploadFileTask<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    pub fn new(storage_hub_handler: StorageHubHandler<T, FL, FS>) -> Self {
        Self {
            storage_hub_handler,
            file_key_cleanup: None,
        }
    }
}

/// Handles the `NewStorageRequest` event.
///
/// This event is triggered by an on-chain event of a user submitting a storage request to StorageHub.
/// It responds by sending a volunteer transaction and registering the interest of this BSP in
/// receiving the file. This task optimistically assumes the transaction will succeed, and registers
/// the user and file key in the registry of the File Transfer Service, which handles incoming p2p
/// upload requests.
impl<T, FL, FS> EventHandler<NewStorageRequest> for BspUploadFileTask<T, FL, FS>
where
    T: TrieLayout + Send + Sync + 'static,
    FL: FileStorage<T> + Send + Sync,
    FS: ForestStorage<T> + Send + Sync + 'static,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    async fn handle_event(&mut self, event: NewStorageRequest) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Initiating BSP volunteer for location: {:?}, fingerprint: {:?}",
            event.location,
            event.fingerprint
        );

        let result = self.handle_new_storage_request_event(event).await;
        if result.is_err() {
            if let Some(file_key) = &self.file_key_cleanup {
                self.unvolunteer_file(*file_key).await?;
            }
        }
        result
    }
}

/// Handles the `RemoteUploadRequest` event.
///
/// This event is triggered by a user sending a chunk of the file to the BSP. It checks the proof
/// for the chunk and if it is valid, stores it, until the whole file is stored.
impl<T, FL, FS> EventHandler<RemoteUploadRequest> for BspUploadFileTask<T, FL, FS>
where
    T: TrieLayout + Send + Sync + 'static,
    FL: FileStorage<T> + Send + Sync,
    FS: ForestStorage<T> + Send + Sync + 'static,
    <T::Hash as sp_core::Hasher>::Out: TryFrom<[u8; H_LENGTH]>,
{
    async fn handle_event(&mut self, event: RemoteUploadRequest) -> anyhow::Result<()> {
        info!(target: LOG_TARGET, "Received remote upload request for file {:?} and peer {:?}", event.file_key, event.peer);

        let file_key: HasherOutT<T> = TryFrom::try_from(*event.file_key.as_ref())
            .map_err(|_| anyhow::anyhow!("File key and HasherOutT mismatch!"))?;

        let proven = match event.file_key_proof.proven::<T>() {
            Ok(proven) => {
                if proven.len() != 1 {
                    Err(anyhow::anyhow!("Expected exactly one proven chunk."))
                } else {
                    Ok(proven[0].clone())
                }
            }
            Err(e) => Err(anyhow::anyhow!(
                "Failed to verify and get proven file key chunks: {:?}",
                e
            )),
        };

        let proven = match proven {
            Ok(proven) => proven,
            Err(e) => {
                warn!(target: LOG_TARGET, "{}", e);

                // Unvolunteer the file.
                self.unvolunteer_file(file_key).await?;
                return Err(e);
            }
        };

        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;
        let write_chunk_result =
            write_file_storage.write_chunk(&file_key, &proven.key, &proven.data);
        // Release the file storage write lock as soon as possible.
        drop(write_file_storage);

        match write_chunk_result {
            Ok(outcome) => match outcome {
                FileStorageWriteOutcome::FileComplete => self.on_file_complete(&file_key).await?,
                FileStorageWriteOutcome::FileIncomplete => {}
            },
            Err(error) => match error {
                FileStorageWriteError::FileChunkAlreadyExists => {
                    warn!(
                        target: LOG_TARGET,
                        "Received duplicate chunk with key: {:?}",
                        proven.key
                    );

                    // TODO: Consider informing this to the file transfer service so that it can handle reputation for this peer id.
                }
                FileStorageWriteError::FileDoesNotExist => {
                    // Unvolunteer the file.
                    self.unvolunteer_file(file_key).await?;

                    return Err(anyhow::anyhow!(format!("File does not exist for key {:?}. Maybe we forgot to unregister before deleting?", event.file_key)));
                }
                FileStorageWriteError::FailedToGetFileChunk
                | FileStorageWriteError::FailedToInsertFileChunk => {
                    // This internal error should not happen.

                    // Unvolunteer the file.
                    self.unvolunteer_file(file_key).await?;

                    return Err(anyhow::anyhow!(format!(
                        "Internal trie read/write error {:?}:{:?}",
                        event.file_key, proven.key
                    )));
                }
                FileStorageWriteError::FingerprintAndStoredFileMismatch => {
                    // This should never happen, given that the first check in the handler is verifying the proof.
                    // This means that something is seriously wrong, so we error out the whole task.

                    // Unvolunteer the file.
                    self.unvolunteer_file(file_key).await?;

                    return Err(anyhow::anyhow!(format!(
                        "Invariant broken! This is a bug! Fingerprint and stored file mismatch for key {:?}.",
                        event.file_key
                    )));
                }
                FileStorageWriteError::FailedToConstructTrieIter => {
                    // This should never happen for a well constructed trie.
                    // This means that something is seriously wrong, so we error out the whole task.

                    // Unvolunteer the file.
                    self.unvolunteer_file(file_key).await?;

                    return Err(anyhow::anyhow!(format!(
                        "This is a bug! Failed to construct trie iter for key {:?}.",
                        event.file_key
                    )));
                }
            },
        }

        Ok(())
    }
}

impl<T, FL, FS> BspUploadFileTask<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    async fn handle_new_storage_request_event(
        &mut self,
        event: NewStorageRequest,
    ) -> anyhow::Result<()>
    where
        HasherOutT<T>: TryFrom<[u8; 32]>,
    {
        // Construct file metadata.
        let metadata = FileMetadata {
            owner: <AccountId32 as AsRef<[u8]>>::as_ref(&event.who).to_vec(),
            size: event.size as u64,
            fingerprint: event.fingerprint,
            location: event.location.to_vec(),
        };

        // Get the file key.
        let file_key: FileKey = metadata
            .file_key::<<T as TrieLayout>::Hash>()
            .as_ref()
            .try_into()?;

        let file_key_hash: HasherOutT<T> = TryFrom::<[u8; 32]>::try_from(*file_key.as_ref())
            .map_err(|_| anyhow::anyhow!("File key and HasherOutT mismatch!"))?;
        self.file_key_cleanup = Some(file_key_hash);

        // Optimistically register the file for upload in the file transfer service.
        // This solves the race condition between the user and the BSP, where the user could react faster
        // to the BSP volunteering than the BSP, and therefore initiate a new upload request before the
        // BSP has registered the file and peer ID in the file transfer service.
        for peer_id in event.user_peer_ids.iter() {
            let peer_id = match std::str::from_utf8(&peer_id.as_slice()) {
                Ok(str_slice) => {
                    let owned_string = str_slice.to_string();
                    PeerId::from_str(owned_string.as_str()).map_err(|e| {
                        error!(target: LOG_TARGET, "Failed to convert peer ID to PeerId: {}", e);
                        e
                    })?
                }
                Err(e) => return Err(anyhow!("Failed to convert peer ID to a string: {}", e)),
            };
            self.storage_hub_handler
                .file_transfer
                .register_new_file_peer(peer_id, file_key)
                .await
                .map_err(|e| anyhow!("Failed to register new file peer: {:?}", e))?;
        }

        // Build extrinsic.
        let call =
            storage_hub_runtime::RuntimeCall::FileSystem(pallet_file_system::Call::bsp_volunteer {
                file_key: H256(file_key.into()),
            });

        // Send extrinsic and wait for it to be included in the block.
        self.storage_hub_handler
            .blockchain
            .send_extrinsic(call)
            .await?
            .with_timeout(Duration::from_secs(60))
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await?;

        // Create file in file storage.
        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;
        write_file_storage
            .insert_file(metadata.file_key::<<T as TrieLayout>::Hash>(), metadata)
            .map_err(|e| anyhow!("Failed to insert file in file storage: {:?}", e))?;
        drop(write_file_storage);

        Ok(())
    }

    async fn unvolunteer_file(&self, file_key: HasherOutT<T>) -> anyhow::Result<()> {
        warn!(target: LOG_TARGET, "Unvolunteering file {:?}", file_key);

        // Unregister the file from the file transfer service.
        // The error is ignored, as the file might already be unregistered.
        let _ = self
            .storage_hub_handler
            .file_transfer
            .unregister_file(file_key.as_ref().into())
            .await;

        // Delete the file from the file storage.
        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;
        write_file_storage.delete_file(&file_key);

        // TODO: Send transaction to runtime to unvolunteer the file.

        Ok(())
    }

    async fn on_file_complete(&self, file_key: &HasherOutT<T>) -> anyhow::Result<()> {
        info!(target: LOG_TARGET, "File upload complete ({:?})", file_key);

        // // Unregister the file from the file transfer service.
        // self.storage_hub_handler
        //     .file_transfer
        //     .unregister_file(file_key.as_ref().into())
        //     .await
        //     .expect("File is not registered. This should not happen!");

        // Get the metadata for the file.
        let read_file_storage = self.storage_hub_handler.file_storage.read().await;
        let metadata = read_file_storage
            .get_metadata(file_key)
            .expect("File metadata not found");
        // TODO: generate the file proof for proper chunk ids/challenges.
        let added_file_key_proof = read_file_storage
            .generate_proof(file_key, &vec![ChunkId::new(0)])
            .expect("File is not in storage, or proof does not exist.");
        // Release the file storage read lock as soon as possible.
        drop(read_file_storage);

        // Get a read lock on the forest storage to generate a proof for the file.
        let read_forest_storage = self.storage_hub_handler.forest_storage.read().await;
        let non_inclusion_forest_proof = read_forest_storage
            .generate_proof(vec![*file_key])
            .expect("Failed to generate forest proof.");
        // Release the forest storage read lock.
        drop(read_forest_storage);

        // TODO: send the proof for the new file to the runtime

        // Build extrinsic.
        let call = storage_hub_runtime::RuntimeCall::FileSystem(
            pallet_file_system::Call::bsp_confirm_storing {
                file_key: H256::from_slice(file_key.as_ref()),
                root: H256::from_slice(non_inclusion_forest_proof.root.as_ref()),
                non_inclusion_forest_proof: non_inclusion_forest_proof.proof,
                added_file_key_proof,
            },
        );

        self.storage_hub_handler
            .blockchain
            .send_extrinsic(call)
            .await?
            .with_timeout(Duration::from_secs(60))
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await?;

        // TODO: make this a response to the blockchain event for confirm BSP file storage.
        // Save [`FileMetadata`] of the newly stored file in the forest storage.
        // let mut write_forest_storage = self.storage_hub_handler.forest_storage.write().await;
        // let file_key = write_forest_storage
        //     .insert_metadata(&metadata)
        //     .expect("Failed to insert metadata.");

        // TODO: move this under an RPC call
        let file_path = Path::new("./storage/").join(
            String::from_utf8(metadata.location.clone())
                .expect("File location should be an utf8 string"),
        );
        dbg!(
            "Current dir: {}",
            std::env::current_dir().unwrap().display()
        );
        info!("Intended file path: {:?}", file_path);

        create_dir_all(&file_path.parent().unwrap()).expect("Failed to create directory");
        let mut file = File::create(file_path)
            .await
            .expect("Failed to open file for writing.");

        let read_file_storage = self.storage_hub_handler.file_storage.read().await;
        for chunk_id in 0..metadata.chunks_count() {
            let chunk = read_file_storage
                .get_chunk(&file_key, &ChunkId::new(chunk_id))
                .expect("Chunk not found in storage.");
            file.write_all(&chunk)
                .await
                .expect("Failed to write file chunk.");
        }
        drop(read_file_storage);

        Ok(())
    }
}
