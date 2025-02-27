use log::error;
use shc_actors_framework::actor::Actor;
use shc_common::types::BlockNumber;
use shc_forest_manager::traits::ForestStorageHandler;
use sp_blockchain::TreeRoute;
use sp_core::H256;
use storage_hub_runtime::RuntimeEvent;

use crate::{
    events::{
        FileDeletionRequest, FinalisedMspStoppedStoringBucket,
        FinalisedProofSubmittedForPendingFileDeletionRequest, MoveBucketRequestedForMsp,
        StartMovedBucketDownload,
    },
    handler::LOG_TARGET,
    types::ManagedProvider,
    BlockchainService,
};

impl<FSH> BlockchainService<FSH>
where
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    /// Handles the initial sync of a MSP, after coming out of syncing mode.
    ///
    /// Steps:
    /// TODO
    pub(crate) fn msp_initial_sync(&mut self) {
        // TODO: Send events to check that this node has a Forest Storage for each Bucket this MSP manages.
        // TODO: Catch up to Forest root writes in the Bucket's Forests.
    }

    /// Initialises the block processing flow for a MSP.
    ///
    /// Steps:
    /// 1. Catch up to Forest root changes in the Forests of the Buckets this MSP manages.
    pub(crate) fn msp_init_block_processing<Block>(
        &mut self,
        _block_hash: &H256,
        _block_number: &BlockNumber,
        tree_route: TreeRoute<Block>,
    ) where
        Block: cumulus_primitives_core::BlockT<Hash = H256>,
    {
        self.forest_root_changes_catchup(&tree_route);
    }

    /// Processes new block imported events that are only relevant for an MSP.
    pub(crate) fn msp_process_block_events(&self, _block_hash: &H256, event: RuntimeEvent) {
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
            // Ignore all other events.
            _ => {}
        }
    }
}
