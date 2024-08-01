use std::{str::FromStr, time::Duration};

use anyhow::anyhow;
use sc_network::PeerId;
use sc_tracing::tracing::*;
use shp_constants::H_LENGTH;
use sp_core::H256;
use sp_runtime::AccountId32;
use sp_trie::TrieLayout;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceInterface,
    events::{BspConfirmedStoring, NewStorageRequest},
};
use shc_common::types::{FileKey, FileMetadata, HasherOutT};
use shc_file_manager::traits::{FileStorage, FileStorageWriteError, FileStorageWriteOutcome};
use shc_file_transfer_service::{
    commands::FileTransferServiceInterface, events::RemoteUploadRequest,
};
use shc_forest_manager::traits::ForestStorage;

use crate::services::handler::StorageHubHandler;

const LOG_TARGET: &str = "bsp-upload-file-task";

/// BSP Upload File Task: Handles the whole flow of a file being uploaded to a BSP, from
/// the BSP's perspective.
///
/// The flow is split into three parts, which are represented here as 3 handlers for 3
/// different events:
/// - [`NewStorageRequest`] event: The first part of the flow. It is triggered by an
///   on-chain event of a user submitting a storage request to StorageHub. It responds
///   by sending a volunteer transaction and registering the interest of this BSP in
///   receiving the file.
/// - [`RemoteUploadRequest`] event: The second part of the flow. It is triggered by a
///   user sending a chunk of the file to the BSP. It checks the proof for the chunk
///   and if it is valid, stores it, until the whole file is stored.
/// - [`BspConfirmedStoring`] event: The third part of the flow. It is triggered by the
///   runtime confirming that the BSP is now storing the file so that the BSP can update
///   its Forest storage.
pub struct BspUploadFileTask<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    storage_hub_handler: StorageHubHandler<T, FL, FS>,
    file_key_cleanup: Option<HasherOutT<T>>,
}

impl<T, FL, FS> Clone for BspUploadFileTask<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
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
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
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
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
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
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
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
                | FileStorageWriteError::FailedToInsertFileChunk
                | FileStorageWriteError::FailedToDeleteChunk
                | FileStorageWriteError::FailedToPersistChanges
                | FileStorageWriteError::FailedToParseFileMetadata
                | FileStorageWriteError::FailedToParseFingerprint
                | FileStorageWriteError::FailedToReadStorage
                | FileStorageWriteError::FailedToUpdatePartialRoot
                | FileStorageWriteError::FailedToParsePartialRoot
                | FileStorageWriteError::FailedToGetStoredChunksCount => {
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

/// Handles the `BspConfirmedStoring` event.
///
/// This event is triggered by the runtime confirming that the BSP is now storing the file.
impl<T, FL, FS> EventHandler<BspConfirmedStoring> for BspUploadFileTask<T, FL, FS>
where
    T: TrieLayout + Send + Sync + 'static,
    FL: FileStorage<T> + Send + Sync,
    FS: ForestStorage<T> + Send + Sync + 'static,
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    async fn handle_event(&mut self, event: BspConfirmedStoring) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Runtime confirmed BSP storing file: {:?}",
            event.file_key,
        );

        let file_key: HasherOutT<T> = TryFrom::<[u8; 32]>::try_from(*event.file_key.as_ref())
            .map_err(|_| anyhow::anyhow!("File key and HasherOutT mismatch!"))?;

        // Get the metadata of the stored file.
        let read_file_storage = self.storage_hub_handler.file_storage.read().await;
        let file_metadata = read_file_storage
            .get_metadata(&file_key)
            .expect("Failed to get metadata.");
        // Release the file storage lock.
        drop(read_file_storage);

        // Save [`FileMetadata`] of the newly confirmed stored file in the forest storage.
        let mut write_forest_storage = self.storage_hub_handler.forest_storage.write().await;
        write_forest_storage
            .insert_metadata(&file_metadata)
            .expect("Failed to insert metadata.");
        // Release the forest storage lock.
        drop(write_forest_storage);

        Ok(())
    }
}

impl<T, FL, FS> BspUploadFileTask<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    async fn handle_new_storage_request_event(
        &mut self,
        event: NewStorageRequest,
    ) -> anyhow::Result<()>
    where
        HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
    {
        // Construct file metadata.
        let metadata = FileMetadata {
            owner: <AccountId32 as AsRef<[u8]>>::as_ref(&event.who).to_vec(),
            bucket_id: event.bucket_id.as_ref().to_vec(),
            file_size: event.size as u64,
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

        // Get the node's public key needed for threshold calculation.
        let node_public_key = self
            .storage_hub_handler
            .blockchain
            .get_node_public_key()
            .await;

        // Query runtime for the earliest block where the BSP can volunteer for the file.
        let earliest_volunteer_block = self
            .storage_hub_handler
            .blockchain
            .query_file_earliest_volunteer_block(
                node_public_key,
                H256::from_slice(file_key.as_ref()),
            )
            .await
            .map_err(|e| anyhow!("Failed to query file earliest volunteer block: {:?}", e))?;

        // TODO: if the earliest block is too far away, we should drop the task.
        // TODO: based on the limit above, also add a timeout for the task.
        self.storage_hub_handler
            .blockchain
            .wait_for_block(earliest_volunteer_block)
            .await?;

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

        // Also optimistically create file in file storage so we can write uploaded chunks as soon as possible.
        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;
        write_file_storage
            .insert_file(metadata.file_key::<<T as TrieLayout>::Hash>(), metadata)
            .map_err(|e| anyhow!("Failed to insert file in file storage: {:?}", e))?;
        drop(write_file_storage);

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

        // TODO: Handle error
        let _ = write_file_storage.delete_file(&file_key);

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

        // Query runtime for the chunks to prove for the file.
        let chunks_to_prove = self
            .storage_hub_handler
            .blockchain
            .query_bsp_confirm_chunks_to_prove_for_file(
                self.storage_hub_handler
                    .blockchain
                    .get_node_public_key()
                    .await,
                H256::from_slice(file_key.as_ref()),
            )
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to query BSP confirm chunks to prove for file: {:?}",
                    e
                )
            })?;

        // Get the metadata for the file.
        let read_file_storage = self.storage_hub_handler.file_storage.read().await;
        let _metadata = read_file_storage
            .get_metadata(file_key)
            .expect("File metadata not found");
        let added_file_key_proof = read_file_storage
            .generate_proof(file_key, &chunks_to_prove)
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

        Ok(())
    }
}
