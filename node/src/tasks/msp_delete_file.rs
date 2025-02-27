use std::time::Duration;

use anyhow::anyhow;
use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceInterface,
    events::{
        FileDeletionRequest, FinalisedProofSubmittedForPendingFileDeletionRequest,
        ProcessFileDeletionRequest,
    },
    types::{self, RetryStrategy},
};
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};

use crate::services::{
    handler::StorageHubHandler,
    types::{MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "msp-delete-file-task";
const MAX_DELETE_FILE_REQUEST_TRY_COUNT: u32 = 5;
const MAX_DELETE_FILE_REQUEST_TIP: u128 = 100;

/// MSP Delete File Task: Handles the whole flow of a file being deleted from an MSP.
///
/// The flow is split into three parts, which are represented here as 3 handlers for 3
/// different events:
/// - [`FileDeletionRequest`] event: The first part of the flow. It is triggered when a file deletion request is received from the blockchain.
///   The deletion request is queued for processing by the [`ProcessFileDeletionRequest`] event handler in batches.
/// - [`ProcessFileDeletionRequest`] event: The second part of the flow. It is triggered when there are
///   pending file deletion requests to process. The MSP will generate an (non-)inclusion forest proof for all file keys in the batch and
///   submit it to the runtime to delete any existing file keys proven to be in the forest.
/// - [`FinalisedProofSubmittedForPendingFileDeletionRequest`] event: The third part of the flow. It is triggered when
///   the file deletion request is finalized on-chain. The MSP will then delete the file from its file storage.
pub struct MspDeleteFileTask<NT>
where
    NT: ShNodeType,
    NT::FSH: MspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<NT>,
}

impl<NT> Clone for MspDeleteFileTask<NT>
where
    NT: ShNodeType,
    NT::FSH: MspForestStorageHandlerT,
{
    fn clone(&self) -> MspDeleteFileTask<NT> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT> MspDeleteFileTask<NT>
where
    NT: ShNodeType,
    NT::FSH: MspForestStorageHandlerT,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

/// Handles the [`FileDeletionRequest`] event.
///
/// This event is triggered when a file deletion request is received from the blockchain.
/// The MSP will queue the request for processing, which will be handled by the [`ProcessFileDeletionRequest`] event handler.
impl<NT> EventHandler<FileDeletionRequest> for MspDeleteFileTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: FileDeletionRequest) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Queueing file deletion request for file_key {:?}",
            event.file_key
        );

        // Queue the file deletion request
        self.storage_hub_handler
            .blockchain
            .queue_file_deletion_request(types::FileDeletionRequest::from(event.clone()))
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to queue file deletion request for file_key {:?}: {:?}",
                    event.file_key,
                    e
                )
            })?;

        Ok(())
    }
}

/// Handles the [`ProcessFileDeletionRequest`] event.
///
/// This event is triggered when there are pending file deletion requests to process.
/// The MSP will generate an (non-)inclusion forest proof and submit it to confirm each file can(not) be deleted.
/// Files are processed one at a time to ensure proper forest root management.
impl<NT> EventHandler<ProcessFileDeletionRequest> for MspDeleteFileTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: ProcessFileDeletionRequest) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing batch of {} file deletion requests",
            event.data.file_deletion_requests.len()
        );
        trace!(
            target: LOG_TARGET,
            "Processing batch of file deletion requests for file keys [{:?}]",
            event.data.file_deletion_requests.iter().map(|r| r.file_key).collect::<Vec<_>>()
        );

        // Acquire Forest root write lock. This prevents other Forest-root-writing tasks from starting while we are processing this task.
        // That is until we release the lock gracefully with the `release_forest_root_write_lock` method, or `forest_root_write_tx` is dropped.
        let forest_root_write_tx = match event.forest_root_write_tx.lock().await.take() {
            Some(tx) => tx,
            None => {
                let err_msg = "CRITICAL❗️❗️ This is a bug! Forest root write tx already taken. This is a critical bug. Please report it to the StorageHub team.";
                error!(target: LOG_TARGET, err_msg);
                return Err(anyhow!(err_msg));
            }
        };

        // TODO: Remove this once batching is supported by the runtime.
        if event.data.file_deletion_requests.len() > 1 {
            let err_msg = format!(
                "Processing batch of {} file deletion requests. This is not supported yet. Please report this to the StorageHub team.",
                event.data.file_deletion_requests.len()
            );
            error!(target: LOG_TARGET, err_msg);
            return Err(anyhow!(err_msg));
        } else if event.data.file_deletion_requests.is_empty() {
            let err_msg = "No file deletion requests to process.";
            error!(target: LOG_TARGET, err_msg);
            return Err(anyhow!(err_msg));
        }

        let delete_file_request = event.data.file_deletion_requests.first().unwrap();

        trace!(
            target: LOG_TARGET,
            "Processing file deletion request for file_key {:?}",
            delete_file_request.file_key
        );

        // Get the forest storage for the bucket
        let forest_storage = self
            .storage_hub_handler
            .forest_storage_handler
            .get(&delete_file_request.bucket_id.as_ref().to_vec())
            .await
            .ok_or_else(|| anyhow!("Failed to get forest storage."))?;

        // Acquire write lock once for the entire operation
        let forest_storage_read = forest_storage.read().await;

        // TODO: Pass multiple file keys to generate_proof once batching is supported by the runtime.
        let forest_proof =
            forest_storage_read.generate_proof(vec![delete_file_request.file_key.into()])?;

        drop(forest_storage_read);

        // Build and submit extrinsic
        let call = storage_hub_runtime::RuntimeCall::FileSystem(
            pallet_file_system::Call::pending_file_deletion_request_submit_proof {
                user: delete_file_request.user.clone(),
                file_key: delete_file_request.file_key.into(),
                file_size: delete_file_request.file_size,
                bucket_id: delete_file_request.bucket_id,
                forest_proof: forest_proof.proof.clone(),
            },
        );

        // Submit extrinsic with retry and wait for it to be included in a block
        self.storage_hub_handler
            .blockchain
            .submit_extrinsic_with_retry(
                call,
                RetryStrategy::default()
                    .with_max_retries(MAX_DELETE_FILE_REQUEST_TRY_COUNT)
                    .with_max_tip(MAX_DELETE_FILE_REQUEST_TIP as f64)
                    .with_timeout(Duration::from_secs(
                        self.storage_hub_handler
                            .provider_config
                            .extrinsic_retry_timeout,
                    ))
                    .retry_only_if_timeout(),
                false,
            )
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to submit file deletion proof after {} retries: {:?}",
                    MAX_DELETE_FILE_REQUEST_TRY_COUNT,
                    e
                )
            })?;

        if forest_proof.contains_file_key(&delete_file_request.file_key.into()) {
            let mut forest_storage_write = forest_storage.write().await;
            // Delete the file key from forest storage
            forest_storage_write
                .delete_file_key(&delete_file_request.file_key.into())
                .map_err(|e| {
                    let err_msg = format!(
                        "CRITICAL❗️❗️ Failed to remove file key from Forest storage after remove delta was applied on chain for file_key {:?}, error: {:?}",
                        delete_file_request.file_key,
                        e
                    );
                    error!(target: LOG_TARGET, err_msg);
                    anyhow!(err_msg)
                })?;
        }

        info!(
            target: LOG_TARGET,
            "Successfully processed file deletion request for file_key {:?}",
            delete_file_request.file_key
        );

        // Release the forest root write lock
        self.storage_hub_handler
            .blockchain
            .release_forest_root_write_lock(forest_root_write_tx)
            .await
    }
}

/// Handles the [`FinalisedProofSubmittedForPendingFileDeletionRequest`] event.
///
/// This event is triggered when the file deletion request is finalized on-chain.
/// The MSP will delete the file from its file storage.
impl<NT> EventHandler<FinalisedProofSubmittedForPendingFileDeletionRequest>
    for MspDeleteFileTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
{
    async fn handle_event(
        &mut self,
        event: FinalisedProofSubmittedForPendingFileDeletionRequest,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing finalized file deletion request for file_key {:?}",
            event.file_key
        );

        // Only proceed if proof of inclusion was provided, meaning the file was actually deleted from the forest
        if !event.proof_of_inclusion {
            info!(
                target: LOG_TARGET,
                "Skipping file deletion as no proof of inclusion was provided for file_key {:?}",
                event.file_key
            );
            return Ok(());
        }

        let forest_storage = self
            .storage_hub_handler
            .forest_storage_handler
            .get(&event.bucket_id.as_ref().to_vec())
            .await
            .ok_or_else(|| anyhow!("Failed to get forest storage."))?;

        if forest_storage
            .read()
            .await
            .contains_file_key(&event.file_key.into())?
        {
            warn!(
                target: LOG_TARGET,
                "FinalisedProofSubmittedForPendingFileDeletionRequest applied and finalised for file key {:?}, but file key is still in Forest. This can only happen if the same file key was added again after being deleted by this MSP.",
                event.file_key,
            );
        } else {
            // If file key is not in Forest, we can now safely remove it from the File Storage.
            let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;
            write_file_storage.delete_file(&event.file_key.into()).map_err(|e| {
                error!(target: LOG_TARGET, "Failed to remove file from File Storage after it was removed from the Forest. \nError: {:?}", e);
                anyhow!(
                    "Failed to delete file from File Storage after it was removed from the Forest: {:?}",
                    e
                )
            })?;

            // Release the file storage write lock.
            drop(write_file_storage);
        }

        Ok(())
    }
}
