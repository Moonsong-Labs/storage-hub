use std::{str::FromStr, time::Duration};

use anyhow::anyhow;
use frame_support::BoundedVec;
use sc_network::PeerId;
use sc_tracing::tracing::*;
use sp_core::H256;
use sp_runtime::AccountId32;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceInterface,
    events::{NewStorageRequest, ProcessConfirmStoringRequest},
    handler::ConfirmStoringRequest,
};
use shc_common::types::{
    FileKey, FileMetadata, HashT, StorageProofsMerkleTrieLayout, StorageProviderId,
};
use shc_file_manager::traits::{FileStorage, FileStorageWriteError, FileStorageWriteOutcome};
use shc_file_transfer_service::{
    commands::FileTransferServiceInterface, events::RemoteUploadRequest,
};
use shc_forest_manager::traits::ForestStorage;

use crate::services::handler::StorageHubHandler;

const LOG_TARGET: &str = "bsp-upload-file-task";

const MAX_CONFIRM_STORING_REQUEST_TRY_COUNT: usize = 3;

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
/// - [`ProcessConfirmStoringRequest`] event: The third part of the flow. It is triggered by the
///   runtime when the BSP should construct a proof for the new file(s) and submit a confirm storing
///   before updating it's local Forest storage root.
pub struct BspUploadFileTask<FL, FS>
where
    FL: Send + Sync + FileStorage<StorageProofsMerkleTrieLayout>,
    FS: Send + Sync + ForestStorage<StorageProofsMerkleTrieLayout>,
{
    storage_hub_handler: StorageHubHandler<FL, FS>,
    file_key_cleanup: Option<H256>,
}

impl<FL, FS> Clone for BspUploadFileTask<FL, FS>
where
    FL: Send + Sync + FileStorage<StorageProofsMerkleTrieLayout>,
    FS: Send + Sync + ForestStorage<StorageProofsMerkleTrieLayout>,
{
    fn clone(&self) -> BspUploadFileTask<FL, FS> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            file_key_cleanup: self.file_key_cleanup,
        }
    }
}

impl<FL, FS> BspUploadFileTask<FL, FS>
where
    FL: Send + Sync + FileStorage<StorageProofsMerkleTrieLayout>,
    FS: Send + Sync + ForestStorage<StorageProofsMerkleTrieLayout>,
{
    pub fn new(storage_hub_handler: StorageHubHandler<FL, FS>) -> Self {
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
impl<FL, FS> EventHandler<NewStorageRequest> for BspUploadFileTask<FL, FS>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync + 'static,
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
impl<FL, FS> EventHandler<RemoteUploadRequest> for BspUploadFileTask<FL, FS>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync + 'static,
{
    async fn handle_event(&mut self, event: RemoteUploadRequest) -> anyhow::Result<()> {
        info!(target: LOG_TARGET, "Received remote upload request for file {:?} and peer {:?}", event.file_key, event.peer);

        let proven = match event
            .file_key_proof
            .proven::<StorageProofsMerkleTrieLayout>()
        {
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
                self.unvolunteer_file(event.file_key.into()).await?;
                return Err(e);
            }
        };

        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;
        let write_chunk_result =
            write_file_storage.write_chunk(&event.file_key.into(), &proven.key, &proven.data);
        // Release the file storage write lock as soon as possible.
        drop(write_file_storage);

        match write_chunk_result {
            Ok(outcome) => match outcome {
                FileStorageWriteOutcome::FileComplete => {
                    self.on_file_complete(&event.file_key.into()).await?
                }
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
                    self.unvolunteer_file(event.file_key.into()).await?;

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
                    self.unvolunteer_file(event.file_key.into()).await?;

                    return Err(anyhow::anyhow!(format!(
                        "Internal trie read/write error {:?}:{:?}",
                        event.file_key, proven.key
                    )));
                }
                FileStorageWriteError::FingerprintAndStoredFileMismatch => {
                    // This should never happen, given that the first check in the handler is verifying the proof.
                    // This means that something is seriously wrong, so we error out the whole task.

                    // Unvolunteer the file.
                    self.unvolunteer_file(event.file_key.into()).await?;

                    return Err(anyhow::anyhow!(format!(
                        "Invariant broken! This is a bug! Fingerprint and stored file mismatch for key {:?}.",
                        event.file_key
                    )));
                }
                FileStorageWriteError::FailedToConstructTrieIter => {
                    // This should never happen for a well constructed trie.
                    // This means that something is seriously wrong, so we error out the whole task.

                    // Unvolunteer the file.
                    self.unvolunteer_file(event.file_key.into()).await?;

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

/// Handles the `ProcessConfirmStoringRequest` event.
///
/// This event is triggered by the runtime when it decides it is the right time to submit a confirm
/// storing extrinsic (and update the local forest root).
impl<FL, FS> EventHandler<ProcessConfirmStoringRequest> for BspUploadFileTask<FL, FS>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync + 'static,
{
    async fn handle_event(&mut self, event: ProcessConfirmStoringRequest) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing ConfirmStoringRequest: {:?}",
            event.confirm_storing_requests,
        );

        let forest_root_write_tx = match event.forest_root_write_tx.lock().await.take() {
            Some(tx) => tx,
            None => {
                error!(target: LOG_TARGET, "CRITICAL❗️❗️ This is a bug! Forest root write tx already taken.\nThis is a critical bug. Please report it to the StorageHub team.");
                return Err(anyhow!(
                    "CRITICAL❗️❗️ This is a bug! Forest root write tx already taken. Please report it to the StorageHub team."
                ));
            }
        };

        let own_account = self
            .storage_hub_handler
            .blockchain
            .get_node_public_key()
            .await;

        let own_bsp_id = self
            .storage_hub_handler
            .blockchain
            .query_storage_provider_id(own_account)
            .await?;

        let own_bsp_id = match own_bsp_id {
            Some(id) => match id {
                StorageProviderId::MainStorageProvider(_) => {
                    error!(target: LOG_TARGET, "Current node account is a Main Storage Provider. Expected a Backup Storage Provider ID.");
                    return Err(anyhow!(
                        "Current node account is a Main Storage Provider. Expected a Backup Storage Provider ID."
                    ));
                }
                StorageProviderId::BackupStorageProvider(id) => id,
            },
            None => {
                error!(target: LOG_TARGET, "Failed to get own BSP ID.");
                return Err(anyhow!("Failed to get own BSP ID."));
            }
        };

        // Query runtime for the chunks to prove for the file.
        let mut confirm_storing_requests_with_chunks_to_prove = Vec::new();
        for confirm_storing_request in event.confirm_storing_requests.iter() {
            match self
                .storage_hub_handler
                .blockchain
                .query_bsp_confirm_chunks_to_prove_for_file(
                    own_bsp_id,
                    confirm_storing_request.file_key,
                )
                .await
            {
                Ok(chunks_to_prove) => {
                    confirm_storing_requests_with_chunks_to_prove
                        .push((confirm_storing_request, chunks_to_prove));
                }
                Err(e) => {
                    let mut confirm_storing_request = confirm_storing_request.clone();
                    confirm_storing_request.increment_try_count();
                    if confirm_storing_request.try_count > MAX_CONFIRM_STORING_REQUEST_TRY_COUNT {
                        error!(target: LOG_TARGET, "Failed to query chunks to prove for file {:?}: {:?}\nMax try count exceeded! Dropping request!", confirm_storing_request.file_key, e);
                    } else {
                        error!(target: LOG_TARGET, "Failed to query chunks to prove for file {:?}: {:?}\nEnqueuing file key again! (retry {}/{})", confirm_storing_request.file_key, e, confirm_storing_request.try_count, MAX_CONFIRM_STORING_REQUEST_TRY_COUNT);
                        self.storage_hub_handler
                            .blockchain
                            .queue_confirm_bsp_request(confirm_storing_request)
                            .await?;
                    }
                }
            }
        }

        // Generate the proof for the files and get metadatas.
        let read_file_storage = self.storage_hub_handler.file_storage.read().await;
        let mut file_keys_and_proofs = Vec::new();
        let mut file_metadatas = Vec::new();
        for (confirm_storing_request, chunks_to_prove) in
            confirm_storing_requests_with_chunks_to_prove.into_iter()
        {
            match (
                read_file_storage
                    .generate_proof(&confirm_storing_request.file_key, &chunks_to_prove),
                read_file_storage.get_metadata(&confirm_storing_request.file_key),
            ) {
                (Ok(proof), Ok(metadata)) => {
                    file_keys_and_proofs.push((confirm_storing_request.file_key, proof));
                    file_metadatas.push(metadata);
                }
                _ => {
                    let mut confirm_storing_request = confirm_storing_request.clone();
                    confirm_storing_request.increment_try_count();
                    if confirm_storing_request.try_count > MAX_CONFIRM_STORING_REQUEST_TRY_COUNT {
                        error!(target: LOG_TARGET, "Failed to generate proof or get metadatas for file {:?}.\nMax try count exceeded! Dropping request!", confirm_storing_request.file_key);
                    } else {
                        error!(target: LOG_TARGET, "Failed to generate proof or get metadatas for file {:?}.\nEnqueuing file key again! (retry {}/{})", confirm_storing_request.file_key, confirm_storing_request.try_count, MAX_CONFIRM_STORING_REQUEST_TRY_COUNT);
                        self.storage_hub_handler
                            .blockchain
                            .queue_confirm_bsp_request(confirm_storing_request)
                            .await?;
                    }
                }
            }
        }
        // Release the file storage read lock as soon as possible.
        drop(read_file_storage);

        if file_keys_and_proofs.is_empty() {
            error!(target: LOG_TARGET, "Failed to generate proofs for ALL the requested files.\n");
            return Err(anyhow!(
                "Failed to generate proofs for ALL the requested files."
            ));
        }

        let file_keys = file_keys_and_proofs
            .iter()
            .map(|(file_key, _)| *file_key)
            .collect::<Vec<_>>();

        // Get a read lock on the forest storage to generate a proof for the file.
        let read_forest_storage = self.storage_hub_handler.forest_storage.read().await;
        let non_inclusion_forest_proof = read_forest_storage
            .generate_proof(file_keys)
            .map_err(|_| anyhow!("Failed to generate forest proof."))?;
        // Release the forest storage read lock.
        drop(read_forest_storage);

        // Build extrinsic.
        let call = storage_hub_runtime::RuntimeCall::FileSystem(
            pallet_file_system::Call::bsp_confirm_storing {
                non_inclusion_forest_proof: non_inclusion_forest_proof.proof,
                file_keys_and_proofs: BoundedVec::try_from(file_keys_and_proofs)
                .map_err(|_| {
                    error!("CRITICAL❗️❗️ This is a bug! Failed to convert file keys and proofs to BoundedVec. Please report it to the StorageHub team.");
                    anyhow!("Failed to convert file keys and proofs to BoundedVec.")
                })?,
            },
        );

        // Send the confirmation transaction and wait for it to be included in the block and
        // continue only if it is successful.
        self.storage_hub_handler
            .blockchain
            .send_extrinsic(call)
            .await?
            .with_timeout(Duration::from_secs(60))
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await?;

        // Save [`FileMetadata`] of the successfully retrieved stored files in the forest storage.
        let mut write_forest_storage = self.storage_hub_handler.forest_storage.write().await;
        write_forest_storage
            .insert_files_metadata(&file_metadatas)
            .map_err(|_| anyhow!("Failed to insert files metadata into forest storage."))?;
        // Release the forest storage write lock.
        drop(write_forest_storage);

        // Release the forest root write "lock".
        let _ = forest_root_write_tx.send(());

        Ok(())
    }
}

impl<FL, FS> BspUploadFileTask<FL, FS>
where
    FL: Send + Sync + FileStorage<StorageProofsMerkleTrieLayout>,
    FS: Send + Sync + ForestStorage<StorageProofsMerkleTrieLayout>,
{
    async fn handle_new_storage_request_event(
        &mut self,
        event: NewStorageRequest,
    ) -> anyhow::Result<()> {
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
            .file_key::<HashT<StorageProofsMerkleTrieLayout>>()
            .as_ref()
            .try_into()?;

        self.file_key_cleanup = Some(file_key.into());

        // Get the node's provider id needed for threshold calculation.
        let provider_id = self
            .storage_hub_handler
            .blockchain
            .get_provider_id(None)
            .await
            .ok_or_else(|| anyhow!("Failed to get BSP provider ID."))?;

        // Query runtime for the earliest block where the BSP can volunteer for the file.
        let earliest_volunteer_block = self
            .storage_hub_handler
            .blockchain
            .query_file_earliest_volunteer_block(provider_id, file_key.into())
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
            .insert_file(
                metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>(),
                metadata,
            )
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

    async fn unvolunteer_file(&self, file_key: H256) -> anyhow::Result<()> {
        warn!(target: LOG_TARGET, "Unvolunteering file {:?}", file_key);

        // Unregister the file from the file transfer service.
        // The error is ignored, as the file might already be unregistered.
        let _ = self
            .storage_hub_handler
            .file_transfer
            .unregister_file(file_key.as_ref().into())
            .await;

        // TODO: Send transaction to runtime to unvolunteer the file.

        // Delete the file from the file storage.
        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;

        // TODO: Handle error
        let _ = write_file_storage.delete_file(&file_key);

        Ok(())
    }

    async fn on_file_complete(&self, file_key: &H256) -> anyhow::Result<()> {
        info!(target: LOG_TARGET, "File upload complete ({:?})", file_key);

        // Unregister the file from the file transfer service.
        self.storage_hub_handler
            .file_transfer
            .unregister_file((*file_key).into())
            .await
            .map_err(|e| anyhow!("File is not registered. This should not happen!: {:?}", e))?;

        // Queue a request to confirm the storing of the file.
        self.storage_hub_handler
            .blockchain
            .queue_confirm_bsp_request(ConfirmStoringRequest::new(*file_key))
            .await?;

        Ok(())
    }
}
