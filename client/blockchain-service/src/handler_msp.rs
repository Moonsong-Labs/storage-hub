use log::{debug, error, info, trace, warn};
use std::{str, sync::Arc};
use tokio::sync::{oneshot::error::TryRecvError, Mutex};

use sc_client_api::HeaderBackend;
use sc_network_types::PeerId;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::TreeRoute;
use sp_runtime::traits::Block as BlockT;

use pallet_file_system_runtime_api::FileSystemApi;
use pallet_storage_providers_runtime_api::StorageProvidersApi;
use shc_actors_framework::actor::Actor;
use shc_common::{
    traits::StorageEnableRuntime,
    typed_store::CFDequeAPI,
    types::{
        BackupStorageProviderId, BlockHash, BlockNumber, BucketId, ProviderId, StorageEnableEvents,
    },
};
use shc_forest_manager::traits::ForestStorageHandler;

use crate::{
    events::{
        DistributeFileToBsp, FinalisedBucketMovedAway, FinalisedBucketMutationsApplied,
        FinalisedMspStopStoringBucketInsolventUser, FinalisedMspStoppedStoringBucket,
        ForestWriteLockTaskData, MoveBucketRequestedForMsp, NewStorageRequest,
        ProcessMspRespondStoringRequest, ProcessMspRespondStoringRequestData,
        ProcessStopStoringForInsolventUserRequest, ProcessStopStoringForInsolventUserRequestData,
        StartMovedBucketDownload, VerifyMspBucketForests,
    },
    handler::LOG_TARGET,
    types::{FileDistributionInfo, ManagedProvider},
    BlockchainService,
};

// TODO: Make this configurable in the config file
const MAX_BATCH_MSP_RESPOND_STORE_REQUESTS: u32 = 100;

impl<FSH, Runtime> BlockchainService<FSH, Runtime>
where
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    /// Handles the initial sync of a MSP, after coming out of syncing mode.
    ///
    /// Steps:
    /// TODO
    pub(crate) fn msp_initial_sync(&self, block_hash: Runtime::Hash, msp_id: ProviderId<Runtime>) {
        // TODO: Catch up to Forest root writes in the Bucket's Forests.
        // Emit event to check that this node has a Forest Storage for each Bucket this MSP manages.
        self.emit(VerifyMspBucketForests {});

        self.emit_pending_storage_requests_for_msp(block_hash, msp_id);
    }

    /// Initialises the block processing flow for a MSP.
    ///
    /// Steps:
    /// 1. Catch up to Forest root changes in the Forests of the Buckets this MSP manages.
    pub(crate) async fn msp_init_block_processing<Block>(
        &self,
        _block_hash: &Runtime::Hash,
        _block_number: &BlockNumber<Runtime>,
        tree_route: TreeRoute<Block>,
    ) where
        Block: BlockT<Hash = Runtime::Hash>,
    {
        self.forest_root_changes_catchup(&tree_route).await;
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

        match event {
            StorageEnableEvents::FileSystem(pallet_file_system::Event::MoveBucketAccepted {
                bucket_id,
                old_msp_id: _,
                new_msp_id,
                value_prop_id,
            }) => {
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
            StorageEnableEvents::FileSystem(pallet_file_system::Event::BspConfirmedStoring {
                who: _,
                bsp_id,
                confirmed_file_keys,
                skipped_file_keys: _,
                new_root: _,
            }) if self.config.enable_msp_distribute_files => {
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
                if let Some(ManagedProvider::Msp(msp_handler)) = &mut self.maybe_managed_provider {
                    msp_handler.files_to_distribute.remove(&file_key.into());

                    debug!(target: LOG_TARGET, "Storage request [{:?}] finished its lifecycle, removing it from the list of files to distribute", file_key);
                }
            }
            // Ignore all other events.
            _ => {}
        }
    }

    /// Runs at the end of every block import for a MSP.
    ///
    /// Steps:
    /// 1. Check for BSPs who volunteered for files this MSP has to distribute, and spawn task
    /// to distribute them.
    pub(crate) async fn msp_end_block_processing<Block>(
        &mut self,
        block_hash: &Runtime::Hash,
        _block_number: &BlockNumber<Runtime>,
        _tree_route: TreeRoute<Block>,
    ) where
        Block: BlockT<Hash = Runtime::Hash>,
    {
        self.spawn_distribute_file_to_bsps_tasks(block_hash);
    }

    /// Processes finality events that are only relevant for an MSP.
    pub(crate) fn msp_process_finality_events(
        &self,
        block_hash: &Runtime::Hash,
        event: StorageEnableEvents<Runtime>,
    ) {
        let managed_msp_id = match &self.maybe_managed_provider {
            Some(ManagedProvider::Msp(msp_handler)) => &msp_handler.msp_id,
            _ => {
                error!(target: LOG_TARGET, "`msp_process_finality_events` should only be called if the node is managing a MSP. Found [{:?}] instead.", self.maybe_managed_provider);
                return;
            }
        };

        match event {
            StorageEnableEvents::FileSystem(
                pallet_file_system::Event::MspStoppedStoringBucket {
                    msp_id,
                    owner,
                    bucket_id,
                },
            ) => {
                if msp_id == *managed_msp_id {
                    self.emit(FinalisedMspStoppedStoringBucket {
                        msp_id,
                        owner,
                        bucket_id,
                    })
                }
            }
            StorageEnableEvents::FileSystem(pallet_file_system::Event::MoveBucketRequested {
                who: _,
                bucket_id,
                new_msp_id,
                new_value_prop_id,
            }) => {
                // As an MSP, this node is interested in the event only if this node is the new MSP.
                if managed_msp_id == &new_msp_id {
                    self.emit(MoveBucketRequestedForMsp {
                        bucket_id,
                        value_prop_id: new_value_prop_id,
                    });
                }
            }
            StorageEnableEvents::FileSystem(
                pallet_file_system::Event::MspStopStoringBucketInsolventUser {
                    msp_id,
                    owner: _,
                    bucket_id,
                },
            ) => {
                if msp_id == *managed_msp_id {
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
                    if managed_msp_id == &old_msp_id {
                        self.emit(FinalisedBucketMovedAway {
                            bucket_id,
                            old_msp_id,
                            new_msp_id,
                        });
                    }
                }
            }
            StorageEnableEvents::ProofsDealer(pallet_proofs_dealer::Event::MutationsApplied {
                mutations,
                old_root: _,
                new_root,
                event_info,
            }) => {
                // The mutations are applied to a Bucket's Forest root.
                // Check that this MSP is managing at least one bucket.
                let buckets_managed_by_msp =
                    self.client
                            .runtime_api()
                            .query_buckets_for_msp(*block_hash, managed_msp_id)
                            .inspect_err(|e| error!(target: LOG_TARGET, "Runtime API call failed while querying buckets for MSP [{:?}]: {:?}", managed_msp_id, e))
                            .ok()
                            .and_then(|api_result| {
                                api_result
                                    .inspect_err(|e| error!(target: LOG_TARGET, "Runtime API error while querying buckets for MSP [{:?}]: {:?}", managed_msp_id, e))
                                    .ok()
                            });

                let Some(bucket_id) = self.validate_bucket_mutations_for_msp(
                    block_hash,
                    buckets_managed_by_msp,
                    event_info,
                ) else {
                    return;
                };

                self.emit(FinalisedBucketMutationsApplied {
                    bucket_id,
                    mutations: mutations.clone().into(),
                    new_root,
                });
            }

            // Ignore all other events.
            _ => {}
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
            }
        }

        // At this point we know that the lock is released and we can start processing new requests.
        let state_store_context = self.persistent_state.open_rw_context_with_overlay();
        let mut next_event_data: Option<ForestWriteLockTaskData<Runtime>> = None;

        if self.maybe_managed_provider.is_none() {
            // If there's no Provider being managed, there's no point in checking for pending requests.
            return;
        }

        // Check for pending respond storing requests.
        {
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
            self.msp_emit_forest_write_event(event_data);
        }
    }

    pub(crate) async fn msp_process_forest_root_changing_events(
        &self,
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

        // Preemptively getting the Buckets managed by this MSP, so that we do the query just once,
        // instead of doing it for every event.
        let buckets_managed_by_msp =
            self.client
                    .runtime_api()
                    .query_buckets_for_msp(*block_hash, managed_msp_id)
                    .inspect_err(|e| error!(target: LOG_TARGET, "Runtime API call failed while querying buckets for MSP [{:?}]: {:?}", managed_msp_id, e))
                    .ok()
                    .and_then(|api_result| {
                        api_result
                            .inspect_err(|e| error!(target: LOG_TARGET, "Runtime API error while querying buckets for MSP [{:?}]: {:?}", managed_msp_id, e))
                            .ok()
                    });

        match event {
            StorageEnableEvents::ProofsDealer(pallet_proofs_dealer::Event::MutationsApplied {
                mutations,
                old_root,
                new_root,
                event_info,
            }) => {
                let Some(bucket_id) = self.validate_bucket_mutations_for_msp(
                    block_hash,
                    buckets_managed_by_msp,
                    event_info,
                ) else {
                    return;
                };

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
            _ => {}
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

    /// Validates that a MutationsApplied event's bucket is managed by this MSP.
    ///
    /// This helper performs the following validation steps:
    /// 1. Checks if this MSP is managing at least one bucket
    /// 2. Validates that the event_info contains the BucketId of the bucket that was mutated
    /// 3. Decodes the BucketId from the event_info
    /// 4. Verifies that the BucketId is in the list of buckets managed by this MSP
    ///
    /// Returns Some(bucket_id) if all validations pass, None otherwise.
    fn validate_bucket_mutations_for_msp(
        &self,
        block_hash: &Runtime::Hash,
        buckets_managed_by_msp: Option<Vec<BucketId<Runtime>>>,
        event_info: Option<Vec<u8>>,
    ) -> Option<BucketId<Runtime>> {
        // Check that this MSP is managing at least one bucket.
        if buckets_managed_by_msp.is_none() {
            debug!(target: LOG_TARGET, "MSP is not managing any buckets. Skipping mutations applied event.");
            return None;
        }
        let buckets_managed_by_msp = buckets_managed_by_msp
            .as_ref()
            .expect("Just checked that this is not None; qed");
        if buckets_managed_by_msp.is_empty() {
            debug!(target: LOG_TARGET, "Buckets managed by MSP is an empty vector. Skipping mutations applied event.");
            return None;
        }

        // In StorageHub, we assume that all `MutationsApplied` events are emitted by bucket
        // root changes, and they should contain the encoded `BucketId` of the bucket that was mutated
        // in the `event_info` field.
        let Some(event_info) = event_info else {
            error!(
                target: LOG_TARGET,
                "MutationsApplied event with `None` event info, when it is expected to contain the BucketId of the bucket that was mutated."
            );
            return None;
        };
        let bucket_id = match self
            .client
            .runtime_api()
            .decode_generic_apply_delta_event_info(*block_hash, event_info)
        {
            Ok(runtime_api_result) => match runtime_api_result {
                Ok(bucket_id) => bucket_id,
                Err(e) => {
                    error!(target: LOG_TARGET, "Failed to decode BucketId from event info: {:?}", e);
                    return None;
                }
            },
            Err(e) => {
                error!(target: LOG_TARGET, "Error while calling runtime API to decode BucketId from event info: {:?}", e);
                return None;
            }
        };

        // Check if the bucket is managed by this MSP.
        if !buckets_managed_by_msp.contains(&bucket_id) {
            debug!(target: LOG_TARGET, "Bucket [{:?}] is not managed by this MSP. Skipping mutations applied event.", bucket_id);
            return None;
        }

        Some(bucket_id)
    }

    /// Scans pending storage requests for this MSP and triggers distribution tasks.
    ///
    /// This function should be called at the end of a block import for MSP-managed nodes.
    /// It queries the runtime for pending storage requests assigned to this MSP, filters
    /// only those already accepted by the MSP (i.e., files that this MSP already has),
    /// and for each eligible file delegates to [`distribute_file_to_bsps`] which emits
    /// a `DistributeFileToBsp` event per volunteering BSP (avoiding duplicates).
    ///
    /// - `block_hash`: Block hash used to perform consistent runtime API queries.
    ///
    /// Behaviour:
    /// - No-ops with an error log if the node is not managing an MSP.
    /// - Logs and returns early if runtime API calls fail.
    /// - Safe to call repeatedly; per-file deduplication is enforced downstream using
    ///   the in-memory `files_to_distribute` state to not spawn duplicate tasks or
    ///   re-emit for already-confirmed BSPs.
    pub(crate) fn spawn_distribute_file_to_bsps_tasks(&mut self, block_hash: &Runtime::Hash) {
        // Only distribute files to BSPs when explicitly enabled via configuration.
        if !self.config.enable_msp_distribute_files {
            trace!(target: LOG_TARGET, "MSP file distribution disabled by configuration. Skipping distribution scan.");
            return;
        }

        let managed_msp_id = match &self.maybe_managed_provider {
            Some(ManagedProvider::Msp(msp_handler)) => msp_handler.msp_id.clone(),
            _ => {
                error!(target: LOG_TARGET, "`spawn_distribute_file_to_bsps_tasks` should only be called if the node is managing a MSP. Found [{:?}] instead.", self.maybe_managed_provider);
                return;
            }
        };

        // Exit early if the MSP node peer ID is not set, meaning it is not meant to be a distributor.
        // Clone to avoid holding an immutable borrow of `self` across the loop below where we need `&mut self`.
        let managed_msp_peer_id = match self.config.peer_id.clone() {
            Some(peer_id) => peer_id,
            None => {
                trace!(target: LOG_TARGET, "MSP node peer ID is not set, meaning it is not meant to be a distributor. Skipping distribution of files.");
                return;
            }
        };

        // Get pending storage requests that this MSP should distribute the file to BSPs for.
        let pending_storage_requests_for_this_msp = match self
            .client
            .runtime_api()
            .storage_requests_by_msp(*block_hash, managed_msp_id)
        {
            Ok(pending_storage_requests) => pending_storage_requests,
            Err(e) => {
                error!(target: LOG_TARGET, "Failed to execute runtime API call to get pending storage requests for MSP [{:?}]: {:?}", managed_msp_id, e);
                return;
            }
        };

        // Filter out storage requests that this MSP has not already accepted.
        // Cannot distribute files that this MSP doesn't have already.
        // Also keep only those for which this MSP node is listed as one of
        // the `user_peer_ids` of the storage request, meaning it is meant to
        // be a distributor of the file.
        let storage_requests_to_distribute =
            pending_storage_requests_for_this_msp
                .iter()
                .filter(|(_, storage_request)| {
                    // We already know that the values in this map are storage requests that
                    // this MSP is assigned to, we just have to check that it has already accepted
                    // the storage request, which is indicated by the second element of the tuple.
                    // See [`shc_common::types::StorageRequestMetadata`] for more details.
                    let msp_accepted = if let Some(msp) = storage_request.msp {
                        msp.1
                    } else {
                        false
                    };

                    if !msp_accepted {
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
                });

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

    /// Emits `NewStorageRequest` events for all pending storage requests assigned to an MSP.
    fn emit_pending_storage_requests_for_msp(
        &self,
        block_hash: Runtime::Hash,
        msp_id: ProviderId<Runtime>,
    ) {
        info!(target: LOG_TARGET, "Checking for storage requests for this MSP");

        let storage_requests = match self
            .client
            .runtime_api()
            .pending_storage_requests_by_msp(block_hash, msp_id)
        {
            Ok(sr) => sr,
            Err(_) => {
                // If querying for pending storage requests fail, do not try to answer them
                warn!(target: LOG_TARGET, "Failed to get pending storage request");
                return;
            }
        };

        info!(
            target: LOG_TARGET,
            "We have {} pending storage requests",
            storage_requests.len()
        );

        // Loop over each pending storage request to start a new storage request task for the MSP
        for (file_key, sr) in storage_requests {
            self.emit(NewStorageRequest {
                who: sr.owner,
                file_key: file_key.into(),
                bucket_id: sr.bucket_id,
                location: sr.location,
                fingerprint: sr.fingerprint.as_ref().into(),
                size: sr.size,
                user_peer_ids: sr.user_peer_ids,
                expires_at: sr.expires_at,
            })
        }
    }
}
