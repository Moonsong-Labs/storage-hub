use anyhow::anyhow;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    time::Duration,
};

use sc_network::PeerId;
use sc_tracing::tracing::*;
use shc_blockchain_service::types::{
    MspRespondStorageRequest, RespondStorageRequest, RetryStrategy,
};
use shc_blockchain_service::{capacity_manager::CapacityRequestData, types::SendExtrinsicOptions};
use sp_core::H256;
use sp_runtime::traits::{CheckedAdd, CheckedSub, SaturatedConversion, Zero};

use pallet_file_system::types::RejectedStorageRequest;
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

/// Information about a storage request that should be rejected.
#[derive(Debug, Clone)]
struct RejectionInfo {
    file_key: H256,
    bucket_id: H256,
    reason: RejectedStorageRequestReason,
}

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

/// MSP Upload File Task: Handles the whole flow of a file being uploaded to a MSP, from
/// the MSP's perspective.
///
/// The flow is split into three parts, which are represented here as 3 handlers for 3
/// different events:
/// - [`NewStorageRequest`] event: The first part of the flow. It is triggered by a user
///   submitting a storage request to StorageHub. The MSP will check if it has enough
///   storage capacity to store the file and increase it if necessary (up to a maximum).
///   If the MSP does not have enough capacity still, it will reject the storage request.
///   It will register the user and file key in the registry of the File Transfer Service,
///   which handles incoming p2p upload requests. Finally, it will create a file in the
///   file storage so that it can write uploaded chunks as soon as possible.
/// - [`RemoteUploadRequest`] event: The second part of the flow. It is triggered by a
///   user sending a chunk of the file to the MSP. It checks the proof for the chunk
///   and if it is valid, stores it, until the whole file is stored. Finally the MSP will
///   queue a response to accept storing the file.
/// - [`ProcessMspRespondStoringRequest`] event: The third part of the flow. It is triggered
///   when there are new storage request(s) to respond to. The batch of storage requests
///   will be responded to in a single call to the FileSystem pallet `msp_respond_storage_requests_multiple_buckets` extrinsic
///   which will emit an event that describes the final result of the batch response (i.e. all accepted,
///   rejected and/or failed file keys). The MSP will then apply the necessary deltas to each one of the bucket's
///   forest storage to reflect the result.
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

    /// Ensures sufficient capacity for a batch of storage requests.
    ///
    /// This method pre-calculates the total capacity needed for all requests,
    /// increases capacity once if needed, and splits requests into processable
    /// and rejected based on capacity constraints.
    ///
    /// Returns a tuple of (processable_requests, rejections).
    async fn ensure_batch_capacity(
        &mut self,
        pending_requests: Vec<NewStorageRequest<Runtime>>,
    ) -> anyhow::Result<(Vec<NewStorageRequest<Runtime>>, Vec<RejectionInfo>)> {
        if pending_requests.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

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

        let mut available_capacity = self
            .storage_hub_handler
            .blockchain
            .query_available_storage_capacity(own_msp_id)
            .await
            .map_err(|e| {
                error!(target: LOG_TARGET, "Failed to query available storage capacity: {:?}", e);
                anyhow!("Failed to query available storage capacity: {:?}", e)
            })?;

        let used_capacity = current_capacity
            .checked_sub(&available_capacity)
            .ok_or_else(|| {
                anyhow!(
                    "Available capacity ({}) exceeds current capacity ({})",
                    available_capacity,
                    current_capacity
                )
            })?;

        // Note: We assume files with pending storage requests are not in forest storage,
        // as storage requests represent new files to be stored.
        let mut total_capacity_needed = Runtime::StorageDataUnit::zero();
        let mut files_to_process = Vec::new();

        for request in pending_requests {
            if request.size == Zero::zero() {
                warn!(target: LOG_TARGET, "Skipping storage request with zero file size");
                continue;
            }

            // Skip if this MSP has already accepted the storage request
            if let Some((msp_id, already_accepted)) = request.msp {
                if msp_id == own_msp_id && already_accepted {
                    trace!(
                        target: LOG_TARGET,
                        "Skipping already accepted storage request for bucket {:?}",
                        request.bucket_id
                    );
                    continue;
                }
            }

            match self
                .storage_hub_handler
                .blockchain
                .query_msp_id_of_bucket_id(request.bucket_id)
                .await
            {
                Ok(Some(id)) if id == own_msp_id => {
                    total_capacity_needed = total_capacity_needed
                        .checked_add(&request.size)
                        .ok_or_else(|| anyhow!("Overflow calculating total capacity needed"))?;
                    files_to_process.push(request);
                }
                Ok(Some(_)) => {
                    trace!(target: LOG_TARGET, "Skipping storage request - not our bucket");
                }
                Ok(None) => {
                    warn!(target: LOG_TARGET, "Skipping storage request - MSP ID not found for bucket");
                }
                Err(e) => {
                    error!(target: LOG_TARGET, "Failed to query MSP ID of bucket: {:?}", e);
                }
            }
        }

        info!(
            target: LOG_TARGET,
            "Batch requires capacity: {}, available: {}",
            total_capacity_needed,
            available_capacity
        );

        if total_capacity_needed > available_capacity {
            // Respect the maximum storage capacity configured locally
            let max_possible_increase = max_storage_capacity
                .checked_sub(&current_capacity)
                .unwrap_or(Runtime::StorageDataUnit::zero());

            let needed_increase = total_capacity_needed
                .checked_sub(&available_capacity)
                .unwrap_or(Runtime::StorageDataUnit::zero());

            let capacity_to_add = needed_increase.min(max_possible_increase);

            if capacity_to_add > Runtime::StorageDataUnit::zero() {
                info!(
                    target: LOG_TARGET,
                    "Increasing capacity by {}, needed: {}, max: {}",
                    capacity_to_add,
                    needed_increase,
                    max_possible_increase
                );

                self.storage_hub_handler
                    .blockchain
                    .increase_capacity(CapacityRequestData::new(capacity_to_add))
                    .await?;

                available_capacity = self
                    .storage_hub_handler
                    .blockchain
                    .query_available_storage_capacity(own_msp_id)
                    .await
                    .map_err(|e| {
                        error!(target: LOG_TARGET, "Failed to query available capacity after increase: {:?}", e);
                        anyhow!("Failed to query available capacity after increase: {:?}", e)
                    })?;

                info!(
                    target: LOG_TARGET,
                    "Capacity increased to {}, used: {}, max: {}",
                    available_capacity,
                    used_capacity,
                    max_storage_capacity
                );
            } else {
                info!(
                    target: LOG_TARGET,
                    "Already at maximum capacity. Current: {}, max: {}",
                    current_capacity,
                    max_storage_capacity
                );
            }
        }

        if total_capacity_needed > available_capacity {
            info!(
                target: LOG_TARGET,
                "Trimming batch to fit capacity. Needed: {}, available: {}",
                total_capacity_needed,
                available_capacity
            );

            let (accepted, rejected) = self.trim_batch_to_fit_capacity(
                files_to_process,
                available_capacity,
                used_capacity,
                max_storage_capacity,
            )?;

            if accepted.is_empty() {
                warn!(
                    target: LOG_TARGET,
                    "Rejecting all {} files - exceeded capacity",
                    rejected.len()
                );
            } else {
                info!(
                    target: LOG_TARGET,
                    "Processing {} files, rejecting {} due to capacity",
                    accepted.len(),
                    rejected.len()
                );
            }

            return Ok((accepted, rejected));
        }

        info!(
            target: LOG_TARGET,
            "Processing all {} files - capacity sufficient",
            files_to_process.len()
        );

        Ok((files_to_process, Vec::new()))
    }

    /// Trims a batch of storage requests to fit within available capacity.
    ///
    /// This method selects the maximum number of files that can fit within the available
    /// capacity, trimming files from the end (reverse order/LIFO) when necessary.
    ///
    /// Returns a tuple of (accepted_requests, rejection_info).
    fn trim_batch_to_fit_capacity(
        &self,
        requests: Vec<NewStorageRequest<Runtime>>,
        available_capacity: Runtime::StorageDataUnit,
        used_capacity: Runtime::StorageDataUnit,
        max_storage_capacity: Runtime::StorageDataUnit,
    ) -> anyhow::Result<(Vec<NewStorageRequest<Runtime>>, Vec<RejectionInfo>)> {
        let max_usable = max_storage_capacity
            .checked_sub(&used_capacity)
            .unwrap_or(Runtime::StorageDataUnit::zero());

        let capacity_limit = available_capacity.min(max_usable);

        let mut accepted = Vec::new();
        let mut rejected = Vec::new();
        let mut total_size = Runtime::StorageDataUnit::zero();

        for request in requests {
            match total_size.checked_add(&request.size) {
                Some(new_total) if new_total <= capacity_limit => {
                    total_size = new_total;
                    accepted.push(request);
                }
                _ => {
                    let (_, file_key) = Self::construct_file_metadata_and_key(&request)?;
                    rejected.push(RejectionInfo {
                        file_key: H256(file_key.into()),
                        bucket_id: request.bucket_id,
                        reason: RejectedStorageRequestReason::ReachedMaximumCapacity,
                    });
                }
            }
        }

        info!(
            target: LOG_TARGET,
            "Accepted {} files (size: {}), rejected {}, limit: {}",
            accepted.len(),
            total_size,
            rejected.len(),
            capacity_limit
        );

        Ok((accepted, rejected))
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
            self.on_file_complete(&event.file_key.into()).await?;
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

        let mut file_key_responses = HashMap::new();

        let read_file_storage = self.storage_hub_handler.file_storage.read().await;

        for respond in &event.data.respond_storing_requests {
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

        self.storage_hub_handler
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
                false,
            )
            .await?;

        // Log accepted and rejected files, and remove rejected files from File Storage.
        // Accepted files will be added to the Bucket's Forest Storage by the BlockchainService.
        for storage_request_msp_bucket_response in storage_request_msp_response {
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
                }
            }

            // Log and delete rejected file keys
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

                let mut fs = self.storage_hub_handler.file_storage.write().await;
                for RejectedStorageRequest { file_key, .. } in
                    &storage_request_msp_bucket_response.reject
                {
                    if let Err(e) = fs.delete_file(&file_key) {
                        error!(target: LOG_TARGET, "Failed to delete file {:?}: {:?}", file_key, e);
                    }
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

/// Handles the [`BatchProcessStorageRequests`] event.
///
/// This event is triggered periodically by the BlockchainService to process pending storage requests
/// that may have been missed. The handler queries the runtime for pending storage requests and processes
/// each one using the existing `handle_new_storage_request_event` logic.
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
        let _permit_arc = event.permit;

        info!(
            target: LOG_TARGET,
            "Processing batch storage requests"
        );

        let pending_requests = self
            .storage_hub_handler
            .blockchain
            .query_pending_storage_requests()
            .await
            .map_err(|e| anyhow!("Failed to query pending storage requests: {:?}", e))?;

        info!(
            target: LOG_TARGET,
            "Found {} pending storage requests to process",
            pending_requests.len()
        );

        // Phase 1: Ensure capacity for entire batch (single capacity increase)
        let (processable_requests, rejections) =
            self.ensure_batch_capacity(pending_requests).await?;

        if !rejections.is_empty() && !processable_requests.is_empty() {
            info!(
                target: LOG_TARGET,
                "Batch: accepting {} files, rejecting {} files",
                processable_requests.len(),
                rejections.len()
            );
        } else if rejections.is_empty() {
            info!(
                target: LOG_TARGET,
                "Processing batch of {} files",
                processable_requests.len()
            );
        } else {
            warn!(
                target: LOG_TARGET,
                "Rejecting all {} files - insufficient capacity",
                rejections.len()
            );
        }

        // Phase 2: Batch reject requests exceeding capacity (single extrinsic)
        if !rejections.is_empty() {
            if let Err(e) = self.batch_reject_storage_requests(rejections).await {
                error!(
                    target: LOG_TARGET,
                    "Failed to batch reject storage requests: {:?}",
                    e
                );
            }
        }

        // Phase 3: Process valid requests sequentially
        // Note: Sequential processing is used here because parallel processing would require
        // cloning or sharing the handler state, which is complex with the current architecture.
        // The main performance benefit comes from the batch capacity management above.
        let mut success_count = 0;
        let mut error_count = 0;

        for request in processable_requests {
            match self.handle_new_storage_request_event(request).await {
                Ok(()) => success_count += 1,
                Err(e) => {
                    error_count += 1;
                    error!(
                        target: LOG_TARGET,
                        "Failed to process storage request in batch: {:?}",
                        e
                    );
                }
            }
        }

        info!(
            target: LOG_TARGET,
            "Batch processing complete: {} succeeded, {} failed",
            success_count,
            error_count
        );

        // Permit is automatically released when handler returns
        Ok(())
    }
}

impl<NT, Runtime> MspUploadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    /// Submits a batch rejection extrinsic for all rejected storage requests.
    ///
    /// Groups rejections by bucket ID and submits a single
    /// `msp_respond_storage_requests_multiple_buckets` extrinsic with all rejections.
    async fn batch_reject_storage_requests(
        &self,
        rejections: Vec<RejectionInfo>,
    ) -> anyhow::Result<()> {
        if rejections.is_empty() {
            return Ok(());
        }

        info!(
            target: LOG_TARGET,
            "Rejecting {} storage requests",
            rejections.len()
        );

        let mut rejections_by_bucket: HashMap<H256, Vec<RejectedStorageRequest<Runtime>>> =
            HashMap::new();

        for rejection in &rejections {
            rejections_by_bucket
                .entry(rejection.bucket_id)
                .or_default()
                .push(RejectedStorageRequest {
                    file_key: rejection.file_key,
                    reason: rejection.reason.clone(),
                });
        }

        let storage_request_msp_response: Vec<_> = rejections_by_bucket
            .into_iter()
            .map(|(bucket_id, reject)| StorageRequestMspBucketResponse {
                bucket_id,
                accept: None,
                reject,
            })
            .collect();

        let call: Runtime::Call =
            pallet_file_system::Call::<Runtime>::msp_respond_storage_requests_multiple_buckets {
                storage_request_msp_response,
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

        info!(
            target: LOG_TARGET,
            "Rejected {} storage requests successfully",
            rejections.len()
        );

        // Clean up file storage for rejected files
        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;
        for rejection in rejections {
            if let Err(e) = write_file_storage.delete_file(&rejection.file_key) {
                error!(
                    target: LOG_TARGET,
                    "Failed to delete file {:?} after rejection: {:?}",
                    rejection.file_key,
                    e
                );
            }
        }

        Ok(())
    }

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

        // Check if this MSP has already accepted the storage request
        if let Some((msp_id, already_accepted)) = event.msp {
            if msp_id == own_msp_id && already_accepted {
                debug!(
                    target: LOG_TARGET,
                    "Skipping storage request - MSP has already accepted for bucket {:?}",
                    event.bucket_id
                );
                return Ok(());
            }
        }

        // Construct file metadata and derive file key.
        let (metadata, file_key) = Self::construct_file_metadata_and_key(&event)?;

        let fs = self
            .storage_hub_handler
            .forest_storage_handler
            .get_or_create(&ForestStorageKey::from(event.bucket_id.as_ref().to_vec()))
            .await;
        let read_fs = fs.read().await;

        // Check if file is already in forest storage (for informational logging).
        // Capacity has already been ensured by the batch processing handler.
        let file_in_forest_storage = read_fs.contains_file_key(&file_key.into())?;
        if !file_in_forest_storage {
            debug!(target: LOG_TARGET, "File key {:?} not found in forest storage.", file_key);
        } else {
            debug!(target: LOG_TARGET, "File key {:?} already in forest storage.", file_key);
        }

        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;

        // Create file in file storage if it is not present so we can write uploaded chunks as soon as possible.
        let file_in_file_storage = write_file_storage
            .get_metadata(&file_key.into())
            .map_err(|e| anyhow!("Failed to get metadata from file storage: {:?}", e))?
            .is_some();
        if !file_in_file_storage {
            debug!(target: LOG_TARGET, "File key {:?} not found in file storage. Inserting file.", file_key);
            write_file_storage
                .insert_file(
                    metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>(),
                    metadata,
                )
                .map_err(|e| anyhow!("Failed to insert file in file storage: {:?}", e))?;
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
                self.on_file_complete(&file_key.into()).await?;

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
