use std::sync::Arc;

use anyhow::{anyhow, Result};
use codec::Encode;
use cumulus_primitives_core::BlockT;
use log::{debug, error, info, trace, warn};
use pallet_proofs_dealer_runtime_api::{
    GetChallengePeriodError, GetChallengeSeedError, GetLastTickProviderSubmittedProofError,
    ProofsDealerApi,
};
use pallet_storage_providers::types::StorageProviderId;
use pallet_storage_providers_runtime_api::StorageProvidersApi;
use polkadot_runtime_common::BlockHashCount;
use sc_client_api::{BlockBackend, BlockImportNotification, HeaderBackend};
use serde_json::Number;
use shc_actors_framework::actor::Actor;
use shc_common::{
    blockchain_utils::get_events_at_block,
    types::{
        BlockNumber, MaxBatchMspRespondStorageRequests, ParachainClient, ProviderId, BCSV_KEY_TYPE,
    },
};
use sp_api::ProvideRuntimeApi;
use sp_core::{Blake2Hasher, Get, Hasher, H256};
use sp_keystore::KeystorePtr;
use sp_runtime::{
    generic::{self, SignedPayload},
    SaturatedConversion,
};
use storage_hub_runtime::{Runtime, SignedExtra, UncheckedExtrinsic};
use substrate_frame_rpc_system::AccountNonceApi;
use tokio::sync::{oneshot::error::TryRecvError, Mutex};

use crate::{
    events::{
        ForestWriteLockTaskData, MultipleNewChallengeSeeds, NotifyPeriod,
        ProcessConfirmStoringRequest, ProcessConfirmStoringRequestData,
        ProcessMspRespondStoringRequest, ProcessMspRespondStoringRequestData,
        ProcessStopStoringForInsolventUserRequest, ProcessStopStoringForInsolventUserRequestData,
        ProcessSubmitProofRequest, ProcessSubmitProofRequestData,
    },
    handler::LOG_TARGET,
    state::{
        OngoingProcessConfirmStoringRequestCf, OngoingProcessMspRespondStorageRequestCf,
        OngoingProcessStopStoringForInsolventUserRequestCf,
    },
    typed_store::{CFDequeAPI, ProvidesTypedDbSingleAccess},
    types::{BestBlockInfo, Extrinsic, NewBlockNotificationKind, Tip},
    BlockchainService,
};

impl BlockchainService {
    /// Notify tasks waiting for a block number.
    pub(crate) fn notify_import_block_number(&mut self, block_number: &BlockNumber) {
        let mut keys_to_remove = Vec::new();

        for (block_number, waiters) in self
            .wait_for_block_request_by_number
            .range_mut(..=block_number)
        {
            keys_to_remove.push(*block_number);
            for waiter in waiters.drain(..) {
                match waiter.send(()) {
                    Ok(_) => {}
                    Err(_) => {
                        error!(target: LOG_TARGET, "Failed to notify task about block number.");
                    }
                }
            }
        }

        for key in keys_to_remove {
            self.wait_for_block_request_by_number.remove(&key);
        }
    }

    /// Notify tasks waiting for a tick number.
    pub(crate) fn notify_tick_number(&mut self, block_hash: &H256) {
        // Get the current tick number.
        let tick_number = match self.client.runtime_api().get_current_tick(*block_hash) {
            Ok(current_tick) => current_tick,
            Err(_) => {
                error!(target: LOG_TARGET, "CRITICAL❗️❗️ Failed to query current tick from runtime in block hash {:?} and block number {:?}. This should not happen.", block_hash, self.client.info().best_number);
                return;
            }
        };

        let mut keys_to_remove = Vec::new();

        for (tick_number, waiters) in self
            .wait_for_tick_request_by_number
            .range_mut(..=tick_number)
        {
            keys_to_remove.push(*tick_number);
            for waiter in waiters.drain(..) {
                match waiter.send(Ok(())) {
                    Ok(_) => {}
                    Err(_) => {
                        error!(target: LOG_TARGET, "Failed to notify task about tick number.");
                    }
                }
            }
        }

        for key in keys_to_remove {
            self.wait_for_tick_request_by_number.remove(&key);
        }
    }

    pub(crate) fn register_best_block_and_check_reorg<Block>(
        &mut self,
        block_import_notification: &BlockImportNotification<Block>,
    ) -> NewBlockNotificationKind
    where
        Block: cumulus_primitives_core::BlockT<Hash = H256>,
    {
        let last_best_block = self.best_block.clone();
        let new_block_info: BestBlockInfo = block_import_notification.into();

        // If the new block is NOT the new best, this is a block from a non-best fork branch.
        if !block_import_notification.is_new_best {
            trace!(target: LOG_TARGET, "New non-best block imported: {:?}", new_block_info);
            return NewBlockNotificationKind::NewNonBestBlock(new_block_info);
        }

        // At this point we know that the new block is a new best block.
        trace!(target: LOG_TARGET, "New best block imported: {:?}", new_block_info);
        self.best_block = new_block_info;

        // If `tree_route` is `None`, this means that there was NO reorg while importing the block.
        if block_import_notification.tree_route.is_none() {
            return NewBlockNotificationKind::NewBestBlock(new_block_info);
        }

        // At this point we know that the new block is the new best block, and that it also caused a reorg.
        let tree_route = block_import_notification
            .tree_route
            .as_ref()
            .expect("Tree route should exist, it was just checked to be `Some`; qed")
            .clone();
        info!(target: LOG_TARGET, "New best block caused a reorg: {:?}", new_block_info);
        info!(target: LOG_TARGET, "Tree route: {:?}", tree_route);
        NewBlockNotificationKind::Reorg {
            old_best_block: last_best_block,
            new_best_block: new_block_info,
        }
    }

    /// Checks if the account nonce on-chain is higher than the nonce in the [`BlockchainService`].
    ///
    /// If the nonce is higher, the account nonce is updated in the [`BlockchainService`].
    pub(crate) fn check_nonce(&mut self, block_hash: &H256) {
        let pub_key = Self::caller_pub_key(self.keystore.clone());
        let latest_nonce = self
            .client
            .runtime_api()
            .account_nonce(*block_hash, pub_key.into())
            .expect("Fetching account nonce works; qed");
        if latest_nonce > self.nonce_counter {
            self.nonce_counter = latest_nonce
        }
    }

    /// Get all the provider IDs linked to keys in this node's keystore.
    ///
    /// The provider IDs found are added to the [`BlockchainService`]'s list of provider IDs.
    pub(crate) fn get_provider_ids(&mut self, block_hash: &H256) {
        for key in self.keystore.sr25519_public_keys(BCSV_KEY_TYPE) {
            self.client
                .runtime_api()
                .get_storage_provider_id(*block_hash, &key.into())
                .map(|provider_id| {
                    if let Some(provider_id) = provider_id {
                        match provider_id {
                            StorageProviderId::BackupStorageProvider(bsp_id) => {
                                self.provider_ids.insert(bsp_id);
                            }
                            StorageProviderId::MainStorageProvider(msp_id) => {
                                self.provider_ids.insert(msp_id);
                            }
                        }
                    } else {
                        warn!(target: LOG_TARGET, "There is no provider ID for key: {:?}. This means that the node has a BCSV key in the keystore for which there is no provider ID.", key);
                    }
                })
                .unwrap_or_else(|_| {
                    warn!(target: LOG_TARGET, "Failed to get provider ID for key: {:?}.", key);
                });
        }
    }

    /// Send an extrinsic to this node using an RPC call.
    pub(crate) async fn send_extrinsic(
        &mut self,
        call: impl Into<storage_hub_runtime::RuntimeCall>,
        tip: Tip,
    ) -> Result<RpcExtrinsicOutput> {
        debug!(target: LOG_TARGET, "Sending extrinsic to the runtime");

        // Get the nonce for the caller and increment it for the next transaction.
        // TODO: Handle nonce overflow.
        let nonce = self.nonce_counter;

        // Construct the extrinsic.
        let extrinsic = self.construct_extrinsic(self.client.clone(), call, nonce, tip);

        // Generate a unique ID for this query.
        let id_hash = Blake2Hasher::hash(&extrinsic.encode());
        // TODO: Consider storing the ID in a hashmap if later retrieval is needed.

        let (result, rx) = self
            .rpc_handlers
            .rpc_query(&format!(
                r#"{{
                    "jsonrpc": "2.0",
                    "method": "author_submitAndWatchExtrinsic",
                    "params": ["0x{}"],
                    "id": {:?}
                }}"#,
                array_bytes::bytes2hex("", &extrinsic.encode()),
                array_bytes::bytes2hex("", &id_hash.as_bytes())
            ))
            .await
            .expect("Sending query failed even when it is correctly formatted as JSON-RPC; qed");

        let json: serde_json::Value =
            serde_json::from_str(&result).expect("the result can only be a JSONRPC string; qed");
        let error = json
            .as_object()
            .expect("JSON result is always an object; qed")
            .get("error");

        if let Some(error) = error {
            // TODO: Consider how to handle a low nonce error, and retry.
            return Err(anyhow::anyhow!("Error in RPC call: {}", error.to_string()));
        }

        // Only update nonce after we are sure no errors
        // occurred submitting the extrinsic.
        self.nonce_counter += 1;

        Ok(RpcExtrinsicOutput {
            hash: id_hash,
            result,
            receiver: rx,
        })
    }

    /// Construct an extrinsic that can be applied to the runtime.
    pub fn construct_extrinsic(
        &self,
        client: Arc<ParachainClient>,
        function: impl Into<storage_hub_runtime::RuntimeCall>,
        nonce: u32,
        tip: Tip,
    ) -> UncheckedExtrinsic {
        let function = function.into();
        let current_block_hash = client.info().best_hash;
        let current_block = client.info().best_number.saturated_into();
        let genesis_block = client
            .hash(0)
            .expect("Failed to get genesis block hash, always present; qed")
            .expect("Genesis block hash should never not be on-chain; qed");
        let period = BlockHashCount::get()
            .checked_next_power_of_two()
            .map(|c| c / 2)
            .unwrap_or(2) as u64;
        let extra: SignedExtra = (
            frame_system::CheckNonZeroSender::<storage_hub_runtime::Runtime>::new(),
            frame_system::CheckSpecVersion::<storage_hub_runtime::Runtime>::new(),
            frame_system::CheckTxVersion::<storage_hub_runtime::Runtime>::new(),
            frame_system::CheckGenesis::<storage_hub_runtime::Runtime>::new(),
            frame_system::CheckEra::<storage_hub_runtime::Runtime>::from(generic::Era::mortal(
                period,
                current_block,
            )),
            frame_system::CheckNonce::<storage_hub_runtime::Runtime>::from(nonce),
            frame_system::CheckWeight::<storage_hub_runtime::Runtime>::new(),
            tip,
            cumulus_primitives_storage_weight_reclaim::StorageWeightReclaim::<
                storage_hub_runtime::Runtime,
            >::new(),
            frame_metadata_hash_extension::CheckMetadataHash::new(false),
        );

        let raw_payload = SignedPayload::from_raw(
            function.clone(),
            extra.clone(),
            (
                (),
                storage_hub_runtime::VERSION.spec_version,
                storage_hub_runtime::VERSION.transaction_version,
                genesis_block,
                current_block_hash,
                (),
                (),
                (),
                (),
                None,
            ),
        );

        let caller_pub_key = Self::caller_pub_key(self.keystore.clone());

        // Sign the payload.
        let signature = raw_payload
            .using_encoded(|e| self.keystore.sr25519_sign(BCSV_KEY_TYPE, &caller_pub_key, e))
            .expect("The payload is always valid and should be possible to sign; qed")
            .expect("They key type and public key are valid because we just extracted them from the keystore; qed");

        // Construct the extrinsic.
        UncheckedExtrinsic::new_signed(
            function.clone(),
            storage_hub_runtime::Address::Id(<sp_core::sr25519::Public as Into<
                storage_hub_runtime::AccountId,
            >>::into(caller_pub_key)),
            polkadot_primitives::Signature::Sr25519(signature),
            extra.clone(),
        )
    }

    // Getting signer public key.
    pub fn caller_pub_key(keystore: KeystorePtr) -> sp_core::sr25519::Public {
        let caller_pub_key = keystore.sr25519_public_keys(BCSV_KEY_TYPE).pop().expect(
            format!(
                "There should be at least one sr25519 key in the keystore with key type '{:?}' ; qed",
                BCSV_KEY_TYPE
            )
            .as_str(),
        );
        caller_pub_key
    }

    /// Get an extrinsic from a block.
    pub(crate) async fn get_extrinsic_from_block(
        &self,
        block_hash: H256,
        extrinsic_hash: H256,
    ) -> Result<Extrinsic> {
        // Get the block.
        let block = self
            .client
            .block(block_hash)
            .expect("Failed to get block. This shouldn't be possible for known existing block hash; qed")
            .expect("Block returned None for known existing block hash. This shouldn't be the case for a block known to have at least one transaction; qed");

        // Get the extrinsics.
        let extrinsics = block.block.extrinsics();

        // Find the extrinsic index in the block.
        let extrinsic_index = extrinsics
            .iter()
            .position(|e| {
                let hash = Blake2Hasher::hash(&e.encode());
                hash == extrinsic_hash
            })
            .expect("Extrinsic not found in block. This shouldn't be possible if we're looking into a block for which we got confirmation that the extrinsic was included; qed");

        // Get the events from storage.
        let events_in_block = get_events_at_block(&self.client, &block_hash)?;

        // Filter the events for the extrinsic.
        // Each event record is composed of the `phase`, `event` and `topics` fields.
        // We are interested in those events whose `phase` is equal to `ApplyExtrinsic` with the index of the extrinsic.
        // For more information see: https://polkadot.js.org/docs/api/cookbook/blocks/#how-do-i-map-extrinsics-to-their-events
        let events = events_in_block
            .into_iter()
            .filter(|ev| ev.phase == frame_system::Phase::ApplyExtrinsic(extrinsic_index as u32))
            .collect();

        // Construct the extrinsic.
        Ok(Extrinsic {
            hash: extrinsic_hash,
            block_hash,
            events,
        })
    }

    /// Unwatch an extrinsic.
    pub(crate) async fn unwatch_extrinsic(&self, subscription_id: Number) -> Result<String> {
        let (result, _rx) = self
            .rpc_handlers
            .rpc_query(&format!(
                r#"{{
                    "jsonrpc": "2.0",
                    "method": "author_unwatchExtrinsic",
                    "params": [{}],
                    "id": {}
                }}"#,
                subscription_id, subscription_id
            ))
            .await
            .expect("Sending query failed even when it is correctly formatted as JSON-RPC; qed");

        let json: serde_json::Value =
            serde_json::from_str(&result).expect("the result can only be a JSONRPC string; qed");
        let unwatch_result = json
            .as_object()
            .expect("JSON result is always an object; qed")
            .get("result");

        if let Some(unwatch_result) = unwatch_result {
            if unwatch_result
                .as_bool()
                .expect("Result is always a boolean; qed")
            {
                debug!(target: LOG_TARGET, "Extrinsic unwatched successfully");
            } else {
                return Err(anyhow::anyhow!("Failed to unwatch extrinsic"));
            }
        } else {
            return Err(anyhow::anyhow!("Failed to unwatch extrinsic"));
        }

        Ok(result)
    }

    /// Check if the challenges tick is one that this provider has to submit a proof for,
    /// and if so, return true.
    pub(crate) fn should_provider_submit_proof(
        &self,
        block_hash: &H256,
        provider_id: &ProviderId,
        current_tick: &BlockNumber,
    ) -> bool {
        // Get the last tick for which the BSP submitted a proof.
        let last_tick_provided = match self
            .client
            .runtime_api()
            .get_last_tick_provider_submitted_proof(*block_hash, provider_id)
        {
            Ok(last_tick_provided_result) => match last_tick_provided_result {
                Ok(last_tick_provided) => last_tick_provided,
                Err(e) => match e {
                    GetLastTickProviderSubmittedProofError::ProviderNotRegistered => {
                        debug!(target: LOG_TARGET, "Provider [{:?}] is not registered", provider_id);
                        return false;
                    }
                    GetLastTickProviderSubmittedProofError::ProviderNeverSubmittedProof => {
                        debug!(target: LOG_TARGET, "Provider [{:?}] does not have an initialised challenge cycle", provider_id);
                        return false;
                    }
                    GetLastTickProviderSubmittedProofError::InternalApiError => {
                        error!(target: LOG_TARGET, "This should be impossible, we just checked the API error. \nInternal API error while getting last tick Provider [{:?}] submitted a proof for: {:?}", provider_id, e);
                        return false;
                    }
                },
            },
            Err(e) => {
                error!(target: LOG_TARGET, "Runtime API error while getting last tick Provider [{:?}] submitted a proof for: {:?}", provider_id, e);
                return false;
            }
        };

        // Get the challenge period for the provider.
        let provider_challenge_period = match self
            .client
            .runtime_api()
            .get_challenge_period(*block_hash, provider_id)
        {
            Ok(provider_challenge_period_result) => match provider_challenge_period_result {
                Ok(provider_challenge_period) => provider_challenge_period,
                Err(e) => match e {
                    GetChallengePeriodError::ProviderNotRegistered => {
                        debug!(target: LOG_TARGET, "Provider [{:?}] is not registered", provider_id);
                        return false;
                    }
                    GetChallengePeriodError::InternalApiError => {
                        error!(target: LOG_TARGET, "This should be impossible, we just checked the API error. \nInternal API error while getting challenge period for Provider [{:?}]", provider_id);
                        return false;
                    }
                },
            },
            Err(e) => {
                error!(target: LOG_TARGET, "Runtime API error while getting challenge period for Provider [{:?}]: {:?}", provider_id, e);
                return false;
            }
        };

        // Check if the current tick is a tick this provider should submit a proof for.
        let current_tick_minus_last_submission = match current_tick.checked_sub(last_tick_provided)
        {
            Some(tick) => tick,
            None => {
                error!(target: LOG_TARGET, "CRITICAL❗️❗️ Current tick is smaller than the last tick this provider submitted a proof for. This should not happen. \nThis is a bug. Please report it to the StorageHub team.");
                return false;
            }
        };

        (current_tick_minus_last_submission % provider_challenge_period) == 0
    }

    /// Check if there are any pending requests to update the forest root on the runtime, and process them.
    /// Takes care of prioritizing requests, favouring `SubmitProofRequest` over `ConfirmStoringRequest` over `StopStoringForInsolventUserRequest`.
    /// This function is called every time a new block is imported and after each request is queued.
    pub(crate) fn check_pending_forest_root_writes(&mut self) {
        if let Some(mut rx) = self.forest_root_write_lock.take() {
            // Note: tasks that get ownership of the lock are responsible for sending a message back when done processing.
            match rx.try_recv() {
                // If the channel is empty, means we still need to wait for the current task to finish.
                Err(TryRecvError::Empty) => {
                    // If we have a task writing to the runtime, we don't want to start another one.
                    self.forest_root_write_lock = Some(rx);
                    trace!(target: LOG_TARGET, "Waiting for current forest root write task to finish");
                    return;
                }
                Ok(_) => {
                    trace!(target: LOG_TARGET, "Forest root write task finished, lock is released!");
                    let state_store_context = self.persistent_state.open_rw_context_with_overlay();
                    state_store_context
                        .access_value(&OngoingProcessConfirmStoringRequestCf)
                        .delete();
                    state_store_context
                        .access_value(&OngoingProcessMspRespondStorageRequestCf)
                        .delete();
                    state_store_context
                        .access_value(&OngoingProcessStopStoringForInsolventUserRequestCf)
                        .delete();
                    state_store_context.commit();
                }
                Err(TryRecvError::Closed) => {
                    error!(target: LOG_TARGET, "Forest root write task channel closed unexpectedly. Lock is released anyway!");
                    let state_store_context = self.persistent_state.open_rw_context_with_overlay();
                    state_store_context
                        .access_value(&OngoingProcessConfirmStoringRequestCf)
                        .delete();
                    state_store_context
                        .access_value(&OngoingProcessMspRespondStorageRequestCf)
                        .delete();
                    state_store_context
                        .access_value(&OngoingProcessStopStoringForInsolventUserRequestCf)
                        .delete();
                    state_store_context.commit();
                }
            }
        }

        // At this point we know that the lock is released and we can start processing new requests.
        let state_store_context = self.persistent_state.open_rw_context_with_overlay();
        let mut next_event_data = None;

        // If we have a submit proof request, prioritise it.
        // This is a BSP only operation, since MSPs don't have to submit proofs.
        while let Some(request) = self.pending_submit_proof_requests.pop_first() {
            // Check if the proof is still the next one to be submitted.
            let provider_id = request.provider_id;
            let next_challenge_tick = match self.get_next_challenge_tick_for_provider(&provider_id)
            {
                Ok(next_challenge_tick) => next_challenge_tick,
                Err(e) => {
                    error!(target: LOG_TARGET, "Failed to get next challenge tick for provider [{:?}]: {:?}", provider_id, e);

                    // If this is the case, no reason to continue to the next pending proof request.
                    // We can just break the loop.
                    break;
                }
            };

            // This is to avoid starting a new task if the proof is not the next one to be submitted.
            if next_challenge_tick != request.tick {
                // If the proof is not the next one to be submitted, we can remove it from the list of pending submit proof requests.
                trace!(target: LOG_TARGET, "Proof for tick [{:?}] is not the next one to be submitted. Removing it from the list of pending submit proof requests.", request.tick);
                self.pending_submit_proof_requests.remove(&request);

                // Continue to the next pending proof request.
                continue;
            }

            // If the proof is still the next one to be submitted, we can process it.
            trace!(target: LOG_TARGET, "Proof for tick [{:?}] is the next one to be submitted. Processing it.", request.tick);
            let current_forest_root = self.current_forest_roots.get(&provider_id).cloned();
            if current_forest_root.is_none() {
                error!(target: LOG_TARGET, "CRITICAL ❗️❗️ Current Forest root for Provider [{:?}] is not set. This should never happen. This is a bug. Please report it to the StorageHub team.", provider_id);

                // If this is the case, no reason to continue to the next pending proof request.
                // We can just break the loop.
                break;
            }

            next_event_data = Some(ForestWriteLockTaskData::SubmitProofRequest(
                ProcessSubmitProofRequestData {
                    seed: request.seed,
                    provider_id: request.provider_id,
                    tick: request.tick,
                    forest_challenges: request.forest_challenges,
                    checkpoint_challenges: request.checkpoint_challenges,
                    current_forest_root: current_forest_root
                        .expect("We just checked that it's Some; qed"),
                },
            ));

            // Exit the loop since we have found the next proof to be submitted.
            break;
        }

        // If we have no pending submit proof requests, we can also check for pending confirm storing requests.
        // This is a BSP only operation, since MSPs don't have to confirm storing.
        if next_event_data.is_none() {
            let max_batch_confirm =
                <<Runtime as pallet_file_system::Config>::MaxBatchConfirmStorageRequests as Get<
                    u32,
                >>::get();

            // Batch multiple confirm file storing taking the runtime maximum.
            let mut confirm_storing_requests = Vec::new();
            for _ in 0..max_batch_confirm {
                if let Some(request) = state_store_context
                    .pending_confirm_storing_request_deque()
                    .pop_front()
                {
                    trace!(target: LOG_TARGET, "Processing confirm storing request for file [{:?}]", request.file_key);
                    confirm_storing_requests.push(request);
                } else {
                    break;
                }
            }

            // If we have at least 1 confirm storing request, send the process event.
            if confirm_storing_requests.len() > 0 {
                next_event_data = Some(
                    ProcessConfirmStoringRequestData {
                        confirm_storing_requests,
                    }
                    .into(),
                );
            }
        }

        // If we have no pending submit proof requests nor pending confirm storing requests, we can also check for pending respond storing requests.
        // This is a MSP only operation, since BSPs don't have to respond to storage requests, they volunteer and confirm.
        if next_event_data.is_none() {
            let max_batch_respond: u32 = MaxBatchMspRespondStorageRequests::get();

            // Batch multiple respond storing requests up to the runtime configured maximum.
            let mut respond_storage_requests = Vec::new();
            for _ in 0..max_batch_respond {
                if let Some(request) = state_store_context
                    .pending_msp_respond_storage_request_deque()
                    .pop_front()
                {
                    respond_storage_requests.push(request);
                } else {
                    break;
                }
            }

            // If we have at least 1 respond storing request, send the process event.
            if respond_storage_requests.len() > 0 {
                next_event_data = Some(
                    ProcessMspRespondStoringRequestData {
                        respond_storing_requests: respond_storage_requests,
                    }
                    .into(),
                );
            }
        }

        // If we have no pending storage requests to respond to, we can also check for pending stop storing for insolvent user requests.
        if next_event_data.is_none() {
            if let Some(request) = state_store_context
                .pending_stop_storing_for_insolvent_user_request_deque()
                .pop_front()
            {
                next_event_data = Some(
                    ProcessStopStoringForInsolventUserRequestData { who: request.user }.into(),
                );
            }
        }
        state_store_context.commit();

        if let Some(event_data) = next_event_data {
            self.emit_forest_write_event(event_data);
        }
    }

    pub(crate) fn emit_forest_write_event(&mut self, data: impl Into<ForestWriteLockTaskData>) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.forest_root_write_lock = Some(rx);

        let data = data.into();

        // If this is a confirm storing request, respond storage request, or a stop storing for insolvent user request, we need to store it in the state store.
        match &data {
            ForestWriteLockTaskData::ConfirmStoringRequest(data) => {
                let state_store_context = self.persistent_state.open_rw_context_with_overlay();
                state_store_context
                    .access_value(&OngoingProcessConfirmStoringRequestCf)
                    .write(data);
                state_store_context.commit();
            }
            ForestWriteLockTaskData::MspRespondStorageRequest(data) => {
                let state_store_context = self.persistent_state.open_rw_context_with_overlay();
                state_store_context
                    .access_value(&OngoingProcessMspRespondStorageRequestCf)
                    .write(data);
                state_store_context.commit();
            }
            ForestWriteLockTaskData::StopStoringForInsolventUserRequest(data) => {
                let state_store_context = self.persistent_state.open_rw_context_with_overlay();
                state_store_context
                    .access_value(&OngoingProcessStopStoringForInsolventUserRequestCf)
                    .write(data);
                state_store_context.commit();
            }
            _ => {}
        }

        // This is an [`Arc<Mutex<Option<T>>>`] (in this case [`oneshot::Sender<()>`]) instead of just
        // T so that we can keep using the current actors event bus (emit) which requires Clone on the
        // event. Clone is required because there is no constraint on the number of listeners that can
        // subscribe to the event (and each is guaranteed to receive all emitted events).
        let forest_root_write_tx = Arc::new(Mutex::new(Some(tx)));
        match data.into() {
            ForestWriteLockTaskData::SubmitProofRequest(data) => {
                self.emit(ProcessSubmitProofRequest {
                    data,
                    forest_root_write_tx,
                });
            }
            ForestWriteLockTaskData::ConfirmStoringRequest(data) => {
                self.emit(ProcessConfirmStoringRequest {
                    data,
                    forest_root_write_tx,
                });
            }
            ForestWriteLockTaskData::MspRespondStorageRequest(data) => {
                self.emit(ProcessMspRespondStoringRequest {
                    data,
                    forest_root_write_tx,
                });
            }
            ForestWriteLockTaskData::StopStoringForInsolventUserRequest(data) => {
                self.emit(ProcessStopStoringForInsolventUserRequest {
                    data,
                    forest_root_write_tx,
                });
            }
        }
    }

    /// Emits a `MultipleNewChallengeSeeds` event with all the pending proof submissions for this provider.
    /// This is used to catch up to the latest proof submissions that were missed due to a node restart.
    /// Also, it can help to catch up to proofs in case there is a change in the BSP's stake (therefore
    /// also a change in it's challenge period).
    ///
    /// IMPORTANT: This function takes into account whether a proof should be submitted for the current tick.
    pub(crate) fn proof_submission_catch_up(
        &self,
        current_block_hash: &H256,
        provider_id: &ProviderId,
    ) {
        // Get the last tick for which the BSP submitted a proof, according to the runtime right now.
        let last_tick_provider_submitted_proof = match self
            .client
            .runtime_api()
            .get_last_tick_provider_submitted_proof(*current_block_hash, provider_id)
        {
            Ok(last_tick_provided_result) => match last_tick_provided_result {
                Ok(last_tick_provided) => last_tick_provided,
                Err(e) => match e {
                    GetLastTickProviderSubmittedProofError::ProviderNotRegistered => {
                        debug!(target: LOG_TARGET, "Provider [{:?}] is not registered", provider_id);
                        return;
                    }
                    GetLastTickProviderSubmittedProofError::ProviderNeverSubmittedProof => {
                        debug!(target: LOG_TARGET, "Provider [{:?}] does not have an initialised challenge cycle", provider_id);
                        return;
                    }
                    GetLastTickProviderSubmittedProofError::InternalApiError => {
                        error!(target: LOG_TARGET, "This should be impossible, we just checked the API error. \nInternal API error while getting last tick Provider [{:?}] submitted a proof for: {:?}", provider_id, e);
                        return;
                    }
                },
            },
            Err(e) => {
                error!(target: LOG_TARGET, "Runtime API error while getting last tick Provider [{:?}] submitted a proof for: {:?}", provider_id, e);
                return;
            }
        };
        trace!(target: LOG_TARGET, "Last tick Provider [{:?}] submitted a proof for: {}", provider_id, last_tick_provider_submitted_proof);

        // Get the current challenge period for this provider.
        let challenge_period = match self
            .client
            .runtime_api()
            .get_challenge_period(*current_block_hash, provider_id)
        {
            Ok(challenge_period_result) => match challenge_period_result {
                Ok(challenge_period) => challenge_period,
                Err(e) => match e {
                    GetChallengePeriodError::ProviderNotRegistered => {
                        debug!(target: LOG_TARGET, "Provider [{:?}] is not registered", provider_id);
                        return;
                    }
                    GetChallengePeriodError::InternalApiError => {
                        error!(target: LOG_TARGET, "This should be impossible, we just checked the API error. \nInternal API error while getting challenge period for Provider [{:?}]", provider_id);
                        return;
                    }
                },
            },
            Err(e) => {
                error!(target: LOG_TARGET, "Runtime API error while getting challenge period for Provider [{:?}]: {:?}", provider_id, e);
                return;
            }
        };

        // Get the current tick.
        let current_tick = match self
            .client
            .runtime_api()
            .get_current_tick(*current_block_hash)
        {
            Ok(current_tick) => current_tick,
            Err(e) => {
                error!(target: LOG_TARGET, "Runtime API error while getting current tick for Provider [{:?}]: {:?}", provider_id, e);
                return;
            }
        };

        // Advance by `challenge_period` ticks and add the seed to the list of challenge seeds.
        let mut challenge_seeds = Vec::new();
        let mut next_challenge_tick = last_tick_provider_submitted_proof + challenge_period;
        while next_challenge_tick <= current_tick {
            // Get the seed for the challenge tick.
            let seed = match self
                .client
                .runtime_api()
                .get_challenge_seed(*current_block_hash, next_challenge_tick)
            {
                Ok(seed_result) => match seed_result {
                    Ok(seed) => seed,
                    Err(e) => match e {
                        GetChallengeSeedError::TickBeyondLastSeedStored => {
                            error!(target: LOG_TARGET, "CRITICAL❗️❗️ Tick [{:?}] is beyond last seed stored and this provider needs to submit a proof for it.", next_challenge_tick);
                            return;
                        }
                        GetChallengeSeedError::TickIsInTheFuture => {
                            error!(target: LOG_TARGET, "CRITICAL❗️❗️ Tick [{:?}] is in the future. This should never happen. \nThis is a bug. Please report it to the StorageHub team.", next_challenge_tick);
                            return;
                        }
                        GetChallengeSeedError::InternalApiError => {
                            error!(target: LOG_TARGET, "This should be impossible, we just checked the API error. \nInternal API error while getting challenge seed for challenge tick [{:?}]: {:?}", next_challenge_tick, e);
                            return;
                        }
                    },
                },
                Err(e) => {
                    error!(target: LOG_TARGET, "Runtime API error while getting challenges from seed for challenge tick [{:?}]: {:?}", next_challenge_tick, e);
                    return;
                }
            };
            challenge_seeds.push((next_challenge_tick, seed));
            next_challenge_tick += challenge_period;
        }

        // Emit the `MultiNewChallengeSeeds` event.
        if challenge_seeds.len() > 0 {
            trace!(target: LOG_TARGET, "Emitting MultipleNewChallengeSeeds event for provider [{:?}] with challenge seeds: {:?}", provider_id, challenge_seeds);
            self.emit(MultipleNewChallengeSeeds {
                provider_id: *provider_id,
                seeds: challenge_seeds,
            });
        }
    }

    pub(crate) fn get_next_challenge_tick_for_provider(
        &self,
        provider_id: &ProviderId,
    ) -> Result<BlockNumber> {
        // Get the current block hash.
        let current_block_hash = self.client.info().best_hash;

        // Get the last tick for which the provider submitted a proof.
        let last_tick_provider_submitted_proof = match self
            .client
            .runtime_api()
            .get_last_tick_provider_submitted_proof(current_block_hash, provider_id)
        {
            Ok(last_tick_provided_result) => match last_tick_provided_result {
                Ok(last_tick_provided) => last_tick_provided,
                Err(e) => match e {
                    GetLastTickProviderSubmittedProofError::ProviderNotRegistered => {
                        return Err(anyhow!("Provider [{:?}] is not registered", provider_id));
                    }
                    GetLastTickProviderSubmittedProofError::ProviderNeverSubmittedProof => {
                        return Err(anyhow!(
                            "Provider [{:?}] does not have an initialised challenge cycle",
                            provider_id
                        ));
                    }
                    GetLastTickProviderSubmittedProofError::InternalApiError => {
                        return Err(anyhow!(
                            "Internal API error while getting last tick Provider [{:?}] submitted a proof for: {:?}",
                            provider_id, e
                        ));
                    }
                },
            },
            Err(e) => {
                return Err(anyhow!(
                    "Runtime API error while getting last tick Provider [{:?}] submitted a proof for: {:?}",
                    provider_id,
                    e
                ));
            }
        };

        // Get the challenge period for the provider.
        let challenge_period = match self
            .client
            .runtime_api()
            .get_challenge_period(current_block_hash, provider_id)
        {
            Ok(challenge_period_result) => match challenge_period_result {
                Ok(challenge_period) => challenge_period,
                Err(e) => match e {
                    GetChallengePeriodError::ProviderNotRegistered => {
                        return Err(anyhow!("Provider [{:?}] is not registered", provider_id));
                    }
                    GetChallengePeriodError::InternalApiError => {
                        return Err(anyhow!(
                            "Internal API error while getting challenge period for Provider [{:?}]",
                            provider_id
                        ));
                    }
                },
            },
            Err(e) => {
                return Err(anyhow!(
                    "Runtime API error while getting challenge period for Provider [{:?}]: {:?}",
                    provider_id,
                    e
                ));
            }
        };

        // Calculate the next challenge tick.
        let next_challenge_tick = last_tick_provider_submitted_proof + challenge_period;

        // Check if the current tick is a tick this provider should submit a proof for.
        Ok(next_challenge_tick)
    }

    pub(crate) fn check_for_notify(&self, block_number: &BlockNumber) {
        if let Some(np) = self.notify_period {
            if block_number % np == 0 {
                self.emit(NotifyPeriod {});
            }
        }
    }
}

/// The output of an RPC transaction.
pub struct RpcExtrinsicOutput {
    /// Hash of the extrinsic.
    pub hash: H256,
    /// The output string of the transaction if any.
    pub result: String,
    /// An async receiver if data will be returned via a callback.
    pub receiver: tokio::sync::mpsc::Receiver<String>,
}

impl std::fmt::Debug for RpcExtrinsicOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "RpcExtrinsicOutput {{ hash: {:?}, result: {:?}, receiver }}",
            self.hash, self.result
        )
    }
}
