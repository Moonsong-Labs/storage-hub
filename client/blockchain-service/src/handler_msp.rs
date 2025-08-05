use log::{debug, error, info, trace, warn};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::{oneshot::error::TryRecvError, Mutex};

use sc_client_api::HeaderBackend;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::TreeRoute;
use sp_core::H256;

use pallet_file_system_runtime_api::FileSystemApi;
use pallet_storage_providers_runtime_api::StorageProvidersApi;
use shc_actors_framework::actor::Actor;
use shc_common::traits::{
    StorageEnableApiCollection, StorageEnableRuntime, StorageEnableRuntimeApi,
};
use shc_common::{
    typed_store::{CFDequeAPI, ProvidesTypedDbSingleAccess},
    types::{BlockHash, BlockNumber, Fingerprint, ProviderId, StorageRequestMetadata},
};
use shc_forest_manager::traits::ForestStorageHandler;
use storage_hub_runtime::RuntimeEvent;

use crate::{
    events::{
        FileDeletionRequest, FinalisedBucketMovedAway, FinalisedMspStopStoringBucketInsolventUser,
        FinalisedMspStoppedStoringBucket, FinalisedProofSubmittedForPendingFileDeletionRequest,
        ForestWriteLockTaskData, MoveBucketRequestedForMsp, NewStorageRequest,
        ProcessFileDeletionRequest, ProcessFileDeletionRequestData,
        ProcessMspRespondStoringRequest, ProcessMspRespondStoringRequestData,
        ProcessStopStoringForInsolventUserRequest, ProcessStopStoringForInsolventUserRequestData,
        StartMovedBucketDownload,
    },
    handler::LOG_TARGET,
    state::{
        OngoingProcessFileDeletionRequestCf, OngoingProcessMspRespondStorageRequestCf,
        OngoingProcessStopStoringForInsolventUserRequestCf,
    },
    types::ManagedProvider,
    BlockchainService,
};

// TODO: Make this configurable in the config file
const MAX_BATCH_MSP_RESPOND_STORE_REQUESTS: u32 = 100;

impl<FSH, Runtime> BlockchainService<FSH, Runtime>
where
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    /// Handles the initial sync of a MSP, after coming out of syncing mode.
    ///
    /// Steps:
    /// TODO
    pub(crate) fn msp_initial_sync(&self, block_hash: H256, msp_id: ProviderId) {
        // TODO: Send events to check that this node has a Forest Storage for each Bucket this MSP manages.
        // TODO: Catch up to Forest root writes in the Bucket's Forests.

        info!(target: LOG_TARGET, "Checking for storage requests for this MSP");

        let storage_requests: BTreeMap<H256, StorageRequestMetadata> = match self
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
            "We have {} pending storage requests",
            storage_requests.len()
        );

        // loop over each pending storage requests to start a new storage request task for the MSP
        for (file_key, sr) in storage_requests {
            self.emit(NewStorageRequest {
                who: sr.owner,
                file_key: file_key.into(),
                bucket_id: sr.bucket_id,
                location: sr.location,
                fingerprint: Fingerprint::from(sr.fingerprint.as_bytes()),
                size: sr.size,
                user_peer_ids: sr.user_peer_ids,
                expires_at: sr.expires_at,
            })
        }
    }

    /// Initialises the block processing flow for a MSP.
    ///
    /// Steps:
    /// 1. Catch up to Forest root changes in the Forests of the Buckets this MSP manages.
    pub(crate) async fn msp_init_block_processing<Block>(
        &self,
        _block_hash: &H256,
        _block_number: &BlockNumber,
        tree_route: TreeRoute<Block>,
    ) where
        Block: cumulus_primitives_core::BlockT<Hash = H256>,
    {
        self.forest_root_changes_catchup(&tree_route).await;
    }

    /// Processes new block imported events that are only relevant for an MSP.
    pub(crate) fn msp_process_block_import_events(&self, _block_hash: &H256, event: RuntimeEvent) {
        let managed_msp_id = match &self.maybe_managed_provider {
            Some(ManagedProvider::Msp(msp_handler)) => &msp_handler.msp_id,
            _ => {
                error!(target: LOG_TARGET, "`msp_process_block_events` should only be called if the node is managing a MSP. Found [{:?}] instead.", self.maybe_managed_provider);
                return;
            }
        };

        match event {
            RuntimeEvent::FileSystem(pallet_file_system::Event::MoveBucketAccepted {
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
            RuntimeEvent::FileSystem(pallet_file_system::Event::FileDeletionRequest {
                user,
                file_key,
                file_size,
                bucket_id,
                msp_id,
                proof_of_inclusion,
            }) => {
                // As an MSP, this node is interested in the event only if this node is the MSP being requested to delete a file.
                if managed_msp_id == &msp_id {
                    self.emit(FileDeletionRequest {
                        user,
                        file_key: file_key.into(),
                        file_size: file_size.into(),
                        bucket_id,
                        msp_id,
                        proof_of_inclusion,
                    });
                }
            }
            // Ignore all other events.
            _ => {}
        }
    }

    /// Processes finality events that are only relevant for an MSP.
    pub(crate) fn msp_process_finality_events(&self, _block_hash: &H256, event: RuntimeEvent) {
        let managed_msp_id = match &self.maybe_managed_provider {
            Some(ManagedProvider::Msp(msp_handler)) => &msp_handler.msp_id,
            _ => {
                error!(target: LOG_TARGET, "`msp_process_finality_events` should only be called if the node is managing a MSP. Found [{:?}] instead.", self.maybe_managed_provider);
                return;
            }
        };

        match event {
            RuntimeEvent::FileSystem(pallet_file_system::Event::MspStoppedStoringBucket {
                msp_id,
                owner,
                bucket_id,
            }) => {
                if msp_id == *managed_msp_id {
                    self.emit(FinalisedMspStoppedStoringBucket {
                        msp_id,
                        owner,
                        bucket_id,
                    })
                }
            }
            RuntimeEvent::FileSystem(
                pallet_file_system::Event::ProofSubmittedForPendingFileDeletionRequest {
                    msp_id,
                    user,
                    file_key,
                    file_size,
                    bucket_id,
                    proof_of_inclusion,
                },
            ) => {
                // Only emit the event if the MSP provided a proof of inclusion, meaning the file key was deleted from the bucket's forest.
                if managed_msp_id == &msp_id && proof_of_inclusion {
                    self.emit(FinalisedProofSubmittedForPendingFileDeletionRequest {
                        user,
                        file_key: file_key.into(),
                        file_size: file_size.into(),
                        bucket_id,
                        msp_id,
                        proof_of_inclusion,
                    });
                }
            }
            RuntimeEvent::FileSystem(pallet_file_system::Event::MoveBucketRequested {
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
            RuntimeEvent::FileSystem(
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
            RuntimeEvent::FileSystem(pallet_file_system::Event::MoveBucketAccepted {
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

            // Ignore all other events.
            _ => {}
        }
    }

    /// TODO: UPDATE THIS FUNCTION TO HANDLE FOREST WRITE LOCKS PER-BUCKET, AND UPDATE DOCS.
    /// Check if there are any pending requests to update the Forest root on the runtime, and process them.
    ///
    /// The priority is given by:
    /// 1. `FileDeletionRequest` over...
    /// 2. `RespondStorageRequest` over...
    /// 3. `StopStoringForInsolventUserRequest`.
    ///
    /// This function is called every time a new block is imported and after each request is queued.
    ///
    /// _IMPORTANT: This check will be skipped if the latest processed block does not match the current best block._
    pub(crate) fn msp_assign_forest_root_write_lock(&mut self) {
        let client_best_hash = self.client.info().best_hash;
        let client_best_number = self.client.info().best_number;

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

                let state_store_context = self.persistent_state.open_rw_context_with_overlay();
                state_store_context
                    .access_value(&OngoingProcessFileDeletionRequestCf)
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

        // At this point we know that the lock is released and we can start processing new requests.
        let state_store_context = self.persistent_state.open_rw_context_with_overlay();
        let mut next_event_data: Option<ForestWriteLockTaskData> = None;

        if self.maybe_managed_provider.is_none() {
            // If there's no Provider being managed, there's no point in checking for pending requests.
            return;
        }

        // We prioritize file deletion requests over respond storing requests since MSPs cannot charge
        // any users while there are pending file deletion requests.
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
        event: RuntimeEvent,
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
            RuntimeEvent::ProofsDealer(pallet_proofs_dealer::Event::MutationsApplied {
                mutations,
                old_root,
                new_root,
                event_info,
            }) => {
                // The mutations are applied to a Bucket's Forest root.
                // Check that this MSP is managing at least one bucket.
                if buckets_managed_by_msp.is_none() {
                    debug!(target: LOG_TARGET, "MSP is not managing any buckets. Skipping mutations applied event.");
                    return;
                }
                let buckets_managed_by_msp = buckets_managed_by_msp
                    .as_ref()
                    .expect("Just checked that this is not None; qed");
                if buckets_managed_by_msp.is_empty() {
                    debug!(target: LOG_TARGET, "Buckets managed by MSP is an empty vector. Skipping mutations applied event.");
                    return;
                }

                // In StorageHub, we assume that all `MutationsApplied` events are emitted by bucket
                // root changes, and they should contain the encoded `BucketId` of the bucket that was mutated
                // in the `event_info` field.
                if event_info.is_none() {
                    error!(target: LOG_TARGET, "MutationsApplied event with `None` event info, when it is expected to contain the BucketId of the bucket that was mutated. This should never happen. This is a bug. Please report it to the StorageHub team.");
                    return;
                }
                let event_info = event_info.expect("Just checked that this is not None; qed");
                let bucket_id = match self
                    .client
                    .runtime_api()
                    .decode_generic_apply_delta_event_info(*block_hash, event_info)
                {
                    Ok(runtime_api_result) => match runtime_api_result {
                        Ok(bucket_id) => bucket_id,
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to decode BucketId from event info: {:?}", e);
                            return;
                        }
                    },
                    Err(e) => {
                        error!(target: LOG_TARGET, "Error while calling runtime API to decode BucketId from event info: {:?}", e);
                        return;
                    }
                };

                // Check if Bucket is managed by this MSP.
                if !buckets_managed_by_msp.contains(&bucket_id) {
                    debug!(target: LOG_TARGET, "Bucket [{:?}] is not managed by this MSP. Skipping mutations applied event.", bucket_id);
                    return;
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
            _ => {}
        }
    }

    fn msp_emit_forest_write_event(&mut self, data: impl Into<ForestWriteLockTaskData>) {
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

        // If this is a respond storage request, stop storing for insolvent user request, or
        // file deletion request, we need to store it in the state store.
        let data = data.into();
        match &data {
            ForestWriteLockTaskData::MspRespondStorageRequest(data) => {
                let state_store_context = self.persistent_state.open_rw_context_with_overlay();
                state_store_context
                    .access_value(&OngoingProcessMspRespondStorageRequestCf)
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
            ForestWriteLockTaskData::StopStoringForInsolventUserRequest(data) => {
                let state_store_context = self.persistent_state.open_rw_context_with_overlay();
                state_store_context
                    .access_value(&OngoingProcessStopStoringForInsolventUserRequestCf)
                    .write(data);
                state_store_context.commit();
            }
            ForestWriteLockTaskData::ConfirmStoringRequest(_) => {
                unreachable!("MSPs do not confirm storing requests the way BSPs do.")
            }
            ForestWriteLockTaskData::SubmitProofRequest(_) => {
                unreachable!("MSPs do not submit proofs.")
            }
        }

        // This is an [`Arc<Mutex<Option<T>>>`] (in this case [`oneshot::Sender<()>`]) instead of just
        // T so that we can keep using the current actors event bus (emit) which requires Clone on the
        // event. Clone is required because there is no constraint on the number of listeners that can
        // subscribe to the event (and each is guaranteed to receive all emitted events).
        let forest_root_write_tx = Arc::new(Mutex::new(Some(tx)));
        match data {
            ForestWriteLockTaskData::MspRespondStorageRequest(data) => {
                self.emit(ProcessMspRespondStoringRequest {
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
}
