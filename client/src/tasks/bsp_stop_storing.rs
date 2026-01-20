use std::time::Duration;

use anyhow::anyhow;
use codec::Decode;
use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface,
    events::{BspRequestedToStopStoringNotification, RequestBspStopStoring},
    types::SendExtrinsicOptions,
};
use shc_common::{consts::CURRENT_FOREST_KEY, traits::StorageEnableRuntime};
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use sp_core::H256;
use sp_runtime::{traits::SaturatedConversion, Saturating};

use crate::{
    handler::StorageHubHandler,
    types::{BspForestStorageHandlerT, ForestStorageKey, ShNodeType},
};

const LOG_TARGET: &str = "bsp-stop-storing-task";

/// BSP Stop Storing Task: Handles the two-phase process of a BSP voluntarily stopping
/// storage of a file.
///
/// This task reacts to the events:
/// - **[`RequestBspStopStoring`] Event:**
///   - Triggered by the RPC method `requestBspStopStoring`.
///   - Retrieves file metadata from the forest storage.
///   - Generates a forest inclusion proof.
///   - Submits the `bsp_request_stop_storing` extrinsic to initiate the stop storing process.
///
/// - **[`BspRequestedToStopStoringNotification`] Event:**
///   - Triggered when the on-chain `BspRequestedToStopStoring` event is detected.
///   - Queries the `MinWaitForStopStoring` config from runtime.
///   - Waits for the required number of ticks.
///   - Generates a new forest inclusion proof.
///   - Submits the `bsp_confirm_stop_storing` extrinsic to complete the process.
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
}

/// Handles the [`RequestBspStopStoring`] event.
///
/// This event is triggered by the RPC method `requestBspStopStoring` to initiate
/// the stop storing process for a file.
///
/// This handler performs the following actions:
/// 1. Retrieves the file metadata from the forest storage.
/// 2. Generates a forest inclusion proof for the file key.
/// 3. Submits the `bsp_request_stop_storing` extrinsic.
impl<NT, Runtime> EventHandler<RequestBspStopStoring> for BspStopStoringTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: RequestBspStopStoring) -> anyhow::Result<String> {
        info!(
            target: LOG_TARGET,
            "Processing RequestBspStopStoring for file key [{:x}]",
            event.file_key
        );

        // Convert the file key to the corresponding type.
        let file_key: H256 = event.file_key.into();

        // Get the forest storage.
        let current_forest_key = ForestStorageKey::from(CURRENT_FOREST_KEY.to_vec());
        let read_fs = self
            .storage_hub_handler
            .forest_storage_handler
            .get(&current_forest_key)
            .await
            .ok_or_else(|| anyhow!("CRITICAL: Failed to get forest storage."))?;

        // Get file metadata from forest storage.
        let file_metadata = {
            let fs = read_fs.read().await;
            fs.get_file_metadata(&file_key)?
                .ok_or_else(|| anyhow!("File key [{:x}] not found in forest storage", file_key))?
        };

        // Generate forest inclusion proof.
        let forest_proof = {
            let fs = read_fs.read().await;
            fs.generate_proof(vec![file_key])?
        };

        // Parse file metadata fields.
        let owner_bytes = file_metadata.owner();
        let owner = Runtime::AccountId::decode(&mut &owner_bytes[..])
            .map_err(|e| anyhow!("Failed to decode owner account ID: {:?}", e))?;

        let bucket_id_bytes = file_metadata.bucket_id();
        let bucket_id = Runtime::Hash::decode(&mut &bucket_id_bytes[..])
            .map_err(|e| anyhow!("Failed to decode bucket ID: {:?}", e))?;

        let location_bytes = file_metadata.location().to_vec();
        let location = location_bytes
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
            can_serve: true, // We can still serve the file during the waiting period
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
            .await;

        match result {
            Ok(submitted_ext_info) => {
                info!(
                    target: LOG_TARGET,
                    "Successfully submitted bsp_request_stop_storing for file key [{:x}]. Extrinsic hash: {:?}",
                    file_key,
                    submitted_ext_info.hash
                );
            }
            Err(e) => {
                error!(
                    target: LOG_TARGET,
                    "Failed to submit bsp_request_stop_storing for file key [{:x}]: {:?}",
                    file_key,
                    e
                );
                return Err(anyhow!(
                    "Failed to submit bsp_request_stop_storing: {:?}",
                    e
                ));
            }
        }

        Ok(format!(
            "Handled RequestBspStopStoring for file key [{:x}]",
            file_key
        ))
    }
}

/// Handles the [`BspRequestedToStopStoringNotification`] event.
///
/// This event is triggered when the on-chain `BspRequestedToStopStoring` event is detected.
/// The handler waits for the minimum required period and then submits the confirmation.
///
/// This handler performs the following actions:
/// 1. Queries the `MinWaitForStopStoring` config from the runtime.
/// 2. Calculates the tick at which confirmation can be submitted.
/// 3. Waits for that tick.
/// 4. Generates a new forest inclusion proof.
/// 5. Submits the `bsp_confirm_stop_storing` extrinsic.
impl<NT, Runtime> EventHandler<BspRequestedToStopStoringNotification<Runtime>>
    for BspStopStoringTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: BspRequestedToStopStoringNotification<Runtime>,
    ) -> anyhow::Result<String> {
        info!(
            target: LOG_TARGET,
            "Processing BspRequestedToStopStoringNotification for file key [{:x}], BSP [{:x}]",
            event.file_key,
            event.bsp_id
        );

        // Convert the file key to the corresponding type.
        let file_key: H256 = event.file_key.into();

        // Query MinWaitForStopStoring from the runtime.
        let min_wait = self
            .storage_hub_handler
            .blockchain
            .query_min_wait_for_stop_storing()
            .await
            .map_err(|e| anyhow!("Failed to query MinWaitForStopStoring: {:?}", e))?;

        // Get the current tick to calculate when we can confirm.
        let current_block_info = self
            .storage_hub_handler
            .blockchain
            .get_best_block_info()
            .await
            .map_err(|e| anyhow!("Failed to get current block info: {:?}", e))?;

        // Calculate the tick at which we can confirm stopping.
        // We add 1 to be safe and ensure we're past the minimum wait.
        let confirm_tick = current_block_info
            .number
            .saturating_add(min_wait)
            .saturating_add(1u32.into());

        info!(
            target: LOG_TARGET,
            "Waiting until tick {} to confirm stop storing for file key [{:x}]. Current tick: {}, MinWait: {}",
            confirm_tick,
            file_key,
            current_block_info.number,
            min_wait
        );

        // Wait for the tick.
        self.storage_hub_handler
            .blockchain
            .wait_for_tick(confirm_tick)
            .await
            .map_err(|e| anyhow!("Failed to wait for tick {}: {:?}", confirm_tick, e))?;

        info!(
            target: LOG_TARGET,
            "Tick {} reached, proceeding to confirm stop storing for file key [{:x}]",
            confirm_tick,
            file_key
        );

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
            .await;

        match result {
            Ok(submitted_ext_info) => {
                info!(
                    target: LOG_TARGET,
                    "Successfully submitted bsp_confirm_stop_storing for file key [{:x}]. Extrinsic hash: {:?}",
                    file_key,
                    submitted_ext_info.hash
                );
            }
            Err(e) => {
                error!(
                    target: LOG_TARGET,
                    "Failed to submit bsp_confirm_stop_storing for file key [{:x}]: {:?}",
                    file_key,
                    e
                );
                return Err(anyhow!(
                    "Failed to submit bsp_confirm_stop_storing: {:?}",
                    e
                ));
            }
        }

        Ok(format!(
            "Handled BspRequestedToStopStoringNotification for file key [{:x}]",
            file_key
        ))
    }
}
