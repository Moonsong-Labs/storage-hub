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
use shc_blockchain_service::events::ProcessMspRespondStoringRequest;
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
use shp_file_metadata::{Chunk, ChunkId, Leaf};

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
    file_key_cleanup: Option<H256>,
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
            file_key_cleanup: self.file_key_cleanup,
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
            file_key_cleanup: None,
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
/// This event is triggered by an on-chain event of a user submitting a storage request to StorageHub.
///
/// This task will:
/// - Check if the MSP has enough storage capacity to store the file and increase it if necessary (up to a maximum).
/// - Register the user and file key in the registry of the File Transfer Service, which handles incoming p2p
/// upload requests.
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

        let file_key = event.file_key;
        let result = self.handle_new_storage_request_event(event).await;

        match result {
            Ok(()) => Ok(format!(
                "Handled NewStorageRequest for file_key [{:x}]",
                file_key
            )),
            Err(e) => {
                if let Some(file_key) = &self.file_key_cleanup {
                    self.unregister_file(*file_key).await?;
                }
                Err(e)
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

        for respond in &event.data.respond_storing_requests {
            info!(target: LOG_TARGET, "Processing respond storing request.");

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
                        error!(target: LOG_TARGET, "Failed to delete file {:x}: {:?}", file_key, e);
                    }
                }
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
        .map_err(|_| anyhow::anyhow!("Invalid file metadata"))?;

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
                    anyhow::anyhow!("Failed to query storage provider capacity: {:?}", e)
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
                    "Insufficient storage capacity to volunteer for file key: {:x}",
                    event.file_key
                );

                self.storage_hub_handler
                    .blockchain
                    .increase_capacity(CapacityRequestData::new(event.size))
                    .await?;

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
                        anyhow::anyhow!(err_msg)
                    })?;

                // Reject storage request if the new available capacity is still less than the file size.
                if available_capacity < event.size {
                    let err_msg = "Increased storage capacity is still insufficient to volunteer for file. Rejecting storage request.";
                    warn!(
                        target: LOG_TARGET, "{}", err_msg
                    );

                    // Build extrinsic.
                    let call: Runtime::Call =
                        pallet_file_system::Call::<Runtime>::msp_respond_storage_requests_multiple_buckets {
                            storage_request_msp_response: vec![StorageRequestMspBucketResponse {
                                bucket_id: event.bucket_id,
                                accept: None,
                                reject: vec![RejectedStorageRequest {
                                    file_key: H256(event.file_key.into()),
                                    reason: RejectedStorageRequestReason::ReachedMaximumCapacity,
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

                    return Err(anyhow::anyhow!(err_msg));
                }
            }
        } else {
            debug!(target: LOG_TARGET, "File key [{:x}] found in forest storage.", file_key);
        }

        self.file_key_cleanup = Some(file_key.into());

        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;

        // Create file in file storage if it is not present so we can write uploaded chunks as soon as possible.
        let file_in_file_storage = write_file_storage
            .get_metadata(&file_key.into())
            .map_err(|e| anyhow!("Failed to get metadata from file storage: {:?}", e))?
            .is_some();
        if !file_in_file_storage {
            debug!(target: LOG_TARGET, "File key [{:x}] not found in file storage. Inserting file.", file_key);
            write_file_storage
                .insert_file(
                    metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>(),
                    metadata,
                )
                .map_err(|e| anyhow!("Failed to insert file in file storage: {:?}", e))?;
        } else {
            debug!(target: LOG_TARGET, "File key [{:x}] found in file storage.", file_key);
        }

        // If the file is in both file storage and forest storage, we can skip the file transfer,
        // and proceed to accepting the storage request directly, provided that we have the entire file in file storage.
        if file_in_file_storage && file_in_forest_storage {
            info!(target: LOG_TARGET, "File key [{:x}] found in both file storage and forest storage. No need to receive the file from the user.", file_key);

            // Check if the file is complete in file storage.
            let file_complete = match write_file_storage.is_file_complete(&file_key.into()) {
                Ok(is_complete) => is_complete,
                Err(e) => {
                    warn!(target: LOG_TARGET, "Failed to check if file is complete. The file key [{:x}] is in a bad state with error: {:?}", file_key, e);
                    warn!(target: LOG_TARGET, "Assuming the file is not complete.");
                    false
                }
            };

            if file_complete {
                info!(target: LOG_TARGET, "File key [{:x}] is complete in file storage. Proceeding to accept storage request.", file_key);
                self.on_file_complete(&file_key.into()).await?;

                // This finishes the task, as we already have the entire file in file storage and we queued
                // the accept transaction to the blockchain, so we can finish the task early.
                return Ok(());
            } else {
                debug!(target: LOG_TARGET, "File key [{:x}] is not complete in file storage. Need to receive the file from the user.", file_key);
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
            Ok(file_complete) => Ok(file_complete),
            Err(rejection) => {
                self.handle_rejected_storage_request(
                    &rejection.file_key,
                    rejection.bucket_id,
                    rejection.reason,
                )
                .await?;
                Err(anyhow::anyhow!(rejection.error_message))
            }
        }
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

    async fn on_file_complete(&self, file_key: &H256) -> anyhow::Result<()> {
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
