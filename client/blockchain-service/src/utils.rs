use anyhow::{anyhow, Result};
use log::{debug, error, info, trace, warn};
use std::{cmp::max, sync::Arc, vec};

use codec::{Decode, Encode};
use pallet_proofs_dealer_runtime_api::{
    GetChallengePeriodError, GetProofSubmissionRecordError, ProofsDealerApi,
};
use pallet_storage_providers_runtime_api::{
    QueryEarliestChangeCapacityBlockError, StorageProvidersApi,
};
use polkadot_runtime_common::BlockHashCount;
use sc_client_api::{BlockBackend, BlockImportNotification, HeaderBackend};
use sc_network::Multiaddr;
use sc_transaction_pool_api::TransactionStatus;
use shc_actors_framework::actor::Actor;
use shc_common::{
    blockchain_utils::{
        convert_raw_multiaddresses_to_multiaddr, get_events_at_block,
        get_provider_id_from_keystore, GetProviderIdError,
    },
    traits::{ExtensionOperations, KeyTypeOperations, StorageEnableRuntime},
    types::{
        BlockNumber, FileKey, Fingerprint, ForestRoot, MinimalExtension, OpaqueBlock,
        ParachainClient, ProofsDealerProviderId, StorageEnableEvents, StorageProviderId,
        TrieAddMutation, TrieMutation, TrieRemoveMutation, BCSV_KEY_TYPE,
    },
};
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use shp_file_metadata::FileMetadata;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::{HashAndNumber, TreeRoute};
use sp_core::{Blake2Hasher, Hasher, U256};
use sp_keystore::KeystorePtr;
use sp_runtime::{
    generic::{self, SignedPayload},
    traits::{Block as BlockT, CheckedSub, One, Saturating, Zero},
    SaturatedConversion,
};
use substrate_frame_rpc_system::AccountNonceApi;

use crate::{
    events::{
        AcceptedBspVolunteer, LastChargeableInfoUpdated, NewStorageRequest, NotifyPeriod,
        SlashableProvider, SpStopStoringInsolventUser, UserWithoutFunds,
    },
    handler::LOG_TARGET,
    transaction_watchers::spawn_transaction_watcher,
    types::{
        BspHandler, Extrinsic, ManagedProvider, MinimalBlockInfo, MspHandler,
        NewBlockNotificationKind, SendExtrinsicOptions, SubmittedExtrinsicInfo,
    },
    BlockchainService,
};

impl<FSH, Runtime> BlockchainService<FSH, Runtime>
where
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    /// Notify tasks waiting for a block number.
    pub(crate) fn notify_import_block_number(&mut self, block_number: &BlockNumber<Runtime>) {
        let mut keys_to_remove = Vec::new();

        for (block_number, waiters) in self
            .wait_for_block_request_by_number
            .range_mut(..=block_number)
        {
            keys_to_remove.push(*block_number);
            for waiter in waiters.drain(..) {
                match waiter.send(Ok(())) {
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
    pub(crate) fn notify_tick_number(&mut self, block_hash: &Runtime::Hash) {
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

    /// Sends back the result of the submitted transaction for all capacity requests waiting for inclusion if there is one.
    ///
    /// Begins another batch process of pending capacity requests if there are any and if
    /// we are past the block at which the capacity can be increased.
    pub(crate) async fn notify_capacity_manager(&mut self, block_number: &BlockNumber<Runtime>) {
        if self.capacity_manager.is_none() {
            return;
        };

        let current_block_hash = self.client.info().best_hash;

        let provider_id = match &self.maybe_managed_provider {
            Some(ManagedProvider::Msp(msp_handler)) => msp_handler.msp_id,
            Some(ManagedProvider::Bsp(bsp_handler)) => bsp_handler.bsp_id,
            None => return,
        };

        let capacity_manager_ref = self
            .capacity_manager
            .as_ref()
            .expect("Capacity manager should exist when calling this function");

        // Send response to all callers waiting for their capacity request to be included in a block.
        if capacity_manager_ref.has_requests_waiting_for_inclusion() {
            if let Some(last_submitted_transaction) =
                capacity_manager_ref.last_submitted_transaction()
            {
                // Check if extrinsic was included in the current block.
                if let Ok(extrinsic) = self
                    .get_extrinsic_from_block(current_block_hash, last_submitted_transaction.hash)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to get extrinsic from block: {:?}", e))
                {
                    // Check if the extrinsic succeeded or failed.
                    let result = extrinsic
                        .events
                        .iter()
                        .find_map(|event| {
                            if let StorageEnableEvents::System(system_event) =
                                &event.event.clone().into()
                            {
                                match system_event {
                                    frame_system::Event::ExtrinsicSuccess { dispatch_info: _ } => {
                                        Some(Ok(()))
                                    }
                                    frame_system::Event::ExtrinsicFailed {
                                        dispatch_error,
                                        dispatch_info: _,
                                    } => {
                                        Some(Err(format!("Extrinsic failed: {:?}", dispatch_error)))
                                    }
                                    _ => None,
                                }
                            } else {
                                None
                            }
                        })
                        .unwrap_or(Ok(()));

                    // Notify all callers of the result.
                    if let Some(capacity_manager) = self.capacity_manager.as_mut() {
                        capacity_manager.complete_requests_waiting_for_inclusion(result);
                    } else {
                        error!(target: LOG_TARGET, "[notify_capacity_manager] Capacity manager not initialized");
                    }
                }
            }
        }

        // We will only attempt to process the next batch of requests in the queue if there are no requests waiting for inclusion.
        if self
            .capacity_manager
            .as_ref()
            .unwrap()
            .has_requests_waiting_for_inclusion()
        {
            return;
        }

        // Query earliest block to change capacity
        let Ok(earliest_block) = self
            .client
            .runtime_api()
            .query_earliest_change_capacity_block(current_block_hash, &provider_id)
            .unwrap_or_else(|_| {
                error!(target: LOG_TARGET, "[notify_capacity_manager] Failed to query earliest block to change capacity");
                Err(QueryEarliestChangeCapacityBlockError::InternalError)
            })
        else {
            return;
        };

        // We can send the transaction 1 block before the earliest block to change capacity since it will be included in the next block.
        if *block_number >= earliest_block.saturating_sub(One::one()) {
            if let Err(e) = self.process_capacity_requests(*block_number).await {
                error!(target: LOG_TARGET, "[notify_capacity_manager] Failed to process capacity requests: {:?}", e);
            }
        }
    }

    /// From a [`BlockImportNotification`], gets the imported block, and checks if:
    /// 1. The block is not the new best block. For example, it could be a block from a non-best fork branch.
    ///     - If so, it returns [`NewBlockNotificationKind::NewNonBestBlock`].
    /// 2. The block is the new best block, and its parent is the previous best block.
    ///     - If so, it registers it as the new best block and returns [`NewBlockNotificationKind::NewBestBlock`].
    /// 3. The block is the new best block, and its parent is NOT the previous best block (i.e. it's a reorg).
    ///     - If so, it registers it as the new best block and returns [`NewBlockNotificationKind::Reorg`].
    pub(crate) fn register_best_block_and_check_reorg(
        &mut self,
        block_import_notification: &BlockImportNotification<OpaqueBlock>,
    ) -> NewBlockNotificationKind<Runtime> {
        let last_best_block = self.best_block;
        let new_block_info: MinimalBlockInfo<Runtime> = block_import_notification.into();

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
            // - The size of the route is equal to `BlockchainServiceConfig::max_blocks_behind_to_catch_up_root_changes`, or
            // - The parent block is not found, or
            // - We reach the last best block processed.
            let mut route = vec![new_block_info.into()];
            let mut last_block_added = new_block_info;
            loop {
                // Check if we are at the genesis block.
                if last_block_added.number == Zero::zero() {
                    trace!(target: LOG_TARGET, "Reached genesis block while building tree route for new best block");
                    break;
                }

                // Check if the route reached the maximum number of blocks to catch up on.
                // The cast is safe because it is reasonable to assume the route length is less than u32::MAX.
                let route_length = BlockNumber::<Runtime>::from(route.len() as u32);
                if route_length == self.config.max_blocks_behind_to_catch_up_root_changes {
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
                let parent_block_info: MinimalBlockInfo<Runtime> = MinimalBlockInfo {
                    number: parent_block.block.header.number.into(),
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

    /// Get the current account nonce on-chain for a generic signature type.
    pub(crate) fn account_nonce(&self, block_hash: &Runtime::Hash) -> u32 {
        let pub_key = Self::caller_pub_key(self.keystore.clone());
        self.client
            .runtime_api()
            .account_nonce(*block_hash, pub_key.into())
            .expect("Fetching account nonce works; qed")
    }

    /// Checks if the account nonce on-chain is higher than the nonce in the [`BlockchainService`].
    ///
    /// If the nonce is higher, the `nonce_counter` is updated in the [`BlockchainService`].
    pub(crate) fn sync_nonce(&mut self, block_hash: &Runtime::Hash) {
        let on_chain_nonce = self.account_nonce(block_hash);
        if on_chain_nonce > self.nonce_counter {
            debug!(
                target: LOG_TARGET,
                "Syncing nonce from {} (local) to {} (on-chain)",
                self.nonce_counter,
                on_chain_nonce
            );
            self.nonce_counter = on_chain_nonce;
        }
    }

    /// Get the Provider ID linked to the [`BCSV_KEY_TYPE`] key in this node's keystore.
    ///
    /// IMPORTANT! If there is more than one [`BCSV_KEY_TYPE`] key in this node's keystore, linked to
    /// different Provider IDs, this function will panic. In other words, this node doesn't support
    /// managing multiple Providers at once.
    pub(crate) fn sync_provider_id(&mut self, block_hash: &Runtime::Hash) {
        let provider_id = match get_provider_id_from_keystore::<Runtime>(
            &self.client,
            &self.keystore,
            block_hash,
        ) {
            Ok(None) => {
                warn!(target: LOG_TARGET, "🔑 There is no Provider ID linked to any of the BCSV keys in this node's keystore. This is expected, if this node starts up before the BSP has been registered.");
                return;
            }
            Ok(Some(provider_id)) => provider_id,
            Err(GetProviderIdError::MultipleProviderIds) => {
                panic!("There are more than one BCSV keys linked to Provider IDs in this node's keystore. Managing multiple Providers at once is not supported.");
            }
            Err(GetProviderIdError::RuntimeApiError(e)) => {
                error!(target: LOG_TARGET, "Runtime API error while getting Provider ID: {}", e);
                return;
            }
        };

        // Replace the provider ID only if it is not already managed.
        match (&self.maybe_managed_provider, provider_id) {
            // Case: The node was not managing any Provider.
            (None, _) => {
                info!(target: LOG_TARGET, "🔑 This node is not managing any Provider. Starting to manage Provider ID {:?}", provider_id);
                self.maybe_managed_provider = Some(ManagedProvider::new(provider_id));
            }
            // Case: The node goes from managing a BSP, to managing another BSP with a different ID.
            (
                Some(ManagedProvider::Bsp(bsp_handler)),
                StorageProviderId::BackupStorageProvider(bsp_id),
            ) if bsp_handler.bsp_id != bsp_id => {
                warn!(target: LOG_TARGET, "🔄 This node is already managing a BSP. Stopping managing BSP ID {:?} in favour of BSP ID {:?}", bsp_handler.bsp_id, bsp_id);
                self.maybe_managed_provider = Some(ManagedProvider::Bsp(BspHandler::new(bsp_id)));
            }
            // Case: The node goes from managing a MSP, to managing a MSP with a different ID.
            (
                Some(ManagedProvider::Msp(msp_handler)),
                StorageProviderId::MainStorageProvider(msp_id),
            ) if msp_handler.msp_id != msp_id => {
                warn!(target: LOG_TARGET, "🔄 This node is already managing a MSP. Stopping managing MSP ID {:?} in favour of MSP ID {:?}", msp_handler.msp_id, msp_id);
                self.maybe_managed_provider = Some(ManagedProvider::Msp(MspHandler::new(msp_id)));
            }
            // Case: The node goes from managing a BSP, to managing a MSP   .
            (
                Some(ManagedProvider::Bsp(bsp_handler)),
                StorageProviderId::MainStorageProvider(msp_id),
            ) => {
                warn!(target: LOG_TARGET, "🔄 This node is already managing a BSP. Stopping managing BSP ID {:?} in favour of MSP ID {:?}", bsp_handler.bsp_id, msp_id);
                self.maybe_managed_provider = Some(ManagedProvider::Msp(MspHandler::new(msp_id)));
            }
            // Case: The node goes from managing a MSP, to managing a BSP.
            (
                Some(ManagedProvider::Msp(msp_handler)),
                StorageProviderId::BackupStorageProvider(bsp_id),
            ) => {
                warn!(target: LOG_TARGET, "🔄 This node is already managing a MSP. Stopping managing MSP ID {:?} in favour of BSP ID {:?}", msp_handler.msp_id, bsp_id);
                self.maybe_managed_provider = Some(ManagedProvider::Bsp(BspHandler::new(bsp_id)));
            }
            // Rest of the cases are ignored.
            (Some(ManagedProvider::Bsp(_)), StorageProviderId::BackupStorageProvider(_))
            | (Some(ManagedProvider::Msp(_)), StorageProviderId::MainStorageProvider(_)) => {}
        }
    }

    /// Send an extrinsic to this node using an RPC call.
    ///
    /// Passing a specific `nonce` will be used to construct the extrinsic if it is higher than the current on-chain nonce.
    /// Otherwise, the current on-chain nonce will be used.
    /// Passing `None` for the `nonce` will use the [`nonce_counter`](BlockchainService::nonce_counter) as the nonce while still
    /// checking that the on-chain nonce is not lower.
    pub(crate) async fn send_extrinsic(
        &mut self,
        call: impl Into<Runtime::Call>,
        options: &SendExtrinsicOptions,
    ) -> Result<SubmittedExtrinsicInfo<Runtime>> {
        debug!(target: LOG_TARGET, "Sending extrinsic to the runtime");

        let block_hash = self.client.info().best_hash;
        let block_number = self.client.info().best_number.saturated_into();

        // Check if there's a nonce gap we can fill with this transaction
        let on_chain_nonce = self.account_nonce(&block_hash);
        let gaps =
            self.transaction_manager
                .detect_gaps(on_chain_nonce, self.nonce_counter, block_number);

        // Use the highest valid nonce, OR the first gap nonce if one exists
        let nonce = if !gaps.is_empty() && options.nonce().is_none() {
            let gap_nonce = gaps[0].nonce;
            info!(
                target: LOG_TARGET,
                "🔧 Using transaction to fill nonce gap at {} (would have been {})",
                gap_nonce,
                self.nonce_counter
            );
            gap_nonce
        } else {
            max(
                options.nonce().unwrap_or(self.nonce_counter),
                on_chain_nonce,
            )
        };

        // Construct the extrinsic.
        let call: Runtime::Call = call.into();
        let extrinsic =
            self.construct_extrinsic(self.client.clone(), call.clone(), nonce, options.tip());

        // Generate a unique ID for this query.
        let id_hash = Blake2Hasher::hash(&extrinsic.encode());

        // Submit the transaction and set up the watcher infrastructure for it.
        // We submit before tracking because Substrate's transaction pool validates everything
        // (including nonce conflicts, tip comparisons, etc.). If the RPC accepts it, it's safe to track
        let (tx_hash, watch_rx) = self
            .submit_and_watch_extrinsic(extrinsic.encode(), nonce, id_hash)
            .await?;

        // Add the transaction to the transaction manager to track it
        if let Err(e) =
            self.transaction_manager
                .track_transaction(nonce, id_hash, call, block_number)
        {
            warn!(
                target: LOG_TARGET,
                "Failed to track transaction in manager: {:?}. Transaction will still be watched but not tracked for gap detection.",
                e
            );
        }

        // TODO: Handle nonce overflow.
        // Only update nonce after we are sure no errors occurred submitting the extrinsic to the node.
        // Use max() to prevent regression when filling gaps. For example, if we're filling a gap at
        // nonce 25 but our local nonce counter is already at 28, we want to keep it at 28, not drop it to 26
        self.nonce_counter = max(self.nonce_counter, nonce + 1);

        // Spawn the transaction watcher
        spawn_transaction_watcher::<Runtime>(
            nonce,
            tx_hash,
            watch_rx,
            self.tx_status_sender.clone(),
        );

        // Create a status subscription for this transaction
        let status_subscription = self
            .transaction_manager
            .subscribe_to_status(nonce)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Transaction was just added to the manager, so it must have a status subscription"
                )
            })?;

        Ok(SubmittedExtrinsicInfo {
            hash: id_hash,
            nonce,
            status_subscription,
        })
    }

    /// Construct an extrinsic that can be applied to the runtime using a generic signature type.
    pub fn construct_extrinsic(
        &self,
        client: Arc<ParachainClient<Runtime::RuntimeApi>>,
        function: impl Into<Runtime::Call>,
        nonce: u32,
        tip: u128,
    ) -> generic::UncheckedExtrinsic<
        Runtime::Address,
        Runtime::Call,
        Runtime::Signature,
        Runtime::Extension,
    > {
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

        let minimal_extra =
            MinimalExtension::new(generic::Era::mortal(period, current_block), nonce, tip);
        let extra: Runtime::Extension = Runtime::Extension::from_minimal_extension(minimal_extra);
        let implicit =
            <Runtime::Extension as ExtensionOperations<Runtime::Call, Runtime>>::implicit(
                genesis_block,
                current_block_hash,
            );

        let raw_payload = SignedPayload::from_raw(function.clone(), extra.clone(), implicit);

        let caller_pub_key = Self::caller_pub_key(self.keystore.clone());

        // Sign the payload.
        let signature = raw_payload
            .using_encoded(|e| {
                Runtime::Signature::sign(&self.keystore, BCSV_KEY_TYPE, &caller_pub_key, e)
            })
            .expect("The payload is always valid and should be possible to sign; qed");

        // Construct the extrinsic.
        generic::UncheckedExtrinsic::new_signed(
            function,
            Runtime::Signature::public_to_address(&caller_pub_key),
            signature,
            extra,
        )
    }

    // Generic function to get signer public key for any signature type
    pub fn caller_pub_key(
        keystore: KeystorePtr,
    ) -> <Runtime::Signature as KeyTypeOperations>::Public {
        let caller_pub_key = Runtime::Signature::public_keys(&keystore, BCSV_KEY_TYPE)
            .pop()
            .expect(
                format!(
                    "There should be at least one key in the keystore with key type '{:?}' ; qed",
                    BCSV_KEY_TYPE
                )
                .as_str(),
            );
        caller_pub_key
    }

    /// Get an extrinsic from a block.
    pub(crate) async fn get_extrinsic_from_block(
        &self,
        block_hash: Runtime::Hash,
        extrinsic_hash: Runtime::Hash,
    ) -> Result<Extrinsic<Runtime>> {
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
        let events_in_block = get_events_at_block::<Runtime>(&self.client, &block_hash)?;

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

    /// Check if the challenges tick is one that this provider has to submit a proof for,
    /// and if so, return true.
    pub(crate) fn should_provider_submit_proof(
        &self,
        block_hash: &Runtime::Hash,
        provider_id: &ProofsDealerProviderId<Runtime>,
        current_tick: &BlockNumber<Runtime>,
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
        let current_tick_minus_last_submission = match current_tick.checked_sub(&last_tick_provided)
        {
            Some(tick) => tick,
            None => {
                error!(target: LOG_TARGET, "CRITICAL❗️❗️ Current tick is smaller than the last tick this provider submitted a proof for. This should not happen. \nThis is a bug. Please report it to the StorageHub team.");
                return false;
            }
        };

        (current_tick_minus_last_submission % provider_challenge_period) == Zero::zero()
    }

    /// Process pending transaction status updates from watchers.
    ///
    /// Immediate removal from our transaction manager (all terminal states):
    /// - Invalid (retriable - gap preserved)
    /// - Dropped (retriable - gap preserved)
    /// - Usurped (replaced - gap cleared)
    /// - Finalized (success - gap cleared)
    /// - FinalityTimeout (timeout - gap cleared)
    ///
    /// Kept in our transaction manager (non-terminal states):
    /// - Future
    /// - Ready
    /// - Broadcast
    /// - InBlock
    /// - Retracted
    pub(crate) fn process_transaction_status_updates(&mut self) {
        while let Ok((nonce, tx_hash, status)) = self.tx_status_receiver.try_recv() {
            // Notify subscribers about the status change
            self.transaction_manager
                .notify_status_change(nonce, status.clone());

            // Check if this is a terminal state that requires immediate removal
            let should_remove = matches!(
                status,
                TransactionStatus::Invalid
                    | TransactionStatus::Dropped
                    | TransactionStatus::Usurped(_)
                    | TransactionStatus::Finalized(_)
                    | TransactionStatus::FinalityTimeout(_)
            );

            if should_remove {
                // Check if this transaction is still the current one in the manager
                // (it might have been replaced by a newer transaction with the same nonce)
                let is_current_transaction = self
                    .transaction_manager
                    .pending
                    .get(&nonce)
                    .map(|tx| tx.hash == tx_hash)
                    .unwrap_or(false);

                match &status {
                    TransactionStatus::Dropped => {
                        if is_current_transaction {
                            warn!(
                                target: LOG_TARGET,
                                "⚠️ Transaction with nonce {} (hash: {:?}) was dropped from Substrate's transaction pool. Removing from tracking but keeping gap detection.",
                                nonce, tx_hash
                            );
                            self.transaction_manager.remove_pending_but_keep_gap(nonce);
                        } else {
                            debug!(
                                target: LOG_TARGET,
                                "Ignoring Dropped event for old transaction with nonce {} (hash: {:?}), current transaction is different",
                                nonce, tx_hash
                            );
                        }
                    }
                    TransactionStatus::Invalid => {
                        if is_current_transaction {
                            warn!(
                                target: LOG_TARGET,
                                "⚠️ Transaction with nonce {} (hash: {:?}) is invalid. Removing from tracking but keeping gap detection.",
                                nonce, tx_hash
                            );
                            self.transaction_manager.remove_pending_but_keep_gap(nonce);
                        } else {
                            debug!(
                                target: LOG_TARGET,
                                "Ignoring Invalid event for old transaction with nonce {} (hash: {:?}), current transaction is different",
                                nonce, tx_hash
                            );
                        }
                    }
                    TransactionStatus::Usurped(_) => {
                        if is_current_transaction {
                            debug!(
                                target: LOG_TARGET,
                                "✓ Transaction with nonce {} (hash: {:?}) was usurped. Removing from tracking.",
                                nonce, tx_hash
                            );
                            self.transaction_manager.remove(nonce);
                        } else {
                            debug!(
                                target: LOG_TARGET,
                                "Ignoring Usurped event for old transaction with nonce {} (hash: {:?}), it was already replaced",
                                nonce, tx_hash
                            );
                        }
                    }
                    TransactionStatus::Finalized(_) => {
                        if is_current_transaction {
                            debug!(
                                target: LOG_TARGET,
                                "✓ Transaction with nonce {} (hash: {:?}) was finalized. Removing from tracking.",
                                nonce, tx_hash
                            );
                        } else {
                            warn!(
                                target: LOG_TARGET,
                                "⚠️ Old transaction with nonce {} (hash: {:?}) was finalized, but we have a different transaction ({:?}) in manager. \
                                Removing newer transaction as nonce is now consumed.",
                                nonce, tx_hash, self.transaction_manager.pending.get(&nonce).map(|tx| tx.hash)
                            );
                        }
                        self.transaction_manager.remove(nonce);
                    }
                    TransactionStatus::FinalityTimeout(_) => {
                        if is_current_transaction {
                            debug!(
                                target: LOG_TARGET,
                                "⏱️ Transaction with nonce {} (hash: {:?}) had finality timeout. Removing from tracking.",
                                nonce, tx_hash
                            );
                            self.transaction_manager.remove(nonce);
                        } else {
                            debug!(
                                target: LOG_TARGET,
                                "Ignoring FinalityTimeout event for old transaction with nonce {} (hash: {:?}), current transaction is different",
                                nonce, tx_hash
                            );
                        }
                    }
                    _ => {}
                }
            } else if let Some(tx) = self.transaction_manager.pending.get_mut(&nonce) {
                // Only update status if this is the current transaction
                if tx.hash == tx_hash {
                    debug!(
                        target: LOG_TARGET,
                        "📊 Transaction with nonce {} (hash: {:?}) status updated: {:?}",
                        nonce, tx_hash, status
                    );
                    tx.latest_status = status;
                } else {
                    debug!(
                        target: LOG_TARGET,
                        "Ignoring status update for old transaction with nonce {} (hash: {:?}), current hash is {:?}",
                        nonce, tx_hash, tx.hash
                    );
                }
            }
        }
    }

    /// Handle old nonce gaps that haven't been filled in the transaction manager.
    ///
    /// Nonce gaps can occur when a transaction is dropped from the mempool after RPC acceptance
    /// so it fails to be included, but higher nonces were submitted optimistically.
    ///
    /// Normally, nonce gaps are filled automatically when a new transaction is submitted, but in case
    /// a new transaction is not submitted after a certain number of blocks, we will send a `remark`
    /// transaction to fill the gap and avoid the client getting stuck.
    pub(crate) async fn handle_old_nonce_gaps(&mut self, block_number: BlockNumber<Runtime>) {
        // Detect gaps in the nonce sequence
        let on_chain_nonce = self.account_nonce(&self.client.info().best_hash);
        let gaps =
            self.transaction_manager
                .detect_gaps(on_chain_nonce, self.nonce_counter, block_number);

        if gaps.is_empty() {
            return;
        }

        // Send gap-filling transactions for old gaps
        let gap_fill_threshold = self.transaction_manager.config.gap_fill_threshold_blocks;

        for gap in gaps {
            if gap.age_in_blocks >= gap_fill_threshold {
                warn!(
                    target: LOG_TARGET,
                    "Gap at nonce {} is {} blocks old, sending gap-filling transaction",
                    gap.nonce,
                    gap.age_in_blocks
                );

                if let Err(e) = self.send_gap_filling_transaction(gap.nonce).await {
                    error!(
                        target: LOG_TARGET,
                        "Failed to send gap-filling transaction for nonce {}: {:?}",
                        gap.nonce,
                        e
                    );
                }
            } else {
                debug!(
                    target: LOG_TARGET,
                    "Gap at nonce {} is only {} blocks old, waiting before filling",
                    gap.nonce,
                    gap.age_in_blocks
                );
            }
        }
    }

    /// Send a gap-filling transaction using system.remark("").
    ///
    /// This is used as a fallback when a nonce gap persists after a timeout
    /// and no other transaction have been submitted to fill the gap.
    async fn send_gap_filling_transaction(&mut self, nonce: u32) -> Result<()> {
        info!(
                target: LOG_TARGET,
                "Sending gap-filling transaction (system.remark) for nonce {}",
                nonce
        );

        // Create a system.remark("") call
        let remark_call = frame_system::Call::<Runtime>::remark { remark: vec![] };
        let call: Runtime::Call = remark_call.into();

        // Construct the extrinsic
        let extrinsic = self.construct_extrinsic(self.client.clone(), call.clone(), nonce, 0);

        // Calculate the transaction hash
        let id_hash = sp_core::Blake2Hasher::hash(&extrinsic.encode());

        // Submit the transaction and set up the watcher infrastructure for it.
        // We submit before tracking because Substrate's transaction pool validates everything
        // (including nonce conflicts, tip comparisons, etc.). If the RPC accepts it, it's safe to track
        let (tx_hash, watch_rx) = self
            .submit_and_watch_extrinsic(extrinsic.encode(), nonce, id_hash)
            .await?;

        // Add the transaction to the transaction manager to track it
        let block_number = self.client.info().best_number.saturated_into();
        if let Err(e) =
            self.transaction_manager
                .track_transaction(nonce, id_hash, call.clone(), block_number)
        {
            warn!(
                target: LOG_TARGET,
                "Failed to track gap-filling transaction: {:?}. It will still be watched.",
                e
            );
        }

        // Spawn the watcher for the gap-filling transaction
        spawn_transaction_watcher::<Runtime>(
            nonce,
            tx_hash,
            watch_rx,
            self.tx_status_sender.clone(),
        );

        info!(
                target: LOG_TARGET,
                "Successfully sent gap-filling transaction for nonce {}",
                nonce
        );

        Ok(())
    }

    /// Submit an extrinsic via RPC and return the status receiver.
    ///
    /// This is the common logic for submitting transactions and monitoring their status.
    /// It handles RPC errors, JSON parsing, and returns the receiver for status updates.
    ///
    /// Returns a tuple of (transaction_hash, receiver_for_watcher)
    async fn submit_and_watch_extrinsic(
        &self,
        extrinsic_encoded: Vec<u8>,
        nonce: u32,
        id_hash: Runtime::Hash,
    ) -> Result<(Runtime::Hash, tokio::sync::mpsc::Receiver<String>)> {
        // Submit the transaction via RPC
        let (result, rx) = match self
            .rpc_handlers
            .rpc_query(&format!(
                r#"{{
                    "jsonrpc": "2.0",
                    "method": "author_submitAndWatchExtrinsic",
                    "params": ["0x{}"],
                    "id": {:?}
                }}"#,
                array_bytes::bytes2hex("", &extrinsic_encoded),
                array_bytes::bytes2hex("", &id_hash.as_bytes())
            ))
            .await
        {
            Ok((result, rx)) => (result, rx),
            Err(e) => {
                error!(
                    target: LOG_TARGET,
                    "RPC query failed for transaction with nonce {}: {}",
                    nonce,
                    e
                );
                return Err(anyhow::anyhow!("RPC query failed: {}", e));
            }
        };

        // Parse JSON response
        let json: serde_json::Value = match serde_json::from_str(&result) {
            Ok(json) => json,
            Err(e) => {
                error!(
                    target: LOG_TARGET,
                    "Failed to parse RPC response for nonce {}: {}",
                    nonce,
                    e
                );
                return Err(anyhow::anyhow!("Failed to parse RPC response: {}", e));
            }
        };

        // Check for errors in response
        let error = match json.as_object() {
            Some(obj) => obj.get("error"),
            None => {
                error!(
                    target: LOG_TARGET,
                    "RPC response is not a JSON object for nonce {}",
                    nonce
                );
                return Err(anyhow::anyhow!("RPC response is not a JSON object"));
            }
        };

        if let Some(error) = error {
            return Err(anyhow::anyhow!("Error in RPC call: {}", error.to_string()));
        }

        // Return the RPC receiver
        Ok((id_hash, rx))
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
        Block: BlockT<Hash = Runtime::Hash>,
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
        provider_id: &ProofsDealerProviderId<Runtime>,
    ) -> Result<BlockNumber<Runtime>, GetProofSubmissionRecordError> {
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
    pub(crate) fn check_for_notify(&self, block_number: &BlockNumber<Runtime>) {
        if let Some(np) = self.notify_period {
            let block_number: U256 = (*block_number).into();
            if block_number % np == Zero::zero() {
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
        Block: BlockT<Hash = Runtime::Hash>,
    {
        if revert {
            trace!(target: LOG_TARGET, "Reverting Forest root changes for block number {:?} and hash {:?}", block.number, block.hash);
        } else {
            trace!(target: LOG_TARGET, "Applying Forest root changes for block number {:?} and hash {:?}", block.number, block.hash);
        }

        // Process the events in the block, specifically those that are related to the Forest root changes.
        match get_events_at_block::<Runtime>(&self.client, &block.hash) {
            Ok(events) => {
                for ev in events {
                    if let Some(managed_provider) = &self.maybe_managed_provider {
                        match managed_provider {
                            ManagedProvider::Bsp(_) => {
                                self.bsp_process_forest_root_changing_events(
                                    ev.event.clone().into(),
                                    revert,
                                )
                                .await;
                            }
                            ManagedProvider::Msp(_) => {
                                self.msp_process_forest_root_changing_events(
                                    &block.hash,
                                    ev.event.clone().into(),
                                    revert,
                                )
                                .await;
                            }
                        }
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
    pub(crate) async fn apply_forest_mutations_and_verify_root(
        &self,
        forest_key: Vec<u8>,
        mutations: &[(Runtime::Hash, TrieMutation)],
        revert: bool,
        old_root: ForestRoot<Runtime>,
        new_root: ForestRoot<Runtime>,
    ) -> Result<()> {
        debug!(target: LOG_TARGET, "Applying Forest mutations to Forest key [{:?}], reverting: {}, old root: {:?}, new root: {:?}", forest_key, revert, old_root, new_root);

        for (file_key, mutation) in mutations {
            // If we are reverting the Forest root changes, we need to revert the mutation.
            let mutation = if revert {
                debug!(target: LOG_TARGET, "Reverting mutation [{:?}] with file key [{:?}]", mutation, file_key);
                match self.revert_mutation(mutation) {
                    Ok(mutation) => mutation,
                    Err(e) => {
                        error!(target: LOG_TARGET, "CRITICAL❗️❗️ Failed to revert mutation. This is a bug. Please report it to the StorageHub team. \nError: {:?}", e);
                        return Err(anyhow!("Failed to revert mutation."));
                    }
                }
            } else {
                debug!(target: LOG_TARGET, "Applying mutation [{:?}] with file key [{:?}]", mutation, file_key);
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

        debug!(target: LOG_TARGET, "Mutations applied. New local Forest root: {:?}", local_new_root);

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
        file_key: &Runtime::Hash,
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
                let metadata = <FileMetadata<{shp_constants::H_LENGTH}, {shp_constants::FILE_CHUNK_SIZE}, {shp_constants::FILE_SIZE_TO_CHALLENGES}> as Decode>::decode(&mut &encoded_metadata[..]).map_err(|e| {
                    error!(target: LOG_TARGET, "CRITICAL❗️❗️ Failed to decode metadata from encoded metadata when applying mutation to Forest storage. This may result in a mismatch between the Forest root on-chain and in this node. \nThis is a critical bug. Please report it to the StorageHub team. \nError: {:?}", e);
                    anyhow!("Failed to decode metadata from encoded metadata: {:?}", e)
                })?;

                let inserted_file_keys = fs.write()
                    .await
                    .insert_files_metadata(vec![metadata].as_slice()).map_err(|e| {
                        error!(target: LOG_TARGET, "CRITICAL❗️❗️ Failed to apply mutation to Forest storage. This may result in a mismatch between the Forest root on-chain and in this node. \nThis is a critical bug. Please report it to the StorageHub team. \nError: {:?}", e);
                        anyhow!(
                            "Failed to insert file key into Forest storage: {:?}",
                            e
                        )
                    })?;

                debug!(target: LOG_TARGET, "Inserted file keys: {:?}", inserted_file_keys);
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

    pub(crate) fn process_common_block_import_events(
        &mut self,
        event: StorageEnableEvents<Runtime>,
    ) {
        match event {
            // New storage request event coming from pallet-file-system.
            StorageEnableEvents::FileSystem(pallet_file_system::Event::NewStorageRequest {
                who,
                file_key,
                bucket_id,
                location,
                fingerprint,
                size,
                peer_ids,
                expires_at,
            }) => self.emit(NewStorageRequest {
                who,
                file_key: FileKey::from(file_key.as_ref()),
                bucket_id,
                location,
                fingerprint: fingerprint.as_ref().into(),
                size,
                user_peer_ids: peer_ids,
                expires_at: expires_at,
            }),
            // A provider has been marked as slashable.
            StorageEnableEvents::ProofsDealer(pallet_proofs_dealer::Event::SlashableProvider {
                provider,
                next_challenge_deadline,
            }) => self.emit(SlashableProvider {
                provider,
                next_challenge_deadline: next_challenge_deadline.saturated_into(),
            }),
            // The last chargeable info of a provider has been updated
            StorageEnableEvents::PaymentStreams(
                pallet_payment_streams::Event::LastChargeableInfoUpdated {
                    provider_id,
                    last_chargeable_tick,
                    last_chargeable_price_index,
                },
            ) => {
                if let Some(managed_provider_id) = &self.maybe_managed_provider {
                    // We only emit the event if the Provider ID is the one that this node is managing.
                    // It's irrelevant if the Provider ID is a MSP or a BSP.
                    let managed_provider_id = match managed_provider_id {
                        ManagedProvider::Bsp(bsp_handler) => &bsp_handler.bsp_id,
                        ManagedProvider::Msp(msp_handler) => &msp_handler.msp_id,
                    };
                    if provider_id == *managed_provider_id {
                        self.emit(LastChargeableInfoUpdated {
                            provider_id,
                            last_chargeable_tick,
                            last_chargeable_price_index,
                        })
                    }
                }
            }
            // A user has been flagged as without funds in the runtime
            StorageEnableEvents::PaymentStreams(
                pallet_payment_streams::Event::UserWithoutFunds { who },
            ) => {
                self.emit(UserWithoutFunds { who });
            }
            // A file was correctly deleted from a user without funds
            StorageEnableEvents::FileSystem(
                pallet_file_system::Event::SpStopStoringInsolventUser {
                    sp_id,
                    file_key,
                    owner,
                    location,
                    new_root,
                },
            ) => {
                if let Some(managed_provider_id) = &self.maybe_managed_provider {
                    // We only emit the event if the Provider ID is the one that this node is managing.
                    // It's irrelevant if the Provider ID is a MSP or a BSP.
                    let managed_provider_id = match managed_provider_id {
                        ManagedProvider::Bsp(bsp_handler) => &bsp_handler.bsp_id,
                        ManagedProvider::Msp(msp_handler) => &msp_handler.msp_id,
                    };
                    if sp_id == *managed_provider_id {
                        self.emit(SpStopStoringInsolventUser {
                            sp_id,
                            file_key: file_key.into(),
                            owner,
                            location,
                            new_root,
                        })
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn process_common_finality_events(&self, _event: StorageEnableEvents<Runtime>) {
        {}
    }

    pub(crate) fn process_test_user_events(&self, event: StorageEnableEvents<Runtime>) {
        match event {
            StorageEnableEvents::FileSystem(pallet_file_system::Event::AcceptedBspVolunteer {
                bsp_id,
                bucket_id,
                location,
                fingerprint,
                multiaddresses,
                owner,
                size,
            }) if owner == Self::caller_pub_key(self.keystore.clone()).into() => {
                // This event should only be of any use if a node is run by as a user.
                if self.maybe_managed_provider.is_none() {
                    log::info!(
                        target: LOG_TARGET,
                        "AcceptedBspVolunteer event for BSP ID: {:?}",
                        bsp_id
                    );

                    // We try to convert the types coming from the runtime into our expected types.
                    let fingerprint: Fingerprint = fingerprint.as_bytes().into();

                    let multiaddress_vec: Vec<Multiaddr> =
                        convert_raw_multiaddresses_to_multiaddr::<Runtime>(multiaddresses);

                    self.emit(AcceptedBspVolunteer {
                        bsp_id,
                        bucket_id,
                        location,
                        fingerprint,
                        multiaddresses: multiaddress_vec,
                        owner,
                        size,
                    })
                }
            }
            _ => {}
        }
    }
}
