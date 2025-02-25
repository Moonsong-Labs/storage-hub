use log::{debug, error, trace};
use pallet_proofs_dealer_runtime_api::ProofsDealerApi;
use pallet_proofs_dealer_runtime_api::{GetChallengePeriodError, GetChallengeSeedError};
use sc_client_api::HeaderBackend;
use shc_actors_framework::actor::Actor;
use shc_common::types::BlockNumber;
use shc_forest_manager::traits::ForestStorageHandler;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::TreeRoute;
use sp_core::H256;
use sp_runtime::traits::Zero;
use storage_hub_runtime::RuntimeEvent;

use crate::events::{
    BspConfirmStoppedStoring, FinalisedBspConfirmStoppedStoring,
    FinalisedTrieRemoveMutationsApplied, MoveBucketAccepted, MoveBucketExpired, MoveBucketRejected,
    MoveBucketRequested,
};
use crate::{
    events::MultipleNewChallengeSeeds,
    handler::{CHECK_FOR_PENDING_PROOFS_PERIOD, LOG_TARGET},
    types::ManagedProvider,
    BlockchainService,
};

impl<FSH> BlockchainService<FSH>
where
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    /// Handles the initial sync of a BSP, after coming out of syncing mode.
    ///
    /// Steps:
    /// 1. Catch up to the latest proof submissions that were missed due to a node restart.
    pub(crate) fn bsp_initial_sync(&mut self) {
        self.proof_submission_catch_up(&self.client.info().best_hash);
        // TODO: Send events to check that this node has a Forest Storage for the BSP that it manages.
        // TODO: Catch up to Forest root writes in the BSP Forest.
    }

    /// Initialises the block processing flow for a BSP.
    ///
    /// Steps:
    /// 1. Catch up to Forest root changes in this BSP's Forest.
    /// 2. In blocks that are a multiple of [`CHECK_FOR_PENDING_PROOFS_PERIOD`], catch up to proof submissions for the current tick.
    pub(crate) fn bsp_init_block_processing<Block>(
        &mut self,
        block_hash: &H256,
        block_number: &BlockNumber,
        tree_route: TreeRoute<Block>,
    ) where
        Block: cumulus_primitives_core::BlockT<Hash = H256>,
    {
        self.forest_root_changes_catchup(&tree_route);
        if block_number % CHECK_FOR_PENDING_PROOFS_PERIOD == BlockNumber::zero() {
            self.proof_submission_catch_up(block_hash);
        }
    }

    /// Processes new block imported events that are only relevant for a BSP.
    pub(crate) fn bsp_process_block_events(&self, block_hash: &H256, event: RuntimeEvent) {
        let managed_bsp_id = match &self.maybe_managed_provider {
            Some(ManagedProvider::Bsp(bsp_handler)) => &bsp_handler.bsp_id,
            _ => {
                error!(target: LOG_TARGET, "`bsp_process_block_events` should only be called if the node is managing a BSP. Found [{:?}] instead.", self.maybe_managed_provider);
                return;
            }
        };

        match event {
            RuntimeEvent::ProofsDealer(pallet_proofs_dealer::Event::NewChallengeSeed {
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
            RuntimeEvent::FileSystem(pallet_file_system::Event::MoveBucketRejected {
                bucket_id,
                msp_id,
            }) => {
                self.emit(MoveBucketRejected { bucket_id, msp_id });
            }
            RuntimeEvent::FileSystem(pallet_file_system::Event::MoveBucketAccepted {
                bucket_id,
                msp_id,
                value_prop_id: _,
            }) => {
                // As a BSP, this node is interested in the event to allow the new MSP to request files from it.
                self.emit(MoveBucketAccepted { bucket_id, msp_id });
            }
            RuntimeEvent::FileSystem(pallet_file_system::Event::MoveBucketRequestExpired {
                bucket_id,
            }) => {
                self.emit(MoveBucketExpired { bucket_id });
            }
            RuntimeEvent::FileSystem(pallet_file_system::Event::BspConfirmStoppedStoring {
                bsp_id,
                file_key,
                new_root,
            }) => {
                if managed_bsp_id == &bsp_id {
                    self.emit(BspConfirmStoppedStoring {
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

    /// Processes finality events that are only relevant for a BSP.
    pub(crate) fn bsp_process_finality_events(&self, _block_hash: &H256, event: RuntimeEvent) {
        let managed_bsp_id = match &self.maybe_managed_provider {
            Some(ManagedProvider::Bsp(bsp_handler)) => &bsp_handler.bsp_id,
            _ => {
                error!(target: LOG_TARGET, "`bsp_process_finality_events` should only be called if the node is managing a BSP. Found [{:?}] instead.", self.maybe_managed_provider);
                return;
            }
        };

        match event {
            RuntimeEvent::ProofsDealer(
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
            RuntimeEvent::FileSystem(pallet_file_system::Event::BspConfirmStoppedStoring {
                bsp_id,
                file_key,
                new_root,
            }) => {
                if managed_bsp_id == &bsp_id {
                    self.emit(FinalisedBspConfirmStoppedStoring {
                        bsp_id,
                        file_key: file_key.into(),
                        new_root,
                    });
                }
            }
            RuntimeEvent::FileSystem(pallet_file_system::Event::MoveBucketRequested {
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

    /// Emits a [`MultipleNewChallengeSeeds`] event with all the pending proof submissions for this provider.
    /// This is used to catch up to the latest proof submissions that were missed due to a node restart.
    /// Also, it can help to catch up to proofs in case there is a change in the BSP's stake (therefore
    /// also a change in it's challenge period).
    ///
    /// IMPORTANT: This function takes into account whether a proof should be submitted for the current tick.
    fn proof_submission_catch_up(&self, current_block_hash: &H256) {
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
}
