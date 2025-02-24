use std::{
    cmp::max,
    collections::{HashMap, HashSet},
    ops::Add,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use anyhow::anyhow;
use frame_support::BoundedVec;
use sc_network::PeerId;
use sc_tracing::tracing::*;
use sp_core::H256;
use sp_runtime::AccountId32;
use tokio::sync::Mutex;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceInterface,
    events::{NewStorageRequest, ProcessConfirmStoringRequest},
    types::{ConfirmStoringRequest, RetryStrategy, SendExtrinsicOptions},
};
use shc_common::{
    consts::CURRENT_FOREST_KEY,
    types::{
        Balance, FileKey, FileKeyWithProof, FileMetadata, HashT, StorageProofsMerkleTrieLayout,
        StorageProviderId, BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE,
    },
};
use shc_file_manager::traits::{FileStorage, FileStorageWriteError, FileStorageWriteOutcome};
use shc_file_transfer_service::{
    commands::FileTransferServiceInterface, events::RemoteUploadRequest,
};
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use storage_hub_runtime::{StorageDataUnit, MILLIUNIT};

use crate::services::{
    handler::StorageHubHandler,
    types::{BspForestStorageHandlerT, ShNodeType},
};

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
///   extrinsic, waiting for it to be successfully included in a block.
pub struct BspUploadFileTask<NT>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<NT>,
    file_key_cleanup: Option<H256>,
    capacity_queue: Arc<Mutex<u64>>,
}

impl<NT> Clone for BspUploadFileTask<NT>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
{
    fn clone(&self) -> BspUploadFileTask<NT> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            file_key_cleanup: self.file_key_cleanup,
            capacity_queue: Arc::clone(&self.capacity_queue),
        }
    }
}

impl<NT> BspUploadFileTask<NT>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT>) -> Self {
        Self {
            storage_hub_handler,
            file_key_cleanup: None,
            capacity_queue: Arc::new(Mutex::new(0_u64)),
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
impl<NT> EventHandler<NewStorageRequest> for BspUploadFileTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
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
                self.unvolunteer_file(*file_key).await;
            }
        }
        result
    }
}

/// Handles the [`RemoteUploadRequest`] event.
///
/// This event is triggered by a user sending a chunk of the file to the BSP. It checks the proof
/// for the chunk and if it is valid, stores it, until the whole file is stored.
impl<NT> EventHandler<RemoteUploadRequest> for BspUploadFileTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: RemoteUploadRequest) -> anyhow::Result<()> {
        trace!(target: LOG_TARGET, "Received remote upload request for file {:?} and peer {:?}", event.file_key, event.peer);

        let file_complete = match self.handle_remote_upload_request_event(event.clone()).await {
            Ok(complete) => complete,
            Err(e) => {
                // Send error response through FileTransferService
                if let Err(e) = self
                    .storage_hub_handler
                    .file_transfer
                    .upload_response(false, event.request_id)
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
            .upload_response(file_complete, event.request_id)
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

        Ok(())
    }
}

/// Handles the [`ProcessConfirmStoringRequest`] event.
///
/// This event is triggered by the runtime when it decides it is the right time to submit a confirm
/// storing extrinsic (and update the local forest root).
impl<NT> EventHandler<ProcessConfirmStoringRequest> for BspUploadFileTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: ProcessConfirmStoringRequest) -> anyhow::Result<()> {
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
        let current_forest_key = CURRENT_FOREST_KEY.to_vec();

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
        let mut file_metadatas = HashMap::new();
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
                    ))
                    .retry_only_if_timeout(),
                true,
            )
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to confirm file after {} retries: {:?}",
                    MAX_CONFIRM_STORING_REQUEST_TRY_COUNT,
                    e
                )
            })?;

        // Release the forest root write "lock" and finish the task.
        self.storage_hub_handler
            .blockchain
            .release_forest_root_write_lock(forest_root_write_tx)
            .await
    }
}

impl<NT> BspUploadFileTask<NT>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
{
    async fn handle_new_storage_request_event(
        &mut self,
        event: NewStorageRequest,
    ) -> anyhow::Result<()> {
        if event.size == 0 {
            let err_msg = "File size cannot be 0";
            error!(target: LOG_TARGET, err_msg);
            return Err(anyhow!(err_msg));
        }

        // First check if the file is not on our exclude list
        let is_allowed = self.is_allowed(&event).await?;

        if !is_allowed {
            return Ok(());
        }

        // Get the current Forest key of the Provider running this node.
        let current_forest_key = CURRENT_FOREST_KEY.to_vec();

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
                "Skipping file key {:?} NewStorageRequest because we are already storing it.",
                event.file_key
            );
            return Ok(());
        }

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

            // Note: the logic below is not ideal since it's not efficient (multiple increases in
            // capacity might occur when one would suffice if multiple tasks are executing it, for example),
            // but it's a temporary solution until we have a better way to handle this.

            // Check that the BSP has not reached the maximum storage capacity.
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

            // Register the capacity change in the queue.
            let mut capacity_queue = self.capacity_queue.lock().await;
            *capacity_queue = capacity_queue.add(event.size);
            drop(capacity_queue);

            // Get the earliest block at which this BSP can change its capacity.
            // This is done after registering the capacity increase in the queue in case another task was currently
            // increasing the capacity as well, so we have the most up-to-date earliest change capacity block.
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

            // Read from the queue if there is a capacity change remaining.
            let mut capacity_queue = self.capacity_queue.lock().await;

            // If there is, apply it.
            if *capacity_queue > 0 {
                let size: u64 = *capacity_queue;

                // Get the current capacity of the BSP. This is needed since it could have changed between the time we
                // registered the capacity increase in the queue and the time we are actually increasing the capacity.
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

                // Calculate the new capacity that the BSP has to have to be able to volunteer for the storage request.
                let new_capacity = self.calculate_capacity(size, current_capacity)?;

                // Send the extrinsic to change this BSP's capacity and wait for it to succeed.
                let call = storage_hub_runtime::RuntimeCall::Providers(
                    pallet_storage_providers::Call::change_capacity { new_capacity },
                );

                self.storage_hub_handler
                    .blockchain
                    .send_extrinsic(call, SendExtrinsicOptions::default())
                    .await?
                    .with_timeout(Duration::from_secs(
                        self.storage_hub_handler
                            .provider_config
                            .extrinsic_retry_timeout,
                    ))
                    .watch_for_success(&self.storage_hub_handler.blockchain)
                    .await?;

                // Reset the capacity queue.
                *capacity_queue = 0;

                info!(
                    target: LOG_TARGET,
                    "Increased storage capacity to {:?} bytes",
                    new_capacity
                );
            }

            drop(capacity_queue);

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

        // Calculate the tick in which the BSP should send the extrinsic. It's one less that the tick
        // in which the BSP can volunteer for the file because that way it the extrinsic will get included
        // in the tick where the BSP can actually volunteer for the file.
        let tick_to_wait_to_submit_volunteer = earliest_volunteer_tick.saturating_sub(1);

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
        let result = self
            .storage_hub_handler
            .blockchain
            .send_extrinsic(call.clone(), SendExtrinsicOptions::default())
            .await?
            .with_timeout(Duration::from_secs(
                self.storage_hub_handler
                    .provider_config
                    .extrinsic_retry_timeout,
            ))
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await;

        if let Err(e) = result {
            error!(
                target: LOG_TARGET,
                "Failed to volunteer for file {:?}: {:?}",
                file_key,
                e
            );

            // If the initial call errored out, it could mean the chain was spammed so the tick did not advance.
            // Wait until the actual earliest volunteer tick to occur and retry volunteering.
            self.storage_hub_handler
                .blockchain
                .wait_for_tick(earliest_volunteer_tick)
                .await?;

            // Send extrinsic and wait for it to be included in the block.
            let result = self
                .storage_hub_handler
                .blockchain
                .send_extrinsic(call, SendExtrinsicOptions::default())
                .await?
                .with_timeout(Duration::from_secs(
                    self.storage_hub_handler
                        .provider_config
                        .extrinsic_retry_timeout,
                ))
                .watch_for_success(&self.storage_hub_handler.blockchain)
                .await;

            if let Err(e) = result {
                error!(
                    target: LOG_TARGET,
                    "Failed to volunteer for file {:?} after retry in volunteer tick: {:?}",
                    file_key,
                    e
                );

                self.unvolunteer_file(file_key.into()).await;
            }
        }

        Ok(())
    }

    /// Handles the [`RemoteUploadRequest`] event.
    ///
    /// Returns `true` if the file is complete, `false` if the file is incomplete.
    async fn handle_remote_upload_request_event(
        &mut self,
        event: RemoteUploadRequest,
    ) -> anyhow::Result<bool> {
        let file_key = event.file_key.into();
        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;

        // Get the file metadata to verify the fingerprint
        let file_metadata = write_file_storage
            .get_metadata(&file_key)
            .map_err(|e| anyhow!("Failed to get file metadata: {:?}", e))?
            .ok_or_else(|| anyhow!("File metadata not found"))?;

        // Verify that the fingerprint in the proof matches the expected file fingerprint
        let expected_fingerprint = file_metadata.fingerprint;
        if event.file_key_proof.file_metadata.fingerprint != expected_fingerprint {
            error!(
                target: LOG_TARGET,
                "Fingerprint mismatch for file {:?}. Expected: {:?}, got: {:?}",
                file_key, expected_fingerprint, event.file_key_proof.file_metadata.fingerprint
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

        let mut file_complete = false;

        // Process each proven chunk in the batch
        for chunk in proven {
            // TODO: Add a batched write chunk method to the file storage.

            // Validate chunk size
            let chunk_idx = chunk.key.as_u64();
            if !file_metadata.is_valid_chunk_size(chunk_idx, chunk.data.len()) {
                error!(
                    target: LOG_TARGET,
                    "Invalid chunk size for chunk {:?} of file {:?}. Expected: {}, got: {}",
                    chunk.key,
                    file_key,
                    file_metadata.chunk_size_at(chunk_idx),
                    chunk.data.len()
                );
                return Err(anyhow!(
                    "Invalid chunk size. Expected {}, got {}",
                    file_metadata.chunk_size_at(chunk_idx),
                    chunk.data.len()
                ));
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
                    | FileStorageWriteError::ChunkCountOverflow => {
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
                    | FileStorageWriteError::FailedToContructFileTrie => {
                        return Err(anyhow::anyhow!(format!(
                            "This is a bug! Failed to construct trie iter for key {:?}.",
                            event.file_key
                        )));
                    }
                },
            }
        }

        Ok(file_complete)
    }

    async fn is_allowed(&self, event: &NewStorageRequest) -> anyhow::Result<bool> {
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

        let owner = H256::from(event.who.as_ref());
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

        return Ok(true);
    }

    /// Calculate the new capacity after adding the required capacity for the file.
    ///
    /// The new storage capacity will be increased by the jump capacity until it reaches the
    /// `max_storage_capacity`.
    ///
    /// The `max_storage_capacity` is returned if the new capacity exceeds it.
    fn calculate_capacity(
        &self,
        required_additional_capacity: StorageDataUnit,
        current_capacity: StorageDataUnit,
    ) -> Result<StorageDataUnit, anyhow::Error> {
        let jump_capacity = self.storage_hub_handler.provider_config.jump_capacity;
        let jumps_needed = required_additional_capacity.div_ceil(jump_capacity);
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

    async fn unvolunteer_file(&self, file_key: H256) {
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
