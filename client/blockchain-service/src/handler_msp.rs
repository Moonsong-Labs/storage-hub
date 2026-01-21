use anyhow::Result;
use log::{debug, error, info, trace, warn};
use std::{collections::HashSet, str, sync::Arc};
use tokio::sync::{oneshot::error::TryRecvError, Mutex};

use sc_client_api::HeaderBackend;
use sc_network_types::PeerId;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::TreeRoute;
use sp_core::Get;
use sp_runtime::traits::Block as BlockT;

use pallet_file_system_runtime_api::FileSystemApi;
use pallet_storage_providers_runtime_api::StorageProvidersApi;
use shc_actors_framework::actor::Actor;
use shc_common::{
    blockchain_utils::get_events_at_block,
    traits::StorageEnableRuntime,
    typed_store::CFDequeAPI,
    types::{
        BackupStorageProviderId, BlockHash, BlockNumber, BucketId, DefaultMerkleRoot, FileKey,
        MainStorageProviderId, ProviderId, StorageEnableEvents, TrieMutation,
    },
};
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};

use crate::{
    events::{
        DistributeFileToBsp, FinalisedBucketMovedAway, FinalisedBucketMutationsApplied,
        FinalisedMspStopStoringBucketInsolventUser, FinalisedMspStoppedStoringBucket,
        FinalisedStorageRequestRejected, ForestWriteLockTaskData, MoveBucketRequestedForMsp,
        NewStorageRequest, ProcessMspRespondStoringRequest, ProcessMspRespondStoringRequestData,
        ProcessStopStoringForInsolventUserRequest, ProcessStopStoringForInsolventUserRequestData,
        StartMovedBucketDownload,
    },
    handler::LOG_TARGET,
    types::{FileDistributionInfo, FileKeyStatus, ManagedProvider, MultiInstancesNodeRole},
    BlockchainService,
};

// TODO: Make this configurable in the config file
const MAX_BATCH_MSP_RESPOND_STORE_REQUESTS: u32 = 100;

impl<FSH, Runtime> BlockchainService<FSH, Runtime>
where
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    /// Process mutation events during network initial sync.
    ///
    /// This is called for each sync block to apply `MutationsApplied` events
    /// for buckets managed by this MSP before state pruning can occur.
    /// This ensures the local bucket forests stay in sync with the on-chain state
    /// even when the node has been offline for a long period.
    pub(crate) async fn process_msp_sync_mutations(
        &mut self,
        block_hash: &Runtime::Hash,
        msp_id: ProviderId<Runtime>,
    ) -> Result<()> {
        // Get all events for the block.
        let events = get_events_at_block::<Runtime>(&self.client, block_hash)?;

        // Apply any mutations in the block that are relevant to this MSP
        for ev in events {
            if let StorageEnableEvents::ProofsDealer(
                pallet_proofs_dealer::Event::MutationsApplied {
                    mutations,
                    event_info,
                    ..
                },
            ) = ev.event.clone().into()
            {
                // Decode the bucket ID from the event info
                let bucket_id = match self
                    .get_bucket_id_from_mutations_applied_event_info(block_hash, event_info)
                {
                    Ok(bucket_id) => bucket_id,
                    Err(e) => {
                        error!(target: LOG_TARGET, "Failed to get bucket ID from MutationsApplied event info: {:?}", e);
                        return Err(e);
                    }
                };

                // Check if this bucket is managed by this MSP
                if !self.validate_bucket_mutations_for_msp(block_hash, &msp_id, &bucket_id) {
                    trace!(target: LOG_TARGET, "Bucket [0x{:x}] is not managed by this MSP [0x{:x}]. Skipping mutations applied event.", bucket_id, msp_id);
                    continue;
                }

                debug!(target: LOG_TARGET, "Applying {} mutations during sync for bucket [0x{:x}]", mutations.len(), bucket_id);
                let forest_key = bucket_id.as_ref().to_vec();
                for (file_key, mutation) in mutations {
                    let mutation_type = match &mutation {
                        TrieMutation::Add(_) => "Add",
                        TrieMutation::Remove(_) => "Remove",
                    };
                    info!(
                        target: LOG_TARGET,
                        "🔧 Applying mutation [{}] for file key [{:x}] in bucket [0x{:x}]",
                        mutation_type, file_key, bucket_id
                    );
                    if let Err(e) = self
                        .apply_forest_mutation(forest_key.clone(), &file_key, &mutation)
                        .await
                    {
                        error!(target: LOG_TARGET, "CRITICAL ❗❗ Failed to apply mutation during sync for bucket [0x{:x}]: {:?}", bucket_id, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Handles the initial sync of a MSP, after coming out of syncing mode.
    ///
    /// At this point, mutations have already been applied during sync via
    /// `process_msp_sync_mutations`, so we:
    /// 1. Verify all bucket forest roots match the on-chain roots
    /// 2. Emit pending storage requests
    pub(crate) async fn msp_initial_sync(
        &mut self,
        block_hash: Runtime::Hash,
        msp_id: ProviderId<Runtime>,
    ) {
        // Verify all bucket forest roots match their on-chain roots
        self.verify_msp_bucket_roots(&block_hash, &msp_id).await;

        // Emit pending storage requests
        self.handle_pending_storage_requests(&block_hash, msp_id);
    }

    /// Initialises the block processing flow for a MSP.
    ///
    /// Steps:
    /// 1. Catch up to Forest root changes in the Forests of the Buckets this MSP manages.
    pub(crate) async fn msp_init_block_processing<Block>(
        &mut self,
        _block_hash: &Runtime::Hash,
        _block_number: &BlockNumber<Runtime>,
        tree_route: TreeRoute<Block>,
    ) -> Result<()>
    where
        Block: BlockT<Hash = Runtime::Hash>,
    {
        self.forest_root_changes_catchup(&tree_route).await?;
        Ok(())
    }

    /// Processes new block imported events that are only relevant for an MSP.
    pub(crate) fn msp_process_block_import_events(
        &mut self,
        _block_hash: &Runtime::Hash,
        event: StorageEnableEvents<Runtime>,
    ) {
        let managed_msp_id = match &self.maybe_managed_provider {
            Some(ManagedProvider::Msp(msp_handler)) => &msp_handler.msp_id,
            _ => {
                error!(target: LOG_TARGET, "`msp_process_block_events` should only be called if the node is managing a MSP. Found [{:?}] instead.", self.maybe_managed_provider);
                return;
            }
        };

        // Process the events that are common to all roles.
        match event {
            _ => {
                trace!(target: LOG_TARGET, "No common MSP block import events to process while in LEADER, STANDALONE or FOLLOWER role");
            }
        }

        // Process the events that are common to all roles.
        match self.role {
            MultiInstancesNodeRole::Leader | MultiInstancesNodeRole::Standalone => {
                match event {
                    StorageEnableEvents::FileSystem(
                        pallet_file_system::Event::MoveBucketAccepted {
                            bucket_id,
                            old_msp_id: _,
                            new_msp_id,
                            value_prop_id,
                        },
                    ) => {
                        // As an MSP, this node is interested in the *imported* event if
                        // this node is the new MSP - to start downloading the bucket.
                        // Otherwise, ignore the event. Check finalised events for the old
                        // MSP branch.
                        if managed_msp_id == &new_msp_id {
                            self.emit(StartMovedBucketDownload {
                                bucket_id,
                                value_prop_id,
                            });
                        }
                    }
                    StorageEnableEvents::FileSystem(
                        pallet_file_system::Event::BspConfirmedStoring {
                            who: _,
                            bsp_id,
                            confirmed_file_keys,
                            skipped_file_keys: _,
                            new_root: _,
                        },
                    ) if self.config.enable_msp_distribute_files => {
                        for (file_key, _file_metadata) in confirmed_file_keys {
                            // If this is a BSP confirming a file that this MSP distributed, remove it from
                            // the list of BSPs distributing, and move it into the list of BSPs confirmed.
                            if let Some(ManagedProvider::Msp(msp_handler)) =
                                &mut self.maybe_managed_provider
                            {
                                if let Some(file_distribution_info) =
                                    msp_handler.files_to_distribute.get_mut(&file_key.into())
                                {
                                    file_distribution_info.bsps_distributing.remove(&bsp_id);
                                    file_distribution_info.bsps_confirmed.insert(bsp_id);

                                    debug!(target: LOG_TARGET, "BSP [{:?}] confirmed storing file [{:?}]", bsp_id, file_key);
                                }
                            }
                        }
                    }
                    StorageEnableEvents::FileSystem(
                        pallet_file_system::Event::StorageRequestFulfilled { file_key }
                        | pallet_file_system::Event::StorageRequestExpired { file_key }
                        | pallet_file_system::Event::StorageRequestRevoked { file_key }
                        | pallet_file_system::Event::StorageRequestRejected { file_key, .. },
                    ) if self.config.enable_msp_distribute_files => {
                        // Any of these events means that the storage request has finished its
                        // lifecycle, so we can remove it from the list of files to distribute.
                        if let Some(ManagedProvider::Msp(msp_handler)) =
                            &mut self.maybe_managed_provider
                        {
                            msp_handler.files_to_distribute.remove(&file_key.into());

                            debug!(target: LOG_TARGET, "Storage request [{:?}] finished its lifecycle, removing it from the list of files to distribute", file_key);
                        }
                    }
                    // Ignore all other events.
                    _ => {}
                }
            }
            MultiInstancesNodeRole::Follower => {
                trace!(target: LOG_TARGET, "No MSP block import events to process while in FOLLOWER role");
            }
        }
    }

    /// Runs at the end of every block import for a MSP.
    ///
    /// Steps:
    /// 1. Monitor for new pending storage requests and emit events for processing.
    /// 2. Check for BSPs who volunteered for files this MSP has to distribute, and spawn task
    ///    to distribute them.
    pub(crate) async fn msp_end_block_processing<Block>(
        &mut self,
        block_hash: &Runtime::Hash,
        _block_number: &BlockNumber<Runtime>,
        _tree_route: TreeRoute<Block>,
    ) where
        Block: BlockT<Hash = Runtime::Hash>,
    {
        let managed_msp_id = match &self.maybe_managed_provider {
            Some(ManagedProvider::Msp(msp_handler)) => msp_handler.msp_id.clone(),
            _ => {
                error!(target: LOG_TARGET, "`msp_end_block_processing` called but node is not managing an MSP");
                return;
            }
        };

        // Monitor for new pending storage requests
        self.handle_pending_storage_requests(block_hash, managed_msp_id.clone());

        // Distribute files to BSPs
        self.spawn_distribute_file_to_bsps_tasks(block_hash, managed_msp_id);
    }

    /// Processes finality events that are only relevant for an MSP.
    pub(crate) fn msp_process_finality_events(
        &mut self,
        block_hash: &Runtime::Hash,
        event: StorageEnableEvents<Runtime>,
    ) {
        let managed_msp_id = match &self.maybe_managed_provider {
            Some(ManagedProvider::Msp(msp_handler)) => msp_handler.msp_id.clone(),
            _ => {
                error!(target: LOG_TARGET, "`msp_process_finality_events` should only be called if the node is managing a MSP. Found [{:?}] instead.", self.maybe_managed_provider);
                return;
            }
        };

        // Process the events that are common to all roles.
        match event.clone() {
            StorageEnableEvents::FileSystem(
                pallet_file_system::Event::MspStoppedStoringBucket {
                    msp_id,
                    owner,
                    bucket_id,
                },
            ) => {
                if msp_id == managed_msp_id {
                    self.emit(FinalisedMspStoppedStoringBucket {
                        msp_id,
                        owner,
                        bucket_id,
                    })
                }
            }
            StorageEnableEvents::FileSystem(
                pallet_file_system::Event::MspStopStoringBucketInsolventUser {
                    msp_id,
                    owner: _,
                    bucket_id,
                },
            ) => {
                if msp_id == managed_msp_id {
                    self.emit(FinalisedMspStopStoringBucketInsolventUser { msp_id, bucket_id })
                }
            }
            StorageEnableEvents::FileSystem(pallet_file_system::Event::MoveBucketAccepted {
                bucket_id,
                old_msp_id,
                new_msp_id,
                value_prop_id: _,
            }) => {
                // This event is relevant in case the Provider managed is the old MSP,
                // in which case we should clean up the bucket.
                // Note: we do this in finality to ensure we don't lose data in case
                // of a reorg.
                if let Some(old_msp_id) = old_msp_id {
                    if managed_msp_id == old_msp_id {
                        self.emit(FinalisedBucketMovedAway {
                            bucket_id,
                            old_msp_id,
                            new_msp_id,
                        });
                    }
                }
            }
            StorageEnableEvents::FileSystem(
                pallet_file_system::Event::StorageRequestRejected {
                    file_key,
                    msp_id,
                    bucket_id,
                    reason: _,
                },
            ) => {
                // Process either InternalError or RequestExpire if this provider is managing the bucket.
                if msp_id == managed_msp_id {
                    self.emit(FinalisedStorageRequestRejected {
                        file_key: file_key.into(),
                        provider_id: msp_id.into(),
                        bucket_id,
                    })
                }
            }
            StorageEnableEvents::ProofsDealer(pallet_proofs_dealer::Event::MutationsApplied {
                mutations,
                old_root: _,
                new_root,
                event_info,
            }) => {
                // The mutations are applied to a Bucket's Forest root.
                let bucket_id = match self
                    .get_bucket_id_from_mutations_applied_event_info(block_hash, event_info)
                {
                    Ok(bucket_id) => bucket_id,
                    Err(e) => {
                        error!(target: LOG_TARGET, "Failed to get bucket ID from MutationsApplied event info: {:?}", e);
                        return;
                    }
                };

                if !self.validate_bucket_mutations_for_msp(block_hash, &managed_msp_id, &bucket_id)
                {
                    trace!(target: LOG_TARGET, "Bucket [0x{:x}] is not managed by this MSP [0x{:x}]. Skipping mutations applied event.", bucket_id, managed_msp_id);
                    return;
                }

                self.emit(FinalisedBucketMutationsApplied {
                    bucket_id,
                    mutations: mutations.clone().into(),
                    new_root,
                });
            }
            _ => {}
        }

        // Process the events that are specific to the role of the node.
        match self.role {
            MultiInstancesNodeRole::Leader | MultiInstancesNodeRole::Standalone => {
                match event {
                    StorageEnableEvents::FileSystem(
                        pallet_file_system::Event::MoveBucketRequested {
                            who: _,
                            bucket_id,
                            new_msp_id,
                            new_value_prop_id,
                        },
                    ) => {
                        // As an MSP, this node is interested in the event only if this node is the new MSP.
                        if managed_msp_id == new_msp_id {
                            self.emit(MoveBucketRequestedForMsp {
                                bucket_id,
                                value_prop_id: new_value_prop_id,
                            });
                        }
                    }

                    // Ignore all other events.
                    _ => {}
                }
            }
            MultiInstancesNodeRole::Follower => {
                trace!(target: LOG_TARGET, "No MSP finality events to process while in FOLLOWER role");
            }
        }
    }

    /// TODO: UPDATE THIS FUNCTION TO HANDLE FOREST WRITE LOCKS PER-BUCKET, AND UPDATE DOCS.
    /// Check if there are any pending requests to update the Forest root on the runtime, and process them.
    ///
    /// The priority is given by:
    /// 1. `RespondStorageRequest` over...
    /// 2. `StopStoringForInsolventUserRequest`.
    ///
    /// This function is called every time a new block is imported and after each request is queued.
    ///
    /// _IMPORTANT: This check will be skipped if the latest processed block does not match the current best block._
    pub(crate) fn msp_assign_forest_root_write_lock(&mut self) {
        let client_best_hash: Runtime::Hash = self.client.info().best_hash;
        let client_best_number: BlockNumber<Runtime> = self.client.info().best_number.into();

        // Skip if the latest processed block doesn't match the current best block
        if self.best_block.hash != client_best_hash || self.best_block.number != client_best_number
        {
            trace!(target: LOG_TARGET, "Skipping Forest root write lock assignment because latest processed block does not match current best block (local block hash and number [{}, {}], best block hash and number [{}, {}])", self.best_block.hash, self.best_block.number, client_best_hash, client_best_number);
            return;
        }

        match &self.maybe_managed_provider {
            Some(ManagedProvider::Msp(_)) => {}
            _ => {
                error!(target: LOG_TARGET, "`msp_check_pending_forest_root_writes` should only be called if the node is managing a MSP. Found [{:?}] instead.", self.maybe_managed_provider);
                return;
            }
        };

        // This is done in a closure to avoid borrowing `self` immutably and then mutably.
        // Inside of this closure, we borrow `self` mutably when taking ownership of the lock.
        {
            let forest_root_write_lock = match &mut self.maybe_managed_provider {
                Some(ManagedProvider::Msp(msp_handler)) => &mut msp_handler.forest_root_write_lock,
                _ => unreachable!("We just checked this is a MSP"),
            };

            if let Some(mut rx) = forest_root_write_lock.take() {
                // Note: tasks that get ownership of the lock are responsible for sending a message back when done processing.
                match rx.try_recv() {
                    // If the channel is empty, means we still need to wait for the current task to finish.
                    Err(TryRecvError::Empty) => {
                        // If we have a task writing to the runtime, we don't want to start another one.
                        *forest_root_write_lock = Some(rx);
                        trace!(target: LOG_TARGET, "Waiting for current Forest root write task to finish (lock held)");
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
                trace!(target: LOG_TARGET, "No forest root write lock held, proceeding to check pending requests");
            }
        }

        // At this point we know that the lock is released and we can start processing new requests.
        let mut next_event_data: Option<ForestWriteLockTaskData<Runtime>> = None;

        let msp_handler = match &mut self.maybe_managed_provider {
            Some(ManagedProvider::Msp(msp_handler)) => msp_handler,
            _ => {
                // If there's no MSP being managed, there's no point in checking for pending requests.
                error!(target: LOG_TARGET, "`msp_assign_forest_root_write_lock` called but node is not managing an MSP");
                return;
            }
        };

        // Check for pending respond storing requests from in-memory queue.
        {
            trace!(
                target: LOG_TARGET,
                "Checking pending respond storage requests. Queue size: {}, pending_file_keys: {:?}",
                msp_handler.pending_respond_storage_requests.len(),
                msp_handler.pending_respond_storage_request_file_keys
            );

            let max_batch_respond = MAX_BATCH_MSP_RESPOND_STORE_REQUESTS;

            // Batch multiple respond storing requests up to the runtime configured maximum.
            let mut respond_storage_requests = Vec::new();
            for _ in 0..max_batch_respond {
                if let Some(request) = msp_handler.pending_respond_storage_requests.pop_front() {
                    trace!(
                        target: LOG_TARGET,
                        "Popped respond storage request for file key [{:x}] from queue",
                        request.file_key
                    );
                    // Remove from dedup tracking set so the file key can be re-queued if needed.
                    msp_handler
                        .pending_respond_storage_request_file_keys
                        .remove(&request.file_key);
                    respond_storage_requests.push(request);
                } else {
                    break;
                }
            }

            // If we have at least 1 respond storing request, send the process event.
            if !respond_storage_requests.is_empty() {
                trace!(
                    target: LOG_TARGET,
                    "Preparing to emit ProcessMspRespondStoringRequest with {} requests",
                    respond_storage_requests.len()
                );
                next_event_data = Some(
                    ProcessMspRespondStoringRequestData {
                        respond_storing_requests: respond_storage_requests,
                    }
                    .into(),
                );
            } else {
                trace!(target: LOG_TARGET, "No pending respond storage requests in queue");
            }
        }

        // If we have no pending storage requests to respond to, we can also check for pending stop storing for insolvent user requests.
        if next_event_data.is_none() {
            let state_store_context = self.persistent_state.open_rw_context_with_overlay();
            if let Some(request) = state_store_context
                .pending_stop_storing_for_insolvent_user_request_deque::<Runtime>()
                .pop_front()
            {
                next_event_data = Some(
                    ProcessStopStoringForInsolventUserRequestData { who: request.user }.into(),
                );
            }
            state_store_context.commit();
        }

        // If there is any event data to process, emit the event.
        if let Some(event_data) = next_event_data {
            trace!(target: LOG_TARGET, "Emitting forest write event");
            self.msp_emit_forest_write_event(event_data);
        } else {
            trace!(target: LOG_TARGET, "No event data to emit");
        }
    }

    pub(crate) async fn msp_process_forest_root_changing_events(
        &mut self,
        block_hash: &BlockHash,
        event: StorageEnableEvents<Runtime>,
        revert: bool,
    ) {
        let managed_msp_id = match &self.maybe_managed_provider {
            Some(ManagedProvider::Msp(msp_handler)) => &msp_handler.msp_id,
            _ => {
                error!(target: LOG_TARGET, "`msp_process_forest_root_changing_events` should only be called if the node is managing a MSP. Found [{:?}] instead.", self.maybe_managed_provider);
                return;
            }
        };

        if let StorageEnableEvents::ProofsDealer(pallet_proofs_dealer::Event::MutationsApplied {
            mutations,
            old_root,
            new_root,
            event_info,
        }) = event
        {
            let bucket_id = match self
                .get_bucket_id_from_mutations_applied_event_info(block_hash, event_info)
            {
                Ok(bucket_id) => bucket_id,
                Err(e) => {
                    error!(target: LOG_TARGET, "Failed to get bucket ID from MutationsApplied event info: {:?}", e);
                    return;
                }
            };

            if !self.validate_bucket_mutations_for_msp(block_hash, managed_msp_id, &bucket_id) {
                trace!(target: LOG_TARGET, "Bucket [0x{:x}] is not managed by this MSP [0x{:x}]. Skipping mutations applied event.", bucket_id, managed_msp_id);
                return;
            }

            info!(target: LOG_TARGET, "🪾 Applying mutations to bucket [0x{:x}]", bucket_id);

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
                        "🔧 {} mutation [{}] for file key [{:x}] in bucket [0x{:x}]",
                        action, mutation_type, file_key, bucket_id
                    );
                }
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

    fn msp_emit_forest_write_event(&mut self, data: impl Into<ForestWriteLockTaskData<Runtime>>) {
        // Get the MSP's Forest root write lock.
        let forest_root_write_lock = match &mut self.maybe_managed_provider {
            Some(ManagedProvider::Msp(msp_handler)) => &mut msp_handler.forest_root_write_lock,
            _ => {
                error!(target: LOG_TARGET, "`msp_emit_forest_write_event` should only be called if the node is managing a MSP. Found [{:?}] instead.", self.maybe_managed_provider);
                return;
            }
        };

        // Create a new channel to assign ownership of the MSP's Forest root write lock.
        let (tx, rx) = tokio::sync::oneshot::channel();
        *forest_root_write_lock = Some(rx);

        // This is an [`Arc<Mutex<Option<T>>>`] (in this case [`oneshot::Sender<()>`]) instead of just
        // T so that we can keep using the current actors event bus (emit) which requires Clone on the
        // event. Clone is required because there is no constraint on the number of listeners that can
        // subscribe to the event (and each is guaranteed to receive all emitted events).
        let forest_root_write_tx = Arc::new(Mutex::new(Some(tx)));
        match data.into() {
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
            ForestWriteLockTaskData::ConfirmStoringRequest(_) => {
                unreachable!("MSPs do not confirm storing requests the way BSPs do.")
            }
            ForestWriteLockTaskData::SubmitProofRequest(_) => {
                unreachable!("MSPs do not submit proofs.")
            }
        }
    }

    /// Extracts the bucket ID encoded in a `MutationsApplied` event's `event_info`.
    ///
    /// The `ProofsDealer::MutationsApplied` event includes an opaque `event_info` payload which,
    /// for generic apply-delta mutations, is expected to contain the SCALE-encoded `BucketId` of
    /// the bucket whose Forest root was mutated.
    ///
    /// Behaviour:
    /// - Logs an error and returns an error if `event_info` is `None`.
    /// - Calls the runtime API (`decode_generic_apply_delta_event_info`) at `block_hash` to decode
    ///   the bucket ID.
    /// - Logs an error and returns an error if the runtime API call fails or if decoding fails.
    fn get_bucket_id_from_mutations_applied_event_info(
        &self,
        block_hash: &Runtime::Hash,
        event_info: Option<Vec<u8>>,
    ) -> Result<BucketId<Runtime>> {
        let Some(event_info) = event_info else {
            let msg = "MutationsApplied event with `None` event info, when it is expected to contain the BucketId of the bucket that was mutated.";
            error!(target: LOG_TARGET, "{}", msg);
            return Err(anyhow::anyhow!(msg));
        };

        // In StorageHub, we assume that all `MutationsApplied` events are emitted by bucket
        // root changes, and they should contain the encoded `BucketId` of the bucket that was mutated
        // in the `event_info` field.
        let bucket_id = match self
            .client
            .runtime_api()
            .decode_generic_apply_delta_event_info(*block_hash, event_info)
        {
            Ok(runtime_api_result) => match runtime_api_result {
                Ok(bucket_id) => bucket_id,
                Err(e) => {
                    let msg = format!("Failed to decode BucketId from event info: {:?}", e);
                    error!(target: LOG_TARGET, "{}", msg);
                    return Err(anyhow::anyhow!(msg));
                }
            },
            Err(e) => {
                let msg = format!(
                    "Error while calling runtime API to decode BucketId from event info: {:?}",
                    e
                );
                error!(target: LOG_TARGET, "{}", msg);
                return Err(anyhow::anyhow!(msg));
            }
        };

        Ok(bucket_id)
    }

    /// Checks whether a bucket is managed by the MSP this node is handling.
    ///
    /// Queries the runtime for the MSP ID associated with `bucket_id` and compares it with
    /// `managed_msp_id`.
    ///
    /// Returns `true` iff the runtime reports the bucket is managed by `managed_msp_id`.
    /// Returns `false` when:
    /// - The bucket is managed by a different MSP
    /// - The bucket is not managed by any MSP
    /// - The runtime API call fails (an error is logged)
    fn validate_bucket_mutations_for_msp(
        &self,
        block_hash: &Runtime::Hash,
        managed_msp_id: &ProviderId<Runtime>,
        bucket_id: &BucketId<Runtime>,
    ) -> bool {
        // Check if the bucket is managed by this MSP.
        match self
            .client
            .runtime_api()
            .query_msp_id_of_bucket_id(*block_hash, &bucket_id)
        {
            Ok(runtime_api_result) => match runtime_api_result {
                Ok(Some(msp_id)) if msp_id == *managed_msp_id => {
                    // This is a valid scenario. It would be the case where the bucket is managed by this MSP.
                    trace!(target: LOG_TARGET, "Bucket [0x{:x}] is managed by this MSP [0x{:x}].", bucket_id, managed_msp_id);
                    return true;
                }
                Ok(Some(msp_id)) => {
                    // This is a valid scenario. It would be the case where the mutation is being applied to a bucket that is managed by another MSP.
                    trace!(target: LOG_TARGET, "Bucket [0x{:x}] is not managed by this MSP [0x{:x}]. It is managed by MSP [0x{:x}].", bucket_id, managed_msp_id, msp_id);
                    return false;
                }
                Ok(None) => {
                    // This is a valid scenario. It would be the case where the bucket is not managed by any MSP.
                    trace!(target: LOG_TARGET, "Bucket [0x{:x}] is not managed by any MSP.", bucket_id);
                    return false;
                }
                Err(e) => {
                    error!(target: LOG_TARGET, "Error querying MSP ID of bucket [0x{:x}]: {:?}", bucket_id, e);
                    return false;
                }
            },
            Err(e) => {
                error!(target: LOG_TARGET, "Error while calling runtime API to query MSP ID of bucket [0x{:x}]: {:?}", bucket_id, e);
                return false;
            }
        }
    }

    /// Scans pending storage requests for this MSP and triggers distribution tasks.
    ///
    /// This function should be called at the end of a block import for MSP-managed nodes.
    /// It queries the runtime for pending storage requests assigned to this MSP, filters
    /// only those already accepted by the MSP (i.e., files that this MSP already has),
    /// and for each eligible file delegates to [`distribute_file_to_bsps`] which emits
    /// a `DistributeFileToBsp` event per volunteering BSP (avoiding duplicates).
    ///
    /// Parameters:
    /// - `block_hash`: Block hash used to perform consistent runtime API queries.
    /// - `msp_id`: The MSP ID of the provider this node is managing.
    ///
    /// Behaviour:
    /// - Logs and returns early if runtime API calls fail.
    /// - Safe to call repeatedly; per-file deduplication is enforced downstream using
    ///   the in-memory `files_to_distribute` state to not spawn duplicate tasks or
    ///   re-emit for already-confirmed BSPs.
    pub(crate) fn spawn_distribute_file_to_bsps_tasks(
        &mut self,
        block_hash: &Runtime::Hash,
        msp_id: MainStorageProviderId<Runtime>,
    ) {
        debug!(target: LOG_TARGET, "Spawning distribute file to BSPs tasks");

        // Followers do not distribute files to BSPs.
        if self.role == MultiInstancesNodeRole::Follower {
            debug!(target: LOG_TARGET, "Follower node does not distribute files to BSPs. Skipping distribution scan.");
            return;
        }

        // Only distribute files to BSPs when explicitly enabled via configuration.
        if !self.config.enable_msp_distribute_files {
            debug!(target: LOG_TARGET, "MSP file distribution disabled by configuration. Skipping distribution scan.");
            return;
        }

        // Exit early if the MSP node peer ID is not set, meaning it is not meant to be a distributor.
        // Clone to avoid holding an immutable borrow of `self` across the loop below where we need `&mut self`.
        let managed_msp_peer_id = match self.config.peer_id.clone() {
            Some(peer_id) => peer_id,
            None => {
                debug!(target: LOG_TARGET, "MSP node peer ID is not set, meaning it is not meant to be a distributor. Skipping distribution of files.");
                return;
            }
        };

        // Get pending storage requests that this MSP should distribute the file to BSPs for.
        let pending_storage_requests_for_this_msp = match self
            .client
            .runtime_api()
            .storage_requests_by_msp(*block_hash, msp_id)
        {
            Ok(pending_storage_requests) => pending_storage_requests,
            Err(e) => {
                error!(target: LOG_TARGET, "Failed to execute runtime API call to get pending storage requests for MSP [{:?}]: {:?}", msp_id, e);
                return;
            }
        };

        // Filter out storage requests that this MSP has not already accepted.
        // Cannot distribute files that this MSP doesn't have already.
        // Also keep only those for which this MSP node is listed as one of
        // the `user_peer_ids` of the storage request, meaning it is meant to
        // be a distributor of the file.
        let mut storage_requests_to_distribute =
            pending_storage_requests_for_this_msp
                .iter()
                .filter(|(_, storage_request)| {
                    // We already know that the values in this map are storage requests that
                    // this MSP is assigned to, we just have to check that it has already accepted
                    // the storage request. See [`shc_common::types::StorageRequestMetadata`] for more details.
                    if !storage_request.msp_status.is_accepted() {
                        // Return early if the MSP has not accepted the storage request.
                        return false;
                    }

                    let msp_is_distributor = storage_request.user_peer_ids.iter().any(|peer_id| {
                        let peer_id_str = match str::from_utf8(peer_id.as_ref()) {
                            Ok(peer_id_str) => peer_id_str,
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to convert peer ID from storage request to string: {:?}", e);
                                return false;
                            }
                        };

                        let peer_id: PeerId = match peer_id_str.parse() {
                            Ok(peer_id) => peer_id,
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to convert peer ID from storage request to PeerId: {:?}", e);
                                return false;
                            }
                        };

                        peer_id == managed_msp_peer_id
                    });

                    msp_is_distributor
                })
								.peekable();

        if storage_requests_to_distribute.peek().is_none() {
            debug!(target: LOG_TARGET, "No storage requests to distribute for MSP [{:?}]", msp_id);
            return;
        }

        // Distribute the files to the BSPs.
        for storage_request in storage_requests_to_distribute {
            let file_key = storage_request.0;
            self.distribute_file_to_bsps(block_hash, file_key);
        }
    }

    /// Emits distribution events for all volunteering BSPs for a given file.
    ///
    /// Queries the runtime for BSPs that volunteered to store `file_key` and emits
    /// a `DistributeFileToBsp` event for each BSP that doesn't already have a running
    /// task. BSPs that have already confirmed storage are skipped as well. Uses the
    /// MSP's `files_to_distribute` to avoid duplicate work.
    ///
    /// Preconditions:
    /// - This node must be managing an MSP; otherwise the function logs and returns.
    /// - Typically invoked by [`spawn_distribute_file_to_bsps_tasks`], after ensuring the
    ///   MSP accepted the corresponding storage request.
    ///
    /// Parameters:
    /// - `block_hash`: Block hash used for consistent runtime API queries.
    /// - `file_key`: The file key to distribute.
    ///
    /// Errors:
    /// - Logs and returns early if runtime API queries fail or return errors.
    pub(crate) fn distribute_file_to_bsps(
        &mut self,
        block_hash: &Runtime::Hash,
        file_key: &Runtime::Hash,
    ) {
        debug!(target: LOG_TARGET, "Distributing file [{:?}] to BSPs", file_key);

        let managed_msp = match &mut self.maybe_managed_provider {
            Some(ManagedProvider::Msp(msp_handler)) => msp_handler,
            _ => {
                error!(target: LOG_TARGET, "`spawn_distribute_file_to_bsps_tasks` should only be called if the node is managing a MSP. Found [{:?}] instead.", self.maybe_managed_provider);
                return;
            }
        };

        let file_key = file_key.clone().into();

        // Get the BSPs who volunteered to store the file.
        let bsps_volunteered: Vec<BackupStorageProviderId<Runtime>> = match self
            .client
            .runtime_api()
            .query_bsps_volunteered_for_file(*block_hash, file_key)
        {
            Ok(bsps_volunteered_result) => match bsps_volunteered_result {
                Ok(bsps_volunteered) => bsps_volunteered,
                Err(e) => {
                    error!(target: LOG_TARGET, "Failed to get BSPs volunteered for file [{:?}]: {:?}", file_key, e);
                    return;
                }
            },
            Err(e) => {
                error!(target: LOG_TARGET, "Failed run runtime API call to query BSPs volunteered for file [{:?}]: {:?}", file_key, e);
                return;
            }
        };

        let to_emit = {
            // Get the BSPs for which there are tasks currently distributing the file.
            // If there is no entry for the file key, create a new one.
            let file_distribution_info = managed_msp
                .files_to_distribute
                .entry(file_key.clone().into())
                .or_insert(FileDistributionInfo::new());

            // Filter out BSPs that are already distributing the file or have already confirmed to store it.
            bsps_volunteered
                .into_iter()
                .filter(|bsp_id| {
                    !file_distribution_info.bsps_distributing.contains(bsp_id)
                        && !file_distribution_info.bsps_confirmed.contains(bsp_id)
                })
                .collect::<Vec<_>>()
        };

        // For each BSP who volunteered to store the file, send an event to distribute the file to them,
        // as long as there is not already a task for that BSP, or the file has already been confirmed to be stored.
        // This loop is executed separately from the one above to avoid compiler error with `self` being borrowed mutably.
        for bsp_id in to_emit {
            self.emit(DistributeFileToBsp {
                file_key: file_key.into(),
                bsp_id,
            });
        }
    }

    /// Verifies that all MSP bucket forest roots match the on-chain roots.
    ///
    /// This is a sanity check after coming out of sync to ensure mutations were
    /// correctly applied during the sync process.
    ///
    /// If a forest doesn't exist locally, it checks that it should be an empty bucket. This is because a user could have
    /// created a bucket during the downtime, and since the MSP didn't confirm any storage request for it, no mutations
    /// were applied and as such the forest storage was not created previously during the initial sync.
    ///
    /// TODO: A bucket could have been unassigned from this MSP if a user requested to move it to another MSP.
    /// We should handle this by deleting the bucket's forest storage and cleaning up the file storage from
    /// the file keys of that bucket.
    async fn verify_msp_bucket_roots(
        &mut self,
        block_hash: &Runtime::Hash,
        msp_id: &ProviderId<Runtime>,
    ) {
        // Get all buckets managed by this MSP
        //! DANGER: This runtime API call is extremely expensive, as it iterates over all buckets in the system.
        let buckets = match self
            .client
            .runtime_api()
            .query_buckets_for_msp(*block_hash, msp_id)
        {
            Ok(Ok(buckets)) => buckets,
            Ok(Err(e)) => {
                error!(target: LOG_TARGET, "Failed to query buckets for MSP during root verification: {:?}", e);
                return;
            }
            Err(e) => {
                error!(target: LOG_TARGET, "Runtime API call failed for query_buckets_for_msp: {:?}", e);
                return;
            }
        };

        if buckets.is_empty() {
            info!(target: LOG_TARGET, "✅ MSP has no buckets to verify after sync");
            return;
        }

        let mut mismatches = 0;
        let mut verified = 0;
        let mut missing = 0;

        for bucket_id in buckets {
            let forest_key = bucket_id.as_ref().to_vec();

            // Get the local root of the bucket.
            // Not having the bucket is valid, so long as the bucket on-chain is empty,
            // i.e. it's on-chain root is the default root.
            let maybe_local_root = match self
                .forest_storage_handler
                .get(&forest_key.clone().into())
                .await
            {
                Some(fs) => Some(fs.read().await.root()),
                None => None,
            };

            // Get the on-chain root of the bucket
            let onchain_root = match self
                .client
                .runtime_api()
                .query_bucket_root(*block_hash, &bucket_id)
            {
                Ok(Ok(root)) => root,
                Ok(Err(e)) => {
                    error!(target: LOG_TARGET, "Failed to query bucket root for [0x{:x}]: {:?}", bucket_id, e);
                    continue;
                }
                Err(e) => {
                    error!(target: LOG_TARGET, "Runtime API call failed for query_bucket_root [0x{:x}]: {:?}", bucket_id, e);
                    continue;
                }
            };

            // Compare the roots
            match maybe_local_root {
                // Failure Case: Local forest exists and its root does not match the on-chain root.
                Some(local_root) if local_root != onchain_root => {
                    error!(target: LOG_TARGET, "❌ CRITICAL: Bucket [0x{:x}] root mismatch after sync! Local: [0x{:x}], On-chain: [0x{:x}]", bucket_id, local_root, onchain_root);
                    mismatches += 1;
                }
                // Failure Case: Local forest does not exist and the on-chain root is not the default root (bucket is not empty)
                None if onchain_root != DefaultMerkleRoot::<Runtime>::get() => {
                    error!(target: LOG_TARGET, "❌ CRITICAL: Bucket [0x{:x}] forest storage not found locally after sync, and on-chain root is not the default root (bucket is not empty). On-chain root: [0x{:x}]", bucket_id, onchain_root);
                    missing += 1;
                }
                // Success Case: Local forest exists and its root matches the on-chain root.
                Some(_) => {
                    trace!(target: LOG_TARGET, "Bucket [0x{:x}] root verified: [0x{:x}]", bucket_id, onchain_root);
                    verified += 1;
                }
                // Success Case: Local forest does not exist and the on-chain root is the default root (bucket is empty).
                None => {
                    trace!(target: LOG_TARGET, "Bucket [0x{:x}] root verified: [0x{:x}]", bucket_id, onchain_root);
                    verified += 1;
                }
            }
        }

        if mismatches > 0 || missing > 0 {
            error!(
                    target: LOG_TARGET,
                    "❌ MSP bucket verification after sync: {} verified, {} mismatches, {} missing",
                    verified, mismatches, missing
            );
        } else {
            info!(
                    target: LOG_TARGET,
                    "✅ All {} MSP bucket roots verified after sync",
                    verified
            );
        }
    }

    /// Process pending storage requests for the given MSP.
    ///
    /// Queries pending storage requests and emits a [`NewStorageRequest`] event for file keys
    /// that are not already being processed. This mirrors the pattern used by
    /// [`distribute_file_to_bsps`] with [`files_to_distribute`].
    ///
    /// ## Status Tracking
    ///
    /// The status tracking prevents duplicate processing:
    /// - File keys with any status (`Processing`, `Abandoned`) are skipped
    /// - New file keys are marked as `Processing` when emitting the event
    /// - Tasks update statuses via commands as they process requests
    /// - Stale entries are cleaned up when file keys no longer appear in pending requests
    ///
    /// ## Cleanup
    ///
    /// Stale entries (file keys no longer in pending requests) are removed regardless of status.
    /// If a file key is not pending, its storage request lifecycle is complete.
    fn handle_pending_storage_requests(
        &mut self,
        current_block_hash: &Runtime::Hash,
        msp_id: MainStorageProviderId<Runtime>,
    ) {
        // Query pending storage requests (not yet accepted by MSP)
        let pending_storage_requests = match self
            .client
            .runtime_api()
            .pending_storage_requests_by_msp(*current_block_hash, msp_id)
        {
            Ok(sr) => sr,
            Err(e) => {
                warn!(target: LOG_TARGET, "Failed to get pending storage requests: {:?}", e);
                return;
            }
        };

        // Filter and collect requests to emit, setting status to Processing for new ones.
        // Also clean up stale entries from file_key_statuses.
        let requests_to_emit: Vec<_> = {
            let msp_handler = match &mut self.maybe_managed_provider {
                Some(ManagedProvider::Msp(msp_handler)) => msp_handler,
                _ => {
                    error!(target: LOG_TARGET, "`handle_pending_storage_requests` called but node is not managing an MSP");
                    return;
                }
            };

            // Collect the set of pending file keys for cleanup check
            let pending_file_keys: HashSet<FileKey> = pending_storage_requests
                .iter()
                .map(|(file_key, _)| (*file_key).into())
                .collect();

            // Clean up stale entries: remove file keys that are no longer in pending requests.
            // If a file key is not pending, its storage request lifecycle is complete and we
            // don't need to track it anymore, regardless of its current status.
            let stale_keys: Vec<_> = msp_handler
                .file_key_statuses
                .keys()
                .filter(|file_key| !pending_file_keys.contains(*file_key))
                .copied()
                .collect();

            if !stale_keys.is_empty() {
                debug!(
                    target: LOG_TARGET,
                    "Cleaning up {} stale file key statuses (no longer in pending requests)",
                    stale_keys.len()
                );
                for file_key in stale_keys {
                    trace!(target: LOG_TARGET, "Removing stale file key [{:x}] from statuses", file_key);
                    msp_handler.file_key_statuses.remove(&file_key);
                }
            }

            pending_storage_requests
                .into_iter()
                .filter_map(|(file_key, sr)| {
                    let file_key_h256 = file_key.into();

                    // Skip file keys that already have a status (i.e., Processing or Abandoned)
                    if let Some(status) = msp_handler.file_key_statuses.get(&file_key_h256) {
                        trace!(
                            target: LOG_TARGET,
                            "Skipping file key [{:x}] - status: {:?}",
                            file_key_h256,
                            status
                        );
                        return None;
                    }

                    // Mark as Processing before emitting
                    debug!(target: LOG_TARGET, "Processing new file key [{:x}]", file_key_h256);
                    msp_handler
                        .file_key_statuses
                        .insert(file_key_h256, FileKeyStatus::Processing);

                    Some(NewStorageRequest {
                        who: sr.owner,
                        file_key: file_key_h256,
                        bucket_id: sr.bucket_id,
                        location: sr.location,
                        fingerprint: sr.fingerprint.as_ref().into(),
                        size: sr.size,
                        user_peer_ids: sr.user_peer_ids,
                        expires_at: sr.expires_at,
                    })
                })
                .collect()
        };

        if requests_to_emit.is_empty() {
            trace!(target: LOG_TARGET, "No new storage requests to process (all already have status)");
            return;
        }

        info!(
            target: LOG_TARGET,
            "Emitting {} NewStorageRequest events (filtered from pending requests)",
            requests_to_emit.len()
        );

        // Emit events for filtered requests
        for request in requests_to_emit {
            self.emit(request);
        }
    }
}
