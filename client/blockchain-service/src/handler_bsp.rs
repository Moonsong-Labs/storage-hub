use log::{debug, error, info, trace};
use shc_common::traits::StorageEnableRuntime;
use std::sync::Arc;
use tokio::sync::{oneshot::error::TryRecvError, Mutex};

use sc_client_api::HeaderBackend;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::TreeRoute;
use sp_core::{Get, U256};
use sp_runtime::traits::{Block as BlockT, Zero};

use pallet_proofs_dealer_runtime_api::{
    GetChallengePeriodError, GetChallengeSeedError, ProofsDealerApi,
};
use shc_actors_framework::actor::Actor;
use shc_common::{
    consts::CURRENT_FOREST_KEY,
    typed_store::CFDequeAPI,
    types::{BlockNumber, MaxBatchConfirmStorageRequests, StorageEnableEvents},
};
use shc_forest_manager::traits::ForestStorageHandler;

use crate::{
    events::{
        BspConfirmStoppedStoring, FinalisedBspConfirmStoppedStoring,
        FinalisedTrieRemoveMutationsApplied, ForestWriteLockTaskData, MoveBucketAccepted,
        MoveBucketExpired, MoveBucketRejected, MoveBucketRequested, MultipleNewChallengeSeeds,
        ProcessConfirmStoringRequest, ProcessConfirmStoringRequestData,
        ProcessStopStoringForInsolventUserRequest, ProcessStopStoringForInsolventUserRequestData,
        ProcessSubmitProofRequest, ProcessSubmitProofRequestData,
    },
    handler::LOG_TARGET,
    types::ManagedProvider,
    BlockchainService,
};

impl<FSH, Runtime> BlockchainService<FSH, Runtime>
where
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    /// Handles the initial sync of a BSP, after coming out of syncing mode.
    ///
    /// Steps:
    /// 1. Catch up to the latest proof submissions that were missed due to a node restart.
    pub(crate) fn bsp_initial_sync(&self) {
        self.proof_submission_catch_up(&self.client.info().best_hash);
        // TODO: Send events to check that this node has a Forest Storage for the BSP that it manages.
        // TODO: Catch up to Forest root writes in the BSP Forest.
    }

    /// Initialises the block processing flow for a BSP.
    ///
    /// Steps:
    /// 1. Catch up to Forest root changes in this BSP's Forest.
    /// 2. In blocks that are a multiple of `BlockchainServiceConfig::check_for_pending_proofs_period`, catch up to proof submissions for the current tick.
    pub(crate) async fn bsp_init_block_processing<Block>(
        &self,
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

        match event {
            StorageEnableEvents::ProofsDealer(pallet_proofs_dealer::Event::NewChallengeSeed {
                challenges_ticker,
                seed: _,
            }) => {
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
            StorageEnableEvents::FileSystem(
                pallet_file_system::Event::BspConfirmStoppedStoring {
                    bsp_id,
                    file_key,
                    new_root,
                },
            ) => {
                if managed_bsp_id == &bsp_id {
                    self.emit(BspConfirmStoppedStoring {
                        bsp_id,
                        file_key: file_key.into(),
                        new_root,
                    });
                }
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
            // Ignore all other events.
            _ => {}
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
                    self.emit(FinalisedTrieRemoveMutationsApplied {
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
    /// _IMPORTANT: This check will be skipped if the latest processed block does not match the current best block._
    pub(crate) fn bsp_assign_forest_root_write_lock(&mut self) {
        let client_best_hash = self.client.info().best_hash;
        let client_best_number = self.client.info().best_number;

        // Skip if the latest processed block doesn't match the current best block
        if self.best_block.hash != client_best_hash
            || self.best_block.number != client_best_number.into()
        {
            trace!(target: LOG_TARGET, "Skipping Forest root write lock assignment because latest processed block does not match current best block (local block hash and number [{}, {}], best block hash and number [{}, {}])", self.best_block.hash, self.best_block.number, client_best_hash, client_best_number);
            return;
        }

        // Verify we have a BSP handler.
        let managed_bsp_id = match &self.maybe_managed_provider {
            Some(ManagedProvider::Bsp(bsp_handler)) => bsp_handler.bsp_id,
            _ => {
                error!(target: LOG_TARGET, "`bsp_check_pending_forest_root_writes` should only be called if the node is managing a BSP. Found [{:?}] instead.", self.maybe_managed_provider);
                return;
            }
        };

        // This is done in a closure to avoid borrowing `self` immutably and then mutably.
        // Inside of this closure, we borrow `self` mutably when taking ownership of the lock.
        {
            let forest_root_write_lock = match &mut self.maybe_managed_provider {
                Some(ManagedProvider::Bsp(bsp_handler)) => &mut bsp_handler.forest_root_write_lock,
                _ => unreachable!("We just checked this is a BSP"),
            };
            if let Some(rx) = forest_root_write_lock.as_mut() {
                // Note: tasks that get ownership of the lock are responsible for sending a message back when done processing.
                match rx.try_recv() {
                    // If the channel is empty, means we still need to wait for the current task to finish.
                    Err(TryRecvError::Empty) => {
                        trace!(target: LOG_TARGET, "Waiting for current Forest root write task to finish");
                        return;
                    }
                    Ok(_) => {
                        trace!(target: LOG_TARGET, "Forest root write task finished, lock is released!");
                    }
                    Err(TryRecvError::Closed) => {
                        error!(target: LOG_TARGET, "Forest root write task channel closed unexpectedly. Lock is released anyway!");
                    }
                }
            } else {
                info!(target: LOG_TARGET, "Forest root write lock not assigned to any task");
            }

            // Clear the lock, given that at this point we know that the lock is released.
            *forest_root_write_lock = None;
        }

        // At this point we know that the lock is released and we can start processing new requests.
        let state_store_context = self.persistent_state.open_rw_context_with_overlay();
        let mut next_event_data = None;

        // Process proof requests one at a time, releasing the mutable borrow between iterations.
        // This is because when matching on `maybe_managed_provider`, we need to borrow it mutably
        // to remove the request from the pending list. This mutable borrow is released after used
        // and before needing to borrow immutably again when calling `self.get_next_challenge_tick_for_provider`.
        'submit_proof_requests_loop: loop {
            // Get the next request if any. Mutable borrow of `maybe_managed_provider` is released after use.
            let request = match &mut self.maybe_managed_provider {
                Some(ManagedProvider::Bsp(bsp_handler)) => {
                    bsp_handler.pending_submit_proof_requests.pop_first()
                }
                _ => unreachable!("We just checked this is a BSP"),
            };

            // If there is no request, break the loop.
            let Some(request) = request else { break };

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

        // If we have no pending submit proof requests, we can also check for pending confirm storing requests.
        if next_event_data.is_none() {
            let max_batch_confirm = <MaxBatchConfirmStorageRequests<Runtime> as Get<u32>>::get();

            // Batch multiple confirm file storing taking the runtime maximum.
            let mut confirm_storing_requests = Vec::new();
            for _ in 0..max_batch_confirm {
                if let Some(request) = state_store_context
                    .pending_confirm_storing_request_deque::<Runtime>()
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

        // If we have no pending storage requests to respond to, we can also check for pending stop storing for insolvent user requests.
        if next_event_data.is_none() {
            if let Some(request) = state_store_context
                .pending_stop_storing_for_insolvent_user_request_deque::<Runtime>()
                .pop_front()
            {
                next_event_data = Some(
                    ProcessStopStoringForInsolventUserRequestData { who: request.user }.into(),
                );
            }
        }

        // Commit the state store context.
        state_store_context.commit();

        // If there is any event data to process, emit the event.
        if let Some(event_data) = next_event_data {
            self.bsp_emit_forest_write_event(event_data);
        }
    }

    pub(crate) async fn bsp_process_forest_root_changing_events(
        &self,
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

                debug!(target: LOG_TARGET, "Applying on-chain Forest root mutations to BSP [{:?}]", provider_id);
                debug!(target: LOG_TARGET, "Mutations: {:?}", mutations);

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

        // Emit the `MultipleNewChallengeSeeds` event.
        if challenge_seeds.len() > 0 {
            trace!(target: LOG_TARGET, "Emitting MultipleNewChallengeSeeds event for provider [{:?}] with challenge seeds: {:?}", bsp_id, challenge_seeds);
            self.emit(MultipleNewChallengeSeeds {
                provider_id: *bsp_id,
                seeds: challenge_seeds,
            });
        }
    }

    fn bsp_emit_forest_write_event(&mut self, data: impl Into<ForestWriteLockTaskData<Runtime>>) {
        // Get the BSP's Forest root write lock.
        let forest_root_write_lock = match &mut self.maybe_managed_provider {
            Some(ManagedProvider::Bsp(bsp_handler)) => &mut bsp_handler.forest_root_write_lock,
            _ => {
                error!(target: LOG_TARGET, "`bsp_emit_forest_write_event` should only be called if the node is managing a BSP. Found [{:?}] instead.", self.maybe_managed_provider);
                return;
            }
        };

        // Create a new channel to assign ownership of the BSP's Forest root write lock.
        let (tx, rx) = tokio::sync::oneshot::channel();
        *forest_root_write_lock = Some(rx);

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
            ForestWriteLockTaskData::StopStoringForInsolventUserRequest(data) => {
                self.emit(ProcessStopStoringForInsolventUserRequest {
                    data,
                    forest_root_write_tx,
                });
            }
            ForestWriteLockTaskData::MspRespondStorageRequest(_) => {
                unreachable!("BSPs do not respond to storage requests as MSPs do.")
            }
            ForestWriteLockTaskData::FileDeletionRequest(_) => {
                unreachable!("BSPs do not respond to file deletions as MSPs do.")
            }
        }
    }
}
