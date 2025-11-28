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
//! │                         BatchProcessStorageRequests                              │
//! │                          (periodic, from BlockchainService)                      │
//! │                                      │                                           │
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
//! │   queue_msp_respond_storage_request (queues Accept)                              │
//! │          │                                                                       │
//! │          ▼                                                                       │
//! │   ProcessMspRespondStoringRequest                                                │
//! │   (batched extrinsic submission)                                                 │
//! └──────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Status Tracking with [`FileKeyStatus`]
//!
//! Since events are processed concurrently, the same file key could potentially be processed
//! multiple times (e.g., if [`BatchProcessStorageRequests`] fires while a file is mid-upload).
//! The [`file_key_statuses`](MspUploadFileTask::file_key_statuses) HashMap prevents this:
//!
//! | Status | Meaning | Action in [`BatchProcessStorageRequests`] |
//! |--------|---------|-------------------------------------------|
//! | [`FileKeyStatus::Processing`] | File key is in the pipeline | **Skip** |
//! | [`FileKeyStatus::Accepted`] | Successfully accepted on-chain | **Skip** |
//! | [`FileKeyStatus::Rejected`] | Rejected on-chain | **Skip** |
//! | [`FileKeyStatus::Failed`] | Processing failed (proof error) | **Retry** |
//! | [`FileKeyStatus::Abandoned`] | Exhausted retries (non-proof error) | **Skip** |
//! | *Not present* | New file key | **Process** |
//!
//! **Note:** When an extrinsic submission fails, file keys are marked as `Failed` only for
//! proof-related errors from [`pallet_proofs_dealer`]: `ForestProofVerificationFailed`,
//! `KeyProofVerificationFailed`, or `FailedToApplyDelta`. Non-proof errors mark file keys as
//! `Abandoned` after exhausting retries. Stale entries (not in pending requests) are cleaned up.
//!
//! ### Event Handlers
//!
//! - [`BatchProcessStorageRequests`]: Periodic trigger from [`BlockchainService`](shc_blockchain_service)
//!   that queries pending storage requests and emits [`NewStorageRequest`] for each via
//!   [`PreprocessStorageRequestEvent`](shc_blockchain_service::commands::BlockchainServiceCommand::PreprocessStorageRequestEvent).
//!
//! - [`NewStorageRequest`]: Validates capacity, creates file in storage, registers for P2P upload.
//!   If the file is already complete in file storage, immediately queues an accept response via
//!   [`queue_msp_respond_storage_request`](shc_blockchain_service::commands::BlockchainServiceCommandInterface::queue_msp_respond_storage_request).
//!
//! - [`RemoteUploadRequest`]: Receives and validates file chunks from users. When the file is
//!   fully received, queues an accept response via
//!   [`queue_msp_respond_storage_request`](shc_blockchain_service::commands::BlockchainServiceCommandInterface::queue_msp_respond_storage_request).
//!
//! - [`ProcessMspRespondStoringRequest`]: Processes queued accept/reject responses and submits
//!   them in a single batched `msp_respond_storage_requests_multiple_buckets` extrinsic.

use anyhow::anyhow;
use codec::Decode;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use tokio::sync::RwLock;

use sc_network::PeerId;
use sc_tracing::tracing::*;
use shc_blockchain_service::types::{
    MspRespondStorageRequest, RespondStorageRequest, RetryStrategy,
};
use shc_blockchain_service::{capacity_manager::CapacityRequestData, types::SendExtrinsicOptions};
use sp_core::H256;
use sp_runtime::traits::{CheckedAdd, CheckedSub, SaturatedConversion, Zero};

use pallet_file_system::types::RejectedStorageRequest;
use pallet_proofs_dealer;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::events::{
    BatchProcessStorageRequests, ProcessMspRespondStoringRequest,
};
use shc_blockchain_service::{
    commands::{BlockchainServiceCommandInterface, BlockchainServiceCommandInterfaceExt},
    events::NewStorageRequest,
};
use shc_common::{
    traits::StorageEnableRuntime,
    types::{
        FileKey, FileKeyWithProof, FileMetadata, HashT, RejectedStorageRequestReason,
        StorageProofsMerkleTrieLayout, StorageProviderId, StorageRequestMspAcceptedFileKeys,
        StorageRequestMspBucketResponse, BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE,
    },
};
use shc_file_manager::traits::{FileStorage, FileStorageWriteError, FileStorageWriteOutcome};
use shc_file_transfer_service::{
    commands::FileTransferServiceCommandInterface, events::RemoteUploadRequest,
};
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};

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

/// Status of a file key in the MSP upload pipeline.
///
/// Used by [`MspUploadFileTask`] to track processing state, prevent duplicate processing,
/// and enable automatic retries. See [module documentation](self) for the full status
/// transition flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileKeyStatus {
    /// File key is currently being processed (in the pipeline).
    ///
    /// Set when [`BatchProcessStorageRequests`] emits a [`NewStorageRequest`] for the file key.
    Processing,
    /// File key was successfully accepted on-chain.
    ///
    /// Set after [`ProcessMspRespondStoringRequest`] successfully submits the extrinsic.
    Accepted,
    /// File key was explicitly rejected on-chain (e.g., capacity issues, invalid proof).
    ///
    /// Set after [`ProcessMspRespondStoringRequest`] successfully submits a rejection.
    Rejected,
    /// File key processing failed at some stage and can be retried.
    ///
    /// Set when a handler encounters an error during processing. For extrinsic submission
    /// errors in [`ProcessMspRespondStoringRequest`], this status is only set for
    /// proof-related errors from [`pallet_proofs_dealer`]: `ForestProofVerificationFailed`,
    /// `KeyProofVerificationFailed`, or `FailedToApplyDelta`. Error detection uses type-safe
    /// decoding of the dispatch error, ensuring compile-time safety.
    ///
    /// The next [`BatchProcessStorageRequests`] cycle will automatically retry processing
    /// file keys with this status.
    Failed,
    /// File key processing exhausted all retries with non-proof errors.
    ///
    /// Set when extrinsic submission fails after exhausting all retries (timeout, dropped)
    /// or when the extrinsic is included but fails with a non-proof-related dispatch error.
    /// Unlike [`Failed`](Self::Failed), file keys with this status will NOT be retried in
    /// subsequent [`BatchProcessStorageRequests`] cycles.
    ///
    /// The file key entry will be cleaned up from the status map when it is no longer
    /// present in the pending storage requests list.
    Abandoned,
}

/// Handles the complete file upload flow for Main Storage Providers (MSPs).
///
/// This task processes multiple concurrent events using an actor-based model.
/// See [module documentation](self) for the full architecture and event flow diagram.
///
/// # Event Handlers
///
/// | Event | Purpose |
/// |-------|---------|
/// | [`BatchProcessStorageRequests`] | Periodic discovery of pending requests |
/// | [`NewStorageRequest`] | Capacity check, upload registration, or queue if file exists |
/// | [`RemoteUploadRequest`] | Chunk reception; queues accept when file complete |
/// | [`ProcessMspRespondStoringRequest`] | Batched on-chain response submission |
///
/// # Status Tracking
///
/// The [`file_key_statuses`](Self::file_key_statuses) field tracks each file key's
/// [`FileKeyStatus`] to prevent duplicate processing and enable automatic retries
/// for failed requests.
pub struct MspUploadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
    config: MspUploadFileConfig,
    /// Tracks the processing status of each file key to prevent duplicate processing
    /// and enable retries for failed requests.
    ///
    /// Uses `Arc<RwLock<...>>` to share state across all concurrent event handlers.
    /// Without this, each spawned handler would get its own clone of the HashMap,
    /// making status updates invisible to other handlers.
    file_key_statuses: Arc<RwLock<HashMap<H256, FileKeyStatus>>>,
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
            file_key_statuses: self.file_key_statuses.clone(),
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
            file_key_statuses: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_config(mut self, config: MspUploadFileConfig) -> Self {
        self.config = config;
        self
    }

    /// Constructs file metadata and derives the file key from a storage request.
    ///
    /// This is a pure computation helper that converts storage request data into
    /// the internal [`FileMetadata`] and [`FileKey`] representations.
    fn construct_file_metadata_and_key(
        event: &NewStorageRequest<Runtime>,
    ) -> anyhow::Result<(FileMetadata, FileKey)> {
        let who = event.who.as_ref().to_vec();
        let metadata = FileMetadata::new(
            who,
            event.bucket_id.as_ref().to_vec(),
            event.location.to_vec(),
            event.size.saturated_into(),
            event.fingerprint,
        )
        .map_err(|_| anyhow::anyhow!("Invalid file metadata"))?;

        let file_key: FileKey = metadata
            .file_key::<HashT<StorageProofsMerkleTrieLayout>>()
            .as_ref()
            .try_into()?;

        Ok((metadata, file_key))
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
    async fn handle_event(&mut self, event: RemoteUploadRequest<Runtime>) -> anyhow::Result<()> {
        trace!(target: LOG_TARGET, "Received remote upload request for file {:?} and peer {:?}", event.file_key, event.peer);

        let file_key: H256 = event.file_key.into();

        let file_complete = match self.handle_remote_upload_request_event(event.clone()).await {
            Ok(complete) => complete,
            Err(e) => {
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
            if let Err(e) = self.on_file_complete(&file_key).await {
                // Mark the file key as failed so it can be retried
                self.file_key_statuses
                    .write()
                    .await
                    .insert(file_key, FileKeyStatus::Failed);
                return Err(e);
            }
        }

        Ok(())
    }
}

/// Handles the [`BatchProcessStorageRequests`] event.
///
/// This event is triggered periodically by the BlockchainService to process pending storage requests
/// that may have been missed. The handler queries the runtime for pending storage requests and
/// emits a NewStorageRequest event for each via the PreprocessStorageRequestEvent command.
impl<NT, Runtime> EventHandler<BatchProcessStorageRequests> for MspUploadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: BatchProcessStorageRequests) -> anyhow::Result<()> {
        // Hold the Arc reference to the permit for the lifetime of this handler
        // The permit will be automatically released when this handler completes or fails
        // (when the Arc is dropped, the permit is dropped, releasing the semaphore)
        let _permit_guard = event.permit;

        info!(
            target: LOG_TARGET,
            "Processing batch storage requests"
        );

        let pending_requests = self
            .storage_hub_handler
            .blockchain
            .query_pending_storage_requests(None)
            .await
            .map_err(|e| anyhow!("Failed to query pending storage requests: {:?}", e))?;

        info!(
            target: LOG_TARGET,
            "Found {} pending storage requests to process",
            pending_requests.len()
        );

        // Collect pending file keys for cleanup of stale entries
        let pending_file_keys: HashSet<H256> = pending_requests
            .iter()
            .map(|r| H256::from_slice(r.file_key.as_ref()))
            .collect();

        // Take a snapshot of current statuses to avoid repeated lock acquisitions in the loop.
        // This is safe because BatchProcessStorageRequests is serialized by the semaphore.
        let current_statuses = self.file_key_statuses.read().await.clone();

        // Collect status updates to apply in a single write lock acquisition at the end
        let mut keys_to_mark_processing = Vec::new();
        let mut keys_to_mark_failed = Vec::new();

        // For each pending request, check status and call PreprocessStorageRequestEvent command
        // which will emit a NewStorageRequest event for per-file processing.
        for request in &pending_requests {
            let file_key = H256::from_slice(request.file_key.as_ref());

            // Check if this file key should be processed based on its current status
            match current_statuses.get(&file_key) {
                Some(FileKeyStatus::Processing) => {
                    trace!(
                        target: LOG_TARGET,
                        "Skipping file key {:?} - already processing",
                        file_key
                    );
                    continue;
                }
                Some(FileKeyStatus::Accepted) => {
                    trace!(
                        target: LOG_TARGET,
                        "Skipping file key {:?} - already accepted",
                        file_key
                    );
                    continue;
                }
                Some(FileKeyStatus::Rejected) => {
                    trace!(
                        target: LOG_TARGET,
                        "Skipping file key {:?} - already rejected",
                        file_key
                    );
                    continue;
                }
                Some(FileKeyStatus::Abandoned) => {
                    trace!(
                        target: LOG_TARGET,
                        "Skipping file key {:?} - abandoned after exhausting retries",
                        file_key
                    );
                    continue;
                }
                Some(FileKeyStatus::Failed) => {
                    info!(
                        target: LOG_TARGET,
                        "Retrying failed file key {:?}",
                        file_key
                    );
                }
                None => {
                    trace!(
                        target: LOG_TARGET,
                        "Processing new file key {:?}",
                        file_key
                    );
                }
            }

            if let Err(e) = self
                .storage_hub_handler
                .blockchain
                .preprocess_storage_request(request.clone())
                .await
            {
                error!(
                    target: LOG_TARGET,
                    "Failed to preprocess storage request for file key {:?}: {:?}",
                    file_key,
                    e
                );
                // Mark as failed so it can be retried on the next batch cycle
                keys_to_mark_failed.push(file_key);
            } else {
                // Mark as processing only on success
                keys_to_mark_processing.push(file_key);
            }
        }

        // Apply all status updates and cleanup in a single write lock acquisition
        let removed_count = {
            let mut statuses = self.file_key_statuses.write().await;

            // Apply failed status updates
            for file_key in keys_to_mark_failed {
                statuses.insert(file_key, FileKeyStatus::Failed);
            }

            // Apply processing status updates
            for file_key in keys_to_mark_processing {
                statuses.insert(file_key, FileKeyStatus::Processing);
            }

            // Cleanup: remove stale entries that are no longer in pending requests.
            // This prevents the hashmap from growing indefinitely with old entries.
            let before_count = statuses.len();
            statuses.retain(|file_key, _| pending_file_keys.contains(file_key));
            before_count - statuses.len()
        };
        if removed_count > 0 {
            info!(
                target: LOG_TARGET,
                "Cleaned up {} stale file key entries not in pending requests",
                removed_count
            );
        }

        // Permit is automatically released when handler returns
        Ok(())
    }
}

/// Handles the [`NewStorageRequest`] event.
///
/// This event is emitted for each pending storage request. The MSP will check if it has enough
/// storage capacity to store the file and increase it if necessary (up to a maximum).
/// If the MSP does not have enough capacity still, it will reject the storage request.
/// It will register the user and file key in the registry of the File Transfer Service,
/// which handles incoming p2p upload requests. Finally, it will create a file in the
/// file storage so that it can write uploaded chunks as soon as possible.
impl<NT, Runtime> EventHandler<NewStorageRequest<Runtime>> for MspUploadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: NewStorageRequest<Runtime>) -> anyhow::Result<()> {
        let bucket_id = H256::from_slice(event.bucket_id.as_ref());
        let file_key = H256::from_slice(event.file_key.as_ref());
        let result = self.handle_new_storage_request_event(event).await;
        if let Err(reason) = result {
            self.handle_rejected_storage_request(&file_key, bucket_id, reason.clone())
                .await?;

            return Err(anyhow::anyhow!(
                "Failed to handle new storage request: {:?}",
                reason
            ));
        }
        Ok(())
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
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing ProcessMspRespondStoringRequest: {:?}",
            event.data.respond_storing_requests,
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

        // Collect file keys we want to accept to check if they're still pending
        let file_keys_to_check: Vec<FileKey> = event
            .data
            .respond_storing_requests
            .iter()
            .filter_map(|r| {
                if let MspRespondStorageRequest::Accept = &r.response {
                    Some(r.file_key.into())
                } else {
                    None
                }
            })
            .collect();

        // Query pending storage requests for these specific file keys.
        // The runtime API already filters to only return requests that are:
        // 1. Assigned to this MSP
        // 2. Not yet accepted (msp.1 == false)
        let pending_file_keys: HashSet<H256> = if !file_keys_to_check.is_empty() {
            self.storage_hub_handler
                .blockchain
                .query_pending_storage_requests(Some(file_keys_to_check))
                .await
                .unwrap_or_else(|e| {
                    warn!(
                        target: LOG_TARGET,
                        "Failed to query storage requests: {:?}. Proceeding with all requests.",
                        e
                    );
                    Vec::new()
                })
                .into_iter()
                .map(|r| H256::from_slice(r.file_key.as_ref()))
                .collect()
        } else {
            HashSet::new()
        };

        let mut file_key_responses = HashMap::new();

        let read_file_storage = self.storage_hub_handler.file_storage.read().await;
        // Filter out Accept requests that are already accepted (not in pending_file_keys)
        for respond in event
            .data
            .respond_storing_requests
            .iter()
            .filter(|r| match &r.response {
                MspRespondStorageRequest::Accept => pending_file_keys.contains(&r.file_key),
                MspRespondStorageRequest::Reject(_) => true,
            })
        {
            info!(target: LOG_TARGET, "Processing respond storing request.");
            let bucket_id = match read_file_storage.get_metadata(&respond.file_key) {
                Ok(Some(metadata)) => H256::from_slice(metadata.bucket_id().as_ref()),
                Ok(None) => {
                    error!(target: LOG_TARGET, "File does not exist for key {:?}. Maybe we forgot to unregister before deleting?", respond.file_key);
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

                    let proof = match read_file_storage
                        .generate_proof(&respond.file_key, &HashSet::from_iter(chunks_to_prove))
                    {
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

        drop(read_file_storage);

        let mut storage_request_msp_response = Vec::new();

        for (bucket_id, (accept, reject)) in file_key_responses.iter_mut() {
            let fs = self
                .storage_hub_handler
                .forest_storage_handler
                .get_or_create(&ForestStorageKey::from(bucket_id.as_ref().to_vec()))
                .await;

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
        // This enables type-safe error checking against pallet_proofs_dealer::Error variants.
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
                    .with_max_tip(self.config.max_tip.saturated_into()),
                true, // Request events to enable type-safe error checking
            )
            .await;

        // Handle extrinsic submission result
        match extrinsic_result {
            Err(e) => {
                // Extrinsic submission failed after exhausting all retries (timeout, dropped, etc.)
                // Mark file keys as Abandoned since retries have been exhausted with non-proof errors.
                warn!(
                    target: LOG_TARGET,
                    "Extrinsic submission failed after exhausting retries, marking file keys as Abandoned: {:?}",
                    e
                );

                {
                    let mut statuses = self.file_key_statuses.write().await;
                    for storage_request_msp_bucket_response in &storage_request_msp_response {
                        if let Some(ref accepted) = storage_request_msp_bucket_response.accept {
                            for fk in &accepted.file_keys_and_proofs {
                                statuses.insert(fk.file_key, FileKeyStatus::Abandoned);
                            }
                        }
                        for rejected in &storage_request_msp_bucket_response.reject {
                            statuses.insert(rejected.file_key, FileKeyStatus::Abandoned);
                        }
                    }
                }

                // Release the forest root write lock before returning error
                let _ = self
                    .storage_hub_handler
                    .blockchain
                    .release_forest_root_write_lock(forest_root_write_tx)
                    .await;

                return Err(e);
            }
            Ok(Some(events)) => {
                // Extrinsic was included in a block, check if it succeeded or failed by
                // looking for an ExtrinsicFailed event
                let dispatch_error = events.iter().find_map(|event_record| {
                    if let shc_common::types::StorageEnableEvents::System(
                        frame_system::Event::ExtrinsicFailed { dispatch_error, .. },
                    ) = event_record.event.clone().into()
                    {
                        Some(dispatch_error)
                    } else {
                        None
                    }
                });

                if let Some(dispatch_error) = dispatch_error {
                    // Proof errors are transient and can be retried with regenerated proofs (mark as Failed).
                    // Non-proof errors are permanent failures (mark as Abandoned).
                    let is_proof_error = if let sp_runtime::DispatchError::Module(module_error) =
                        &dispatch_error
                    {
                        // Decode the error bytes into the proofs-dealer error type.
                        // This is type-safe: if error variants are renamed, compilation fails.
                        pallet_proofs_dealer::Error::<Runtime>::decode(&mut &module_error.error[..])
                            .ok()
                            .map(|pallet_error| {
                                matches!(
                                    pallet_error,
                                    pallet_proofs_dealer::Error::ForestProofVerificationFailed
                                        | pallet_proofs_dealer::Error::KeyProofVerificationFailed
                                        | pallet_proofs_dealer::Error::FailedToApplyDelta
                                )
                            })
                            .unwrap_or(false)
                    } else {
                        false
                    };

                    if is_proof_error {
                        // Proof-related error: mark file keys as Failed so they can be retried
                        // with regenerated proofs
                        warn!(
                            target: LOG_TARGET,
                            "Proof-related error detected, marking file keys as Failed for retry: {:?}",
                            dispatch_error
                        );
                        let mut statuses = self.file_key_statuses.write().await;
                        for storage_request_msp_bucket_response in &storage_request_msp_response {
                            if let Some(ref accepted) = storage_request_msp_bucket_response.accept {
                                for fk in &accepted.file_keys_and_proofs {
                                    statuses.insert(fk.file_key, FileKeyStatus::Failed);
                                }
                            }
                            for rejected in &storage_request_msp_bucket_response.reject {
                                statuses.insert(rejected.file_key, FileKeyStatus::Failed);
                            }
                        }
                    } else {
                        // Non-proof error: mark file keys as Abandoned since this is a permanent failure
                        // that is not guaranteed to be resolved by retrying (e.g., invalid parameters, authorization errors, inconsistent runtime state, etc.).
                        warn!(
                            target: LOG_TARGET,
                            "Non-proof dispatch error, marking file keys as Abandoned: {:?}",
                            dispatch_error
                        );
                        let mut statuses = self.file_key_statuses.write().await;
                        for storage_request_msp_bucket_response in &storage_request_msp_response {
                            if let Some(ref accepted) = storage_request_msp_bucket_response.accept {
                                for fk in &accepted.file_keys_and_proofs {
                                    statuses.insert(fk.file_key, FileKeyStatus::Abandoned);
                                }
                            }
                            for rejected in &storage_request_msp_bucket_response.reject {
                                statuses.insert(rejected.file_key, FileKeyStatus::Abandoned);
                            }
                        }
                    }

                    // Release the forest root write lock before returning error
                    let _ = self
                        .storage_hub_handler
                        .blockchain
                        .release_forest_root_write_lock(forest_root_write_tx)
                        .await;

                    return Err(anyhow!("Extrinsic failed: {:?}", dispatch_error));
                }
                // No ExtrinsicFailed event means success - continue to process results
            }
            Ok(None) => {
                // This shouldn't happen since we requested events with `with_events: true`
                warn!(
                    target: LOG_TARGET,
                    "Expected events but got None - assuming extrinsic succeeded"
                );
            }
        }

        // Collect all accepted and rejected file keys for batch status update
        let mut all_accepted_keys = Vec::new();
        let mut all_rejected_keys = Vec::new();

        // Log accepted and rejected files, collect keys for status updates
        // Accepted files will be added to the Bucket's Forest Storage by the BlockchainService.
        for storage_request_msp_bucket_response in &storage_request_msp_response {
            // Log accepted file keys
            if let Some(ref accepted) = storage_request_msp_bucket_response.accept {
                let accepted_file_keys: Vec<_> = accepted
                    .file_keys_and_proofs
                    .iter()
                    .map(|fk| fk.file_key)
                    .collect();

                if !accepted_file_keys.is_empty() {
                    info!(
                        target: LOG_TARGET,
                        "✅ Accepted {} file(s) for bucket {:?}: {:?}",
                        accepted_file_keys.len(),
                        storage_request_msp_bucket_response.bucket_id,
                        accepted_file_keys
                    );
                    all_accepted_keys.extend(accepted_file_keys);
                }
            }

            // Log rejected file keys
            if !storage_request_msp_bucket_response.reject.is_empty() {
                let rejected_file_keys: Vec<_> = storage_request_msp_bucket_response
                    .reject
                    .iter()
                    .map(|r| (r.file_key, &r.reason))
                    .collect();

                info!(
                    target: LOG_TARGET,
                    "❌ Rejected {} file(s) for bucket {:?}: {:?}",
                    rejected_file_keys.len(),
                    storage_request_msp_bucket_response.bucket_id,
                    rejected_file_keys
                );

                all_rejected_keys.extend(
                    storage_request_msp_bucket_response
                        .reject
                        .iter()
                        .map(|r| r.file_key),
                );
            }
        }

        // Apply all status updates in a single write lock acquisition
        if !all_accepted_keys.is_empty() || !all_rejected_keys.is_empty() {
            let mut statuses = self.file_key_statuses.write().await;
            for file_key in &all_accepted_keys {
                statuses.insert(*file_key, FileKeyStatus::Accepted);
            }
            for file_key in &all_rejected_keys {
                statuses.insert(*file_key, FileKeyStatus::Rejected);
            }
        }

        // Delete rejected files from file storage
        if !all_rejected_keys.is_empty() {
            let mut fs = self.storage_hub_handler.file_storage.write().await;
            for file_key in &all_rejected_keys {
                if let Err(e) = fs.delete_file(file_key) {
                    error!(target: LOG_TARGET, "Failed to delete file {:?}: {:?}", file_key, e);
                }
            }
        }

        // Release the forest root write "lock" and finish the task.
        self.storage_hub_handler
            .blockchain
            .release_forest_root_write_lock(forest_root_write_tx)
            .await
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
    ) -> Result<(), RejectedStorageRequestReason> {
        if event.size.is_zero() {
            let err_msg = "File size cannot be 0";
            error!(target: LOG_TARGET, err_msg);
            return Err(RejectedStorageRequestReason::InternalError);
        }

        let own_provider_id = self
            .storage_hub_handler
            .blockchain
            .query_storage_provider_id(None)
            .await
            .map_err(|e| {
                error!(target: LOG_TARGET, "Failed to query storage provider ID: {:?}", e);
                RejectedStorageRequestReason::InternalError
            })?;

        let own_msp_id = match own_provider_id {
            Some(id) => match id {
                StorageProviderId::MainStorageProvider(id) => id,
                StorageProviderId::BackupStorageProvider(_) => {
                    let err_msg = "Current node account is a Backup Storage Provider. Expected a Main Storage Provider ID.";
                    error!(target: LOG_TARGET, err_msg);
                    return Err(RejectedStorageRequestReason::InternalError);
                }
            },
            None => {
                let err_msg = "Failed to get own MSP ID.";
                error!(target: LOG_TARGET, err_msg);
                return Err(RejectedStorageRequestReason::InternalError);
            }
        };

        let msp_id_of_bucket_id = self
            .storage_hub_handler
            .blockchain
            .query_msp_id_of_bucket_id(event.bucket_id)
            .await
            .map_err(|e| {
                error!(
                    target: LOG_TARGET,
                    "Failed to query MSP ID of bucket ID {:?}: {:?}",
                    event.bucket_id, e
                );
                RejectedStorageRequestReason::InternalError
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

        // Construct file metadata and derive file key.
        let (metadata, file_key) = Self::construct_file_metadata_and_key(&event).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to construct file metadata and key: {:?}", e);
            RejectedStorageRequestReason::InternalError
        })?;

        let fs = self
            .storage_hub_handler
            .forest_storage_handler
            .get_or_create(&ForestStorageKey::from(event.bucket_id.as_ref().to_vec()))
            .await;
        let read_fs = fs.read().await;

        // If we do not have the file already in forest storage, we must take into account the
        // available storage capacity.
        let file_in_forest_storage = read_fs.contains_file_key(&file_key.into()).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to check if file key is in forest storage: {:?}", e);
            RejectedStorageRequestReason::InternalError
        })?;
        if !file_in_forest_storage {
            info!(target: LOG_TARGET, "File key {:?} not found in forest storage. Checking available storage capacity.", file_key);

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
                    RejectedStorageRequestReason::InternalError
                })?;

            let available_capacity = self
                .storage_hub_handler
                .blockchain
                .query_available_storage_capacity(own_msp_id)
                .await
                .map_err(|e| {
                    let err_msg = format!("Failed to query available storage capacity: {:?}", e);
                    error!(target: LOG_TARGET, "{}", err_msg);
                    RejectedStorageRequestReason::InternalError
                })?;

            // Calculate currently used storage
            let used_capacity = current_capacity
                .checked_sub(&available_capacity)
                .ok_or_else(|| RejectedStorageRequestReason::ReachedMaximumCapacity)?;

            // Check if accepting this file would exceed our local max storage capacity limit
            let projected_usage = used_capacity
                .checked_add(&event.size)
                .ok_or_else(|| RejectedStorageRequestReason::ReachedMaximumCapacity)?;

            if projected_usage > max_storage_capacity {
                let err_msg = format!(
                    "Accepting file would exceed maximum storage capacity limit. Used: {}, Required: {}, Max: {}",
                    used_capacity, event.size, max_storage_capacity
                );
                warn!(target: LOG_TARGET, "{}", err_msg);
                return Err(RejectedStorageRequestReason::ReachedMaximumCapacity);
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
                    .await
                    .map_err(|e| {
                        let err_msg = format!("Failed to increase storage capacity: {:?}", e);
                        error!(target: LOG_TARGET, "{}", err_msg);
                        RejectedStorageRequestReason::InternalError
                    })?;

                let available_capacity = self
                    .storage_hub_handler
                    .blockchain
                    .query_available_storage_capacity(own_msp_id)
                    .await
                    .map_err(|e| {
                        let err_msg =
                            format!("Failed to query available storage capacity: {:?}", e);
                        error!(target: LOG_TARGET, "{}", err_msg);
                        RejectedStorageRequestReason::InternalError
                    })?;

                // Reject storage request if the new available capacity is still less than the file size.
                if available_capacity < event.size {
                    let err_msg = "Increased storage capacity is still insufficient to volunteer for file. Rejecting storage request.";
                    warn!(target: LOG_TARGET, "{}", err_msg);

                    return Err(RejectedStorageRequestReason::ReachedMaximumCapacity);
                }
            }
        } else {
            debug!(target: LOG_TARGET, "File key {:?} found in forest storage.", file_key);
        }

        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;

        // Create file in file storage if it is not present so we can write uploaded chunks as soon as possible.
        let file_in_file_storage = write_file_storage
            .get_metadata(&file_key.into())
            .map_err(|e| {
                let err_msg = format!("Failed to get metadata from file storage: {:?}", e);
                error!(target: LOG_TARGET, "{}", err_msg);
                RejectedStorageRequestReason::InternalError
            })?
            .is_some();
        if !file_in_file_storage {
            debug!(target: LOG_TARGET, "File key {:?} not found in file storage. Inserting file.", file_key);
            write_file_storage
                .insert_file(
                    metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>(),
                    metadata,
                )
                .map_err(|e| {
                    let err_msg = format!("Failed to insert file in file storage: {:?}", e);
                    error!(target: LOG_TARGET, "{}", err_msg);
                    RejectedStorageRequestReason::InternalError
                })?;
        } else {
            debug!(target: LOG_TARGET, "File key {:?} found in file storage.", file_key);
        }

        // If the file is in file storage, we can skip the file transfer,
        // and proceed to accepting the storage request directly, provided that we have the entire file in file storage.
        if file_in_file_storage {
            info!(target: LOG_TARGET, "File key {:?} found in both file storage. No need to receive the file from the user.", file_key);

            if file_in_forest_storage {
                warn!(target: LOG_TARGET, "File key {:?} found in forest storage when storage request is still open. This is an odd state as the file key should not be in the forest storage until the storage request is accepted.", file_key);
            }

            // Check if the file is complete in file storage.
            let file_complete = match write_file_storage.is_file_complete(&file_key.into()) {
                Ok(is_complete) => is_complete,
                Err(e) => {
                    warn!(target: LOG_TARGET, "Failed to check if file is complete. The file key {:?} is in a bad state with error: {:?}", file_key, e);
                    warn!(target: LOG_TARGET, "Assuming the file is not complete.");
                    false
                }
            };

            if file_complete {
                info!(target: LOG_TARGET, "File key {:?} is complete in file storage. Proceeding to accept storage request.", file_key);
                self.on_file_complete(&file_key.into()).await.map_err(|e| {
                    error!(target: LOG_TARGET, "Failed to on file complete: {:?}", e);
                    RejectedStorageRequestReason::InternalError
                })?;

                // This finishes the task, as we already have the entire file in file storage and we queued
                // the accept transaction to the blockchain, so we can finish the task early.
                return Ok(());
            } else {
                debug!(target: LOG_TARGET, "File key {:?} is not complete in file storage. Need to receive the file from the user.", file_key);
            }
        };

        drop(write_file_storage);

        // Register the file for upload in the file transfer service.
        // Even though we could already have the entire file in file storage, we
        // allow the user to connect to us and upload the file. Once they do, we will
        // send back the `file_complete` flag to true signalling to the user that we have
        // the entire file so that the file uploading process is complete.
        for peer_id in event.user_peer_ids.iter() {
            let peer_id = match std::str::from_utf8(&peer_id.as_slice()) {
                Ok(str_slice) => PeerId::from_str(str_slice).map_err(|e| {
                    error!(target: LOG_TARGET, "Failed to convert peer ID to PeerId: {}", e);
                    RejectedStorageRequestReason::InternalError
                })?,
                Err(e) => {
                    error!(target: LOG_TARGET, "Failed to convert peer ID to a string: {}", e);
                    return Err(RejectedStorageRequestReason::InternalError);
                }
            };
            self.storage_hub_handler
                .file_transfer
                .register_new_file(peer_id, file_key)
                .await
                .map_err(|e| {
                    let err_msg = format!("Failed to register new file peer: {:?}", e);
                    error!(target: LOG_TARGET, "{}", err_msg);
                    RejectedStorageRequestReason::InternalError
                })?;
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
                        "File does not exist for key {:?}. Maybe we forgot to unregister before deleting?",
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
                "Fingerprint mismatch for file {:?}. Expected: {:?}, got: {:?}",
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

        // Handle invalid proof case
        let proven = match proven {
            Ok(proven) => proven,
            Err(error) => {
                error!(
                    target: LOG_TARGET,
                    "Failed to verify proof for file {:?}: {:?}",
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

        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;
        let mut file_complete = false;

        // Process each proven chunk in the batch
        for chunk in proven {
            let chunk_idx = chunk.key.as_u64();
            let expected_chunk_size = file_metadata.chunk_size_at(chunk_idx).map_err(|e| {
                anyhow!("Failed to get chunk size for chunk {}: {:?}", chunk_idx, e)
            })?;

            if chunk.data.len() != expected_chunk_size {
                error!(
                    target: LOG_TARGET,
                    "Invalid chunk size for chunk {}: Expected: {}, got: {}",
                    chunk_idx,
                    expected_chunk_size,
                    chunk.data.len()
                );
                self.handle_rejected_storage_request(
                    &file_key,
                    bucket_id,
                    RejectedStorageRequestReason::ReceivedInvalidProof,
                )
                .await?;
                return Err(anyhow!(
                    "Invalid chunk size for chunk {}: Expected: {}, got: {}",
                    chunk_idx,
                    expected_chunk_size,
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
                        self.handle_rejected_storage_request(
                            &file_key,
                            bucket_id,
                            RejectedStorageRequestReason::InternalError,
                        )
                        .await?;
                        return Err(anyhow::anyhow!(format!(
                            "File does not exist for key {:?}. Maybe we forgot to unregister before deleting?",
                            file_key
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
                        self.handle_rejected_storage_request(
                            &file_key,
                            bucket_id,
                            RejectedStorageRequestReason::InternalError,
                        )
                        .await?;
                        return Err(anyhow::anyhow!(format!(
                            "Internal trie read/write error {:?}:{:?}",
                            file_key, chunk.key
                        )));
                    }
                    FileStorageWriteError::FingerprintAndStoredFileMismatch => {
                        self.handle_rejected_storage_request(
                            &file_key,
                            bucket_id,
                            RejectedStorageRequestReason::InternalError,
                        )
                        .await?;
                        return Err(anyhow::anyhow!(format!(
                            "Invariant broken! This is a bug! Fingerprint and stored file mismatch for key {:?}.",
                            file_key
                        )));
                    }
                    FileStorageWriteError::FailedToConstructTrieIter
                    | FileStorageWriteError::FailedToConstructFileTrie => {
                        self.handle_rejected_storage_request(
                            &file_key,
                            bucket_id,
                            RejectedStorageRequestReason::InternalError,
                        )
                        .await?;
                        return Err(anyhow::anyhow!(format!(
                            "This is a bug! Failed to construct trie iter for key {:?}.",
                            file_key
                        )));
                    }
                },
            }
        }

        // If we haven't found the file to be complete during chunk processing,
        // check if it's complete now (in case this was the last batch)
        if !file_complete {
            match write_file_storage.is_file_complete(&file_key) {
                Ok(is_complete) => file_complete = is_complete,
                Err(e) => {
                    self.handle_rejected_storage_request(
                        &file_key,
                        bucket_id,
                        RejectedStorageRequestReason::InternalError,
                    )
                    .await?;
                    let err_msg = format!(
                        "Failed to check if file is complete. The file key {:?} is in a bad state with error: {:?}",
                        file_key, e
                    );
                    error!(target: LOG_TARGET, "{}", err_msg);
                    return Err(anyhow::anyhow!(err_msg));
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
        let call: Runtime::Call =
            pallet_file_system::Call::<Runtime>::msp_respond_storage_requests_multiple_buckets {
                storage_request_msp_response: vec![StorageRequestMspBucketResponse {
                    bucket_id,
                    accept: None,
                    reject: vec![RejectedStorageRequest {
                        file_key: *file_key,
                        reason,
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
            .await?
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await?;

        // Unregister the file
        self.unregister_file(*file_key).await?;

        // Mark the file key as rejected so it won't be retried
        self.file_key_statuses
            .write()
            .await
            .insert(*file_key, FileKeyStatus::Rejected);

        Ok(())
    }

    async fn unregister_file(&self, file_key: H256) -> anyhow::Result<()> {
        warn!(target: LOG_TARGET, "Unregistering file {:?}", file_key);

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

    async fn on_file_complete(&self, file_key: &H256) -> anyhow::Result<()> {
        info!(target: LOG_TARGET, "File upload complete (file_key {:x})", file_key);

        // Unregister the file from the file transfer service.
        if let Err(e) = self
            .storage_hub_handler
            .file_transfer
            .unregister_file((*file_key).into())
            .await
        {
            warn!(target: LOG_TARGET, "Failed to unregister file {:x} from file transfer service: {:?}", file_key, e);
        }

        trace!(target: LOG_TARGET, "File unregistered from file transfer service.");

        // Queue a request to confirm the storing of the file.
        self.storage_hub_handler
            .blockchain
            .queue_msp_respond_storage_request(RespondStorageRequest::new(
                *file_key,
                MspRespondStorageRequest::Accept,
            ))
            .await?;

        trace!(target: LOG_TARGET, "File queued for confirmation.");

        Ok(())
    }
}
