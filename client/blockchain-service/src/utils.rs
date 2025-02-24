use std::{cmp::max, sync::Arc, vec};

use anyhow::{anyhow, Result};
use codec::{Decode, Encode};
use cumulus_primitives_core::BlockT;
use pallet_file_system_runtime_api::FileSystemApi;
use pallet_proofs_dealer_runtime_api::{
    GetChallengePeriodError, GetChallengeSeedError, GetProofSubmissionRecordError, ProofsDealerApi,
};
use pallet_storage_providers_runtime_api::StorageProvidersApi;
use polkadot_runtime_common::BlockHashCount;
use sc_client_api::{BlockBackend, BlockImportNotification, HeaderBackend};
use sc_tracing::tracing::{debug, error, info, trace, warn};
use serde_json::Number;
use shc_actors_framework::actor::Actor;
use shc_common::{
    blockchain_utils::get_events_at_block,
    consts::CURRENT_FOREST_KEY,
    types::{
        BlockNumber, ForestRoot, ParachainClient, ProofsDealerProviderId, StorageProviderId,
        TrieAddMutation, TrieMutation, TrieRemoveMutation, BCSV_KEY_TYPE,
    },
};
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use shp_file_metadata::FileMetadata;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::{HashAndNumber, TreeRoute};
use sp_core::{Blake2Hasher, Get, Hasher, H256};
use sp_keystore::KeystorePtr;
use sp_runtime::{
    generic::{self, SignedPayload},
    traits::Zero,
    SaturatedConversion,
};
use storage_hub_runtime::{Runtime, RuntimeEvent, SignedExtra, UncheckedExtrinsic};
use substrate_frame_rpc_system::AccountNonceApi;
use tokio::sync::{oneshot::error::TryRecvError, Mutex};

use crate::{
    events::{
        ForestWriteLockTaskData, MultipleNewChallengeSeeds, NotifyPeriod,
        ProcessConfirmStoringRequest, ProcessConfirmStoringRequestData, ProcessFileDeletionRequest,
        ProcessFileDeletionRequestData, ProcessMspRespondStoringRequest,
        ProcessMspRespondStoringRequestData, ProcessStopStoringForInsolventUserRequest,
        ProcessStopStoringForInsolventUserRequestData, ProcessSubmitProofRequest,
        ProcessSubmitProofRequestData,
    },
    handler::{LOG_TARGET, MAX_BLOCKS_BEHIND_TO_CATCH_UP_ROOT_CHANGES},
    state::{
        OngoingProcessConfirmStoringRequestCf, OngoingProcessFileDeletionRequestCf,
        OngoingProcessMspRespondStorageRequestCf,
        OngoingProcessStopStoringForInsolventUserRequestCf,
    },
    typed_store::{CFDequeAPI, ProvidesTypedDbSingleAccess},
    types::{Extrinsic, MinimalBlockInfo, NewBlockNotificationKind, SendExtrinsicOptions, Tip},
    BlockchainService,
};

// TODO: Make this configurable in the config file
const MAX_BATCH_MSP_RESPOND_STORE_REQUESTS: u32 = 100;

impl<FSH> BlockchainService<FSH>
where
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
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

    /// From a [`BlockImportNotification`], gets the imported block, and checks if:
    /// 1. The block is not the new best block. For example, it could be a block from a non-best fork branch.
    ///     - If so, it returns [`NewBlockNotificationKind::NewNonBestBlock`].
    /// 2. The block is the new best block, and its parent is the previous best block.
    ///     - If so, it registers it as the new best block and returns [`NewBlockNotificationKind::NewBestBlock`].
    /// 3. The block is the new best block, and its parent is NOT the previous best block (i.e. it's a reorg).
    ///     - If so, it registers it as the new best block and returns [`NewBlockNotificationKind::Reorg`].
    pub(crate) fn register_best_block_and_check_reorg<Block>(
        &mut self,
        block_import_notification: &BlockImportNotification<Block>,
    ) -> NewBlockNotificationKind<Block>
    where
        Block: cumulus_primitives_core::BlockT<Hash = H256>,
    {
        let last_best_block = self.best_block;
        let new_block_info: MinimalBlockInfo = block_import_notification.into();

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
            // Construct the tree route from the last best block processed and the new best block.
            // Fetch the parents of the new best block until:
            // - We reach the genesis block, or
            // - The size of the route is equal to `MAX_BLOCKS_BEHIND_TO_CATCH_UP_ROOT_CHANGES`, or
            // - The parent block is not found, or
            // - We reach the last best block processed.
            let mut route = vec![new_block_info.into()];
            let mut last_block_added = new_block_info;
            loop {
                // Check if we are at the genesis block.
                if last_block_added.number == BlockNumber::zero() {
                    trace!(target: LOG_TARGET, "Reached genesis block while building tree route for new best block");
                    break;
                }

                // Check if the route reached the maximum number of blocks to catch up on.
                if route.len() == MAX_BLOCKS_BEHIND_TO_CATCH_UP_ROOT_CHANGES as usize {
                    trace!(target: LOG_TARGET, "Reached maximum blocks to catch up on while building tree route for new best block");
                    break;
                }

                // Get the parent block.
                let parent_hash = match self.client.header(last_block_added.hash) {
                    Ok(Some(header)) => header.parent_hash,
                    Ok(None) => {
                        error!(target: LOG_TARGET, "Parent block hash not found for block {:?}", last_block_added.hash);
                        break;
                    }
                    Err(e) => {
                        error!(target: LOG_TARGET, "Failed to get header for block {:?}: {:?}", last_block_added.hash, e);
                        break;
                    }
                };
                let parent_block = match self.client.block(parent_hash) {
                    Ok(Some(block)) => block,
                    Ok(None) => {
                        error!(target: LOG_TARGET, "Block not found for block hash {:?}", parent_hash);
                        break;
                    }
                    Err(e) => {
                        error!(target: LOG_TARGET, "Failed to get block for block hash {:?}: {:?}", parent_hash, e);
                        break;
                    }
                };
                let parent_block_info = MinimalBlockInfo {
                    number: parent_block.block.header.number,
                    hash: parent_block.block.hash(),
                };

                // Check if we reached the last best block processed.
                if parent_block_info.hash == last_best_block.hash {
                    trace!(target: LOG_TARGET, "Reached last best block processed while building tree route for new best block");
                    break;
                }

                // Add the parent block to the route.
                route.push(parent_block_info.into());

                // Update last block added.
                last_block_added = parent_block_info;
            }

            // The first element in the route is the last best block processed, which will also be the
            // `pivot`, so it will be ignored when processing the `tree_route`.
            route.push(last_best_block.into());

            // Revert the route so that it is in ascending order of blocks, from the last best block processed up to the new imported best block.
            route.reverse();

            // Build the tree route.
            let tree_route = TreeRoute::new(route, 0).expect(
                "Tree route with pivot at 0 index and a route with at least 2 elements should be valid; qed",
            );

            return NewBlockNotificationKind::NewBestBlock {
                last_best_block_processed: last_best_block,
                new_best_block: new_block_info,
                tree_route,
            };
        }

        // At this point we know that the new block is the new best block, and that it also caused a reorg.
        let tree_route = block_import_notification
            .tree_route
            .as_ref()
            .expect("Tree route should exist, it was just checked to be `Some`; qed")
            .clone();

        // Add the new best block to the tree route, so that it is also processed as part of the reorg.
        let retracted = tree_route.retracted();
        let common_block = tree_route.common_block().clone();
        let enacted = tree_route.enacted();
        let modified_route = retracted
            .into_iter()
            .chain(std::iter::once(&common_block))
            .chain(enacted)
            .chain(std::iter::once(&new_block_info.into()))
            .cloned()
            .collect();

        let tree_route = TreeRoute::new(modified_route, retracted.len()).expect(
            "Tree route with one additional block to the enacted chain should be valid; qed",
        );
        info!(target: LOG_TARGET, "🔀 New best block caused a reorg: {:?}", new_block_info);
        info!(target: LOG_TARGET, "⛓️ Tree route: {:?}", tree_route);
        NewBlockNotificationKind::Reorg {
            old_best_block: last_best_block,
            new_best_block: new_block_info,
            tree_route,
        }
    }

    /// Get the current account nonce on-chain.
    pub(crate) fn account_nonce(&mut self, block_hash: &H256) -> u32 {
        let pub_key = Self::caller_pub_key(self.keystore.clone());
        self.client
            .runtime_api()
            .account_nonce(*block_hash, pub_key.into())
            .expect("Fetching account nonce works; qed")
    }

    /// Checks if the account nonce on-chain is higher than the nonce in the [`BlockchainService`].
    ///
    /// If the nonce is higher, the `nonce_counter` is updated in the [`BlockchainService`].
    pub(crate) fn sync_nonce(&mut self, block_hash: &H256) {
        let latest_nonce = self.account_nonce(block_hash);
        if latest_nonce > self.nonce_counter {
            self.nonce_counter = latest_nonce
        }
    }

    /// Get the Provider ID linked to the [`BCSV_KEY_TYPE`] key in this node's keystore.
    ///
    /// IMPORTANT! If there is more than one [`BCSV_KEY_TYPE`] key in this node's keystore, linked to
    /// different Provider IDs, this function will panic. In other words, this node doesn't support
    /// managing multiple Providers at once.
    pub(crate) fn get_provider_id(&mut self, block_hash: &H256) {
        let mut provider_ids_found = Vec::new();
        for key in self.keystore.sr25519_public_keys(BCSV_KEY_TYPE) {
            let maybe_provider_id = match self
                .client
                .runtime_api()
                .get_storage_provider_id(*block_hash, &key.into())
            {
                Ok(provider_id) => provider_id,
                Err(e) => {
                    error!(target: LOG_TARGET, "Runtime API error while getting Provider ID for key: {:?}. Error: {:?}", key, e);
                    continue;
                }
            };

            match maybe_provider_id {
                Some(provider_id) => {
                    provider_ids_found.push(provider_id);
                }
                None => {
                    debug!(target: LOG_TARGET, "There is no Provider ID for key: {:?}. This means that the node has a BCSV key in the keystore for which there is no Provider ID.", key);
                }
            };
        }

        // Case: There is no Provider ID linked to any of the [`BCSV_KEY_TYPE`] keys in this node's keystore.
        // This is expected, if this node starts up before the Provider has been registered.
        if provider_ids_found.is_empty() {
            warn!(target: LOG_TARGET, "🔑 There is no Provider ID linked to any of the BCSV keys in this node's keystore. This is expected, if this node starts up before the BSP has been registered.");
            return;
        }

        // Case: There is more than one Provider ID linked to any of the [`BCSV_KEY_TYPE`] keys in this node's keystore.
        // This is unexpected, and should never happen.
        if provider_ids_found.len() > 1 {
            panic!("There are more than one BCSV keys linked to Provider IDs in this node's keystore. Managing multiple Providers at once is not supported.");
        }

        // Case: There is exactly one Provider ID linked to any of the [`BCSV_KEY_TYPE`] keys in this node's keystore.
        let provider_id = *provider_ids_found.get(0).expect("There is exactly one Provider ID linked to any of the BCSV keys in this node's keystore; qed");
        self.provider_id = Some(provider_id);
    }

    /// Send an extrinsic to this node using an RPC call.
    ///
    /// Passing a specific `nonce` will be used to construct the extrinsic if it is higher than the current on-chain nonce.
    /// Otherwise, the current on-chain nonce will be used.
    /// Passing `None` for the `nonce` will use the [`nonce_counter`](BlockchainService::nonce_counter) as the nonce while still
    /// checking that the on-chain nonce is not lower.
    pub(crate) async fn send_extrinsic(
        &mut self,
        call: impl Into<storage_hub_runtime::RuntimeCall>,
        options: SendExtrinsicOptions,
    ) -> Result<RpcExtrinsicOutput> {
        debug!(target: LOG_TARGET, "Sending extrinsic to the runtime");

        let block_hash = self.client.info().best_hash;

        // Use the highest valid nonce.
        let nonce = max(
            options.nonce().unwrap_or(self.nonce_counter),
            self.account_nonce(&block_hash),
        );

        // Construct the extrinsic.
        let extrinsic = self.construct_extrinsic(self.client.clone(), call, nonce, options.tip());

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

        // TODO: Handle nonce overflow.
        // Only update nonce after we are sure no errors
        // occurred submitting the extrinsic.
        self.nonce_counter = nonce + 1;

        Ok(RpcExtrinsicOutput {
            hash: id_hash,
            result,
            receiver: rx,
            nonce,
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
        let maybe_block = self.client.block(block_hash).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to get block. Error: {:?}", e);
            anyhow!("Failed to get block. Error: {:?}", e)
        })?;
        let block = maybe_block.ok_or_else(|| {
            error!(target: LOG_TARGET, "Block returned None, i.e. block not found");
            anyhow!("Block returned None, i.e. block not found")
        })?;

        // Get the extrinsics.
        let extrinsics = block.block.extrinsics();

        // Find the extrinsic index in the block.
        let extrinsic_index = extrinsics
            .iter()
            .position(|e| {
                let hash = Blake2Hasher::hash(&e.encode());
                hash == extrinsic_hash
            })
            .ok_or_else(|| {
                error!(target: LOG_TARGET, "Extrinsic with hash {:?} not found in block", extrinsic_hash);
                anyhow!("Extrinsic not found in block")
            })?;

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
        provider_id: &ProofsDealerProviderId,
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
                    GetProofSubmissionRecordError::ProviderNotRegistered => {
                        debug!(target: LOG_TARGET, "Provider [{:?}] is not registered", provider_id);
                        return false;
                    }
                    GetProofSubmissionRecordError::ProviderNeverSubmittedProof => {
                        debug!(target: LOG_TARGET, "Provider [{:?}] does not have an initialised challenge cycle", provider_id);
                        return false;
                    }
                    GetProofSubmissionRecordError::InternalApiError => {
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

    /// Check if there are any pending requests to update the Forest root on the runtime, and process them.
    ///
    /// If this node is managing a BSP, the priority is given by:
    /// 1. `SubmitProofRequest` over...
    /// 2. `ConfirmStoringRequest`.
    ///
    /// If this node is managing a MSP, the priority is given by:
    /// 1. `FileDeletionRequest` over...
    /// 2. `RespondStorageRequest`.
    ///
    /// For both BSPs and MSPs, the last priority is given to:
    /// 1. `StopStoringForInsolventUserRequest`.
    ///
    /// This function is called every time a new block is imported and after each request is queued.
    ///
    /// _This check will be skipped if the latest processed block does not match the current best block._
    pub(crate) fn check_pending_forest_root_writes(&mut self) {
        let client_best_hash = self.client.info().best_hash;
        let client_best_number = self.client.info().best_number;

        // Skip if the latest processed block doesn't match the current best block
        if self.best_block.hash != client_best_hash || self.best_block.number != client_best_number
        {
            trace!(target: LOG_TARGET, "Skipping forest root write because latest processed block does not match current best block (local block hash and number [{}, {}], best block hash and number [{}, {}])", self.best_block.hash, self.best_block.number, client_best_hash, client_best_number);
            return;
        }

        if let Some(mut rx) = self.forest_root_write_lock.take() {
            // Note: tasks that get ownership of the lock are responsible for sending a message back when done processing.
            match rx.try_recv() {
                // If the channel is empty, means we still need to wait for the current task to finish.
                Err(TryRecvError::Empty) => {
                    // If we have a task writing to the runtime, we don't want to start another one.
                    self.forest_root_write_lock = Some(rx);
                    trace!(target: LOG_TARGET, "Waiting for current Forest root write task to finish");
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
                    state_store_context
                        .access_value(&OngoingProcessFileDeletionRequestCf)
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
                    state_store_context
                        .access_value(&OngoingProcessFileDeletionRequestCf)
                        .delete();
                    state_store_context.commit();
                }
            }
        }

        // At this point we know that the lock is released and we can start processing new requests.
        let state_store_context = self.persistent_state.open_rw_context_with_overlay();
        let mut next_event_data = None;

        if self.provider_id.is_none() {
            // If there's no Provider being managed, there's no point in checking for pending requests.
            return;
        }

        if let StorageProviderId::BackupStorageProvider(_) = self
            .provider_id
            .expect("Just checked that this node is managing a Provider; qed")
        {
            // If we have a submit proof request, prioritise it.
            // This is a BSP only operation, since MSPs don't have to submit proofs.
            while let Some(request) = self.pending_submit_proof_requests.pop_first() {
                // Check if the proof is still the next one to be submitted.
                let provider_id = request.provider_id;
                let next_challenge_tick = match self
                    .get_next_challenge_tick_for_provider(&provider_id)
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
                next_event_data = Some(ForestWriteLockTaskData::SubmitProofRequest(
                    ProcessSubmitProofRequestData {
                        seed: request.seed,
                        provider_id: request.provider_id,
                        tick: request.tick,
                        forest_challenges: request.forest_challenges,
                        checkpoint_challenges: request.checkpoint_challenges,
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
        }

        if let StorageProviderId::MainStorageProvider(_) = self
            .provider_id
            .expect("Just checked that this node is managing a Provider; qed")
        {
            // If we have no pending submit proof requests nor pending confirm storing requests, we can also check for pending file deletion requests.
            // We prioritize file deletion requests over respond storing requests since MSPs cannot charge any users while there are pending file deletion requests.
            if next_event_data.is_none() {
                // TODO: Update this to some greater value once batching is supported by the runtime.
                let max_batch_delete: u32 = 1;
                let mut file_deletion_requests = Vec::new();
                for _ in 0..max_batch_delete {
                    if let Some(request) = state_store_context
                        .pending_file_deletion_request_deque()
                        .pop_front()
                    {
                        file_deletion_requests.push(request);
                    } else {
                        break;
                    }
                }

                // If we have at least 1 file deletion request, send the process event.
                if file_deletion_requests.len() > 0 {
                    next_event_data = Some(
                        ProcessFileDeletionRequestData {
                            file_deletion_requests,
                        }
                        .into(),
                    );
                }
            }

            // If we have no pending file deletion requests, we can also check for pending respond storing requests.
            // This is a MSP only operation, since BSPs don't have to respond to storage requests, they volunteer and confirm.
            if next_event_data.is_none() {
                let max_batch_respond = MAX_BATCH_MSP_RESPOND_STORE_REQUESTS;

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
            ForestWriteLockTaskData::FileDeletionRequest(data) => {
                let state_store_context = self.persistent_state.open_rw_context_with_overlay();
                state_store_context
                    .access_value(&OngoingProcessFileDeletionRequestCf)
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
        match data {
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
            ForestWriteLockTaskData::FileDeletionRequest(data) => {
                self.emit(ProcessFileDeletionRequest {
                    data,
                    forest_root_write_tx,
                });
            }
        }
    }

    /// Emits a [`MultipleNewChallengeSeeds`] event with all the pending proof submissions for this provider.
    /// This is used to catch up to the latest proof submissions that were missed due to a node restart.
    /// Also, it can help to catch up to proofs in case there is a change in the BSP's stake (therefore
    /// also a change in it's challenge period).
    ///
    /// IMPORTANT: This function takes into account whether a proof should be submitted for the current tick.
    pub(crate) fn proof_submission_catch_up(
        &self,
        current_block_hash: &H256,
        provider_id: &ProofsDealerProviderId,
    ) {
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
        let mut next_challenge_tick = match Self::get_next_challenge_tick_for_provider(
            &self,
            provider_id,
        ) {
            Ok(next_challenge_tick) => next_challenge_tick,
            Err(e) => {
                error!(target: LOG_TARGET, "Failed to get next challenge tick for provider [{:?}]: {:?}", provider_id, e);
                return;
            }
        };
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

        // Emit the `MultipleNewChallengeSeeds` event.
        if challenge_seeds.len() > 0 {
            trace!(target: LOG_TARGET, "Emitting MultipleNewChallengeSeeds event for provider [{:?}] with challenge seeds: {:?}", provider_id, challenge_seeds);
            self.emit(MultipleNewChallengeSeeds {
                provider_id: *provider_id,
                seeds: challenge_seeds,
            });
        }
    }

    /// Applies Forest root changes found in a [`TreeRoute`].
    ///
    /// This function can be used both for new blocks as well as for reorgs.
    /// For new blocks, `tree_route` should be one such that [`TreeRoute::pivot`] is 0, therefore
    /// all blocks in [`TreeRoute::route`] are "enacted" blocks.
    /// For reorgs, `tree_route` should be one such that [`TreeRoute::pivot`] is not 0, therefore
    /// some blocks in [`TreeRoute::route`] are "retracted" blocks and some are "enacted" blocks.
    pub(crate) async fn forest_root_changes_catchup<Block>(&self, tree_route: &TreeRoute<Block>)
    where
        Block: cumulus_primitives_core::BlockT<Hash = H256>,
    {
        // Retracted blocks, i.e. the blocks from the `TreeRoute` that are reverted in the reorg.
        for block in tree_route.retracted() {
            self.apply_forest_root_changes(block, true).await;
        }

        // Enacted blocks, i.e. the blocks from the `TreeRoute` that are applied in the reorg.
        for block in tree_route.enacted() {
            self.apply_forest_root_changes(block, false).await;
        }

        trace!(target: LOG_TARGET, "Applied Forest root changes for tree route {:?}", tree_route);
    }

    /// Gets the next tick for which a Provider (BSP) should submit a proof.
    pub(crate) fn get_next_challenge_tick_for_provider(
        &self,
        provider_id: &ProofsDealerProviderId,
    ) -> Result<BlockNumber, GetProofSubmissionRecordError> {
        // Get the current block hash.
        let current_block_hash = self.client.info().best_hash;

        // Get the next tick for which the provider should submit a proof.
        match self
            .client
            .runtime_api()
            .get_next_tick_to_submit_proof_for(current_block_hash, provider_id)
        {
            Ok(next_tick_to_prove_result) => next_tick_to_prove_result,
            Err(e) => {
                error!(target: LOG_TARGET, "Runtime API error while getting next tick to submit proof for Provider [{:?}]: {:?}", provider_id, e);
                Err(GetProofSubmissionRecordError::InternalApiError)
            }
        }
    }

    /// Checks if `block_number` is one where this Blockchain Service should emit a `NotifyPeriod` event.
    pub(crate) fn check_for_notify(&self, block_number: &BlockNumber) {
        if let Some(np) = self.notify_period {
            if block_number % np == 0 {
                self.emit(NotifyPeriod {});
            }
        }
    }

    /// Applies the Forest root changes that happened in one block.
    ///
    /// Forest root changes can be [`TrieMutation`]s that are either [`TrieAddMutation`]s or [`TrieRemoveMutation`]s.
    /// These two variants add or remove a key from the Forest root, respectively.
    ///
    /// If `revert` is set to `true`, the Forest root changes will be reverted, meaning that if a [`TrieAddMutation`]
    /// is found in the block, it will be reverted with a [`TrieRemoveMutation`], and vice versa.
    ///
    /// A [`TrieRemoveMutation`] is not guaranteed to be convertible to a [`TrieAddMutation`], particularly if
    /// the [`TrieRemoveMutation::maybe_value`] is `None`. In this case, the function will log an error and return.
    ///
    /// Two kinds of events are handled:
    /// 1. [`pallet_proofs_dealer::Event::MutationsAppliedForProvider`]: for mutations applied to a BSP.
    /// 2. [`pallet_proofs_dealer::Event::MutationsApplied`]: for mutations applied to the Buckets of an MSP.
    async fn apply_forest_root_changes<Block>(&self, block: &HashAndNumber<Block>, revert: bool)
    where
        Block: cumulus_primitives_core::BlockT<Hash = H256>,
    {
        if revert {
            trace!(target: LOG_TARGET, "Reverting Forest root changes for block number {:?} and hash {:?}", block.number, block.hash);
        } else {
            trace!(target: LOG_TARGET, "Applying Forest root changes for block number {:?} and hash {:?}", block.number, block.hash);
        }

        // Preemptively getting the Buckets managed by this node, in case it is an MSP, so that we
        // do the query just once, instead of doing it for every event.
        let buckets_managed_by_msp = if let Some(StorageProviderId::MainStorageProvider(msp_id)) =
            &self.provider_id
        {
            self.client
                    .runtime_api()
                    .query_buckets_for_msp(block.hash, msp_id)
                    .inspect_err(|e| error!(target: LOG_TARGET, "Runtime API call failed while querying buckets for MSP [{:?}]: {:?}", msp_id, e))
                    .ok()
                    .and_then(|api_result| {
                        api_result
                            .inspect_err(|e| error!(target: LOG_TARGET, "Runtime API error while querying buckets for MSP [{:?}]: {:?}", msp_id, e))
                            .ok()
                    })
        } else {
            None
        };

        // Process the events in the block, specifically those that are related to the Forest root changes.
        match get_events_at_block(&self.client, &block.hash) {
            Ok(events) => {
                for ev in events {
                    match ev.event.clone() {
                        RuntimeEvent::ProofsDealer(
                            pallet_proofs_dealer::Event::MutationsAppliedForProvider {
                                provider_id,
                                mutations,
                                old_root,
                                new_root,
                            },
                        ) => {
                            // This event is relevant in case the Provider managed is a BSP.
                            if let Some(StorageProviderId::BackupStorageProvider(bsp_id)) =
                                &self.provider_id
                            {
                                // Check if the `provider_id` is the BSP that this node is managing.
                                if provider_id != *bsp_id {
                                    debug!(target: LOG_TARGET, "Provider ID [{:?}] is not the BSP ID [{:?}] that this node is managing. Skipping mutations applied event.", provider_id, bsp_id);
                                    continue;
                                }

                                trace!(target: LOG_TARGET, "Applying on-chain Forest root mutations to BSP [{:?}]", provider_id);
                                trace!(target: LOG_TARGET, "Mutations: {:?}", mutations);

                                // Apply forest root changes to the BSP's Forest Storage.
                                // At this point, we only apply the mutation of this file and its metadata to the Forest of this BSP,
                                // and not to the File Storage.
                                // This is because if in a future block built on top of this one, the BSP needs to provide
                                // a proof, it will be against the Forest root with this change applied.
                                // For file deletions, we will remove the file from the File Storage only after finality is reached.
                                // This gives us the opportunity to put the file back in the Forest if this block is re-orged.
                                let current_forest_key = CURRENT_FOREST_KEY.to_vec();
                                if let Err(e) = self
                                    .apply_forest_mutations_and_verify_root(
                                        current_forest_key,
                                        &mutations,
                                        revert,
                                        old_root,
                                        new_root,
                                    )
                                    .await
                                {
                                    error!(target: LOG_TARGET, "CRITICAL ❗️❗️ Failed to apply mutations and verify root for BSP [{:?}]. \nError: {:?}", provider_id, e);
                                    return;
                                };

                                info!(target: LOG_TARGET, "🌳 New local Forest root matches the one in the block for BSP [{:?}]", provider_id);
                            }
                        }
                        RuntimeEvent::ProofsDealer(
                            pallet_proofs_dealer::Event::MutationsApplied {
                                mutations,
                                old_root,
                                new_root,
                                event_info,
                            },
                        ) => {
                            // This event is relevant in case the Provider managed is an MSP.
                            // In which case the mutations are applied to a Bucket's Forest root.
                            if let Some(StorageProviderId::MainStorageProvider(_)) =
                                &self.provider_id
                            {
                                // Check that this MSP is managing at least one bucket.
                                if buckets_managed_by_msp.is_none() {
                                    debug!(target: LOG_TARGET, "MSP is not managing any buckets. Skipping mutations applied event.");
                                    continue;
                                }
                                let buckets_managed_by_msp = buckets_managed_by_msp
                                    .as_ref()
                                    .expect("Just checked that this is not None; qed");
                                if buckets_managed_by_msp.is_empty() {
                                    debug!(target: LOG_TARGET, "Buckets managed by MSP is an empty vector. Skipping mutations applied event.");
                                    continue;
                                }

                                // In StorageHub, we assume that all `MutationsApplied` events are emitted by bucket
                                // root changes, and they should contain the encoded `BucketId` of the bucket that was mutated
                                // in the `event_info` field.
                                if event_info.is_none() {
                                    error!(target: LOG_TARGET, "MutationsApplied event with `None` event info, when it is expected to contain the BucketId of the bucket that was mutated. This should never happen. This is a bug. Please report it to the StorageHub team.");
                                    continue;
                                }
                                let event_info =
                                    event_info.expect("Just checked that this is not None; qed");
                                let bucket_id = match self
                                    .client
                                    .runtime_api()
                                    .decode_generic_apply_delta_event_info(block.hash, event_info)
                                {
                                    Ok(runtime_api_result) => match runtime_api_result {
                                        Ok(bucket_id) => bucket_id,
                                        Err(e) => {
                                            error!(target: LOG_TARGET, "Failed to decode BucketId from event info: {:?}", e);
                                            continue;
                                        }
                                    },
                                    Err(e) => {
                                        error!(target: LOG_TARGET, "Error while calling runtime API to decode BucketId from event info: {:?}", e);
                                        continue;
                                    }
                                };

                                // Check if Bucket is managed by this MSP.
                                if !buckets_managed_by_msp.contains(&bucket_id) {
                                    debug!(target: LOG_TARGET, "Bucket [{:?}] is not managed by this MSP. Skipping mutations applied event.", bucket_id);
                                    continue;
                                }

                                // Apply forest root changes to the Bucket's Forest Storage.
                                // At this point, we only apply the mutation of this file and its metadata to the Forest of this Bucket,
                                // and not to the File Storage.
                                // For file deletions, we will remove the file from the File Storage only after finality is reached.
                                // This gives us the opportunity to put the file back in the Forest if this block is re-orged.
                                let bucket_forest_key = bucket_id.as_ref().to_vec();
                                if let Err(e) = self
                                    .apply_forest_mutations_and_verify_root(
                                        bucket_forest_key,
                                        &mutations,
                                        revert,
                                        old_root,
                                        new_root,
                                    )
                                    .await
                                {
                                    error!(target: LOG_TARGET, "CRITICAL ❗️❗️ Failed to apply mutations and verify root for Bucket [{:?}]. \nError: {:?}", bucket_id, e);
                                    return;
                                };

                                info!(target: LOG_TARGET, "🌳 New local Forest root matches the one in the block for Bucket [{:?}]", bucket_id);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                error!(target: LOG_TARGET, "Failed to get events at block {:?}: {:?}", block.hash, e);
            }
        }
    }

    /// Applies a set of [`TrieMutation`]s to a Merkle Patricia Forest, and verifies the new local
    /// Forest root against `old_root` or `new_root`, depending on the value of `revert`.
    ///
    /// If `revert` is set to `true`, the Forest root changes will be reverted, and the new local
    /// Forest root will be verified against the `old_root` Forest root.
    ///
    /// If `revert` is set to `false`, the Forest root changes will be applied, and the new local
    /// Forest root will be verified against the `new_root` Forest root.
    ///
    /// Changes are applied to the Forest in `self.forest_storage_handler.get(forest_key)`.
    async fn apply_forest_mutations_and_verify_root(
        &self,
        forest_key: Vec<u8>,
        mutations: &[(H256, TrieMutation)],
        revert: bool,
        old_root: ForestRoot,
        new_root: ForestRoot,
    ) -> Result<()> {
        for (file_key, mutation) in mutations {
            // If we are reverting the Forest root changes, we need to revert the mutation.
            let mutation = if revert {
                trace!(target: LOG_TARGET, "Reverting mutation [{:?}] with file key [{:?}]", mutation, file_key);
                match self.revert_mutation(mutation) {
                    Ok(mutation) => mutation,
                    Err(e) => {
                        error!(target: LOG_TARGET, "CRITICAL❗️❗️ Failed to revert mutation. This is a bug. Please report it to the StorageHub team. \nError: {:?}", e);
                        return Err(anyhow!("Failed to revert mutation."));
                    }
                }
            } else {
                trace!(target: LOG_TARGET, "Applying mutation [{:?}] with file key [{:?}]", mutation, file_key);
                mutation.clone()
            };

            // Apply mutation to the Forest.
            if let Err(e) = self
                .apply_forest_mutation(forest_key.clone(), file_key, &mutation)
                .await
            {
                error!(target: LOG_TARGET, "Failed to apply mutation to Forest [{:?}]", forest_key);
                error!(target: LOG_TARGET, "Mutation: {:?}", mutation);
                error!(target: LOG_TARGET, "Error: {:?}", e);
            }
        }

        // Verify that the new Forest root matches the one in the block.
        let fs = match self.forest_storage_handler.get(&forest_key.into()).await {
            Some(fs) => fs,
            None => {
                error!(target: LOG_TARGET, "CRITICAL❗️❗️ Failed to get Forest Storage.");
                return Err(anyhow!("Failed to get Forest Storage."));
            }
        };

        let local_new_root = fs.read().await.root();

        trace!(target: LOG_TARGET, "Mutations applied. New local Forest root: {:?}", local_new_root);

        if revert {
            if old_root != local_new_root {
                error!(target: LOG_TARGET, "CRITICAL❗️❗️ New local Forest root does not match the one in the block after reverting mutations. This is a bug. Please report it to the StorageHub team.");
                return Err(anyhow!(
                    "New local Forest root does not match the one in the block after reverting mutations."
                ));
            }
        } else {
            if new_root != local_new_root {
                error!(target: LOG_TARGET, "CRITICAL❗️❗️ New local Forest root does not match the one in the block after applying mutations. This is a bug. Please report it to the StorageHub team.");
                return Err(anyhow!(
                    "New local Forest root does not match the one in the block after applying mutations."
                ));
            }
        }

        Ok(())
    }

    /// Applies a [`TrieMutation`] to the a Merkle Patricia Forest.
    ///
    /// If `mutation` is a [`TrieAddMutation`], it will decode the [`TrieAddMutation::value`] as a
    /// [`FileMetadata`] and insert it into the Forest.
    /// If `mutation` is a [`TrieRemoveMutation`], it will remove the file with the key `file_key` from the Forest.
    ///
    /// Changes are applied to the Forest in `self.forest_storage_handler.get(forest_key)`.
    async fn apply_forest_mutation(
        &self,
        forest_key: Vec<u8>,
        file_key: &H256,
        mutation: &TrieMutation,
    ) -> Result<()> {
        let fs = self
            .forest_storage_handler
            .get(&forest_key.into())
            .await
            .ok_or_else(|| anyhow!("CRITICAL❗️❗️ Failed to get forest storage."))?;

        // Write lock is released when exiting the scope of this `match` statement.
        match mutation {
            TrieMutation::Add(TrieAddMutation {
                value: encoded_metadata,
            }) => {
                // Metadata comes encoded, so we need to decode it first to apply the mutation and add it to the Forest.
                let metadata = FileMetadata::decode(&mut &encoded_metadata[..]).map_err(|e| {
                    error!(target: LOG_TARGET, "CRITICAL❗️❗️ Failed to decode metadata from encoded metadata when applying mutation to Forest storage. This may result in a mismatch between the Forest root on-chain and in this node. \nThis is a critical bug. Please report it to the StorageHub team. \nError: {:?}", e);
                    anyhow!("Failed to decode metadata from encoded metadata: {:?}", e)
                })?;

                fs.write()
                    .await
                    .insert_files_metadata(vec![metadata].as_slice()).map_err(|e| {
                        error!(target: LOG_TARGET, "CRITICAL❗️❗️ Failed to apply mutation to Forest storage. This may result in a mismatch between the Forest root on-chain and in this node. \nThis is a critical bug. Please report it to the StorageHub team. \nError: {:?}", e);
                        anyhow!(
                            "Failed to insert file key into Forest storage: {:?}",
                            e
                        )
                    })?;
            }
            TrieMutation::Remove(_) => {
                fs.write().await.delete_file_key(file_key).map_err(|e| {
                          error!(target: LOG_TARGET, "CRITICAL❗️❗️ Failed to apply mutation to Forest storage. This may result in a mismatch between the Forest root on-chain and in this node. \nThis is a critical bug. Please report it to the StorageHub team. \nError: {:?}", e);
                          anyhow!(
                              "Failed to remove file key from Forest storage: {:?}",
                              e
                          )
                      })?;
            }
        };

        Ok(())
    }

    /// Reverts a [`TrieMutation`].
    ///
    /// A [`TrieMutation`] can be either a [`TrieAddMutation`] or a [`TrieRemoveMutation`].
    /// If the [`TrieMutation`] is a [`TrieAddMutation`], it will be reverted to a [`TrieRemoveMutation`].
    /// If the [`TrieMutation`] is a [`TrieRemoveMutation`], it will be reverted to a [`TrieAddMutation`].
    ///
    /// This operation can fail if the [`TrieMutation`] is a [`TrieRemoveMutation`] but its [`TrieRemoveMutation::maybe_value`]
    /// is `None`. In this case, the function will return an error.
    fn revert_mutation(&self, mutation: &TrieMutation) -> Result<TrieMutation> {
        let reverted_mutation = match mutation {
            TrieMutation::Add(TrieAddMutation { value }) => {
                TrieMutation::Remove(TrieRemoveMutation {
                    maybe_value: Some(value.clone()),
                })
            }
            TrieMutation::Remove(TrieRemoveMutation { maybe_value }) => {
                let value = match maybe_value {
                    Some(value) => value.clone(),
                    None => {
                        return Err(anyhow!("Failed to revert mutation: TrieRemoveMutation does not contain a value"));
                    }
                };

                TrieMutation::Add(TrieAddMutation { value })
            }
        };

        Ok(reverted_mutation)
    }
}

/// The output of an RPC extrinsic.
pub struct RpcExtrinsicOutput {
    /// Hash of the extrinsic.
    pub hash: H256,
    /// The nonce of the extrinsic.
    pub nonce: u32,
    /// The output string of the extrinsic if any.
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
