use log::{debug, error, info, warn};
use std::collections::BTreeMap;

use sp_api::ProvideRuntimeApi;
use sp_blockchain::TreeRoute;
use sp_core::H256;

use pallet_file_system_runtime_api::FileSystemApi;
use pallet_storage_providers_runtime_api::StorageProvidersApi;
use shc_actors_framework::actor::Actor;
use shc_common::types::{BlockHash, BlockNumber, Fingerprint, ProviderId, StorageRequestMetadata};
use shc_forest_manager::traits::ForestStorageHandler;
use storage_hub_runtime::RuntimeEvent;

use crate::{
    events::{
        FileDeletionRequest, FinalisedBucketMovedAway, FinalisedMspStopStoringBucketInsolventUser,
        FinalisedMspStoppedStoringBucket, FinalisedProofSubmittedForPendingFileDeletionRequest,
        MoveBucketRequestedForMsp, NewStorageRequest, StartMovedBucketDownload,
    },
    handler::LOG_TARGET,
    types::ManagedProvider,
    BlockchainService,
};

// TODO: Make this configurable in the config file
pub(crate) const MAX_BATCH_MSP_RESPOND_STORE_REQUESTS: u32 = 100;

impl<FSH> BlockchainService<FSH>
where
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
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
}
