use std::{collections::VecDeque, time::Duration};

use anyhow::anyhow;
use log::{debug, error};

use sc_client_api::HeaderBackend;
use sp_api::ProvideRuntimeApi;
use sp_core::H256;

use pallet_storage_providers_runtime_api::{
    QueryEarliestChangeCapacityBlockError, QueryStorageProviderCapacityError, StorageProvidersApi,
};
use shc_common::traits::StorageEnableRuntime;
use shc_common::types::{BlockNumber, StorageData};
use shc_forest_manager::traits::ForestStorageHandler;

use crate::{
    transaction::SubmittedTransaction, types::ManagedProvider, types::SendExtrinsicOptions,
    BlockchainService,
};

const LOG_TARGET: &str = "blockchain-service-capacity-manager";

/// Queue of capacity requests for batching capacity increases in a single transaction.
pub struct CapacityRequestQueue {
    /// Configuration parameters determining values for capacity increases.
    capacity_config: CapacityConfig,
    /// Pending capacity requests which have yet to be part of a transaction.
    pending_requests: VecDeque<CapacityRequest>,
    /// Capacity requests bundled in a single transaction waiting to be included in a block.
    ///
    /// All requesters will be notified via the callback when the transaction is included in the
    /// block important notification pipeline. This list will be cleared subsequently.
    requests_waiting_for_inclusion: Vec<CapacityRequest>,
    /// Total accumulated capacity required by the aggregate of all `pending_requests`.
    ///
    /// This is reset when the `pending_requests` is moved to `requests_waiting_for_inclusion` when they have been batched in a single transaction.
    total_required: StorageData,
    /// The last submitted transaction which `requests_waiting_for_inclusion` is waiting for.
    last_submitted_transaction: Option<SubmittedTransaction>,
}

impl CapacityRequestQueue {
    pub fn new(capacity_config: CapacityConfig) -> Self {
        Self {
            capacity_config,
            pending_requests: VecDeque::new(),
            requests_waiting_for_inclusion: Vec::new(),
            total_required: 0,
            last_submitted_transaction: None,
        }
    }

    /// Get the last submitted transaction.
    pub fn last_submitted_transaction(&self) -> Option<&SubmittedTransaction> {
        self.last_submitted_transaction.as_ref()
    }

    /// Get the configured maximum capacity allowed.
    ///
    /// Capacity requests will be rejected if the current provider capacity is at this limit.
    pub fn max_capacity_allowed(&self) -> StorageData {
        self.capacity_config.max_capacity
    }

    /// Queue a capacity request.
    ///
    /// This will check for overflow and maximum capacity reached.
    /// If the request cannot be queued, the error will be sent back to the caller.
    pub fn queue_capacity_request(
        &mut self,
        request: CapacityRequest,
        current_capacity: StorageData,
    ) {
        let Some(new_total_required) = self.total_required.checked_add(request.data.required)
        else {
            request.send_result(Err(anyhow!("Capacity overflow")));
            return;
        };

        if new_total_required > self.max_capacity_diff(current_capacity) {
            request.send_result(Err(anyhow!("Maximum capacity reached")));
            return;
        }

        self.total_required = new_total_required;

        self.pending_requests.push_back(request);
    }

    /// Calculate the maximum capacity difference that can be requested.
    fn max_capacity_diff(&self, current_capacity: StorageData) -> StorageData {
        self.capacity_config
            .max_capacity
            .saturating_sub(current_capacity)
    }

    /// Calculate the new capacity needed based on the total required capacity
    pub fn calculate_new_capacity(
        &self,
        current_capacity: StorageData,
        total_required: StorageData,
    ) -> StorageData {
        // Calculate how many jumps we need to cover the required capacity
        let jumps_needed = total_required.div_ceil(self.capacity_config.jump_capacity);
        let total_jump_capacity = jumps_needed * self.capacity_config.jump_capacity;

        // Calculate new total capacity
        let new_capacity = current_capacity.saturating_add(total_jump_capacity);

        // Ensure we don't exceed max capacity
        new_capacity.min(self.capacity_config.max_capacity)
    }

    /// Check if there are any pending requests
    pub fn has_pending_requests(&self) -> bool {
        !self.pending_requests.is_empty()
    }

    /// Check if there are any requests waiting for inclusion
    pub fn has_requests_waiting_for_inclusion(&self) -> bool {
        !self.requests_waiting_for_inclusion.is_empty()
    }

    /// Add all pending requests to the list of requests waiting for inclusion of the [`SubmittedTransaction`].
    pub fn add_pending_requests_to_waiting_for_inclusion(
        &mut self,
        submitted_transaction: SubmittedTransaction,
    ) {
        self.requests_waiting_for_inclusion
            .extend(self.pending_requests.drain(..));
        self.last_submitted_transaction = Some(submitted_transaction);
    }

    /// Complete all requests waiting for inclusion, notifying the callers of success.
    ///
    /// The `requests_waiting_for_inclusion` list is cleared after the requests are notified.
    pub fn complete_requests_waiting_for_inclusion(&mut self, result: Result<(), String>) {
        // Notify all callers of result
        while let Some(request) = self.requests_waiting_for_inclusion.pop() {
            request.send_result(result.clone().map_err(anyhow::Error::msg));
        }

        // Clear the last submitted transaction
        self.last_submitted_transaction = None;
    }

    /// Fail all pending requests with an error message
    pub fn fail_requests(&mut self, error_msg: String) {
        while let Some(request) = self.pending_requests.pop_front() {
            request.send_result(Err(anyhow!(error_msg.clone())));
        }
    }

    /// Reset the pending requests queue and total required capacity.
    pub fn reset_queue(&mut self) {
        self.pending_requests.clear();
        self.total_required = 0;
    }
}

/// Configuration parameters determining values for capacity increases.
#[derive(Clone, Debug)]
pub struct CapacityConfig {
    /// Maximum storage capacity of the provider in bytes.
    ///
    /// The node will not increase its on-chain capacity above this value.
    /// This is meant to reflect the actual physical storage capacity of the node.
    max_capacity: StorageData,
    /// Capacity increases by this amount in bytes a number of times based on the required capacity calculated
    /// by the [`calculate_new_capacity`](CapacityRequestQueue::calculate_new_capacity) method.
    ///
    /// The jump capacity is the amount of storage that the node will increase in its on-chain
    /// capacity by adding more stake. For example, if the jump capacity is set to 1k, and the
    /// node needs 100 units of storage more to store a file, the node will automatically increase
    /// its on-chain capacity by 1k units.
    jump_capacity: StorageData,
}

impl CapacityConfig {
    pub fn new(max_capacity: StorageData, jump_capacity: StorageData) -> Self {
        Self {
            max_capacity,
            jump_capacity,
        }
    }

    pub fn max_capacity(&self) -> StorageData {
        self.max_capacity
    }
}

/// Individual capacity request for every caller.
pub struct CapacityRequest {
    /// Data needed to process the capacity request.
    data: CapacityRequestData,
    /// Callback to notify the caller when the capacity request is processed.
    callback: tokio::sync::oneshot::Sender<Result<(), anyhow::Error>>,
}

impl CapacityRequest {
    pub fn new(
        data: CapacityRequestData,
        callback: tokio::sync::oneshot::Sender<Result<(), anyhow::Error>>,
    ) -> Self {
        Self { data, callback }
    }

    pub fn send_result(self, result: Result<(), anyhow::Error>) {
        if let Err(e) = self.callback.send(result) {
            error!(target: LOG_TARGET, "Failed to send capacity request result: {:?}", e);
        }
    }
}

/// Data needed to process a capacity request.
pub struct CapacityRequestData {
    /// Capacity requested to be increased.
    required: StorageData,
}

impl CapacityRequestData {
    pub fn new(required: StorageData) -> Self {
        Self { required }
    }
}

impl<FSH, Runtime> BlockchainService<FSH, Runtime>
where
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    /// Queue a capacity request.
    ///
    /// If the capacity request cannot be queued for any reason, the error will be sent back to the caller.
    pub(crate) async fn queue_capacity_request(&mut self, capacity_request: CapacityRequest) {
        match self.check_capacity_request_conditions().await {
            Ok((_, current_capacity, _)) => {
                if let Some(capacity_manager) = self.capacity_manager.as_mut() {
                    capacity_manager.queue_capacity_request(capacity_request, current_capacity);
                } else {
                    capacity_request.send_result(Err(anyhow!("Capacity manager not initialized")));
                    return;
                }
            }
            Err(e) => {
                // Send the error back to the caller.
                capacity_request.send_result(Err(e));
            }
        }
    }

    /// Process any pending capacity requests.
    ///
    /// Since the `pending_requests` queue is kept in a valid state by pushing capacity requests that would still amount to a valid
    /// `total_required` value not exceeding the `max_capacity_allowed`, we add them all to the `requests_waiting_for_inclusion` list
    /// and send the `total_required` value in a single `change_capacity` extrinsic.
    pub(crate) async fn process_capacity_requests(
        &mut self,
        block_number: BlockNumber,
    ) -> Result<(), anyhow::Error> {
        debug!(target: LOG_TARGET, "[process_capacity_requests] Processing capacity requests");
        let (current_block_hash, current_capacity, inner_provider_id) = match self
            .check_capacity_request_conditions()
            .await
        {
            Ok(result) => result,
            Err(e) => {
                error!(target: LOG_TARGET, "Failed to check capacity request conditions: {:?}", e);
                return Ok(());
            }
        };

        let capacity_manager_ref = self
            .capacity_manager
            .as_ref()
            .expect("Capacity manager should be initialized; qed");

        // Skip the process if there are no pending requests.
        if !capacity_manager_ref.has_pending_requests() {
            debug!(target: LOG_TARGET, "[process_capacity_requests] No pending requests, skipping");
            return Ok(());
        }

        // Query earliest block to change capacity
        debug!(target: LOG_TARGET, "[process_capacity_requests] Querying earliest block to change capacity");
        let earliest_block = self
            .client
            .runtime_api()
            .query_earliest_change_capacity_block(current_block_hash, &inner_provider_id)
            .unwrap_or_else(|_| {
                error!(target: LOG_TARGET, "Failed to query earliest block to change capacity");
                Err(QueryEarliestChangeCapacityBlockError::InternalError)
            })
            .map_err(|e| anyhow!("Failed to query earliest block to change capacity: {:?}", e))?;

        if block_number < earliest_block.saturating_sub(1) {
            debug!(target: LOG_TARGET, "[process_capacity_requests] Earliest block to change capacity: {:?}", earliest_block);
            // Must wait until the earliest block to change capacity.
            return Ok(());
        }

        let required_capacity = capacity_manager_ref.total_required;

        // Calculate new capacity based on configuration
        let new_capacity =
            capacity_manager_ref.calculate_new_capacity(current_capacity, required_capacity);

        // Send the extrinsic to change the provider's capacity and wait for it to succeed.
        let call = storage_hub_runtime::RuntimeCall::Providers(
            pallet_storage_providers::Call::change_capacity { new_capacity },
        );

        let extrinsic_retry_timeout = Duration::from_secs(self.config.extrinsic_retry_timeout);

        // Send extrinsic to increase capacity
        match self
            .send_extrinsic(call, &SendExtrinsicOptions::new(extrinsic_retry_timeout))
            .await
        {
            Ok(output) => {
                // Add all pending requests to the list of requests waiting for inclusion.
                if let Some(capacity_manager) = self.capacity_manager.as_mut() {
                    capacity_manager.add_pending_requests_to_waiting_for_inclusion(
                        SubmittedTransaction::new(
                            output.receiver,
                            output.hash,
                            output.nonce,
                            extrinsic_retry_timeout,
                        ),
                    );
                } else {
                    error!(target: LOG_TARGET, "Capacity manager not initialized");
                }
            }
            Err(e) => {
                error!(target: LOG_TARGET, "Failed to send increase capacity extrinsic: {:?}", e);
                // Notify all in-flight requests of the error
                if let Some(capacity_manager) = self.capacity_manager.as_mut() {
                    capacity_manager.fail_requests(e.to_string());
                } else {
                    error!(target: LOG_TARGET, "Capacity manager not initialized");
                }
            }
        };

        // Ensure the pending requests queue and total required capacity are reset so that
        // new capacity requests can be queued and tally up from 0 again.
        if let Some(capacity_manager) = self.capacity_manager.as_mut() {
            capacity_manager.reset_queue();
        } else {
            error!(target: LOG_TARGET, "Capacity manager not initialized");
        }

        Ok(())
    }

    /// Check if the capacity manager is initialized and if the provider ID is set.
    ///
    /// Ensure that the current capacity of the provider registered in the runtime is less than the maximum capacity configured
    /// by the node operator.
    async fn check_capacity_request_conditions(
        &mut self,
    ) -> Result<(H256, StorageData, H256), anyhow::Error> {
        // Any errors in this block is considered a critical error which would not allow processing any capacity requests.
        // Only process capacity requests if the capacity manager is initialized
        let Some(capacity_manager) = &self.capacity_manager else {
            return Err(anyhow!("Capacity manager not initialized"));
        };

        // Get provider ID
        let Some(managed_provider) = &self.maybe_managed_provider else {
            return Err(anyhow!(
                "No provider ID set, cannot process capacity requests"
            ));
        };

        let provider_id = match managed_provider {
            ManagedProvider::Msp(msp_handler) => msp_handler.msp_id,
            ManagedProvider::Bsp(bsp_handler) => bsp_handler.bsp_id,
        };

        // Get current block hash
        let current_block_hash = self.client.info().best_hash;

        // Query current capacity
        let current_capacity = self
            .client
            .runtime_api()
            .query_storage_provider_capacity(current_block_hash, &provider_id)
            .unwrap_or_else(|_| Err(QueryStorageProviderCapacityError::InternalError))
            .map_err(|e| anyhow!("Failed to query current storage capacity: {:?}", e))?;

        if current_capacity >= capacity_manager.max_capacity_allowed() {
            return Err(anyhow!("Provider already at maximum capacity"));
        }

        Ok((current_block_hash, current_capacity, provider_id))
    }
}
