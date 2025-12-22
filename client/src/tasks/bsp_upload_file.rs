use std::{
    collections::{HashMap, HashSet},
    future::Future,
    pin::Pin,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use anyhow::anyhow;
use frame_support::BoundedVec;
use pallet_file_system_runtime_api::QueryBspConfirmChunksToProveForFileError;
use sc_network::PeerId;
use sc_tracing::tracing::*;
use sp_runtime::traits::{CheckedAdd, CheckedSub, Hash, SaturatedConversion, Zero};

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    capacity_manager::CapacityRequestData,
    commands::{BlockchainServiceCommandInterface, BlockchainServiceCommandInterfaceExt},
    events::{NewStorageRequest, ProcessConfirmStoringRequest},
    types::{ConfirmStoringRequest, RetryStrategy, SendExtrinsicOptions, WatchTransactionError},
};
use shc_common::{
    consts::CURRENT_FOREST_KEY,
    traits::StorageEnableRuntime,
    types::{
        FileKey, FileKeyWithProof, FileMetadata, HashT, ProviderId, StorageProofsMerkleTrieLayout,
        StorageProviderId, BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE,
    },
};
use shc_file_manager::traits::{FileStorage, FileStorageWriteError, FileStorageWriteOutcome};
use shc_file_transfer_service::{
    commands::FileTransferServiceCommandInterface, events::RemoteUploadRequest,
};
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};

use crate::{
    handler::StorageHubHandler,
    inc_counter_by,
    metrics::{STATUS_FAILURE, STATUS_SUCCESS},
    observe_histogram,
    types::{BspForestStorageHandlerT, ForestStorageKey, ShNodeType},
};

const LOG_TARGET: &str = "bsp-upload-file-task";

/// Configuration for the BspUploadFileTask
#[derive(Debug, Clone)]
pub struct BspUploadFileConfig {
    /// Maximum number of times to retry an upload file request
    pub max_try_count: u32,
    /// Maximum tip amount to use when submitting an upload file request extrinsic
    pub max_tip: u128,
}

impl Default for BspUploadFileConfig {
    fn default() -> Self {
        Self {
            max_try_count: 3,
            max_tip: 500,
        }
    }
}

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
///   extrinsic, waiting for it to be successfully included in a block.
pub struct BspUploadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime + 'static,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
    file_key_cleanup: Option<Runtime::Hash>,
    /// Configuration for this task
    config: BspUploadFileConfig,
}

impl<NT, Runtime> Clone for BspUploadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> BspUploadFileTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            file_key_cleanup: self.file_key_cleanup,
            config: self.config.clone(),
        }
    }
}

impl<NT, Runtime> BspUploadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler: storage_hub_handler.clone(),
            file_key_cleanup: None,
            config: storage_hub_handler.provider_config.bsp_upload_file.clone(),
        }
    }
}

/// Handles the [`NewStorageRequest`] event.
///
/// This event is triggered by an on-chain event of a user submitting a storage request to StorageHub.
/// It responds by sending a volunteer transaction and registering the interest of this BSP in
/// receiving the file. This task optimistically assumes the transaction will succeed, and registers
/// the user and file key in the registry of the File Transfer Service, which handles incoming p2p
/// upload requests.
impl<NT, Runtime> EventHandler<NewStorageRequest<Runtime>> for BspUploadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: NewStorageRequest<Runtime>) -> anyhow::Result<String> {
        info!(
            target: LOG_TARGET,
            "Initiating BSP volunteer for file_key {:x}, location 0x{}, fingerprint {:x}",
            event.file_key,
            hex::encode(event.location.as_slice()),
            event.fingerprint
        );

        let file_key = event.file_key;
        let result = self.handle_new_storage_request_event(event).await;

        match result {
            Ok(()) => Ok(format!(
                "Handled NewStorageRequest for file_key {:x}",
                file_key
            )),
            Err(e) => {
                if let Some(file_key) = &self.file_key_cleanup {
                    self.unvolunteer_file(*file_key).await;
                }
                Err(e)
            }
        }
    }
}

/// Handles the [`RemoteUploadRequest`] event.
///
/// This event is triggered by a user sending a chunk of the file to the BSP. It checks the proof
/// for the chunk and if it is valid, stores it, until the whole file is stored.
impl<NT, Runtime> EventHandler<RemoteUploadRequest<Runtime>> for BspUploadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: RemoteUploadRequest<Runtime>,
    ) -> anyhow::Result<String> {
        trace!(target: LOG_TARGET, "Received remote upload request for file {:?} and peer {:?}", event.file_key, event.peer);

        let file_complete = match self.handle_remote_upload_request_event(event.clone()).await {
            Ok((complete, bytes_processed)) => {
                inc_counter_by!(
                    handler: self.storage_hub_handler,
                    bytes_uploaded_total,
                    STATUS_SUCCESS,
                    bytes_processed as u64
                );
                complete
            }
            Err(e) => {
                error!(target: LOG_TARGET, "Failed to handle remote upload request: {:?}", e);

                // Send error response through FileTransferService
                if let Err(e) = self
                    .storage_hub_handler
                    .file_transfer
                    .upload_response(event.request_id, false)
                    .await
                {
                    error!(target: LOG_TARGET, "Failed to send error response: {:?}", e);
                }
                return Err(e);
            }
        };

        // Send completion status through FileTransferService
        if let Err(e) = self
            .storage_hub_handler
            .file_transfer
            .upload_response(event.request_id, file_complete)
            .await
        {
            error!(target: LOG_TARGET, "Failed to send response: {:?}", e);
        }

        // Handle file completion if the entire file is uploaded
        if file_complete {
            if let Err(e) = self
                .storage_hub_handler
                .file_transfer
                .unregister_file(event.file_key)
                .await
            {
                error!(
                    target: LOG_TARGET,
                    "Failed to unregister file {:?} from file transfer service: {:?}",
                    event.file_key,
                    e
                );
            }

            self.storage_hub_handler
                .blockchain
                .queue_confirm_bsp_request(ConfirmStoringRequest {
                    file_key: event.file_key.into(),
                    try_count: 0,
                })
                .await?;
        }

        Ok(format!(
            "Handled RemoteUploadRequest for file [{:x}] (complete: {})",
            event.file_key, file_complete
        ))
    }
}

/// Handles the [`ProcessConfirmStoringRequest`] event.
///
/// This event is triggered by the runtime when it decides it is the right time to submit a confirm
/// storing extrinsic (and update the local forest root).
impl<NT, Runtime> EventHandler<ProcessConfirmStoringRequest<Runtime>>
    for BspUploadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: ProcessConfirmStoringRequest<Runtime>,
    ) -> anyhow::Result<String> {
        info!(
            target: LOG_TARGET,
            "Processing ConfirmStoringRequest: {:?}",
            event.data.confirm_storing_requests,
        );

        // Acquire Forest root write lock. This prevents other Forest-root-writing tasks from starting while we are processing this task.
        // That is until we release the lock gracefully with the `release_forest_root_write_lock` method, or `forest_root_write_lock` is dropped.
        let forest_root_write_tx = match event.forest_root_write_tx.lock().await.take() {
            Some(tx) => tx,
            None => {
                let err_msg = "CRITICAL❗️❗️ This is a bug! Forest root write tx already taken. This is a critical bug. Please report it to the StorageHub team.";
                error!(target: LOG_TARGET, err_msg);
                return Err(anyhow!(err_msg));
            }
        };

        // Get the BSP ID of the Provider running this node and its current Forest root.
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
        let current_forest_key = ForestStorageKey::from(CURRENT_FOREST_KEY.to_vec());

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
                Err(e) => match e {
                    QueryBspConfirmChunksToProveForFileError::StorageRequestNotFound => {
                        trace!(target: LOG_TARGET, "Skipping {:?} for stale storage request not found in chain state.", confirm_storing_request.file_key);
                        continue;
                    }
                    QueryBspConfirmChunksToProveForFileError::ConfirmChunks(internal_err) => {
                        trace!(target: LOG_TARGET, "Skipping {:?}. Runtime could not return data due to some corrupted state: {:?}", confirm_storing_request.file_key, internal_err);
                        continue;
                    }
                    _ => {
                        let mut confirm_storing_request = confirm_storing_request.clone();
                        confirm_storing_request.increment_try_count();
                        if confirm_storing_request.try_count > self.config.max_try_count {
                            error!(target: LOG_TARGET, "Failed to query chunks to prove for file {:?}: {:?}\nMax try count exceeded! Dropping request!", confirm_storing_request.file_key, e);
                        } else {
                            error!(target: LOG_TARGET, "Failed to query chunks to prove for file {:?}: {:?}\nEnqueuing file key again! (retry {}/{})", confirm_storing_request.file_key, e, confirm_storing_request.try_count, self.config.max_try_count);
                            self.storage_hub_handler
                                .blockchain
                                .queue_confirm_bsp_request(confirm_storing_request)
                                .await?;
                        }
                    }
                },
            }
        }

        if confirm_storing_requests_with_chunks_to_prove.iter().count() == 0 {
            trace!(target: LOG_TARGET, "Skipping ConfirmStoringRequest: No keys to confirm after querying chunks to prove.");
            return Ok(
                "Skipped ProcessConfirmStoringRequest: no keys to confirm after querying chunks"
                    .to_string(),
            );
        }

        // Generate the proof for the files and get metadatas.
        let read_file_storage = self.storage_hub_handler.file_storage.read().await;
        let mut file_keys_and_proofs = Vec::new();
        let mut file_metadatas = HashMap::new();
        let mut requests_to_retry = Vec::new();
        for (confirm_storing_request, chunks_to_prove) in
            confirm_storing_requests_with_chunks_to_prove.into_iter()
        {
            match (
                read_file_storage.generate_proof(
                    &confirm_storing_request.file_key,
                    &HashSet::from_iter(chunks_to_prove),
                ),
                read_file_storage.get_metadata(&confirm_storing_request.file_key),
            ) {
                (Ok(proof), Ok(Some(metadata))) => {
                    file_keys_and_proofs.push(FileKeyWithProof {
                        file_key: confirm_storing_request.file_key,
                        proof,
                    });
                    file_metadatas.insert(confirm_storing_request.file_key, metadata);
                }
                _ => {
                    let mut confirm_storing_request = confirm_storing_request.clone();
                    confirm_storing_request.increment_try_count();
                    if confirm_storing_request.try_count > self.config.max_try_count {
                        error!(target: LOG_TARGET, "Failed to generate proof or get metadatas for file {:?}.\nMax try count exceeded! Dropping request!", confirm_storing_request.file_key);
                    } else {
                        error!(target: LOG_TARGET, "Failed to generate proof or get metadatas for file {:?}.\nEnqueuing file key again! (retry {}/{})", confirm_storing_request.file_key, confirm_storing_request.try_count, self.config.max_try_count);
                        requests_to_retry.push(confirm_storing_request);
                    }
                }
            }
        }
        // Release the file storage read lock as soon as possible.
        drop(read_file_storage);

        // Re-enqueue any requests that we could not process, now that the file storage lock is dropped.
        for confirm_storing_request in requests_to_retry {
            self.storage_hub_handler
                .blockchain
                .queue_confirm_bsp_request(confirm_storing_request)
                .await?;
        }

        if file_keys_and_proofs.is_empty() {
            error!(target: LOG_TARGET, "Failed to generate proofs for ALL the requested files.\n");
            return Err(anyhow!(
                "Failed to generate proofs for ALL the requested files."
            ));
        }

        let file_keys = file_keys_and_proofs
            .iter()
            .map(|file_key_with_proof| file_key_with_proof.file_key)
            .collect::<Vec<_>>();

        let fs = self
            .storage_hub_handler
            .forest_storage_handler
            .get(&current_forest_key)
            .await
            .ok_or_else(|| anyhow!("Failed to get forest storage."))?;

        // Generate a proof of non-inclusion (executed in closure to drop the read lock on the forest storage).
        let non_inclusion_forest_proof = { fs.read().await.generate_proof(file_keys)? };

        // Build extrinsic.
        let call: Runtime::Call =
            pallet_file_system::Call::<Runtime>::bsp_confirm_storing {
                non_inclusion_forest_proof: non_inclusion_forest_proof.proof,
                file_keys_and_proofs: BoundedVec::try_from(file_keys_and_proofs)
                .map_err(|_| {
                    error!("CRITICAL❗️❗️ This is a bug! Failed to convert file keys and proofs to BoundedVec. Please report it to the StorageHub team.");
                    anyhow!("Failed to convert file keys and proofs to BoundedVec.")
                })?,
            }.into();

        // Send the confirmation transaction and wait for it to be included in the block and
        // continue only if it is successful.
        let submit_result = self
            .storage_hub_handler
            .blockchain
            .submit_extrinsic_with_retry(
                call,
                SendExtrinsicOptions::new(
                    Duration::from_secs(
                        self.storage_hub_handler
                            .provider_config
                            .blockchain_service
                            .extrinsic_retry_timeout,
                    ),
                    Some("fileSystem".to_string()),
                    Some("bspConfirmStoring".to_string()),
                ),
                RetryStrategy::default()
                    .with_max_retries(self.config.max_try_count)
                    .with_max_tip(self.config.max_tip.saturated_into())
                    .retry_only_if_timeout(),
                true,
            )
            .await;

        submit_result.map_err(|e| {
            anyhow!(
                "Failed to confirm file after {} retries: {:?}",
                self.config.max_try_count,
                e
            )
        })?;

        // Release the forest root write "lock" and finish the task.
        self.storage_hub_handler
            .blockchain
            .release_forest_root_write_lock(forest_root_write_tx)
            .await?;

        Ok(format!(
            "Processed ProcessConfirmStoringRequest for BSP [{:x}]",
            own_bsp_id
        ))
    }
}

impl<NT, Runtime> BspUploadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: BspForestStorageHandlerT<Runtime> + 'static,
    Runtime: StorageEnableRuntime,
{
    async fn handle_new_storage_request_event(
        &mut self,
        event: NewStorageRequest<Runtime>,
    ) -> anyhow::Result<()> {
        let start_time = std::time::Instant::now();

        let result = self.handle_new_storage_request_inner(event).await;

        observe_histogram!(
            handler: self.storage_hub_handler,
            storage_request_setup_seconds,
            if result.is_ok() {
                STATUS_SUCCESS
            } else {
                STATUS_FAILURE
            },
            start_time.elapsed().as_secs_f64()
        );

        result
    }

    async fn handle_new_storage_request_inner(
        &mut self,
        event: NewStorageRequest<Runtime>,
    ) -> anyhow::Result<()> {
        if event.size == Zero::zero() {
            let err_msg = "File size cannot be 0";
            error!(target: LOG_TARGET, err_msg);
            return Err(anyhow!(err_msg));
        }

        // First check if the file is not on our exclude list
        let is_allowed = self.is_allowed(&event).await?;

        if !is_allowed {
            warn!(target: LOG_TARGET, "File with file key {:x} is in our exclude list. Skipping volunteer.", event.file_key);
            return Ok(());
        }

        // Get the current Forest key of the Provider running this node.
        let current_forest_key = ForestStorageKey::from(CURRENT_FOREST_KEY.to_vec());

        // Verify if file not already stored
        let fs = self
            .storage_hub_handler
            .forest_storage_handler
            .get(&current_forest_key)
            .await
            .ok_or_else(|| anyhow!("Failed to get forest storage."))?;
        if fs.read().await.contains_file_key(&event.file_key.into())? {
            info!(
                target: LOG_TARGET,
                "Skipping file key {:x} NewStorageRequest because we are already storing it.",
                event.file_key
            );
            return Ok(());
        }

        // Construct file metadata.
        let who = event.who.as_ref().to_vec();
        let metadata = FileMetadata::new(
            who,
            event.bucket_id.as_ref().to_vec(),
            event.location.to_vec(),
            event.size.saturated_into(),
            event.fingerprint,
        )
        .map_err(|_| anyhow::anyhow!("Invalid file metadata"))?;

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

        let max_storage_capacity = self
            .storage_hub_handler
            .provider_config
            .capacity_config
            .max_capacity();

        let current_capacity = self
            .storage_hub_handler
            .blockchain
            .query_storage_provider_capacity(own_bsp_id)
            .await
            .map_err(|e| {
                error!(target: LOG_TARGET, "Failed to query storage provider capacity: {:?}", e);
                anyhow::anyhow!("Failed to query storage provider capacity: {:?}", e)
            })?;

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

        // Calculate currently used storage
        let used_capacity = current_capacity
            .checked_sub(&available_capacity)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Available capacity ({}) exceeds current capacity ({})",
                    available_capacity,
                    current_capacity
                )
            })?;

        // Check if accepting this file would exceed our local max storage capacity limit
        let projected_usage = used_capacity
            .checked_add(&event.size)
            .ok_or_else(|| anyhow::anyhow!("Overflow calculating projected storage usage"))?;

        if projected_usage > max_storage_capacity {
            let err_msg = format!(
                "Accepting file would exceed maximum storage capacity limit. Used: {}, Required: {}, Max: {}",
                used_capacity, event.size, max_storage_capacity
            );
            warn!(target: LOG_TARGET, "{}", err_msg);
            return Err(anyhow::anyhow!(err_msg));
        }

        // Increase storage capacity if the available capacity is less than the file size.
        if available_capacity < event.size {
            warn!(
                target: LOG_TARGET,
                "Insufficient storage capacity to volunteer for file key: {:?}",
                event.file_key
            );

            self.storage_hub_handler
                .blockchain
                .increase_capacity(CapacityRequestData::new(event.size))
                .await?;

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

        // Calculate the tick in which the BSP should send the extrinsic. It's one less that the tick
        // in which the BSP can volunteer for the file because that way it the extrinsic will get included
        // in the tick where the BSP can actually volunteer for the file.
        use sp_runtime::Saturating;
        let tick_to_wait_to_submit_volunteer = earliest_volunteer_tick.saturating_sub(1u32.into());

        info!(
            target: LOG_TARGET,
            "Waiting for tick {:?} to volunteer for file {:x}",
            earliest_volunteer_tick,
            file_key
        );

        // TODO: if the earliest tick is too far away, we should drop the task.
        // TODO: based on the limit above, also add a timeout for the task.
        self.storage_hub_handler
            .blockchain
            .wait_for_tick(tick_to_wait_to_submit_volunteer)
            .await?;

        // TODO: Have this dynamically called at every tick in `wait_for_tick` to exit early without waiting until `earliest_volunteer_tick` in the event the storage request
        // TODO: is closed mid-way through the process.
        let can_volunteer = self
            .storage_hub_handler
            .blockchain
            .is_storage_request_open_to_volunteers(file_key.into())
            .await
            .map_err(|e| anyhow!("Failed to query file can volunteer: {:?}", e))?;

        // Skip volunteering if the storage request is no longer open to volunteers.
        // TODO: Handle the case where were catching up to the latest block. We probably either want to skip volunteering or wait until
        // TODO: we catch up to the latest block and if the storage request is still open to volunteers, volunteer then.
        if !can_volunteer {
            let err_msg = "Storage request is no longer open to volunteers. Skipping volunteering.";
            warn!(
                target: LOG_TARGET, "{}", err_msg
            );
            return Err(anyhow::anyhow!(err_msg));
        }

        // Optimistically create file in file storage so we can write uploaded chunks as soon as possible.
        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;
        write_file_storage
            .insert_file(
                metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>(),
                metadata,
            )
            .map_err(|e| anyhow!("Failed to insert file in file storage: {:?}", e))?;
        drop(write_file_storage);

        // Optimistically register the file for upload in the file transfer service.
        // This solves the race condition between the user and the BSP, where the user could react faster
        // to the BSP volunteering than the BSP, and therefore initiate a new upload request before the
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
                .register_new_file(peer_id, file_key)
                .await
                .map_err(|e| anyhow!("Failed to register new file peer: {:?}", e))?;
        }

        // Build extrinsic.
        let call: Runtime::Call = pallet_file_system::Call::<Runtime>::bsp_volunteer {
            file_key: file_key.into(),
        }
        .into();

        // Clone necessary data for the retry check.
        let cloned_sh_handler = Arc::new(self.storage_hub_handler.clone());
        let cloned_own_bsp_id = Arc::new(own_bsp_id.clone());
        let cloned_file_key: Arc<Runtime::Hash> = Arc::new(file_key.clone().into());

        let should_retry = move |error| {
            let cloned_sh_handler = Arc::clone(&cloned_sh_handler);
            let cloned_own_bsp_id = Arc::clone(&cloned_own_bsp_id);
            let cloned_file_key = Arc::clone(&cloned_file_key);

            // Check:
            // - If we've already successfully volunteered for the file.
            // - If the storage request is no longer open to volunteers.
            // Also waits for the tick to be able to volunteer for the file has actually been reached,
            // not the tick before the BSP can volunteer for the file. To make sure the chain wasn't
            // spammed just before the BSP could volunteer for the file.
            Box::pin(Self::should_retry_volunteer(
                cloned_sh_handler,
                cloned_own_bsp_id,
                cloned_file_key,
                error,
            )) as Pin<Box<dyn Future<Output = bool> + Send>>
        };

        // Try to send the volunteer extrinsic
        if let Err(e) = self
            .storage_hub_handler
            .blockchain
            .submit_extrinsic_with_retry(
                call.clone(),
                SendExtrinsicOptions::new(
                    Duration::from_secs(
                        self.storage_hub_handler
                            .provider_config
                            .blockchain_service
                            .extrinsic_retry_timeout,
                    ),
                    Some("fileSystem".to_string()),
                    Some("bspVolunteer".to_string()),
                ),
                RetryStrategy::default()
                    .with_max_retries(self.config.max_try_count)
                    .with_max_tip(self.config.max_tip.saturated_into())
                    .with_should_retry(Some(Box::new(should_retry))),
                false,
            )
            .await
        {
            error!(target: LOG_TARGET, "Failed to volunteer for file {:x}: {:?}", file_key, e);
        }

        // Check if the BSP has been registered as a volunteer for the file.
        let volunteer_result = self
            .storage_hub_handler
            .blockchain
            .query_bsp_volunteered_for_file(own_bsp_id, file_key.into())
            .await
            .map_err(|e| anyhow!("Failed to query BSP volunteered for file: {:?}", e))?;

        // Handle the volunteer result.
        if volunteer_result {
            info!(
                target: LOG_TARGET,
                "🍾 BSP successfully volunteered for file {:x}",
                file_key
            );
        } else {
            error!(
                target: LOG_TARGET,
                "BSP not registered as a volunteer for file {:x}",
                file_key
            );
            self.unvolunteer_file(file_key.into()).await;
            return Err(anyhow!(
                "BSP not registered as a volunteer for file {:x}",
                file_key
            ));
        }

        Ok(())
    }

    /// Handles the [`RemoteUploadRequest`] event.
    ///
    /// Returns a tuple of (file_complete, bytes_processed) where:
    /// - `file_complete` is `true` if the file is complete, `false` if incomplete.
    /// - `bytes_processed` is the total number of bytes in the batch that was processed.
    async fn handle_remote_upload_request_event(
        &mut self,
        event: RemoteUploadRequest<Runtime>,
    ) -> anyhow::Result<(bool, usize)> {
        debug!(target: LOG_TARGET, "Handling remote upload request for file key {:x}", event.file_key);

        let file_key = event.file_key.into();

        trace!(target: LOG_TARGET, "Waiting to acquire write lock on file storage for file key {:?}", file_key);
        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;

        // Get the file metadata to verify the fingerprint
        trace!(target: LOG_TARGET, "Acquired write lock on file storage for file key {:?}", file_key);
        let file_metadata = write_file_storage
            .get_metadata(&file_key)
            .map_err(|e| anyhow!("Failed to get file metadata: {:?}", e))?
            .ok_or_else(|| anyhow!("File metadata not found"))?;

        // Verify that the fingerprint in the proof matches the expected file fingerprint
        let expected_fingerprint = file_metadata.fingerprint();
        if event.file_key_proof.file_metadata.fingerprint() != expected_fingerprint {
            error!(
                target: LOG_TARGET,
                "Fingerprint mismatch for file {:?}. Expected: {:?}, got: {:?}",
                file_key, expected_fingerprint, event.file_key_proof.file_metadata.fingerprint()
            );
            return Err(anyhow!("Fingerprint mismatch"));
        }

        // Verify and extract chunks from proof
        let proven = match event
            .file_key_proof
            .proven::<StorageProofsMerkleTrieLayout>()
        {
            Ok(proven) => {
                if proven.is_empty() {
                    Err(anyhow::anyhow!(
                        "Expected at least one proven chunk but got none."
                    ))
                } else {
                    // Calculate total batch size
                    let total_batch_size: usize = proven.iter().map(|chunk| chunk.data.len()).sum();

                    if total_batch_size > BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE {
                        Err(anyhow::anyhow!(
                            "Total batch size {} bytes exceeds maximum allowed size of {} bytes",
                            total_batch_size,
                            BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE
                        ))
                    } else {
                        Ok(proven)
                    }
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
                error!(target: LOG_TARGET, "Failed to verify and get proven file key chunks: {}", e);
                return Err(e);
            }
        };

        // Calculate total batch size for metrics
        let total_batch_bytes: usize = proven.iter().map(|chunk| chunk.data.len()).sum();

        let mut file_complete = false;

        // Process each proven chunk in the batch
        for chunk in proven {
            // TODO: Add a batched write chunk method to the file storage.

            // Validate chunk size
            let chunk_idx = chunk.key.as_u64();
            if !file_metadata.is_valid_chunk_size(chunk_idx, chunk.data.len()) {
                match file_metadata.chunk_size_at(chunk_idx) {
                    Ok(actual_chunk_size) => {
                        error!(
                                target: LOG_TARGET,
                                "Invalid chunk size for chunk {:?} of file {:?}. Expected: {}, got: {}",
                                chunk.key,
                                file_key,
                                actual_chunk_size,
                            chunk.data.len()
                        );
                        return Err(anyhow!(
                            "Invalid chunk size. Expected {}, got {}",
                            actual_chunk_size,
                            chunk.data.len()
                        ));
                    }
                    Err(e) => {
                        let err_msg = format!(
                            "Failed to get chunk size for chunk {:?}: {:?}",
                            chunk.key, e
                        );
                        error!(
                            target: LOG_TARGET,
                            "{}",
                            err_msg
                        );
                        return Err(anyhow!(err_msg));
                    }
                }
            }

            let write_result = write_file_storage.write_chunk(&file_key, &chunk.key, &chunk.data);

            match write_result {
                Ok(outcome) => match outcome {
                    FileStorageWriteOutcome::FileComplete => {
                        file_complete = true;
                        break; // We can stop processing chunks if the file is complete
                    }
                    FileStorageWriteOutcome::FileIncomplete => continue,
                },
                Err(error) => match error {
                    FileStorageWriteError::FileChunkAlreadyExists => {
                        trace!(
                            target: LOG_TARGET,
                            "Received duplicate chunk with key: {:?}",
                            chunk.key
                        );
                        // Continue processing other chunks
                        continue;
                    }
                    FileStorageWriteError::FileDoesNotExist => {
                        return Err(anyhow::anyhow!(format!(
                            "File does not exist for key {:?}. Maybe we forgot to unregister before deleting?",
                            event.file_key
                        )));
                    }
                    FileStorageWriteError::FailedToGetFileChunk
                    | FileStorageWriteError::FailedToInsertFileChunk
                    | FileStorageWriteError::FailedToDeleteChunk
                    | FileStorageWriteError::FailedToDeleteRoot
                    | FileStorageWriteError::FailedToPersistChanges
                    | FileStorageWriteError::FailedToParseFileMetadata
                    | FileStorageWriteError::FailedToParseFingerprint
                    | FileStorageWriteError::FailedToReadStorage
                    | FileStorageWriteError::FailedToUpdatePartialRoot
                    | FileStorageWriteError::FailedToParsePartialRoot
                    | FileStorageWriteError::FailedToGetStoredChunksCount
                    | FileStorageWriteError::ChunkCountOverflow
                    | FileStorageWriteError::FailedToCheckFileCompletion(_) => {
                        return Err(anyhow::anyhow!(format!(
                            "Internal trie read/write error {:?}:{:?}",
                            event.file_key, chunk.key
                        )));
                    }
                    FileStorageWriteError::FingerprintAndStoredFileMismatch => {
                        return Err(anyhow::anyhow!(format!(
                            "Invariant broken! This is a bug! Fingerprint and stored file mismatch for key {:?}.",
                            event.file_key
                        )));
                    }
                    FileStorageWriteError::FailedToConstructTrieIter
                    | FileStorageWriteError::FailedToConstructFileTrie => {
                        return Err(anyhow::anyhow!(format!(
                            "This is a bug! Failed to construct trie iter for key {:?}.",
                            event.file_key
                        )));
                    }
                },
            }
        }

        Ok((file_complete, total_batch_bytes))
    }

    async fn is_allowed(&self, event: &NewStorageRequest<Runtime>) -> anyhow::Result<bool> {
        let read_file_storage = self.storage_hub_handler.file_storage.read().await;
        let mut is_allowed = read_file_storage
            .is_allowed(
                &event.file_key.into(),
                shc_file_manager::traits::ExcludeType::File,
            )
            .map_err(|e| {
                let err_msg = format!("Failed to read file exclude list: {:?}", e);
                error!(
                    target: LOG_TARGET,
                    err_msg
                );
                anyhow::anyhow!(err_msg)
            })?;

        if !is_allowed {
            info!("File is in the exclude list");
            drop(read_file_storage);
            return Ok(false);
        }

        is_allowed = read_file_storage
            .is_allowed(
                &event.fingerprint.as_hash().into(),
                shc_file_manager::traits::ExcludeType::Fingerprint,
            )
            .map_err(|e| {
                let err_msg = format!("Failed to read file exclude list: {:?}", e);
                error!(
                    target: LOG_TARGET,
                    err_msg
                );
                anyhow::anyhow!(err_msg)
            })?;

        if !is_allowed {
            info!("File fingerprint is in the exclude list");
            drop(read_file_storage);
            return Ok(false);
        }

        let owner = Runtime::Hashing::hash(event.who.as_ref());
        is_allowed = read_file_storage
            .is_allowed(&owner, shc_file_manager::traits::ExcludeType::User)
            .map_err(|e| {
                let err_msg = format!("Failed to read file exclude list: {:?}", e);
                error!(
                    target: LOG_TARGET,
                    err_msg
                );
                anyhow::anyhow!(err_msg)
            })?;

        if !is_allowed {
            info!("Owner is in the exclude list");
            drop(read_file_storage);
            return Ok(false);
        }

        is_allowed = read_file_storage
            .is_allowed(
                &event.bucket_id,
                shc_file_manager::traits::ExcludeType::Bucket,
            )
            .map_err(|e| {
                let err_msg = format!("Failed to read file exclude list: {:?}", e);
                error!(
                    target: LOG_TARGET,
                    err_msg
                );
                anyhow::anyhow!(err_msg)
            })?;

        if !is_allowed {
            info!("Bucket is in the exclude list");
            drop(read_file_storage);
            return Ok(false);
        }

        drop(read_file_storage);

        Ok(true)
    }

    /// Function to determine if a volunteer request should be retried,
    /// sending the same request again.
    ///
    /// This function will return `true` if and only if the following conditions are met:
    /// 1. If the storage request is no longer open to volunteers.
    /// 2. If we've already successfully volunteered for the file.
    ///
    /// Also waits for the tick to be able to volunteer for the file has actually been reached,
    /// not the tick before the BSP can volunteer for the file. To make sure the chain wasn't
    /// spammed just before the BSP could volunteer for the file.
    async fn should_retry_volunteer(
        sh_handler: Arc<StorageHubHandler<NT, Runtime>>,
        bsp_id: Arc<ProviderId<Runtime>>,
        file_key: Arc<Runtime::Hash>,
        _error: WatchTransactionError,
    ) -> bool {
        // Wait for the tick to be able to volunteer for the file has actually been reached.
        let earliest_volunteer_tick = match sh_handler
            .blockchain
            .query_file_earliest_volunteer_tick(*bsp_id, *file_key)
            .await
        {
            Ok(tick) => tick,
            Err(e) => {
                error!(target: LOG_TARGET, "Failed to query file earliest volunteer block: {:?}", e);
                return false;
            }
        };
        match sh_handler
            .blockchain
            .wait_for_tick(earliest_volunteer_tick)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                error!(target: LOG_TARGET, "Failed to wait for tick: {:?}", e);
                return false;
            }
        }

        // Check if the storage request is no longer open to volunteers.
        let can_volunteer = match sh_handler
            .blockchain
            .is_storage_request_open_to_volunteers(*file_key)
            .await
        {
            Ok(can_volunteer) => can_volunteer,
            Err(e) => {
                error!(target: LOG_TARGET, "Failed to query file can volunteer: {:?}", e);
                return false;
            }
        };

        if !can_volunteer {
            warn!(target: LOG_TARGET, "Storage request is no longer open to volunteers. Stop retrying.");
            return false;
        }

        // Check if we've already successfully volunteered for the file.
        let volunteered = match sh_handler
            .blockchain
            .query_bsp_volunteered_for_file(*bsp_id, *file_key)
            .await
        {
            Ok(volunteered) => volunteered,
            Err(e) => {
                error!(target: LOG_TARGET, "Failed to query file volunteered: {:?}", e);
                return false;
            }
        };

        if volunteered {
            info!(target: LOG_TARGET, "Already successfully volunteered for the file. Stop retrying.");
            return false;
        }

        true
    }

    async fn unvolunteer_file(&self, file_key: Runtime::Hash) {
        warn!(target: LOG_TARGET, "Unvolunteering file {:?}", file_key);

        // Unregister the file from the file transfer service.
        // The error is ignored, as the file might already be unregistered.
        if let Err(e) = self
            .storage_hub_handler
            .file_transfer
            .unregister_file(file_key.as_ref().into())
            .await
        {
            error!(
                target: LOG_TARGET,
                "[unvolunteer_file] Failed to unregister file {:?} from file transfer service: {:?}",
                file_key,
                e
            );
        }

        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;
        if let Err(e) = write_file_storage.delete_file(&file_key) {
            error!(
                target: LOG_TARGET,
                "[unvolunteer_file] Failed to delete file {:?} from file storage: {:?}",
                file_key,
                e
            );
        }
        drop(write_file_storage);
    }
}
