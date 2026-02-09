use log::{debug, error, info, trace, warn};
use shc_common::traits::StorageEnableRuntime;
use std::{collections::HashSet, sync::Arc};

use sc_client_api::HeaderBackend;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::TreeRoute;
use sp_core::{H256, U256};
use sp_runtime::traits::{Block as BlockT, Zero};

use pallet_file_system_runtime_api::FileSystemApi;
use pallet_proofs_dealer_runtime_api::{
    GetChallengePeriodError, GetChallengeSeedError, ProofsDealerApi,
};
use pallet_storage_providers_runtime_api::StorageProvidersApi;
use shc_actors_framework::actor::Actor;
use shc_common::{
    blockchain_utils::get_events_at_block,
    consts::CURRENT_FOREST_KEY,
    typed_store::CFDequeAPI,
    types::{
        BackupStorageProviderId, BlockNumber, FileKey, Fingerprint, StorageEnableEvents,
        TrieMutation,
    },
};
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};

use crate::{
    events::{
        FinalisedBspConfirmStoppedStoring, FinalisedTrieRemoveMutationsAppliedForBsp,
        ForestWriteLockTaskData, MoveBucketAccepted, MoveBucketExpired, MoveBucketRejected,
        MoveBucketRequested, MultipleNewChallengeSeeds, NewStorageRequest,
        ProcessBspConfirmStopStoring, ProcessBspConfirmStopStoringData,
        ProcessBspRequestStopStoring, ProcessBspRequestStopStoringData,
        ProcessConfirmStoringRequest, ProcessStopStoringForInsolventUserRequest,
        ProcessStopStoringForInsolventUserRequestData, ProcessSubmitProofRequest,
        ProcessSubmitProofRequestData,
    },
    handler::LOG_TARGET,
    state::BlockchainServiceStateStoreRwContext,
    types::{
        BspForestWriteQueue, BspForestWriteQueuePop, BspForestWriteWork,
        ConfirmBspStopStoringRequest, ConfirmStoringRequest, ForestWritePermitGuard,
        ManagedProvider, MultiInstancesNodeRole,
    },
    BlockchainService,
};

impl<FSH, Runtime> BlockchainService<FSH, Runtime>
where
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    /// Process mutation events during network initial sync.
    ///
    /// This is called for each sync block to apply `MutationsAppliedForProvider` events
    /// before state pruning can occur. This ensures the local forest stays in sync with
    /// the on-chain state even when the node has been offline for a long period.
    pub(crate) async fn process_bsp_sync_mutations(
        &mut self,
        block_hash: &Runtime::Hash,
        bsp_id: BackupStorageProviderId<Runtime>,
    ) {
        // Get all events for the block
        let events = match get_events_at_block::<Runtime>(&self.client, block_hash) {
            Ok(events) => events,
            Err(e) => {
                warn!(target: LOG_TARGET, "Failed to get events during sync: {:?}", e);
                return;
            }
        };

        // Apply any mutations in the block that are relevant to this BSP
        for ev in events {
            if let StorageEnableEvents::ProofsDealer(
                pallet_proofs_dealer::Event::MutationsAppliedForProvider {
                    provider_id,
                    mutations,
                    ..
                },
            ) = ev.event.into()
            {
                if provider_id == bsp_id {
                    debug!(target: LOG_TARGET, "Applying {} mutations during sync for BSP [{:?}]", mutations.len(), bsp_id);
                    let forest_key = CURRENT_FOREST_KEY.to_vec();
                    for (file_key, mutation) in mutations {
                        let mutation_type = match &mutation {
                            TrieMutation::Add(_) => "Add",
                            TrieMutation::Remove(_) => "Remove",
                        };
                        info!(
                            target: LOG_TARGET,
                            "🔧 Applying mutation [{}] for file key [{:?}] in BSP [{:?}]",
                            mutation_type, file_key, bsp_id
                        );
                        if let Err(e) = self
                            .apply_forest_mutation(forest_key.clone(), &file_key, &mutation)
                            .await
                        {
                            error!(target: LOG_TARGET, "CRITICAL ❗❗ Failed to apply mutation during sync: {:?}", e);
                        }
                    }
                }
            }
        }
    }

    /// Handles the initial sync of a BSP, after coming out of syncing mode.
    ///
    /// At this point, mutations have already been applied during sync via
    /// `process_bsp_sync_mutations`, so we:
    /// 1. Verify the local forest root matches the on-chain root
    /// 2. Catch up on proof submissions
    pub(crate) async fn bsp_initial_sync(
        &self,
        block_hash: Runtime::Hash,
        bsp_id: BackupStorageProviderId<Runtime>,
    ) {
        // Verify that the local forest root matches the on-chain root
        self.verify_bsp_forest_root(&block_hash, &bsp_id).await;

        // Catch up on proof submissions
        self.proof_submission_catch_up(&block_hash);
    }

    /// Initialises the block processing flow for a BSP.
    ///
    /// Steps:
    /// 1. Catch up to Forest root changes in this BSP's Forest.
    /// 2. In blocks that are a multiple of `BlockchainServiceConfig::check_for_pending_proofs_period`, catch up to proof submissions for the current tick.
    pub(crate) async fn bsp_init_block_processing<Block>(
        &mut self,
        block_hash: &Runtime::Hash,
        block_number: &BlockNumber<Runtime>,
        tree_route: TreeRoute<Block>,
    ) where
        Block: BlockT<Hash = Runtime::Hash>,
    {
        self.forest_root_changes_catchup(&tree_route).await;
        let block_number: U256 = (*block_number).into();
        if block_number % self.config.check_for_pending_proofs_period == Zero::zero() {
            self.proof_submission_catch_up(block_hash);
        }
    }

    /// Processes new block imported events that are only relevant for a BSP.
    pub(crate) fn bsp_process_block_import_events(
        &self,
        block_hash: &Runtime::Hash,
        event: StorageEnableEvents<Runtime>,
    ) {
        let managed_bsp_id = match &self.maybe_managed_provider {
            Some(ManagedProvider::Bsp(bsp_handler)) => &bsp_handler.bsp_id,
            _ => {
                error!(target: LOG_TARGET, "`bsp_process_block_events` should only be called if the node is managing a BSP. Found [{:?}] instead.", self.maybe_managed_provider);
                return;
            }
        };

        // Process the events that are common to all roles.
        match event {
            StorageEnableEvents::FileSystem(pallet_file_system::Event::MoveBucketRejected {
                bucket_id,
                old_msp_id,
                new_msp_id,
            }) => {
                self.emit(MoveBucketRejected {
                    bucket_id,
                    old_msp_id,
                    new_msp_id,
                });
            }
            StorageEnableEvents::FileSystem(pallet_file_system::Event::MoveBucketAccepted {
                bucket_id,
                old_msp_id,
                new_msp_id,
                value_prop_id,
            }) => {
                // As a BSP, this node is interested in the event to allow the new MSP to request files from it.
                self.emit(MoveBucketAccepted {
                    bucket_id,
                    old_msp_id,
                    new_msp_id,
                    value_prop_id,
                });
            }
            StorageEnableEvents::FileSystem(
                pallet_file_system::Event::MoveBucketRequestExpired { bucket_id },
            ) => {
                self.emit(MoveBucketExpired { bucket_id });
            }
            StorageEnableEvents::FileSystem(pallet_file_system::Event::MoveBucketRequested {
                who: _,
                bucket_id,
                new_msp_id,
                new_value_prop_id: _,
            }) => {
                // As a BSP, this node is interested in the event to allow the new MSP to request files from it.
                self.emit(MoveBucketRequested {
                    bucket_id,
                    new_msp_id,
                });
            }
            StorageEnableEvents::FileSystem(
                pallet_file_system::Event::BspRequestedToStopStoring {
                    bsp_id,
                    file_key,
                    owner: _,
                    location: _,
                },
            ) => {
                // Queue the confirm stop storing request if this is for our BSP.
                if managed_bsp_id == &bsp_id {
                    // Query MinWaitForStopStoring from runtime
                    let min_wait = match self
                        .client
                        .runtime_api()
                        .query_min_wait_for_stop_storing(self.current_block.hash)
                    {
                        Ok(min_wait) => min_wait,
                        Err(e) => {
                            error!(
                                target: LOG_TARGET,
                                "CRITICAL ❗❗ Failed to query MinWaitForStopStoring: {:?}.",
                                e
                            );
                            return;
                        }
                    };

                    // Calculate confirm_after_tick: current tick + min_wait + 1 (to be safe)
                    let current_tick = match self
                        .client
                        .runtime_api()
                        .get_current_tick(self.current_block.hash)
                    {
                        Ok(current_tick) => current_tick,
                        Err(e) => {
                            error!(
                                target: LOG_TARGET,
                                "CRITICAL ❗❗ Runtime API error while getting current tick: {:?}",
                                e
                            );
                            return;
                        }
                    };
                    let confirm_after_tick = current_tick + min_wait + 1u32.into();

                    info!(
                        target: LOG_TARGET,
                        "BspRequestedToStopStoring detected for file [{:?}]. \
                         Queueing confirm stop storing for tick {:?} (current: {:?}, min_wait: {:?})",
                        file_key,
                        confirm_after_tick,
                        current_tick,
                        min_wait
                    );

                    // Queue the confirm stop storing request
                    let request: ConfirmBspStopStoringRequest<Runtime> =
                        ConfirmBspStopStoringRequest::new(file_key.into(), confirm_after_tick);
                    let state_store_context = self.persistent_state.open_rw_context_with_overlay();
                    state_store_context
                        .pending_confirm_bsp_stop_storing_deque()
                        .push_back(request);
                    state_store_context.commit();
                }
            }
            // Ignore all other events.
            _ => {}
        }

        // Process the events that are specific to the role of the node.
        match self.role {
            MultiInstancesNodeRole::Leader | MultiInstancesNodeRole::Standalone => {
                match event {
                    // New storage request event coming from pallet-file-system.
                    StorageEnableEvents::FileSystem(
                        pallet_file_system::Event::NewStorageRequest {
                            who,
                            file_key,
                            bucket_id,
                            location,
                            fingerprint,
                            size,
                            peer_ids,
                            expires_at,
                        },
                    ) => {
                        self.emit(NewStorageRequest {
                            who,
                            file_key: FileKey::from(file_key.as_ref()),
                            bucket_id,
                            location,
                            fingerprint: Fingerprint::from(fingerprint.as_ref()),
                            size,
                            user_peer_ids: peer_ids,
                            expires_at,
                        });
                    }
                    StorageEnableEvents::ProofsDealer(
                        pallet_proofs_dealer::Event::NewChallengeSeed {
                            challenges_ticker,
                            seed: _,
                        },
                    ) => {
                        // Check if the challenges tick is one that this BSP has to submit a proof for.
                        if self.should_provider_submit_proof(
                            &block_hash,
                            managed_bsp_id,
                            &challenges_ticker,
                        ) {
                            self.proof_submission_catch_up(&block_hash);
                        } else {
                            trace!(target: LOG_TARGET, "Challenges tick is not the next one to be submitted for Provider [{:?}]", managed_bsp_id);
                        }
                    }
                    // Ignore all other events.
                    _ => {}
                }
            }
            MultiInstancesNodeRole::Follower => {
                trace!(target: LOG_TARGET, "No BSP block import events to process while in FOLLOWER role");
            }
        }
    }

    /// Runs at the end of every block import for a BSP.
    ///
    /// Steps:
    pub(crate) async fn bsp_end_block_processing<Block>(
        &self,
        _block_hash: &Runtime::Hash,
        _block_number: &BlockNumber<Runtime>,
        _tree_route: TreeRoute<Block>,
    ) where
        Block: BlockT<Hash = Runtime::Hash>,
    {
        // Nothing to do here so far.
    }

    /// Processes finality events that are only relevant for a BSP.
    pub(crate) fn bsp_process_finality_events(
        &self,
        _block_hash: &Runtime::Hash,
        event: StorageEnableEvents<Runtime>,
    ) {
        let managed_bsp_id = match &self.maybe_managed_provider {
            Some(ManagedProvider::Bsp(bsp_handler)) => &bsp_handler.bsp_id,
            _ => {
                error!(target: LOG_TARGET, "`bsp_process_finality_events` should only be called if the node is managing a BSP. Found [{:?}] instead.", self.maybe_managed_provider);
                return;
            }
        };

        // Process the events that are common to all roles.
        match event {
            StorageEnableEvents::ProofsDealer(
                pallet_proofs_dealer::Event::MutationsAppliedForProvider {
                    provider_id,
                    mutations,
                    old_root: _,
                    new_root,
                },
            ) => {
                // We only emit the event if the Provider ID is the one that this node is managing.
                if provider_id == *managed_bsp_id {
                    self.emit(FinalisedTrieRemoveMutationsAppliedForBsp {
                        provider_id,
                        mutations: mutations.clone().into(),
                        new_root,
                    })
                }
            }
            StorageEnableEvents::FileSystem(
                pallet_file_system::Event::BspConfirmStoppedStoring {
                    bsp_id,
                    file_key,
                    new_root,
                },
            ) => {
                if managed_bsp_id == &bsp_id {
                    self.emit(FinalisedBspConfirmStoppedStoring {
                        bsp_id,
                        file_key: file_key.into(),
                        new_root,
                    });
                }
            }
            // Ignore all other events.
            _ => {}
        }

        // Process the events that are specific to the role of the node.
        match self.role {
            MultiInstancesNodeRole::Leader
            | MultiInstancesNodeRole::Standalone
            | MultiInstancesNodeRole::Follower => {
                trace!(target: LOG_TARGET, "No BSP finality events to process exclusively while in LEADER, STANDALONE or FOLLOWER role");
            }
        }
    }

    /// Check if there are any pending requests to update the Forest root on the runtime, and process them.
    ///
    /// The priority is given by:
    /// 1. `SubmitProofRequest` over...
    /// 2. `ConfirmStoringRequest` over...
    /// 3. `StopStoringForInsolventUserRequest`.
    ///
    /// This function is called every time a new block is imported and after each request is queued.
    ///
    /// _IMPORTANT: This check will be skipped if the block currently being processed does not match the client's best block._
    pub(crate) fn bsp_assign_forest_root_write_lock(&mut self) {
        let client_best_hash = self.client.info().best_hash;
        let client_best_number = self.client.info().best_number;

        // Skip if the block currently being processed doesn't match the client's best block
        if self.current_block.hash != client_best_hash
            || self.current_block.number != client_best_number.into()
        {
            trace!(target: LOG_TARGET, "Skipping Forest root write lock assignment because block currently being processed does not match client's best block (current block hash and number [{}, {}], client best block hash and number [{}, {}])", self.current_block.hash, self.current_block.number, client_best_hash, client_best_number);
            return;
        }

        // Verify we have a BSP handler.
        let managed_bsp_id = match &self.maybe_managed_provider {
            Some(ManagedProvider::Bsp(bsp_handler)) => {
                // Return early if the semaphore has no available permits.
                if bsp_handler.forest_root_write_semaphore.available_permits() == 0 {
                    trace!(target: LOG_TARGET, "Forest root write semaphore already acquired. Skipping assignment.");
                    return;
                }
                bsp_handler.bsp_id
            }
            _ => {
                error!(target: LOG_TARGET, "`bsp_check_pending_forest_root_writes` should only be called if the node is managing a BSP. Found [{:?}] instead.", self.maybe_managed_provider);
                return;
            }
        };

        // If there is no pending work, do NOT acquire the semaphore permit and return early.
        //
        // This avoids a subtle event-loop spin: acquiring + immediately dropping the permit
        // would notify `permit_release_receiver`, which would call back into this method
        // even though there is no work to do.
        {
            // Use a temporary state context just for checking pending work (no mutations).
            let temp_state = self.persistent_state.open_rw_context_with_overlay();
            if !Self::bsp_forest_write_work(&mut self.maybe_managed_provider, &temp_state, None)
                .has_pending_work
            {
                trace!(target: LOG_TARGET, "No pending BSP forest-write work; skipping semaphore acquisition");
                return;
            }
        }

        // Try to acquire a permit from the semaphore.
        // If the permit is unavailable, another task is still processing and we return early.
        let permit = {
            let semaphore = match &self.maybe_managed_provider {
                Some(ManagedProvider::Bsp(bsp_handler)) => {
                    bsp_handler.forest_root_write_semaphore.clone()
                }
                _ => unreachable!("We just checked this is a BSP"),
            };
            match semaphore.try_acquire_owned() {
                Ok(permit) => {
                    trace!(target: LOG_TARGET, "Forest root write semaphore permit acquired");
                    permit
                }
                Err(_) => {
                    trace!(target: LOG_TARGET, "Waiting for current Forest root write task to finish");
                    return;
                }
            }
        };

        // At this point we know that the lock is released and we can start processing new requests.
        let state_store_context = self.persistent_state.open_rw_context_with_overlay();
        let mut next_event_data = None;

        // Process SubmitProof requests one at a time. Pop from the queue, then validate.
        // If validation fails, continue to pop the next request.
        'submit_proof_requests_loop: loop {
            // Pop the next SubmitProof request from the queue.
            let popped = Self::bsp_forest_write_work(
                &mut self.maybe_managed_provider,
                &state_store_context,
                Some(BspForestWriteQueue::SubmitProof),
            )
            .popped;

            let Some(BspForestWriteQueuePop::SubmitProof(request)) = popped else {
                break;
            };

            // Check if the proof is still the next one to be submitted.
            let next_challenge_tick = match self
                .get_next_challenge_tick_for_provider(&managed_bsp_id)
            {
                Ok(next_challenge_tick) => next_challenge_tick,
                Err(e) => {
                    error!(target: LOG_TARGET, "Failed to get next challenge tick for provider [{:?}]: {:?}", managed_bsp_id, e);
                    break 'submit_proof_requests_loop;
                }
            };

            // This is to avoid starting a new task if the proof is not the next one to be submitted.
            if next_challenge_tick != request.tick {
                // If the proof is not the next one to be submitted, we can skip it
                trace!(target: LOG_TARGET, "Proof for tick [{:?}] is not the next one to be submitted. Skipping it.", request.tick);
                continue 'submit_proof_requests_loop;
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
            break 'submit_proof_requests_loop;
        }

        // If we have no pending SubmitProof requests, check for pending ConfirmStoring requests.
        if next_event_data.is_none() {
            if let Some(BspForestWriteQueuePop::ConfirmStoring) = Self::bsp_forest_write_work(
                &mut self.maybe_managed_provider,
                &state_store_context,
                Some(BspForestWriteQueue::ConfirmStoring),
            )
            .popped
            {
                trace!(target: LOG_TARGET, "Triggering ProcessConfirmStoringRequest");
                next_event_data = Some(ForestWriteLockTaskData::ConfirmStoringRequest);
            }
        }

        // If we have no pending storage requests to respond to, check for pending StopStoringForInsolventUser requests.
        if next_event_data.is_none() {
            // Pop the next StopStoringForInsolventUser request from the queue.
            if let Some(BspForestWriteQueuePop::StopStoringForInsolventUser(request)) =
                Self::bsp_forest_write_work(
                    &mut self.maybe_managed_provider,
                    &state_store_context,
                    Some(BspForestWriteQueue::StopStoringForInsolventUser),
                )
                .popped
            {
                next_event_data = Some(
                    ProcessStopStoringForInsolventUserRequestData { who: request.user }.into(),
                );
            }
        }

        // If we have no pending stop storing for insolvent user requests, check for pending BspRequestStopStoring requests.
        if next_event_data.is_none() {
            if let Some(request) = state_store_context
                .pending_request_bsp_stop_storing_deque::<Runtime>()
                .pop_front()
            {
                trace!(target: LOG_TARGET, "Processing BSP request stop storing for file [{:?}]", request.file_key);
                next_event_data = Some(ProcessBspRequestStopStoringData { request }.into());
            }
        }

        // If we have no pending BspRequestStopStoring requests, check for pending BspConfirmStopStoring requests.
        // Items in this queue are ordered chronologically by confirm_after_tick, so if the first
        // item's tick hasn't been reached, none of the others have either.
        if next_event_data.is_none() {
            let current_tick = match self
                .client
                .runtime_api()
                .get_current_tick(self.current_block.hash)
            {
                Ok(current_tick) => current_tick,
                Err(e) => {
                    error!(
                        target: LOG_TARGET,
                        "CRITICAL ❗❗ Runtime API error while getting current tick: {:?}",
                        e
                    );
                    return;
                }
            };
            // Peek first to check if the tick has been reached without modifying the queue
            if let Some(peeked) = state_store_context
                .pending_confirm_bsp_stop_storing_deque::<Runtime>()
                .peek_front()
            {
                if peeked.confirm_after_tick <= current_tick {
                    // Tick reached, pop and process this request
                    let request = state_store_context
                        .pending_confirm_bsp_stop_storing_deque::<Runtime>()
                        .pop_front()
                        .expect("Just peeked, should exist");
                    trace!(
                        target: LOG_TARGET,
                        "Processing BSP confirm stop storing for file [{:?}], confirm_after_tick: {:?}, current_tick: {:?}",
                        request.file_key,
                        request.confirm_after_tick,
                        current_tick
                    );
                    next_event_data = Some(ProcessBspConfirmStopStoringData { request }.into());
                }
                // If tick not reached, do nothing since no other items can be ready either
            }
        }

        // Commit the state store context.
        state_store_context.commit();

        // If there is any event data to process, emit the event.
        if let Some(event_data) = next_event_data {
            // Only wrap the semaphore permit in the notifying guard when we actually emit an event.
            // This ensures the drop notification corresponds to a task completion, not an idle scan.
            let forest_root_write_permit = Arc::new(ForestWritePermitGuard::new(
                permit,
                self.permit_release_sender.clone(),
            ));
            self.bsp_emit_forest_write_event(event_data, forest_root_write_permit);
        }
    }

    /// Checks for pending BSP forest-write work and optionally pops from a queue.
    ///
    /// If `pop` is `Some`, pops from that queue and returns whether it succeeded.
    /// If `pop` is `None`, checks all queues for pending work without modifying state.
    /// Caller must commit the state context after all operations.
    fn bsp_forest_write_work<'a>(
        maybe_managed_provider: &mut Option<ManagedProvider<Runtime>>,
        state: &'a BlockchainServiceStateStoreRwContext<'a>,
        pop: Option<BspForestWriteQueue>,
    ) -> BspForestWriteWork<Runtime> {
        // Only check the relevant queue based on which pop is requested.
        let (has_pending_work, popped) = match pop {
            Some(BspForestWriteQueue::SubmitProof) => {
                let bsp_handler = match maybe_managed_provider {
                    Some(ManagedProvider::Bsp(bsp_handler)) => bsp_handler,
                    _ => unreachable!("bsp_forest_write_work should only be called for BSP"),
                };
                let popped = bsp_handler
                    .pending_submit_proof_requests
                    .pop_first()
                    .map(BspForestWriteQueuePop::SubmitProof);
                (popped.is_some(), popped)
            }
            Some(BspForestWriteQueue::ConfirmStoring) => {
                // Check only (no pop). The task pulls requests via commands.
                let has_pending = state
                    .pending_confirm_storing_request_deque::<Runtime>()
                    .size()
                    > 0;
                let signal = if has_pending {
                    Some(BspForestWriteQueuePop::ConfirmStoring)
                } else {
                    None
                };
                (has_pending, signal)
            }
            Some(BspForestWriteQueue::StopStoringForInsolventUser) => {
                let popped = state
                    .pending_stop_storing_for_insolvent_user_request_deque::<Runtime>()
                    .pop_front()
                    .map(BspForestWriteQueuePop::StopStoringForInsolventUser);
                (popped.is_some(), popped)
            }
            None => {
                // Check all queues when no pop is requested.
                let has_pending_submit_proof = match maybe_managed_provider {
                    Some(ManagedProvider::Bsp(bsp_handler)) => {
                        !bsp_handler.pending_submit_proof_requests.is_empty()
                    }
                    _ => unreachable!("bsp_forest_write_work should only be called for BSP"),
                };
                let has_pending_confirm = state
                    .pending_confirm_storing_request_deque::<Runtime>()
                    .size()
                    > 0;
                let has_pending_stop = state
                    .pending_stop_storing_for_insolvent_user_request_deque::<Runtime>()
                    .size()
                    > 0;
                (
                    has_pending_submit_proof || has_pending_confirm || has_pending_stop,
                    None,
                )
            }
        };

        BspForestWriteWork {
            has_pending_work,
            popped,
        }
    }

    pub(crate) async fn bsp_process_forest_root_changing_events(
        &mut self,
        event: StorageEnableEvents<Runtime>,
        revert: bool,
    ) {
        let managed_bsp_id = match &self.maybe_managed_provider {
            Some(ManagedProvider::Bsp(bsp_handler)) => &bsp_handler.bsp_id,
            _ => {
                error!(target: LOG_TARGET, "`bsp_process_forest_root_changing_events` should only be called if the node is managing a BSP. Found [{:?}] instead.", self.maybe_managed_provider);
                return;
            }
        };

        match event {
            StorageEnableEvents::ProofsDealer(
                pallet_proofs_dealer::Event::MutationsAppliedForProvider {
                    provider_id,
                    mutations,
                    old_root,
                    new_root,
                },
            ) => {
                // Check if the `provider_id` is the BSP that this node is managing.
                if provider_id != *managed_bsp_id {
                    debug!(target: LOG_TARGET, "Provider ID [{:?}] is not the BSP ID [{:?}] that this node is managing. Skipping mutations applied event.", provider_id, managed_bsp_id);
                    return;
                }

                info!(target: LOG_TARGET, "🪾 Applying mutations to BSP [{:?}]", provider_id);
                debug!(target: LOG_TARGET, "Mutations: {:?}", mutations);

                // Log mutations at info level during catchup/sync for better visibility
                if !self.caught_up {
                    let action = if revert { "Reverting" } else { "Applying" };
                    for (file_key, mutation) in &mutations {
                        let mutation_type = match mutation {
                            TrieMutation::Add(_) => "Add",
                            TrieMutation::Remove(_) => "Remove",
                        };
                        info!(
                            target: LOG_TARGET,
                            "🔧 {} mutation [{}] for file key [{:?}] in BSP [{:?}]",
                            action, mutation_type, file_key, provider_id
                        );
                    }
                }

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
            _ => {}
        }
    }

    /// Verifies that the local BSP forest root matches the on-chain root.
    ///
    /// This is a sanity check after coming out of sync to ensure mutations were
    /// correctly applied during the sync process.
    async fn verify_bsp_forest_root(
        &self,
        block_hash: &Runtime::Hash,
        bsp_id: &BackupStorageProviderId<Runtime>,
    ) {
        // Get the local forest root
        let local_root = match self
            .forest_storage_handler
            .get(&CURRENT_FOREST_KEY.to_vec().into())
            .await
        {
            Some(fs) => fs.read().await.root(),
            None => {
                warn!(target: LOG_TARGET, "BSP forest storage not found during initial sync verification");
                return;
            }
        };

        // Get the on-chain root from runtime API
        let onchain_root = match self.client.runtime_api().get_bsp_info(*block_hash, bsp_id) {
            Ok(Ok(bsp_info)) => bsp_info.root,
            Ok(Err(e)) => {
                error!(target: LOG_TARGET, "Failed to get BSP info from runtime: {:?}", e);
                return;
            }
            Err(e) => {
                error!(target: LOG_TARGET, "Runtime API call failed for get_bsp_info: {:?}", e);
                return;
            }
        };

        // Compare roots
        if local_root != onchain_root {
            error!(
                    target: LOG_TARGET,
                    "❌ CRITICAL: BSP forest root mismatch after sync! Local: {:?}, On-chain: {:?}. \
                     This BSP may fail to generate valid proofs.",
                    local_root, onchain_root
            );
        } else {
            info!(
                    target: LOG_TARGET,
                    "✅ BSP forest root verified after sync. Root: {:?}",
                    local_root
            );
        }
    }

    /// Emits a [`MultipleNewChallengeSeeds`] event with all the pending proof submissions for this provider.
    /// This is used to catch up to the latest proof submissions that were missed due to a node restart.
    /// Also, it can help to catch up to proofs in case there is a change in the BSP's stake (therefore
    /// also a change in it's challenge period).
    ///
    /// IMPORTANT: This function takes into account whether a proof should be submitted for the current tick.
    fn proof_submission_catch_up(&self, current_block_hash: &Runtime::Hash) {
        let bsp_id = match &self.maybe_managed_provider {
            Some(ManagedProvider::Bsp(bsp_handler)) => &bsp_handler.bsp_id,
            _ => {
                error!(target: LOG_TARGET, "`proof_submission_catch_up` should only be called if the node is managing a BSP. Found [{:?}] instead.", self.maybe_managed_provider);
                return;
            }
        };

        // Get the current challenge period for this provider.
        let challenge_period = match self
            .client
            .runtime_api()
            .get_challenge_period(*current_block_hash, bsp_id)
        {
            Ok(challenge_period_result) => match challenge_period_result {
                Ok(challenge_period) => challenge_period,
                Err(e) => match e {
                    GetChallengePeriodError::ProviderNotRegistered => {
                        debug!(target: LOG_TARGET, "Provider [{:?}] is not registered", bsp_id);
                        return;
                    }
                    GetChallengePeriodError::InternalApiError => {
                        error!(target: LOG_TARGET, "This should be impossible, we just checked the API error. \nInternal API error while getting challenge period for Provider [{:?}]", bsp_id);
                        return;
                    }
                },
            },
            Err(e) => {
                error!(target: LOG_TARGET, "Runtime API error while getting challenge period for Provider [{:?}]: {:?}", bsp_id, e);
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
                error!(target: LOG_TARGET, "Runtime API error while getting current tick for Provider [{:?}]: {:?}", bsp_id, e);
                return;
            }
        };

        // Advance by `challenge_period` ticks and add the seed to the list of challenge seeds.
        let mut challenge_seeds = Vec::new();
        let mut next_challenge_tick = match Self::get_next_challenge_tick_for_provider(
            &self, bsp_id,
        ) {
            Ok(next_challenge_tick) => next_challenge_tick,
            Err(e) => {
                error!(target: LOG_TARGET, "Failed to get next challenge tick for provider [{:?}]: {:?}", bsp_id, e);
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

        if challenge_seeds.len() > 0 {
            trace!(target: LOG_TARGET, "Emitting MultipleNewChallengeSeeds event for provider [{:?}] with challenge seeds: {:?}", bsp_id, challenge_seeds);
            self.emit(MultipleNewChallengeSeeds {
                provider_id: *bsp_id,
                seeds: challenge_seeds,
            });
        }
    }

    fn bsp_emit_forest_write_event(
        &self,
        data: impl Into<ForestWriteLockTaskData<Runtime>>,
        forest_root_write_permit: Arc<ForestWritePermitGuard>,
    ) {
        match data.into() {
            ForestWriteLockTaskData::SubmitProofRequest(data) => {
                self.emit(ProcessSubmitProofRequest {
                    data,
                    forest_root_write_permit,
                });
            }
            ForestWriteLockTaskData::ConfirmStoringRequest => {
                self.emit(ProcessConfirmStoringRequest::new(forest_root_write_permit));
            }
            ForestWriteLockTaskData::StopStoringForInsolventUserRequest(data) => {
                self.emit(ProcessStopStoringForInsolventUserRequest {
                    data,
                    forest_root_write_permit,
                });
            }
            ForestWriteLockTaskData::BspRequestStopStoring(data) => {
                self.emit(ProcessBspRequestStopStoring {
                    data,
                    forest_root_write_permit,
                });
            }
            ForestWriteLockTaskData::BspConfirmStopStoring(data) => {
                self.emit(ProcessBspConfirmStopStoring {
                    data,
                    forest_root_write_permit,
                });
            }
            ForestWriteLockTaskData::MspRespondStorageRequest(_) => {
                unreachable!("BSPs do not respond to storage requests as MSPs do.")
            }
        }
    }

    /// Pop up to `count` confirm storing requests from the persistent deque.
    ///
    /// Returns the items without filtering; the caller is responsible for
    /// filtering stale requests and re-queuing if needed.
    pub(crate) fn pop_confirm_storing_requests(
        &self,
        count: u32,
    ) -> Vec<ConfirmStoringRequest<Runtime>> {
        let state_store_context = self.persistent_state.open_rw_context_with_overlay();
        let mut deque = state_store_context.pending_confirm_storing_request_deque::<Runtime>();
        let mut popped = Vec::new();
        for _ in 0..count {
            if let Some(request) = deque.pop_front() {
                popped.push(request);
            } else {
                break;
            }
        }
        state_store_context.commit();
        popped
    }

    /// Filter confirm storing requests by checking on-chain state and pending
    /// volunteer transactions.
    ///
    /// Returns `(ready, pending_volunteer)` where:
    /// - `ready`: requests whose BSP has volunteered and are still pending confirmation on-chain.
    /// - `pending_volunteer`: requests whose volunteer transaction has not yet landed on-chain.
    ///
    /// This does **not** re-queue pending volunteer requests.
    pub(crate) fn filter_confirm_storing_requests(
        &self,
        requests: Vec<ConfirmStoringRequest<Runtime>>,
    ) -> anyhow::Result<(
        Vec<ConfirmStoringRequest<Runtime>>,
        Vec<ConfirmStoringRequest<Runtime>>,
    )> {
        let (managed_bsp_id, pending_volunteer_file_keys) = match &self.maybe_managed_provider {
            Some(ManagedProvider::Bsp(bsp_handler)) => (
                bsp_handler.bsp_id.clone(),
                &bsp_handler.pending_volunteer_file_keys,
            ),
            _ => {
                anyhow::bail!(
                    "`filter_confirm_storing_requests` should only be called if the node is managing a BSP. Found [{:?}] instead.",
                    self.maybe_managed_provider,
                );
            }
        };

        // Separate requests with pending volunteer transactions from those ready to query.
        let (pending_volunteer, requests_to_query): (Vec<_>, Vec<_>) =
            requests.into_iter().partition(|request| {
                let file_key: FileKey = request.file_key.as_ref().into();
                pending_volunteer_file_keys.contains(&file_key)
            });

        let current_block_hash = self.client.info().best_hash;

        let file_keys: Vec<H256> = requests_to_query
            .iter()
            .map(|r| H256::from_slice(r.file_key.as_ref()))
            .collect();

        // Query the runtime API to filter file keys to only those pending confirmation.
        let pending_file_keys = self
            .client
            .runtime_api()
            .query_pending_bsp_confirm_storage_requests(
                current_block_hash,
                managed_bsp_id,
                file_keys,
            )
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to query pending BSP confirm storage requests: {:?}",
                    e
                )
            })?;

        let pending_file_keys_set: HashSet<FileKey> = pending_file_keys
            .into_iter()
            .map(|k| k.as_ref().into())
            .collect();

        // Filter to only those that are still pending confirmation.
        let pending_confirmation: Vec<_> = requests_to_query
            .into_iter()
            .filter(|request| {
                let file_key: FileKey = request.file_key.as_ref().into();
                pending_file_keys_set.contains(&file_key)
            })
            .collect();

        Ok((pending_confirmation, pending_volunteer))
    }
}
