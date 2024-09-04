use std::sync::Arc;

use anyhow::Result;
use codec::{Decode, Encode};
use cumulus_primitives_core::BlockT;
use frame_support::{StorageHasher, Twox128};
use lazy_static::lazy_static;
use log::{debug, error, info, trace, warn};
use pallet_proofs_dealer_runtime_api::{
    GetChallengePeriodError, GetChallengeSeedError, GetLastTickProviderSubmittedProofError,
    ProofsDealerApi,
};
use pallet_storage_providers::types::StorageProviderId;
use pallet_storage_providers_runtime_api::StorageProvidersApi;
use polkadot_runtime_common::BlockHashCount;
use sc_client_api::{BlockBackend, HeaderBackend, StorageKey, StorageProvider};
use serde_json::Number;
use shc_actors_framework::actor::Actor;
use shc_common::types::{BlockNumber, ParachainClient, ProviderId, BCSV_KEY_TYPE};
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
        ForestWriteLockTaskData, MultipleNewChallengeSeeds, ProcessConfirmStoringRequest,
        ProcessConfirmStoringRequestData, ProcessSubmitProofRequest, ProcessSubmitProofRequestData,
    },
    handler::LOG_TARGET,
    state::{LastProcessedBlockNumberCf, OngoingForestWriteLockTaskDataCf},
    typed_store::{CFDequeAPI, ProvidesTypedDbSingleAccess},
    types::{EventsVec, Extrinsic},
    BlockchainService,
};

lazy_static! {
    // Would be cool to be able to do this...
    // let events_storage_key = frame_system::Events::<storage_hub_runtime::Runtime>::hashed_key();

    // Static and lazily initialised `events_storage_key`
    static ref EVENTS_STORAGE_KEY: Vec<u8> = {
        let key = [
            Twox128::hash(b"System").to_vec(),
            Twox128::hash(b"Events").to_vec(),
        ]
        .concat();
        key
    };
}

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
                            // TODO: For now, we only care about BSPs.
                            StorageProviderId::MainStorageProvider(_msp_id) => {}
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
    ) -> Result<RpcExtrinsicOutput> {
        debug!(target: LOG_TARGET, "Sending extrinsic to the runtime");

        // Get the nonce for the caller and increment it for the next transaction.
        // TODO: Handle nonce overflow.
        let nonce = self.nonce_counter;

        // Construct the extrinsic.
        let extrinsic = self.construct_extrinsic(self.client.clone(), call, nonce);

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
        // TODO: Consider tipping the transaction.
        let tip = 0;
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
        pallet_transaction_payment::ChargeTransactionPayment::<storage_hub_runtime::Runtime>::from(
            tip,
        ),
        cumulus_primitives_storage_weight_reclaim::StorageWeightReclaim::<
            storage_hub_runtime::Runtime,
        >::new(),
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
            polkadot_primitives::Signature::Sr25519(signature.clone()),
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
        let events_in_block = self.get_events_storage_element(&block_hash)?;

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

    /// Get the events storage element in a block.
    pub(crate) fn get_events_storage_element(&self, block_hash: &H256) -> Result<EventsVec> {
        // Get the events storage.
        let raw_storage_opt = self
            .client
            .storage(*block_hash, &StorageKey(EVENTS_STORAGE_KEY.clone()))
            .expect("Failed to get Events storage element");

        // Decode the events storage.
        if let Some(raw_storage) = raw_storage_opt {
            let block_events = EventsVec::decode(&mut raw_storage.0.as_slice())
                .expect("Failed to decode Events storage element");

            return Ok(block_events);
        } else {
            return Err(anyhow::anyhow!("Failed to get Events storage element"));
        }
    }

    /// Check if the challenges tick is one that this provider has to submit a proof for,
    /// and if so, return true.
    pub(crate) fn should_provider_submit_proof(
        &self,
        block_hash: &H256,
        provider_id: &ProviderId,
        current_tick: &BlockNumber,
    ) -> bool {
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
                },
            },
            Err(e) => {
                error!(target: LOG_TARGET, "Runtime API error while getting challenge period for Provider [{:?}]: {:?}", provider_id, e);
                return false;
            }
        };
        current_tick == &last_tick_provided.saturating_add(provider_challenge_period)
    }

    /// Check if there are any pending requests to update the forest root on the runtime, and process them.
    /// Takes care of prioritizing requests, favouring `SubmitProofRequest` over `ConfirmStoringRequest`.
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
                        .access_value(&OngoingForestWriteLockTaskDataCf)
                        .delete();
                    state_store_context.commit();
                }
                Err(TryRecvError::Closed) => {
                    error!(target: LOG_TARGET, "Forest root write task channel closed unexpectedly. Lock is released anyway!");
                    let state_store_context = self.persistent_state.open_rw_context_with_overlay();
                    state_store_context
                        .access_value(&OngoingForestWriteLockTaskDataCf)
                        .delete();
                    state_store_context.commit();
                }
            }
        }

        // At this point we know that the lock is released and we can start processing new requests.
        let state_store_context = self.persistent_state.open_rw_context_with_overlay();
        let mut next_event_data = None;

        // If we have a submit proof request, prioritize it.
        if let Some(request) = state_store_context
            .pending_submit_proof_request_deque()
            .pop_front()
        {
            next_event_data = Some(ForestWriteLockTaskData::SubmitProofRequest(
                ProcessSubmitProofRequestData {
                    seed: request.seed,
                    provider_id: request.provider_id,
                    tick: request.tick,
                    forest_challenges: request.forest_challenges,
                    checkpoint_challenges: request.checkpoint_challenges,
                },
            ));
        } else {
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
        if let Some(event_data) = &next_event_data {
            state_store_context
                .access_value(&OngoingForestWriteLockTaskDataCf)
                .write(event_data);
        }
        state_store_context.commit();

        if let Some(event_data) = next_event_data {
            self.emit_forest_write_event(event_data);
        }
    }

    pub(crate) fn emit_forest_write_event(&mut self, data: impl Into<ForestWriteLockTaskData>) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.forest_root_write_lock = Some(rx);

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
        }
    }

    /// Emits `NewChallengeSeed` events for all the pending proof submissions for this provider.
    /// This is used to catch up to the latest proof submissions that were missed due to a node restart.
    /// Also, it can help to catch up to proofs in case there is a change in the BSP's stake (therefore
    /// also a change in it's challenge period).
    #[allow(dead_code)] // TODO: Remove this when finally used.
    pub(crate) fn proof_submission_catch_up(
        &mut self,
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

        // Advance by `challenge_period` ticks and generate `NewChallengeSeed` events for the provider.
        let mut challenge_seeds = Vec::new();
        let mut next_challenge_tick = last_tick_provider_submitted_proof + challenge_period;
        while next_challenge_tick < current_tick {
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
                            debug!(target: LOG_TARGET, "CRITICAL❗️❗️ Tick [{:?}] is in the future. This should never happen. \nThis is a bug. Please report it to the StorageHub team.", next_challenge_tick);
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
        self.emit(MultipleNewChallengeSeeds {
            provider_id: *provider_id,
            seeds: challenge_seeds,
        });
    }

    // TODO: Reconsider how to use this for catching up to unsubmitted storage proofs.
    pub async fn catch_up_block_import(&mut self, current_block_number: &BlockNumber) {
        let state_store_context = self.persistent_state.open_rw_context_with_overlay();
        let latest_processed_block_number = match state_store_context
            .access_value(&LastProcessedBlockNumberCf)
            .read()
        {
            Some(block_number) => block_number,
            None => {
                info!(target: LOG_TARGET, "No last processed block number found in the state store, skipping catch-up.");

                return;
            }
        };
        drop(state_store_context);

        info!(target: LOG_TARGET, "Catching up from block #{} to block #{}", latest_processed_block_number, current_block_number);

        for block_number in latest_processed_block_number..=*current_block_number {
            let block_hash = match self.client.hash(block_number.into()) {
                Ok(Some(hash)) => hash,
                Ok(None) => {
                    error!(target: LOG_TARGET, "Block #{} not found.", block_number);
                    continue;
                }
                Err(e) => {
                    error!(target: LOG_TARGET, "Error fetching block hash for block #{}: {:?}", block_number, e);
                    continue;
                }
            };

            self.process_block_import(&block_hash, &block_number).await;
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
