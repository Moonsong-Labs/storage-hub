use std::time::Duration;

use anyhow::anyhow;
use codec::Decode;
use sc_tracing::tracing::*;
use shc_actors_framework::{actor::ActorHandle, event_bus::EventHandler};
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface,
    events::{ProcessBspConfirmStopStoring, ProcessBspRequestStopStoring},
    types::{ConfirmBspStopStoringRequest, RequestBspStopStoringRequest, SendExtrinsicOptions},
    BlockchainService,
};
use shc_common::{
    consts::CURRENT_FOREST_KEY, traits::StorageEnableRuntime, types::StorageProviderId,
};
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use sp_core::H256;
use sp_runtime::traits::SaturatedConversion;
use tokio::sync::oneshot;

use crate::{
    handler::StorageHubHandler,
    types::{BspForestStorageHandlerT, ForestStorageKey, ShNodeType},
};

const LOG_TARGET: &str = "bsp-stop-storing-task";

/// Maximum number of retries for proof-related errors before giving up.
const MAX_PROOF_RETRIES: u32 = 3;

/// RAII guard for the forest root write lock.
///
/// This guard provides an automatic release via Drop of the forest root write lock.
struct ForestLockGuard<FSH, Runtime>
where
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    tx: Option<oneshot::Sender<()>>,
    blockchain: ActorHandle<BlockchainService<FSH, Runtime>>,
}

impl<FSH, Runtime> ForestLockGuard<FSH, Runtime>
where
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    fn new(
        tx: oneshot::Sender<()>,
        blockchain: ActorHandle<BlockchainService<FSH, Runtime>>,
    ) -> Self {
        Self {
            tx: Some(tx),
            blockchain,
        }
    }
}

impl<FSH, Runtime> Drop for ForestLockGuard<FSH, Runtime>
where
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    fn drop(&mut self) {
        if let Some(tx) = self.tx.take() {
            let blockchain = self.blockchain.clone();
            // Spawn a task to release the lock asynchronously since Drop can't be async
            tokio::spawn(async move {
                if let Err(e) = blockchain.release_forest_root_write_lock(tx).await {
                    error!(
                        target: LOG_TARGET,
                        "Failed to release forest root write lock: {:?}",
                        e
                    );
                }
            });
        }
    }
}

/// BSP Stop Storing Task: Handles the two-phase process of a BSP voluntarily stopping
/// storage of a file.
///
/// This task reacts to the events:
/// - **[`ProcessBspRequestStopStoring`] Event:**
///   - Emitted by the blockchain service when the forest root write lock is available.
///   - Retrieves file metadata from the forest storage.
///   - Generates a forest inclusion proof.
///   - Submits the `bsp_request_stop_storing` extrinsic to initiate the stop storing process.
///   - On proof error, requeues the request for retry.
///
/// - **[`ProcessBspConfirmStopStoring`] Event:**
///   - Emitted by the blockchain service when the confirm tick has been reached
///     and the forest root write lock is available.
///   - Generates a new forest inclusion proof.
///   - Submits the `bsp_confirm_stop_storing` extrinsic to complete the process.
///   - On proof error, requeues the request for retry.
pub struct BspStopStoringTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for BspStopStoringTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> BspStopStoringTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, Runtime> BspStopStoringTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler,
        }
    }

    /// Check if an error message indicates a proof-related error that can be retried.
    fn is_proof_error(error: &anyhow::Error) -> bool {
        let error_str = format!("{:?}", error);
        error_str.contains("ForestProofVerificationFailed")
            || error_str.contains("FailedToApplyDelta")
    }

    /// Requeue a request stop storing request for retry.
    async fn requeue_bsp_request_stop_storing(
        &self,
        mut request: RequestBspStopStoringRequest<Runtime>,
    ) {
        request.increment_try_count();
        if request.try_count > MAX_PROOF_RETRIES {
            error!(
                target: LOG_TARGET,
                "BSP request stop storing for file [{:?}] exceeded max retries ({}), dropping request",
                request.file_key,
                MAX_PROOF_RETRIES
            );
            return;
        }

        info!(
            target: LOG_TARGET,
            "Requeuing BSP request stop storing for file [{:?}], attempt {}",
            request.file_key,
            request.try_count
        );

        if let Err(e) = self
            .storage_hub_handler
            .blockchain
            .queue_bsp_request_stop_storing(request.clone())
            .await
        {
            error!(
                target: LOG_TARGET,
                "Failed to requeue BSP request stop storing for file [{:?}]: {:?}",
                request.file_key,
                e
            );
        }
    }

    /// Requeue a confirm stop storing request for retry.
    async fn requeue_bsp_confirm_stop_storing(
        &self,
        mut request: ConfirmBspStopStoringRequest<Runtime>,
    ) {
        request.increment_try_count();
        if request.try_count > MAX_PROOF_RETRIES {
            error!(
                target: LOG_TARGET,
                "BSP confirm stop storing for file [{:?}] exceeded max retries ({}), dropping request",
                request.file_key,
                MAX_PROOF_RETRIES
            );
            return;
        }

        info!(
            target: LOG_TARGET,
            "Requeuing BSP confirm stop storing for file [{:?}], attempt {}",
            request.file_key,
            request.try_count
        );

        if let Err(e) = self
            .storage_hub_handler
            .blockchain
            .queue_bsp_confirm_stop_storing(request.clone())
            .await
        {
            error!(
                target: LOG_TARGET,
                "Failed to requeue BSP confirm stop storing for file [{:?}]: {:?}",
                request.file_key,
                e
            );
        }
    }
}

/// Handles the [`ProcessBspRequestStopStoring`] event.
///
/// This event is emitted when the blockchain service has acquired the forest root write lock
/// and is ready to process the request stop storing.
///
/// This handler performs the following actions:
/// 1. Acquires the forest root write lock from the event (via RAII guard for automatic release).
/// 2. Retrieves the file metadata from the forest storage.
/// 3. Generates a forest inclusion proof for the file key.
/// 4. Submits the `bsp_request_stop_storing` extrinsic.
/// 5. On proof error, requeues the request for retry.
impl<NT, Runtime> EventHandler<ProcessBspRequestStopStoring<Runtime>>
    for BspStopStoringTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: ProcessBspRequestStopStoring<Runtime>,
    ) -> anyhow::Result<String> {
        // Acquire the forest root write lock.
        let forest_root_write_tx =
            event
                .forest_root_write_tx
                .lock()
                .await
                .take()
                .ok_or_else(|| {
                    error!(
                        target: LOG_TARGET,
                        "CRITICAL: Forest root write tx already taken for BSP request stop storing"
                    );
                    anyhow!("Forest root write tx already taken")
                })?;

        let _lock_guard = ForestLockGuard::new(
            forest_root_write_tx,
            self.storage_hub_handler.blockchain.clone(),
        );

        let request = event.data.request;
        let file_key: H256 = request.file_key.into();

        info!(
            target: LOG_TARGET,
            "Processing BSP request stop storing for file key [0x{:x}], attempt {}",
            file_key,
            request.try_count + 1
        );

        // Get the forest storage.
        let current_forest_key = ForestStorageKey::from(CURRENT_FOREST_KEY.to_vec());
        let read_fs = self
            .storage_hub_handler
            .forest_storage_handler
            .get(&current_forest_key)
            .await
            .ok_or_else(|| anyhow!("CRITICAL: Failed to get forest storage."))?;

        // Acquire a read lock on the forest storage.
        let fs = read_fs.read().await;

        // Get file metadata from forest storage.
        let file_metadata = fs
            .get_file_metadata(&file_key)?
            .ok_or_else(|| anyhow!("File key [0x{:x}] not found in forest storage", file_key))?;

        // Generate forest inclusion proof.
        let forest_proof = fs.generate_proof(vec![file_key])?;

        // Drop the read lock on the forest storage.
        drop(fs);

        // Parse file metadata fields.
        let owner_bytes = file_metadata.owner();
        let owner = Runtime::AccountId::decode(&mut &owner_bytes[..])
            .map_err(|e| anyhow!("Failed to decode owner account ID: {:?}", e))?;

        let bucket_id_bytes = file_metadata.bucket_id();
        let bucket_id = Runtime::Hash::decode(&mut &bucket_id_bytes[..])
            .map_err(|e| anyhow!("Failed to decode bucket ID: {:?}", e))?;

        let location_bytes = file_metadata.location().to_vec();
        let location: pallet_file_system::types::FileLocation<Runtime> = location_bytes
            .try_into()
            .map_err(|_| anyhow!("Failed to convert location to BoundedVec"))?;

        let fingerprint_bytes = file_metadata.fingerprint();
        let fingerprint = Runtime::Hash::decode(&mut fingerprint_bytes.as_ref())
            .map_err(|e| anyhow!("Failed to decode fingerprint: {:?}", e))?;

        let size = file_metadata.file_size().saturated_into();

        // Build and submit the extrinsic.
        let call: Runtime::Call = pallet_file_system::Call::<Runtime>::bsp_request_stop_storing {
            file_key,
            bucket_id,
            location,
            owner,
            fingerprint,
            size,
            can_serve: false,
            inclusion_forest_proof: forest_proof.proof.into(),
        }
        .into();

        let options = SendExtrinsicOptions::new(
            Duration::from_secs(
                self.storage_hub_handler
                    .provider_config
                    .blockchain_service
                    .extrinsic_retry_timeout,
            ),
            Some("fileSystem".to_string()),
            Some("bspRequestStopStoring".to_string()),
        );

        let result = self
            .storage_hub_handler
            .blockchain
            .send_extrinsic(call, options)
            .await
            .map_err(|e| anyhow!("Failed to submit BSP request stop storing: {:?}", e))?
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await;

        match result {
            Ok(_) => {
                info!(
                    target: LOG_TARGET,
                    "Successfully submitted bsp_request_stop_storing for file key [0x{:x}]",
                    file_key
                );
                Ok(format!(
                    "Handled ProcessBspRequestStopStoring for file key [0x{:x}]",
                    file_key
                ))
            }
            Err(e) => {
                // Check if this is a proof error that can be retried
                if Self::is_proof_error(&e) {
                    warn!(
                        target: LOG_TARGET,
                        "Proof error for BSP request stop storing file [0x{:x}], requeuing: {:?}",
                        file_key,
                        e
                    );
                    self.requeue_bsp_request_stop_storing(request).await;
                    return Ok(format!(
                        "Requeued BSP request stop storing for file [0x{:x}] due to proof error",
                        file_key
                    ));
                }

                // Non-proof error, fail permanently
                Err(anyhow!(
                    "Failed to watch for success on bsp_request_stop_storing for file [0x{:x}]: {:?}",
                    file_key,
                    e
                ))
            }
        }
    }
}

/// Handles the [`ProcessBspConfirmStopStoring`] event.
///
/// This event is emitted when the blockchain service has acquired the forest root write lock
/// and the minimum wait period has passed, ready to process the confirm stop storing.
///
/// This handler performs the following actions:
/// 1. Acquires the forest root write lock from the event.
/// 2. Generates a new forest inclusion proof.
/// 3. Submits the `bsp_confirm_stop_storing` extrinsic.
/// 4. On proof error, requeues the request for retry.
impl<NT, Runtime> EventHandler<ProcessBspConfirmStopStoring<Runtime>>
    for BspStopStoringTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: ProcessBspConfirmStopStoring<Runtime>,
    ) -> anyhow::Result<String> {
        // Acquire the forest root write lock.
        let forest_root_write_tx =
            event
                .forest_root_write_tx
                .lock()
                .await
                .take()
                .ok_or_else(|| {
                    error!(
                        target: LOG_TARGET,
                        "CRITICAL: Forest root write tx already taken for BSP confirm stop storing"
                    );
                    anyhow!("Forest root write tx already taken")
                })?;

        let _lock_guard = ForestLockGuard::new(
            forest_root_write_tx,
            self.storage_hub_handler.blockchain.clone(),
        );

        let request = event.data.request;
        let file_key: H256 = request.file_key.into();

        info!(
            target: LOG_TARGET,
            "Processing BSP confirm stop storing for file key [0x{:x}], attempt {}",
            file_key,
            request.try_count + 1
        );

        // Get our BSP ID to check if the request still exists on-chain
        let own_provider_id = self
            .storage_hub_handler
            .blockchain
            .query_storage_provider_id(None)
            .await?;
        let own_bsp_id = match own_provider_id {
            Some(StorageProviderId::BackupStorageProvider(id)) => id,
            Some(StorageProviderId::MainStorageProvider(_)) => {
                return Err(anyhow!(
                    "Current node is an MSP, but this task is for BSPs only."
                ));
            }
            None => {
                return Err(anyhow!("Failed to get own BSP ID."));
            }
        };

        // Check if the pending stop storing request still exists on-chain.
        // It may have been removed due to a reorg, manual confirmation, or other circumstances.
        let has_request = self
            .storage_hub_handler
            .blockchain
            .has_pending_stop_storing_request(own_bsp_id, file_key.into())
            .await
            .map_err(|e| anyhow!("Failed to check pending stop storing request: {:?}", e))?;

        if !has_request {
            info!(
                target: LOG_TARGET,
                "Pending stop storing request for file key [0x{:x}] no longer exists on-chain. Skipping confirmation.",
                file_key
            );
            return Ok(format!(
                "Skipped BSP confirm stop storing for file [0x{:x}]: request no longer exists on-chain",
                file_key
            ));
        }

        // Get the forest storage and generate a new inclusion proof.
        let current_forest_key = ForestStorageKey::from(CURRENT_FOREST_KEY.to_vec());
        let read_fs = self
            .storage_hub_handler
            .forest_storage_handler
            .get(&current_forest_key)
            .await
            .ok_or_else(|| anyhow!("CRITICAL: Failed to get forest storage."))?;

        // Generate forest inclusion proof.
        let forest_proof = {
            let fs = read_fs.read().await;
            fs.generate_proof(vec![file_key])?
        };

        // Build and submit the confirm extrinsic.
        let call: Runtime::Call = pallet_file_system::Call::<Runtime>::bsp_confirm_stop_storing {
            file_key,
            inclusion_forest_proof: forest_proof.proof.into(),
        }
        .into();

        let options = SendExtrinsicOptions::new(
            Duration::from_secs(
                self.storage_hub_handler
                    .provider_config
                    .blockchain_service
                    .extrinsic_retry_timeout,
            ),
            Some("fileSystem".to_string()),
            Some("bspConfirmStopStoring".to_string()),
        );

        let result = self
            .storage_hub_handler
            .blockchain
            .send_extrinsic(call, options)
            .await
            .map_err(|e| anyhow!("Failed to submit BSP confirm stop storing: {:?}", e))?
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await;

        match result {
            Ok(_) => {
                info!(
                    target: LOG_TARGET,
                    "Successfully submitted bsp_confirm_stop_storing for file key [0x{:x}]",
                    file_key
                );
                Ok(format!(
                    "Handled ProcessBspConfirmStopStoring for file key [0x{:x}]",
                    file_key
                ))
            }
            Err(e) => {
                // Check if this is a proof error that can be retried
                if Self::is_proof_error(&e) {
                    warn!(
                        target: LOG_TARGET,
                        "Proof error for BSP confirm stop storing file [0x{:x}], requeuing: {:?}",
                        file_key,
                        e
                    );
                    self.requeue_bsp_confirm_stop_storing(request).await;
                    return Ok(format!(
                        "Requeued BSP confirm stop storing for file [0x{:x}] due to proof error",
                        file_key
                    ));
                }

                // Non-proof error, fail permanently
                Err(anyhow!(
                    "Failed to watch for success on bsp_confirm_stop_storing for file [0x{:x}]: {:?}",
                    file_key,
                    e
                ))
            }
        }
    }
}
