//! # MSP Upload File Task
//!
//! This module handles the complete file upload flow for Main Storage Providers (MSPs).
//!
//! ## Concurrent Task Architecture
//!
//! The task uses an **actor-based event-driven model** where multiple events can be processed
//! concurrently. Each event handler is spawned as a separate async task by the actor framework
//! when subscribed via [`subscribe_actor_event_map!`](crate::handler::subscribe_actor_event_map).
//!
//! ### Event Flow
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────────────┐
//! │                          BlockchainService                                       │
//! │     (manages file_key_statuses in MspHandler, emits only new requests)           │
//! │                                      │                                           │
//! │                 Filter: only emit if file key NOT in statuses                    │
//! │                     ┌────────────────┼────────────────┐                          │
//! │                     ▼                ▼                ▼                          │
//! │            NewStorageRequest  NewStorageRequest  NewStorageRequest               │
//! │            (per file key)     (per file key)     (per file key)                  │
//! │                     │                │                │                          │
//! │          ┌──────────┴────────┐       │       ┌───────┴──────────┐                │
//! │          ▼                   ▼       ▼       ▼                  ▼                │
//! │   [File in storage?]    RemoteUploadRequest (chunk uploads from user)            │
//! │          │                          │                                            │
//! │          │ yes                      │ file complete                              │
//! │          ▼                          ▼                                            │
//! │      on_file_complete ◄─────────────┘                                            │
//! │          │                                                                       │
//! │          ▼                                                                       │
//! │   queue_msp_respond_storage_request                                              │
//! │          │                                                                       │
//! │          ▼                                                                       │
//! │   ProcessMspRespondStoringRequest                                                │
//! │   (batched extrinsic submission)                                                 │
//! │          │                                                                       │
//! │          ├─── Success ─────────► status removed (cleanup on next block)          │
//! │          │                                                                       │
//! │          ├─── Proof Error ──────► remove_file_key_status ─┐                      │
//! │          │    (transient, retryable)                      │                      │
//! │          │                                                │                      │
//! │          ├─── Extrinsic Failure ──► remove_file_key_status┤                      │
//! │          │    (timeout after retries)                     │                      │
//! │          │                                                ▼                      │
//! │          │                               BlockchainService                       │
//! │          │                               (will re-emit on next block)            │
//! │          │                                                                       │
//! │          └─── Non-proof Error ──► set_file_key_status(Abandoned)                 │
//! │                                                                                  │
//! │  ───────────────────────── Lifecycle Cleanup ─────────────────────────           │
//! │                                                                                  │
//! │   File key no longer in pending requests ──► status removed (cleanup)            │
//! │   (storage request lifecycle complete: accepted, rejected, expired, etc.)        │
//! └──────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Status Tracking with [`FileKeyStatus`]
//!
//! File key status tracking is centralized in the [`BlockchainService`](shc_blockchain_service)'s
//! `MspHandler`. This prevents duplicate processing and enables automatic retries.
//!
//! The blockchain service filters pending storage requests before emitting events:
//!
//! | Status                        | Meaning                                    | Action in BlockchainService                          |  
//! | ----------------------------- | ------------------------------------------ | -----------------------------------------------------|  
//! | [`FileKeyStatus::Processing`] | File key is in the pipeline                | **Skip** (don't emit)                                |  
//! | [`FileKeyStatus::Abandoned`]  | Failed with non-proof dispatch error       | **Skip** (don't emit)                                |  
//! | *Not present*                 | New or retryable file key                  | **Emit** (set status to `Processing`)                |  
//!
//! ### Retry Mechanism
//!
//! The retry mechanism works by **removing** file keys from statuses to signal that they
//! should be re-processed on the next block. Tasks use the `remove_file_key_status` command:
//!
//! - **Proof errors** (`ForestProofVerificationFailed`,
//!   `FailedToApplyDelta`): File key is **removed** from statuses via command. The next
//!   block's processing will re-emit a [`NewStorageRequest`] event with `Processing` status.
//!
//! - **Extrinsic submission timeouts**: File key is **removed** from statuses via command.
//!   Timeouts are retried automatically (see [`RetryStrategy::retry_only_if_timeout`]) since
//!   they are transient errors (network issues, collator congestion) that may resolve on retry.
//!
//! - **Non-proof dispatch errors** (authorization failures, invalid parameters, etc.):
//!   File key is marked as `Abandoned` via [`FileKeyStatusUpdate`] command.
//!   These are permanent failures not resolved by retrying, so the file key will be skipped.
//!
//! ### Event Handlers
//!
//! - [`NewStorageRequest`]: Emitted by the [`BlockchainService`](shc_blockchain_service) only for
//!   file keys that don't have a status yet. The handler validates capacity, creates the file
//!   in storage, and registers for P2P upload. If the file is already complete in file storage,
//!   immediately queues an accept response via
//!   [`queue_msp_respond_storage_request`](shc_blockchain_service::commands::BlockchainServiceCommandInterface::queue_msp_respond_storage_request).
//!
//! - [`RemoteUploadRequest`]: Receives and validates file chunks from users. When the file is
//!   fully received, queues an accept response via
//!   [`queue_msp_respond_storage_request`](shc_blockchain_service::commands::BlockchainServiceCommandInterface::queue_msp_respond_storage_request).
//!
//! - [`ProcessMspRespondStoringRequest`]: Processes queued accept/reject responses and submits
//!   them in a single batched `msp_respond_storage_requests_multiple_buckets` extrinsic.
//!   Status cleanup happens automatically when the file key no longer appears in pending
//!   storage requests.
//!
//! ### Lifecycle Cleanup
//!
//! When a file key's storage request lifecycle is complete (it no longer appears in pending
//! storage requests), its status is automatically removed during block processing. This
//! happens regardless of how the request was resolved (accepted, rejected, expired, etc.).

use anyhow::anyhow;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    time::Duration,
};

use sc_network::PeerId;
use sc_tracing::tracing::*;
use shc_blockchain_service::types::{
    FileKeyStatusUpdate, MspRespondStorageRequest, RespondStorageRequest, RetryStrategy,
};
use shc_blockchain_service::{capacity_manager::CapacityRequestData, types::SendExtrinsicOptions};
use sp_core::H256;
use sp_runtime::{
    traits::{CheckedAdd, CheckedSub, SaturatedConversion, Zero},
    DispatchError,
};

use pallet_file_system::types::RejectedStorageRequest;
use pallet_proofs_dealer;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::{BlockchainServiceCommandInterface, BlockchainServiceCommandInterfaceExt},
    events::{NewStorageRequest, ProcessMspRespondStoringRequest},
};
use shc_common::{
    blockchain_utils::decode_module_error,
    traits::StorageEnableRuntime,
    types::{
        FileKey, FileKeyWithProof, FileMetadata, HashT, RejectedStorageRequestReason,
        StorageEnableErrors, StorageEnableEvents, StorageHubEventsVec,
        StorageProofsMerkleTrieLayout, StorageProviderId, StorageRequestMspAcceptedFileKeys,
        StorageRequestMspBucketResponse, BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE,
    },
};
use shc_file_manager::traits::{FileStorage, FileStorageWriteError, FileStorageWriteOutcome};
use shc_file_transfer_service::{
    commands::FileTransferServiceCommandInterface, events::RemoteUploadRequest,
};
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use shp_file_metadata::{Chunk, ChunkId, Leaf};

use shc_telemetry::{inc_counter_by, STATUS_SUCCESS};

use crate::{
    handler::StorageHubHandler,
    types::{ForestStorageKey, MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "msp-upload-file-task";

/// Configuration for the MSP upload file task
#[derive(Debug, Clone)]
pub struct MspUploadFileConfig {
    /// Maximum number of times to retry submitting respond storage request extrinsic
    pub max_try_count: u32,
    /// Maximum tip amount to use when submitting respond storage request extrinsic
    pub max_tip: u128,
}

impl Default for MspUploadFileConfig {
    fn default() -> Self {
        Self {
            max_try_count: 3,
            max_tip: 500,
        }
    }
}

/// Information about a storage request rejection that needs to be handled.
#[derive(Debug)]
struct RejectionInfo {
    file_key: H256,
    bucket_id: H256,
    reason: RejectedStorageRequestReason,
    error_message: String,
}

impl RejectionInfo {
    /// Creates a new `RejectionInfo` from file metadata and rejection details.
    fn new(
        file_key: H256,
        file_metadata: &FileMetadata,
        reason: RejectedStorageRequestReason,
        error_message: String,
    ) -> Self {
        Self {
            file_key,
            bucket_id: H256::from_slice(file_metadata.bucket_id().as_ref()),
            reason,
            error_message,
        }
    }
}

/// Handles the complete file upload flow for Main Storage Providers (MSPs).
///
/// This task processes multiple concurrent events using an actor-based model.
/// See [module documentation](self) for the full architecture and event flow diagram.
///
/// # Event Handlers
///
/// | Event                               | Purpose                                                              |
/// | ----------------------------------- | -------------------------------------------------------------------- |
/// | [`NewStorageRequest`]               | Emitted by BlockchainService; checks status, handles capacity/upload |
/// | [`RemoteUploadRequest`]             | Chunk reception; queues accept when file complete                    |
/// | [`ProcessMspRespondStoringRequest`] | Batched on-chain response submission                                 |
///
/// # Status Tracking
///
/// File key status tracking is managed centrally by the
/// [`BlockchainService`](shc_blockchain_service::BlockchainService)'s `MspHandler`.
/// Tasks update statuses via commands (e.g., `set_file_key_status`, `remove_file_key_status`).
pub struct MspUploadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
    config: MspUploadFileConfig,
}

impl<NT, Runtime> Clone for MspUploadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> MspUploadFileTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            config: self.config.clone(),
        }
    }
}

impl<NT, Runtime> MspUploadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler,
            config: MspUploadFileConfig::default(),
        }
    }

    pub fn with_config(mut self, config: MspUploadFileConfig) -> Self {
        self.config = config;
        self
    }
}

/// Handles the [`NewStorageRequest`] event.
///
/// This event is emitted by the blockchain service for each pending storage request.
/// The blockchain service filters out file keys that already have a status (Processing,
/// Accepted, Rejected, or Abandoned), so this handler only receives new file keys.
///
/// The MSP will check if it has enough storage capacity to store the file and increase it
/// if necessary (up to a maximum). If the MSP does not have enough capacity still, it will
/// reject the storage request. It will register the user and file key in the registry of
/// the File Transfer Service, which handles incoming p2p upload requests. Finally, it will
/// create a file in the file storage so that it can write uploaded chunks as soon as possible.
impl<NT, Runtime> EventHandler<NewStorageRequest<Runtime>> for MspUploadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: NewStorageRequest<Runtime>) -> anyhow::Result<String> {
        info!(
            target: LOG_TARGET,
            "Registering user peer for file_key [{:x}], location [{}], fingerprint {:x}, bucket [0x{:x}]",
            event.file_key,
            String::from_utf8_lossy(event.location.as_slice()),
            event.fingerprint,
            event.bucket_id
        );

        let bucket_id = H256::from_slice(event.bucket_id.as_ref());
        let file_key = H256::from_slice(event.file_key.as_ref());

        let result = self.handle_new_storage_request_event(event).await;
        match result {
            Ok(()) => Ok(format!(
                "Handled NewStorageRequest for file_key [{:x}]",
                file_key
            )),
            Err(reason) => {
                error!(target: LOG_TARGET, "Failed to handle new storage request: {:?}", reason);

                self.handle_rejected_storage_request(
                    &file_key,
                    bucket_id,
                    // TODO: Receive actual reason error variant from internal call to `handle_new_storage_request_event`
                    RejectedStorageRequestReason::InternalError,
                )
                .await
                .map_err(|e| anyhow!("Failed to handle rejected storage request: {:?}", e))?;

                return Err(anyhow!(
                    "Failed to handle new storage request: {:?}",
                    reason
                ));
            }
        }
    }
}

/// Handles the [`RemoteUploadRequest`] event.
///
/// This event is triggered by a user sending a chunk of the file to the MSP. It checks the proof
/// for the chunk and if it is valid, stores it, until the whole file is stored.
impl<NT, Runtime> EventHandler<RemoteUploadRequest<Runtime>> for MspUploadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: RemoteUploadRequest<Runtime>,
    ) -> anyhow::Result<String> {
        trace!(target: LOG_TARGET, "Received remote upload request for file {:x} and peer {:?}", event.file_key, event.peer);

        let file_complete = match self.handle_remote_upload_request_event(event.clone()).await {
            Ok(complete) => complete,
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

        // Handle file completion if the entire file is uploaded or is already being stored.
        if file_complete {
            self.on_file_complete(&event.file_key.into()).await;
        }

        Ok(format!(
            "Handled RemoteUploadRequest for file [{:x}] (complete: {})",
            event.file_key, file_complete
        ))
    }
}

/// Handles the [`ProcessMspRespondStoringRequest`] event.
///
/// Triggered when there are new storage request(s) to respond to. Normally, storage requests are
/// immediately rejected if the MSP cannot store the file (e.g. not enough capacity). However, this event
/// is able to respond to storage requests that are either being accepted or rejected either way.
///
/// The MSP will call the `msp_respond_storage_requests_multiple_buckets` extrinsic on the FileSystem pallet to respond to the
/// storage requests.
impl<NT, Runtime> EventHandler<ProcessMspRespondStoringRequest<Runtime>>
    for MspUploadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: ProcessMspRespondStoringRequest<Runtime>,
    ) -> anyhow::Result<String> {
        info!(
            target: LOG_TARGET,
            "Processing ProcessMspRespondStoringRequest: {:?}",
            event.data.respond_storing_requests,
        );

        match self
            .handle_process_msp_respond_storing_request_event(event.clone())
            .await
        {
            Ok(result) => Ok(format!(
                "Handled ProcessMspRespondStoringRequest: {}",
                result
            )),
            Err(e) => {
                error!(target: LOG_TARGET, "Failed to handle process msp respond storing request event: {:?}", e);

                // Remove all file keys from statuses for later retry
                for request in event.data.respond_storing_requests.iter() {
                    self.storage_hub_handler
                        .blockchain
                        .remove_file_key_status(request.file_key.into())
                        .await;
                }

                Err(anyhow!(
                    "Failed to handle process msp respond storing request event: {:?}",
                    e
                ))
            }
        }
    }
}

impl<NT, Runtime> MspUploadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_new_storage_request_event(
        &mut self,
        event: NewStorageRequest<Runtime>,
    ) -> anyhow::Result<()> {
        if event.size == Zero::zero() {
            let err_msg = "File size cannot be 0";
            error!(target: LOG_TARGET, err_msg);
            return Err(anyhow!(err_msg));
        }

        let own_provider_id = self
            .storage_hub_handler
            .blockchain
            .query_storage_provider_id(None)
            .await?;

        let own_msp_id = match own_provider_id {
            Some(id) => match id {
                StorageProviderId::MainStorageProvider(id) => id,
                StorageProviderId::BackupStorageProvider(_) => {
                    let err_msg = "Current node account is a Backup Storage Provider. Expected a Main Storage Provider ID.";
                    error!(target: LOG_TARGET, err_msg);
                    return Err(anyhow!(err_msg));
                }
            },
            None => {
                let err_msg = "Failed to get own MSP ID.";
                error!(target: LOG_TARGET, err_msg);
                return Err(anyhow!(err_msg));
            }
        };

        let msp_id_of_bucket_id = self
            .storage_hub_handler
            .blockchain
            .query_msp_id_of_bucket_id(event.bucket_id)
            .await
            .map_err(|e| {
                let err_msg = format!(
                    "Failed to query MSP ID of bucket ID {:?}\n Error: {:?}",
                    event.bucket_id, e
                );
                error!(target: LOG_TARGET, err_msg);
                anyhow!(err_msg)
            })?;

        if let Some(msp_id) = msp_id_of_bucket_id {
            if own_msp_id != msp_id {
                trace!(target: LOG_TARGET, "Skipping storage request - MSP ID does not match own MSP ID for bucket ID {:?}", event.bucket_id);
                return Ok(());
            }
        } else {
            warn!(target: LOG_TARGET, "Skipping storage request - MSP ID not found for bucket ID {:?}", event.bucket_id);
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
        .map_err(|_| anyhow!("Invalid file metadata"))?;

        // Get the file key.
        let file_key: FileKey = metadata
            .file_key::<HashT<StorageProofsMerkleTrieLayout>>()
            .as_ref()
            .try_into()?;

        let fs = self
            .storage_hub_handler
            .forest_storage_handler
            .get_or_create(&ForestStorageKey::from(event.bucket_id.as_ref().to_vec()))
            .await
            .map_err(|e| {
                anyhow!(
                    "CRITICAL ❗️❗️❗️: Failed to get or create forest storage: {:?}",
                    e
                )
            })?;

        // If we do not have the file already in forest storage, we must take into account the
        // available storage capacity.
        let file_in_forest_storage = {
            let read_fs = fs.read().await;
            read_fs.contains_file_key(&file_key.into())?
        };
        if !file_in_forest_storage {
            info!(target: LOG_TARGET, "File key [{:x}] not found in forest storage. Checking available storage capacity.", file_key);

            let max_storage_capacity = self
                .storage_hub_handler
                .provider_config
                .capacity_config
                .max_capacity();

            let current_capacity = self
                .storage_hub_handler
                .blockchain
                .query_storage_provider_capacity(own_msp_id)
                .await
                .map_err(|e| {
                    error!(target: LOG_TARGET, "Failed to query storage provider capacity: {:?}", e);
                    anyhow!("Failed to query storage provider capacity: {:?}", e)
                })?;

            let available_capacity = self
                .storage_hub_handler
                .blockchain
                .query_available_storage_capacity(own_msp_id)
                .await
                .map_err(|e| {
                    let err_msg = format!("Failed to query available storage capacity: {:?}", e);
                    error!(
                        target: LOG_TARGET,
                        err_msg
                    );
                    anyhow!(err_msg)
                })?;

            // Calculate currently used storage
            let used_capacity = current_capacity
                .checked_sub(&available_capacity)
                .ok_or_else(|| {
                    anyhow!(
                        "Available capacity ({}) exceeds current capacity ({})",
                        available_capacity,
                        current_capacity
                    )
                })?;

            // Check if accepting this file would exceed our local max storage capacity limit
            let projected_usage = used_capacity
                .checked_add(&event.size)
                .ok_or_else(|| anyhow!("Overflow calculating projected storage usage"))?;

            if projected_usage > max_storage_capacity {
                let err_msg = format!(
                    "Accepting file would exceed maximum storage capacity limit. Used: {}, Required: {}, Max: {}",
                    used_capacity, event.size, max_storage_capacity
                );
                warn!(target: LOG_TARGET, "{}", err_msg);
                return Err(anyhow!(err_msg));
            }

            // Increase storage capacity if the available capacity is less than the file size.
            if available_capacity < event.size {
                warn!(
                    target: LOG_TARGET,
                    "Insufficient storage capacity to volunteer for file key: {:x}",
                    event.file_key
                );

                self.storage_hub_handler
                    .blockchain
                    .increase_capacity(CapacityRequestData::new(event.size))
                    .await
                    .map_err(|e| anyhow!("Failed to increase storage capacity: {:?}", e))?;

                let available_capacity = self
                    .storage_hub_handler
                    .blockchain
                    .query_available_storage_capacity(own_msp_id)
                    .await
                    .map_err(|e| {
                        let err_msg =
                            format!("Failed to query available storage capacity: {:?}", e);
                        error!(
                            target: LOG_TARGET,
                            err_msg
                        );
                        anyhow!(err_msg)
                    })?;

                // Reject storage request if the new available capacity is still less than the file size.
                if available_capacity < event.size {
                    let err_msg = format!(
                        "Increased storage capacity is still insufficient to volunteer for file. Rejecting storage request. Available: {}, Required: {}",
                        available_capacity, event.size
                    );
                    warn!(target: LOG_TARGET, err_msg);

                    return Err(anyhow!(err_msg));
                }
            }
        } else {
            debug!(target: LOG_TARGET, "File key [{:x}] found in forest storage.", file_key);
        }

        // Create file in file storage if it is not present so we can write uploaded chunks as soon as possible.
        let file_in_file_storage = {
            let read_file_storage = self.storage_hub_handler.file_storage.read().await;
            read_file_storage
                .get_metadata(&file_key.into())
                .map_err(|e| anyhow!("Failed to get metadata from file storage: {:?}", e))?
                .is_some()
        };

        debug!(
            target: LOG_TARGET,
            "File key [{:x}]: file_in_file_storage={}, file_in_forest_storage={}",
            file_key,
            file_in_file_storage,
            file_in_forest_storage
        );

        if !file_in_file_storage {
            let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;
            debug!(target: LOG_TARGET, "File key [{:x}] not found in file storage. Inserting file.", file_key);
            write_file_storage
                .insert_file(
                    metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>(),
                    metadata,
                )
                .map_err(|e| anyhow!("Failed to insert file in file storage: {:?}", e))?;
        } else {
            // If the file is in file storage, we can skip the file transfer,
            // and proceed to accepting the storage request directly, provided that we have the entire file in file storage.
            info!(target: LOG_TARGET, "File key [{:x}] found in file storage. No need to receive the file from the user.", file_key);

            // Do not skip the file key even if it is in forest storage since not responding to the storage request or rejecting it would result in the file key being deleted from the network entirely.
            if file_in_forest_storage {
                info!(target: LOG_TARGET, "File key [{:x}] found in forest storage when storage request is open. The storage request is most likely opened to increase replication amongst BSPs, but still requires the MSP to accept the request.", file_key);
            }

            let file_complete = {
                let read_file_storage = self.storage_hub_handler.file_storage.read().await;
                match read_file_storage.is_file_complete(&file_key.into()) {
                    Ok(is_complete) => is_complete,
                    Err(e) => {
                        warn!(target: LOG_TARGET, "Failed to check if file is complete. The file key [{:x}] is in a bad state with error: {:?}", file_key, e);
                        warn!(target: LOG_TARGET, "Assuming the file is not complete.");
                        false
                    }
                }
            };

            if file_complete {
                info!(target: LOG_TARGET, "File key [{:x}] is complete in file storage. Proceeding to accept storage request.", file_key);
                self.on_file_complete(&file_key.into()).await;

                // This finishes the task, as we already have the entire file in file storage and we queued
                // the accept transaction to the blockchain, so we can finish the task early.
                return Ok(());
            } else {
                debug!(target: LOG_TARGET, "File key [{:x}] is not complete in file storage. Need to receive the file from the user.", file_key);
            }
        }

        // Register the file for upload in the file transfer service.
        // Even though we could already have the entire file in file storage, we
        // allow the user to connect to us and upload the file. Once they do, we will
        // send back the `file_complete` flag to true signalling to the user that we have
        // the entire file so that the file uploading process is complete.
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

        Ok(())
    }

    async fn handle_remote_upload_request_event(
        &mut self,
        event: RemoteUploadRequest<Runtime>,
    ) -> anyhow::Result<bool> {
        let file_key = event.file_key.into();
        let bucket_id = match self
            .storage_hub_handler
            .file_storage
            .read()
            .await
            .get_metadata(&file_key)
        {
            Ok(metadata) => match metadata {
                Some(metadata) => H256::from_slice(metadata.bucket_id().as_ref()),
                None => {
                    let err_msg = format!(
                        "File does not exist for key [{:x}]. Maybe we forgot to unregister before deleting?",
                        event.file_key
                    );
                    error!(target: LOG_TARGET, err_msg);
                    return Err(anyhow!(err_msg));
                }
            },
            Err(e) => {
                let err_msg = format!("Failed to get file metadata: {:?}", e);
                error!(target: LOG_TARGET, err_msg);
                return Err(anyhow!(err_msg));
            }
        };

        // Get the file metadata to verify the fingerprint
        let file_metadata = {
            let read_file_storage = self.storage_hub_handler.file_storage.read().await;
            read_file_storage
                .get_metadata(&file_key)
                .map_err(|e| anyhow!("Failed to get file metadata: {:?}", e))?
                .ok_or_else(|| anyhow!("File metadata not found"))?
        };

        // Verify that the fingerprint in the proof matches the expected file fingerprint
        let expected_fingerprint = file_metadata.fingerprint();
        if event.file_key_proof.file_metadata.fingerprint() != expected_fingerprint {
            error!(
                target: LOG_TARGET,
                "Fingerprint mismatch for file {:x}. Expected: {:x}, got: {:x}",
                file_key, expected_fingerprint, event.file_key_proof.file_metadata.fingerprint()
            );
            self.handle_rejected_storage_request(
                &file_key,
                bucket_id,
                RejectedStorageRequestReason::ReceivedInvalidProof,
            )
            .await?;
            return Err(anyhow!("Fingerprint mismatch"));
        }

        // Verify and extract chunks from proof
        let proven = match event
            .file_key_proof
            .proven::<StorageProofsMerkleTrieLayout>()
        {
            Ok(proven) => {
                if proven.is_empty() {
                    Err(anyhow!("Expected at least one proven chunk but got none."))
                } else {
                    // Calculate total batch size
                    let total_batch_size: usize = proven.iter().map(|chunk| chunk.data.len()).sum();

                    if total_batch_size > BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE {
                        Err(anyhow!(
                            "Total batch size {} bytes exceeds maximum allowed size of {} bytes",
                            total_batch_size,
                            BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE
                        ))
                    } else {
                        Ok(proven)
                    }
                }
            }
            Err(e) => Err(anyhow!(
                "Failed to verify and get proven file key chunks: {:?}",
                e
            )),
        };

        // Handle invalid proof case
        let proven = match proven {
            Ok(proven) => proven,
            Err(error) => {
                error!(
                    target: LOG_TARGET,
                    "Failed to verify proof for file {:x}: {:?}",
                    file_key, error
                );
                self.handle_rejected_storage_request(
                    &file_key,
                    bucket_id,
                    RejectedStorageRequestReason::ReceivedInvalidProof,
                )
                .await?;
                return Err(anyhow!("Failed to verify proof"));
            }
        };

        // Calculate total bytes received for metrics before processing
        let total_bytes_received: u64 = proven.iter().map(|chunk| chunk.data.len() as u64).sum();

        // Process chunks within a scoped block to ensure the file storage lock is dropped before handling rejections
        let result = {
            let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;
            self.process_chunks_with_lock(
                &file_key,
                &file_metadata,
                proven,
                &mut write_file_storage,
            )
        };

        // Handle the result after the file storage lock is dropped
        match result {
            Ok(file_complete) => {
                // Record MSP bytes received from user on successful chunk processing
                inc_counter_by!(
                    handler: self.storage_hub_handler,
                    msp_bytes_received_total,
                    STATUS_SUCCESS,
                    total_bytes_received
                );
                Ok(file_complete)
            }
            Err(rejection) => {
                self.handle_rejected_storage_request(
                    &rejection.file_key,
                    rejection.bucket_id,
                    rejection.reason,
                )
                .await?;
                Err(anyhow!(rejection.error_message))
            }
        }
    }

    async fn handle_process_msp_respond_storing_request_event(
        &mut self,
        event: ProcessMspRespondStoringRequest<Runtime>,
    ) -> anyhow::Result<String> {
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

        let own_msp_id = match own_provider_id {
            Some(StorageProviderId::MainStorageProvider(id)) => id,
            Some(StorageProviderId::BackupStorageProvider(_)) => {
                return Err(anyhow!(
                    "Current node account is a Backup Storage Provider. Expected a Main Storage Provider ID."
                ));
            }
            None => {
                return Err(anyhow!("Failed to get own MSP ID."));
            }
        };

        // Collect all file keys (both accept and reject) to check if they're still pending to filter
        // any stale file keys which no longer have a pending storage requests.
        let file_keys_to_check: Vec<FileKey> = event
            .data
            .respond_storing_requests
            .iter()
            .map(|r| r.file_key.into())
            .collect();

        if file_keys_to_check.is_empty() {
            warn!(target: LOG_TARGET, "No file keys to respond to in ProcessMspRespondStoringRequest. Responding to {} file keys.", file_keys_to_check.len());
            return Ok(format!(
                "No file keys to respond to in ProcessMspRespondStoringRequest. Responding to {} file keys.",
                file_keys_to_check.len()
            ));
        }

        // Query pending storage requests for all file keys (both accepts and rejects).
        // The runtime API filters to only return requests that are:
        // 1. Assigned to this MSP
        // 2. Not yet responded to (msp.1 == false, meaning not yet accepted/confirmed)
        // Note: We let the blockchain service handle removing stale file keys from statuses.
        let pending_file_keys: HashSet<H256> = match self
            .storage_hub_handler
            .blockchain
            .query_pending_storage_requests(Some(file_keys_to_check.clone()))
            .await
        {
            Ok(requests) => requests
                .into_iter()
                .map(|r| H256::from_slice(r.file_key.as_ref()))
                .collect(),
            Err(e) => {
                warn!(target: LOG_TARGET, "Failed to query storage requests: {:?}. Proceeding with all requests.", e);
                file_keys_to_check
                    .into_iter()
                    .map(|k| H256::from_slice(k.as_ref()))
                    .collect::<HashSet<_>>()
            }
        };

        let mut file_key_responses = HashMap::new();

        // Filter out requests that do not have any pending storage requests.
        let filtered_responses = event
            .data
            .respond_storing_requests
            .iter()
            .filter(|r| pending_file_keys.contains(&r.file_key))
            .collect::<Vec<_>>();
        // For accepted requests, prefetch the chunks to prove without holding any file storage locks.
        let mut chunks_to_prove_by_file_key: HashMap<H256, Vec<_>> = HashMap::new();
        for respond in &filtered_responses {
            if let MspRespondStorageRequest::Accept = &respond.response {
                let chunks_to_prove = match self
                    .storage_hub_handler
                    .blockchain
                    .query_msp_confirm_chunks_to_prove_for_file(own_msp_id, respond.file_key)
                    .await
                {
                    Ok(chunks) => chunks,
                    Err(e) => {
                        error!(target: LOG_TARGET, "Failed to get chunks to prove: {:?}", e);
                        continue;
                    }
                };

                chunks_to_prove_by_file_key.insert(respond.file_key, chunks_to_prove);
            }
        }

        for respond in filtered_responses {
            info!(target: LOG_TARGET, "Processing response for file key [{:x}]", respond.file_key);

            // Acquire a file storage read lock only for metadata/proof generation,
            // for each iteration of the loop, to avoid holding the lock for too long.
            let read_file_storage = self.storage_hub_handler.file_storage.read().await;

            let bucket_id = match read_file_storage.get_metadata(&respond.file_key) {
                Ok(Some(metadata)) => H256::from_slice(metadata.bucket_id().as_ref()),
                Ok(None) => {
                    error!(target: LOG_TARGET, "File does not exist for key [{:x}]. Maybe we forgot to unregister before deleting?", respond.file_key);
                    continue;
                }
                Err(e) => {
                    error!(target: LOG_TARGET, "Failed to get file metadata: {:?}", e);
                    continue;
                }
            };

            let entry = file_key_responses
                .entry(bucket_id)
                .or_insert_with(|| (Vec::new(), Vec::new()));

            match &respond.response {
                MspRespondStorageRequest::Accept => {
                    let Some(chunks_to_prove) = chunks_to_prove_by_file_key.get(&respond.file_key)
                    else {
                        error!(
                            target: LOG_TARGET,
                            "Missing cached chunks_to_prove for accepted file key [{:x}]",
                            respond.file_key
                        );
                        continue;
                    };

                    let proof = match read_file_storage.generate_proof(
                        &respond.file_key,
                        &HashSet::from_iter(chunks_to_prove.iter().cloned()),
                    ) {
                        Ok(p) => p,
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to generate proof: {:?}", e);
                            continue;
                        }
                    };

                    entry.0.push(FileKeyWithProof {
                        file_key: respond.file_key,
                        proof,
                    });
                }
                MspRespondStorageRequest::Reject(reason) => {
                    entry.1.push(RejectedStorageRequest {
                        file_key: respond.file_key,
                        reason: reason.clone(),
                    });
                }
            }
        }

        let mut storage_request_msp_response = Vec::new();

        for (bucket_id, (accept, reject)) in file_key_responses.iter_mut() {
            let fs = self
                .storage_hub_handler
                .forest_storage_handler
                .get_or_create(&ForestStorageKey::from(bucket_id.as_ref().to_vec()))
                .await
                .map_err(|e| {
                    anyhow!(
                        "CRITICAL ❗️❗️❗️: Failed to get or create forest storage: {:?}",
                        e
                    )
                })?;

            let accept = if !accept.is_empty() {
                let file_keys: Vec<_> = accept
                    .iter()
                    .map(|file_key_with_proof| file_key_with_proof.file_key)
                    .collect();

                let forest_proof = match fs.read().await.generate_proof(file_keys) {
                    Ok(proof) => proof,
                    Err(e) => {
                        error!(target: LOG_TARGET, "Failed to generate non-inclusion forest proof: {:?}", e);
                        continue;
                    }
                };

                Some(StorageRequestMspAcceptedFileKeys {
                    file_keys_and_proofs: accept.clone(),
                    forest_proof: forest_proof.proof,
                })
            } else {
                None
            };

            storage_request_msp_response.push(StorageRequestMspBucketResponse {
                bucket_id: *bucket_id,
                accept,
                reject: reject.clone(),
            });
        }

        let call: Runtime::Call =
            pallet_file_system::Call::<Runtime>::msp_respond_storage_requests_multiple_buckets {
                storage_request_msp_response: storage_request_msp_response.clone(),
            }
            .into();

        // Submit the extrinsic with events so we can inspect the dispatch error if it fails.
        let extrinsic_result = self
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
                    Some("mspRespondStorageRequestsMultipleBuckets".to_string()),
                ),
                RetryStrategy::default()
                    .with_max_retries(self.config.max_try_count)
                    .with_max_tip(self.config.max_tip.saturated_into())
                    .retry_only_if_timeout(),
                true,
            )
            .await;

        // Pre collect all file keys from the storage request MSP response so we can update statuses or remove them from statuses.
        let all_file_keys = storage_request_msp_response
            .iter()
            .flat_map(|r| {
                let accepted_keys = r
                    .accept
                    .iter()
                    .flat_map(|a| a.file_keys_and_proofs.iter().map(|fk| fk.file_key));
                let rejected_keys = r.reject.iter().map(|rej| rej.file_key);
                accepted_keys.chain(rejected_keys)
            })
            .collect::<Vec<_>>();

        // Handle extrinsic submission result
        // - If the extrinsic failed, we remove the file keys from statuses to enable automatic retry on the next block.
        // - If the extrinsic succeeded, the file key statuses will be cleaned up when they no longer appear in pending storage requests.
        // - If the extrinsic succeeded but no events were emitted, we remove the file keys from statuses to enable automatic retry on the next block.
        match extrinsic_result {
            Err(e) => {
                error!(
                    target: LOG_TARGET,
                    "Extrinsic submission failed after exhausting retries, removing file keys from statuses for retry: {:?}",
                    e
                );

                self.handle_extrinsic_submission_failure(&all_file_keys)
                    .await;

                // Release the forest root write lock before returning error
                self.storage_hub_handler
                    .blockchain
                    .release_forest_root_write_lock(forest_root_write_tx)
                    .await?;

                return Err(e);
            }
            Ok(Some(events)) => {
                if let Err(err) = self
                    .handle_extrinsic_dispatch_result(events, &all_file_keys)
                    .await
                {
                    error!(target: LOG_TARGET, "Failed to handle extrinsic dispatch result: {:?}", err);
                    // Release the forest root write lock before returning error
                    self.storage_hub_handler
                        .blockchain
                        .release_forest_root_write_lock(forest_root_write_tx)
                        .await?;

                    return Err(err);
                }
            }
            Ok(None) => {
                error!(
                    target: LOG_TARGET,
                    "Expected events but got None - this should not happen. Removing file key statuses to allow re-evaluation on next block."
                );
                self.handle_missing_extrinsic_events(&all_file_keys).await;
            }
        }

        // Delete rejected files from file storage
        for storage_request_msp_bucket_response in storage_request_msp_response {
            for RejectedStorageRequest { file_key, .. } in
                &storage_request_msp_bucket_response.reject
            {
                info!(target: LOG_TARGET, "Deleting rejected file {:x} from file storage", file_key);
                let mut write_fs = self.storage_hub_handler.file_storage.write().await;
                if let Err(e) = write_fs.delete_file(&file_key) {
                    error!(target: LOG_TARGET, "Failed to delete file {:x}: {:?}", file_key, e);
                }
                drop(write_fs);
            }
        }

        // Release the forest root write "lock" and finish the task.
        self.storage_hub_handler
            .blockchain
            .release_forest_root_write_lock(forest_root_write_tx)
            .await?;

        Ok(format!(
            "Processed ProcessMspRespondStoringRequest for MSP [{:x}]",
            own_msp_id
        ))
    }

    /// Processes chunks while holding a file storage write lock. Returns Ok(file_complete) on success,
    /// or Err(RejectionInfo) if the storage request should be rejected.
    fn process_chunks_with_lock(
        &self,
        file_key: &H256,
        file_metadata: &FileMetadata,
        proven: Vec<Leaf<ChunkId, Chunk>>,
        write_file_storage: &mut NT::FL,
    ) -> Result<bool, RejectionInfo>
    where
        NT: ShNodeType<Runtime>,
        Runtime: StorageEnableRuntime,
    {
        let mut file_complete = false;

        // Process each proven chunk in the batch
        for chunk in proven {
            let chunk_idx = chunk.key.as_u64();
            let expected_chunk_size = file_metadata.chunk_size_at(chunk_idx).map_err(|e| {
                RejectionInfo::new(
                    *file_key,
                    file_metadata,
                    RejectedStorageRequestReason::InternalError,
                    format!("Failed to get chunk size for chunk {}: {:?}", chunk_idx, e),
                )
            })?;

            if chunk.data.len() != expected_chunk_size {
                error!(
                    target: LOG_TARGET,
                    "Invalid chunk size for chunk {}: Expected: {}, got: {}",
                    chunk_idx,
                    expected_chunk_size,
                    chunk.data.len()
                );
                return Err(RejectionInfo::new(
                    *file_key,
                    file_metadata,
                    RejectedStorageRequestReason::ReceivedInvalidProof,
                    format!(
                        "Invalid chunk size for chunk {}: Expected: {}, got: {}",
                        chunk_idx,
                        expected_chunk_size,
                        chunk.data.len()
                    ),
                ));
            }

            let write_result = write_file_storage.write_chunk(file_key, &chunk.key, &chunk.data);

            match write_result {
                Ok(FileStorageWriteOutcome::FileComplete) => {
                    file_complete = true;
                    break; // We can stop processing chunks if the file is complete
                }
                Ok(FileStorageWriteOutcome::FileIncomplete) => continue,
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
                        return Err(RejectionInfo::new(
                            *file_key,
                            file_metadata,
                            RejectedStorageRequestReason::InternalError,
                            format!(
                                "File does not exist for key [{:x}]. Maybe we forgot to unregister before deleting?",
                                file_key
                            ),
                        ));
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
                        return Err(RejectionInfo::new(
                            *file_key,
                            file_metadata,
                            RejectedStorageRequestReason::InternalError,
                            format!(
                                "Internal trie read/write error {:x}:{:?}",
                                file_key, chunk.key
                            ),
                        ));
                    }
                    FileStorageWriteError::FingerprintAndStoredFileMismatch => {
                        return Err(RejectionInfo::new(
                            *file_key,
                            file_metadata,
                            RejectedStorageRequestReason::InternalError,
                            format!(
                                "Invariant broken! This is a bug! Fingerprint and stored file mismatch for key [{:x}].",
                                file_key
                            ),
                        ));
                    }
                    FileStorageWriteError::FailedToConstructTrieIter
                    | FileStorageWriteError::FailedToConstructFileTrie => {
                        return Err(RejectionInfo::new(
                            *file_key,
                            file_metadata,
                            RejectedStorageRequestReason::InternalError,
                            format!(
                                "This is a bug! Failed to construct trie iter for key [{:x}].",
                                file_key
                            ),
                        ));
                    }
                },
            }
        }

        // If we haven't found the file to be complete during chunk processing,
        // check if it's complete now (in case this was the last batch)
        if !file_complete {
            match write_file_storage.is_file_complete(file_key) {
                Ok(is_complete) => file_complete = is_complete,
                Err(e) => {
                    let err_msg = format!(
                        "Failed to check if file is complete. The file key [{:x}] is in a bad state with error: {:?}",
                        file_key, e
                    );
                    error!(target: LOG_TARGET, "{}", err_msg);
                    return Err(RejectionInfo::new(
                        *file_key,
                        file_metadata,
                        RejectedStorageRequestReason::InternalError,
                        err_msg,
                    ));
                }
            }
        }

        Ok(file_complete)
    }

    async fn handle_rejected_storage_request(
        &self,
        file_key: &H256,
        bucket_id: H256,
        reason: RejectedStorageRequestReason,
    ) -> anyhow::Result<()> {
        info!(target: LOG_TARGET, "Handling rejected storage request for file key [{:x}] with bucket id [0x{:x}] and reason {:?}", file_key, bucket_id, reason);

        // Unregister the file
        self.unregister_file(*file_key)
            .await
            .map_err(|e| anyhow!("Failed to unregister file: {:?}", e))?;

        info!(target: LOG_TARGET, "Rejected storage request for file key [{:x}]", file_key);

        let call: Runtime::Call =
            pallet_file_system::Call::<Runtime>::msp_respond_storage_requests_multiple_buckets {
                storage_request_msp_response: vec![StorageRequestMspBucketResponse {
                    bucket_id,
                    accept: None,
                    reject: vec![RejectedStorageRequest {
                        file_key: *file_key,
                        reason: reason.clone(),
                    }],
                }],
            }
            .into();

        self.storage_hub_handler
            .blockchain
            .send_extrinsic(
                call,
                SendExtrinsicOptions::new(
                    Duration::from_secs(
                        self.storage_hub_handler
                            .provider_config
                            .blockchain_service
                            .extrinsic_retry_timeout,
                    ),
                    Some("fileSystem".to_string()),
                    Some("mspRespondStorageRequestsMultipleBuckets".to_string()),
                ),
            )
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to submit msp respond storage requests multiple buckets: {:?}",
                    e
                )
            })?
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await
            .map_err(|e| anyhow!("Failed to watch for success: {:?}", e))?;

        info!(target: LOG_TARGET, "Submitted mspRespondStorageRequestsMultipleBuckets extrinsic for file key [{:x}], with reject reason {:?}", file_key, reason);

        Ok(())
    }

    async fn unregister_file(&self, file_key: H256) -> anyhow::Result<()> {
        warn!(target: LOG_TARGET, "Unregistering file [{:x}]", file_key);

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

        Ok(())
    }

    async fn on_file_complete(&self, file_key: &H256) {
        info!(target: LOG_TARGET, "File upload complete (file_key [{:x}])", file_key);

        // Unregister the file from the file transfer service.
        if let Err(e) = self
            .storage_hub_handler
            .file_transfer
            .unregister_file((*file_key).into())
            .await
        {
            warn!(target: LOG_TARGET, "Failed to unregister file [{:x}] from file transfer service: {:?}", file_key, e);
        }

        debug!(target: LOG_TARGET, "File [{:x}] unregistered from file transfer service.", file_key);

        // Queue a request to confirm the storing of the file.
        debug!(target: LOG_TARGET, "Queueing accept request for file key [{:x}]", file_key);
        self.storage_hub_handler
            .blockchain
            .queue_msp_respond_storage_request(RespondStorageRequest::new(
                *file_key,
                MspRespondStorageRequest::Accept,
            ))
            .await;

        debug!(target: LOG_TARGET, "File [{:x}] queued for confirmation", file_key);
    }

    /// Handles extrinsic submission failure after exhausting all retries.
    ///
    /// Removes file keys from statuses to enable automatic retry on the next block.
    /// This is appropriate because submission failures may be transient (network issues,
    /// collator congestion) and retrying with fresh proofs on the next block may succeed.
    async fn handle_extrinsic_submission_failure(&self, file_keys: &[H256]) {
        // Remove file keys from statuses to trigger retry on the next block
        for file_key in file_keys {
            info!(
                target: LOG_TARGET,
                "Removing file key [{:x}] status (extrinsic submission exhausted retries)",
                file_key
            );
            self.storage_hub_handler
                .blockchain
                .remove_file_key_status((*file_key).into())
                .await;
        }
    }

    /// Handles extrinsic dispatch result when events are present.
    ///
    /// Checks for dispatch errors in the events and handles them appropriately:
    /// - Proof errors: Removes file keys from statuses to enable automatic retry
    /// - Non-proof errors: Marks file keys as Abandoned (permanent failure)
    ///
    /// Returns `Ok(())` after successfully handling the dispatch result (whether the
    /// extrinsic succeeded or failed), or `Err(...)` if the module error could not be decoded.
    async fn handle_extrinsic_dispatch_result(
        &self,
        events: StorageHubEventsVec<Runtime>,
        file_keys: &[H256],
    ) -> anyhow::Result<()> {
        // Check if the extrinsic succeeded or failed by looking for an ExtrinsicFailed event
        let maybe_dispatch_error = events.iter().find_map(|event_record| {
            if let StorageEnableEvents::System(frame_system::Event::ExtrinsicFailed {
                dispatch_error,
                ..
            }) = event_record.event.clone().into()
            {
                // Found an ExtrinsicFailed event, return the dispatch error
                Some(dispatch_error)
            } else {
                // No ExtrinsicFailed event found, continue searching
                None
            }
        });

        let Some(dispatch_error) = maybe_dispatch_error else {
            // No dispatch error found, extrinsic succeeded
            return Ok(());
        };

        // Convert dispatch error to known StorageHub errors
        let error: Option<StorageEnableErrors<Runtime>> = match dispatch_error {
            DispatchError::Module(module_error) => {
                match decode_module_error::<Runtime>(module_error) {
                    Ok(decoded) => Some(decoded),
                    Err(e) => {
                        let err_msg = format!("Failed to decode module error. This likely indicates a breaking change in a possible runtime upgrade since a new error variant was encountered and cannot be decoded. Underlying error: {:?}", e);
                        error!(target: LOG_TARGET, "{}", err_msg);
                        return Err(anyhow!(err_msg));
                    }
                }
            }
            _ => {
                warn!(
                    target: LOG_TARGET,
                    "Treating non-module error as non-proof error: {:?}",
                    dispatch_error
                );
                None
            }
        };

        let is_proof_error = matches!(
            error,
            Some(StorageEnableErrors::ProofsDealer(
                pallet_proofs_dealer::Error::ForestProofVerificationFailed
                    | pallet_proofs_dealer::Error::FailedToApplyDelta
            ))
        );

        if is_proof_error {
            // Removes file keys from statuses to enable automatic retry.
            // The next block's NewStorageRequest events will re-insert them with Processing
            // status and regenerate proofs with the updated forest root.
            warn!(
                target: LOG_TARGET,
                "Proof-related error detected, removing file keys from statuses for retry: {:?}",
                dispatch_error
            );

            // Remove file keys from statuses to trigger retry on the next block
            for file_key in file_keys {
                debug!(target: LOG_TARGET, "Removing file key [{:x}] status (proof error)", file_key);
                self.storage_hub_handler
                    .blockchain
                    .remove_file_key_status((*file_key).into())
                    .await;
            }
        } else {
            // Marks file keys as Abandoned since this is a permanent failure
            // that is not guaranteed to be resolved by retrying (e.g., invalid parameters,
            // authorization errors, inconsistent runtime state, etc.).
            warn!(
                target: LOG_TARGET,
                "Non-proof dispatch error, marking file keys as Abandoned: {:?}",
                error
            );

            // Mark file keys as Abandoned
            for file_key in file_keys {
                trace!(
                    target: LOG_TARGET,
                    "Marking file key [{:x}] as Abandoned (non-proof error)",
                    file_key
                );
                self.storage_hub_handler
                    .blockchain
                    .set_file_key_status((*file_key).into(), FileKeyStatusUpdate::Abandoned)
                    .await;
            }
        }

        Ok(())
    }

    /// Handles the case when extrinsic events are missing.
    ///
    /// This shouldn't happen since we requested events with `with_events: true`.
    /// Since we cannot determine the extrinsic outcome, we remove file key statuses
    /// to allow the system to re-evaluate them on the next block.
    async fn handle_missing_extrinsic_events(&self, file_keys: &[H256]) {
        for file_key in file_keys {
            warn!(
                target: LOG_TARGET,
                "Removing file key [{:x}] status (missing events)",
                file_key
            );
            self.storage_hub_handler
                .blockchain
                .remove_file_key_status((*file_key).into())
                .await;
        }
    }
}
