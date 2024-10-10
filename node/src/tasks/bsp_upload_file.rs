use std::{cmp::max, str::FromStr, time::Duration};

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
    types::{ConfirmStoringRequest, RetryStrategy, Tip},
};
use shc_common::types::{
    Balance, FileKey, FileMetadata, HashT, StorageProofsMerkleTrieLayout, StorageProviderId,
};
use shc_file_manager::traits::{FileStorageWriteError, FileStorageWriteOutcome};
use shc_file_transfer_service::{
    commands::FileTransferServiceInterface, events::RemoteUploadRequest,
};
use shc_forest_manager::traits::ForestStorage;
use storage_hub_runtime::{StorageDataUnit, MILLIUNIT};

use crate::services::{forest_storage::NoKey, handler::StorageHubHandler};
use crate::tasks::{BspForestStorageHandlerT, FileStorageT};

const LOG_TARGET: &str = "bsp-upload-file-task";

const MAX_CONFIRM_STORING_REQUEST_TRY_COUNT: u32 = 3;
const MAX_CONFIRM_STORING_REQUEST_TIP: Balance = 500 * MILLIUNIT;

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
pub struct BspUploadFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<FL, FSH>,
    file_key_cleanup: Option<H256>,
}

impl<FL, FSH> Clone for BspUploadFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    fn clone(&self) -> BspUploadFileTask<FL, FSH> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            file_key_cleanup: self.file_key_cleanup,
        }
    }
}

impl<FL, FSH> BspUploadFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    pub fn new(storage_hub_handler: StorageHubHandler<FL, FSH>) -> Self {
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
impl<FL, FSH> EventHandler<NewStorageRequest> for BspUploadFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: NewStorageRequest) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Initiating BSP volunteer for file_key {:?}, location {:?}, fingerprint {:?}",
            event.file_key,
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
impl<FL, FSH> EventHandler<RemoteUploadRequest> for BspUploadFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
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
impl<FL, FSH> EventHandler<ProcessConfirmStoringRequest> for BspUploadFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: ProcessConfirmStoringRequest) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing ConfirmStoringRequest: {:?}",
            event.data.confirm_storing_requests,
        );

        let forest_root_write_tx = match event.forest_root_write_tx.lock().await.take() {
            Some(tx) => tx,
            None => {
                let err_msg = "CRITICAL❗️❗️ This is a bug! Forest root write tx already taken. This is a critical bug. Please report it to the StorageHub team.";
                error!(target: LOG_TARGET, err_msg);
                return Err(anyhow!(err_msg));
            }
        };

        let own_provider_id = self
            .storage_hub_handler
            .blockchain
            .query_storage_provider_id(None)
            .await?;

        let own_bsp_id = match own_provider_id {
            Some(id) => match id {
                StorageProviderId::MainStorageProvider(_) => {
                    let err_msg = "Current node account is a Main Storage Provider. Expected a Backup Storage Provider ID.";
                    error!(target: LOG_TARGET, err_msg);
                    return Err(anyhow!(err_msg));
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
        for confirm_storing_request in event.data.confirm_storing_requests.iter() {
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
                (Ok(proof), Ok(Some(metadata))) => {
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

        let fs = self
            .storage_hub_handler
            .forest_storage_handler
            .get(&NoKey)
            .await
            .ok_or_else(|| anyhow!("Failed to get forest storage."))?;

        // Generate a proof of non-inclusion (executed in closure to drop the read lock on the forest storage).
        let non_inclusion_forest_proof = { fs.read().await.generate_proof(file_keys)? };

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
            .submit_extrinsic_with_retry(
                call,
                RetryStrategy::default()
                    .with_max_retries(MAX_CONFIRM_STORING_REQUEST_TRY_COUNT)
                    .with_max_tip(MAX_CONFIRM_STORING_REQUEST_TIP as f64)
                    .with_timeout(Duration::from_secs(
                        self.storage_hub_handler
                            .provider_config
                            .extrinsic_retry_timeout,
                    )),
            )
            .await?;

        // Save `FileMetadata` of the successfully retrieved stored files in the forest storage (executed in closure to drop the read lock on the forest storage).
        {
            fs.write()
                .await
                .insert_files_metadata(file_metadatas.as_slice())?;
        }

        // Release the forest root write "lock" and finish the task.
        self.storage_hub_handler
            .blockchain
            .release_forest_root_write_lock(forest_root_write_tx)
            .await
    }
}

impl<FL, FSH> BspUploadFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
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

        let own_provider_id = self
            .storage_hub_handler
            .blockchain
            .query_storage_provider_id(None)
            .await?;

        let own_bsp_id = match own_provider_id {
            Some(id) => match id {
                StorageProviderId::MainStorageProvider(_) => {
                    let err_msg = "Current node account is a Main Storage Provider. Expected a Backup Storage Provider ID.";
                    error!(target: LOG_TARGET, err_msg);
                    return Err(anyhow!(err_msg));
                }
                StorageProviderId::BackupStorageProvider(id) => id,
            },
            None => {
                let err_msg = "Failed to get own BSP ID.";
                error!(target: LOG_TARGET, err_msg);
                return Err(anyhow!(err_msg));
            }
        };

        let available_capacity = self
            .storage_hub_handler
            .blockchain
            .query_available_storage_capacity(own_bsp_id)
            .await
            .map_err(|e| {
                let err_msg = format!("Failed to query available storage capacity: {:?}", e);
                error!(
                    target: LOG_TARGET,
                    err_msg
                );
                anyhow::anyhow!(err_msg)
            })?;

        // Increase storage capacity if the available capacity is less than the file size.
        if available_capacity < event.size {
            warn!(
                target: LOG_TARGET,
                "Insufficient storage capacity to volunteer for file key: {:?}",
                event.file_key
            );

            let current_capacity = self
                .storage_hub_handler
                .blockchain
                .query_storage_provider_capacity(own_bsp_id)
                .await
                .map_err(|e| {
                    error!(
                        target: LOG_TARGET,
                        "Failed to query storage provider capacity: {:?}", e
                    );
                    anyhow::anyhow!("Failed to query storage provider capacity: {:?}", e)
                })?;

            let max_storage_capacity = self
                .storage_hub_handler
                .provider_config
                .max_storage_capacity;

            if max_storage_capacity == current_capacity {
                let err_msg = "Reached maximum storage capacity limit. Unable to add more more storage capacity.";
                warn!(
                    target: LOG_TARGET, "{}", err_msg
                );
                return Err(anyhow::anyhow!(err_msg));
            }

            let new_capacity = self.calculate_capacity(&event, current_capacity)?;

            let call = storage_hub_runtime::RuntimeCall::Providers(
                pallet_storage_providers::Call::change_capacity { new_capacity },
            );

            let earliest_change_capacity_block = self
                .storage_hub_handler
                .blockchain
                .query_earliest_change_capacity_block(own_bsp_id)
                .await
                .map_err(|e| {
                    error!(
                        target: LOG_TARGET,
                        "Failed to query storage provider capacity: {:?}", e
                    );
                    anyhow::anyhow!("Failed to query storage provider capacity: {:?}", e)
                })?;

            // Wait for the earliest block where the capacity can be changed.
            self.storage_hub_handler
                .blockchain
                .wait_for_block(earliest_change_capacity_block)
                .await?;

            self.storage_hub_handler
                .blockchain
                .send_extrinsic(call, Tip::from(0))
                .await?
                .with_timeout(Duration::from_secs(
                    self.storage_hub_handler
                        .provider_config
                        .extrinsic_retry_timeout,
                ))
                .watch_for_success(&self.storage_hub_handler.blockchain)
                .await?;

            info!(
                target: LOG_TARGET,
                "Increased storage capacity to {:?} bytes",
                new_capacity
            );

            let available_capacity = self
                .storage_hub_handler
                .blockchain
                .query_available_storage_capacity(own_bsp_id)
                .await
                .map_err(|e| {
                    error!(
                        target: LOG_TARGET,
                        "Failed to query available storage capacity: {:?}", e
                    );
                    anyhow::anyhow!("Failed to query available storage capacity: {:?}", e)
                })?;

            // Skip volunteering if the new available capacity is still less than the file size.
            if available_capacity < event.size {
                let err_msg = "Increased storage capacity is still insufficient to volunteer for file. Skipping volunteering.";
                warn!(
                    target: LOG_TARGET, "{}", err_msg
                );
                return Err(anyhow::anyhow!(err_msg));
            }
        }

        // Get the file key.
        let file_key: FileKey = metadata
            .file_key::<HashT<StorageProofsMerkleTrieLayout>>()
            .as_ref()
            .try_into()?;

        self.file_key_cleanup = Some(file_key.into());

        // Query runtime for the earliest block where the BSP can volunteer for the file.
        let earliest_volunteer_tick = self
            .storage_hub_handler
            .blockchain
            .query_file_earliest_volunteer_tick(own_bsp_id, file_key.into())
            .await
            .map_err(|e| anyhow!("Failed to query file earliest volunteer block: {:?}", e))?;

        info!(
            target: LOG_TARGET,
            "Waiting for tick {:?} to volunteer for file {:?}",
            earliest_volunteer_tick,
            file_key
        );

        // TODO: if the earliest tick is too far away, we should drop the task.
        // TODO: based on the limit above, also add a timeout for the task.
        self.storage_hub_handler
            .blockchain
            .wait_for_tick(earliest_volunteer_tick)
            .await?;

        // Optimistically register the file for upload in the file transfer service.
        // This solves the race condition between the user and the BSP, where the user could react faster
        // to the BSP volunteering than the BSP, and therefore initiate a new upload request before the
        // BSP has registered the file and peer ID in the file transfer service.
        for peer_id in event.user_peer_ids.iter() {
            let peer_id = match std::str::from_utf8(&peer_id.as_slice()) {
                Ok(str_slice) => PeerId::from_str(str_slice).map_err(|e| {
                    error!(target: LOG_TARGET, "Failed to convert peer ID to PeerId: {}", e);
                    e
                })?,
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
            .send_extrinsic(call, Tip::from(0))
            .await?
            .with_timeout(Duration::from_secs(
                self.storage_hub_handler
                    .provider_config
                    .extrinsic_retry_timeout,
            ))
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await?;

        Ok(())
    }

    /// Calculate the new capacity after adding the required capacity for the file.
    ///
    /// The new storage capacity will be increased by the jump capacity until it reaches the
    /// `max_storage_capacity`.
    ///
    /// The `max_storage_capacity` is returned if the new capacity exceeds it.
    fn calculate_capacity(
        &mut self,
        event: &NewStorageRequest,
        current_capacity: StorageDataUnit,
    ) -> Result<StorageDataUnit, anyhow::Error> {
        let jump_capacity = self.storage_hub_handler.provider_config.jump_capacity;
        let jumps_needed = (event.size + jump_capacity - 1) / jump_capacity;
        let jumps = max(jumps_needed, 1);
        let bytes_to_add = jumps * jump_capacity;
        let required_capacity = current_capacity.checked_add(bytes_to_add).ok_or_else(|| {
            anyhow::anyhow!(
                "Reached maximum storage capacity limit. Skipping volunteering for file."
            )
        })?;

        let max_storage_capacity = self
            .storage_hub_handler
            .provider_config
            .max_storage_capacity;

        let new_capacity = std::cmp::min(required_capacity, max_storage_capacity);

        Ok(new_capacity)
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
