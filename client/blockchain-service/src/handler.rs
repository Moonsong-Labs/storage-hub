use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
};

use anyhow::anyhow;
use futures::prelude::*;
use sc_client_api::{
    BlockImportNotification, BlockchainEvents, FinalityNotification, HeaderBackend,
};
use sc_network::Multiaddr;
use sc_service::RpcHandlers;
use sc_tracing::tracing::{debug, error, info, trace, warn};
use shc_forest_manager::traits::ForestStorageHandler;
use sp_api::{ApiError, ProvideRuntimeApi};
use sp_blockchain::TreeRoute;
use sp_core::H256;
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::{
    traits::{Header, Zero},
    AccountId32, SaturatedConversion,
};

use pallet_file_system_runtime_api::{
    FileSystemApi, IsStorageRequestOpenToVolunteersError, QueryBspConfirmChunksToProveForFileError,
    QueryFileEarliestVolunteerTickError, QueryMspConfirmChunksToProveForFileError,
};
use pallet_payment_streams_runtime_api::{GetUsersWithDebtOverThresholdError, PaymentStreamsApi};
use pallet_proofs_dealer_runtime_api::{
    GetChallengePeriodError, GetCheckpointChallengesError, GetProofSubmissionRecordError,
    ProofsDealerApi,
};
use pallet_storage_providers_runtime_api::{
    GetBspInfoError, QueryAvailableStorageCapacityError, QueryEarliestChangeCapacityBlockError,
    QueryMspIdOfBucketIdError, QueryProviderMultiaddressesError, QueryStorageProviderCapacityError,
    StorageProvidersApi,
};
use shc_actors_framework::actor::{Actor, ActorEventLoop};
use shc_common::{
    blockchain_utils::{convert_raw_multiaddresses_to_multiaddr, get_events_at_block},
    types::{
        BlockNumber, EitherBucketOrBspId, Fingerprint, ParachainClient, StorageProviderId,
        TickNumber, BCSV_KEY_TYPE,
    },
};
use shp_file_metadata::FileKey;
use storage_hub_runtime::RuntimeEvent;

use crate::{
    commands::BlockchainServiceCommand,
    events::{
        AcceptedBspVolunteer, BlockchainServiceEventBusProvider, BspConfirmStoppedStoring,
        FileDeletionRequest, FinalisedBspConfirmStoppedStoring, FinalisedMspStoppedStoringBucket,
        FinalisedProofSubmittedForPendingFileDeletionRequest, FinalisedTrieRemoveMutationsApplied,
        LastChargeableInfoUpdated, MoveBucketAccepted, MoveBucketExpired, MoveBucketRejected,
        MoveBucketRequested, MoveBucketRequestedForMsp, NewStorageRequest, SlashableProvider,
        SpStopStoringInsolventUser, StartMovedBucketDownload, UserWithoutFunds,
    },
    state::{
        BlockchainServiceStateStore, LastProcessedBlockNumberCf,
        OngoingProcessConfirmStoringRequestCf, OngoingProcessMspRespondStorageRequestCf,
        OngoingProcessStopStoringForInsolventUserRequestCf,
    },
    transaction::SubmittedTransaction,
    typed_store::{CFDequeAPI, ProvidesTypedDbSingleAccess},
    types::{
        ForestStorageSnapshotInfo, MinimalBlockInfo, NewBlockNotificationKind,
        StopStoringForInsolventUserRequest, SubmitProofRequest,
    },
};

pub(crate) const LOG_TARGET: &str = "blockchain-service";

/// The minimum number of blocks behind the current best block to consider the node out of sync.
///
/// This triggers a catch-up of proofs and Forest root changes in the blockchain service, before
/// continuing to process incoming events.
///
/// TODO: Define properly the number of blocks to come out of sync mode
/// TODO: Make this configurable in the config file
pub(crate) const SYNC_MODE_MIN_BLOCKS_BEHIND: BlockNumber = 5;

/// On blocks that are multiples of this number, the blockchain service will trigger the catch
/// up of proofs (see [`BlockchainService::proof_submission_catch_up`]).
///
/// TODO: Make this configurable in the config file
pub(crate) const CHECK_FOR_PENDING_PROOFS_PERIOD: BlockNumber = 4;

/// The maximum number of blocks from the past that will be processed for catching up the root
/// changes (see [`BlockchainService::forest_root_changes_catchup`]). This constant determines
/// the maximum size of the `tree_route` in the [`NewBlockNotificationKind::NewBestBlock`] enum
/// variant.
///
/// TODO: Make this configurable in the config file
pub(crate) const MAX_BLOCKS_BEHIND_TO_CATCH_UP_ROOT_CHANGES: BlockNumber = 10;

/// The BlockchainService actor.
///
/// This actor is responsible for sending extrinsics to the runtime and handling block import notifications.
/// For such purposes, it uses the [`ParachainClient`] to interact with the runtime, the [`RpcHandlers`] to send
/// extrinsics, and the [`Keystore`] to sign the extrinsics.
pub struct BlockchainService<FSH>
where
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    /// The event bus provider.
    pub(crate) event_bus_provider: BlockchainServiceEventBusProvider,
    /// The parachain client. Used to interact with the runtime.
    pub(crate) client: Arc<ParachainClient>,
    /// The keystore. Used to sign extrinsics.
    pub(crate) keystore: KeystorePtr,
    /// The RPC handlers. Used to send extrinsics.
    pub(crate) rpc_handlers: Arc<RpcHandlers>,
    /// The Forest Storage handler.
    ///
    /// This is used to manage Forest Storage instances and update their roots when there are
    /// Forest-root-changing events on-chain, for the Storage Provider managed by this service.
    pub(crate) forest_storage_handler: FSH,
    /// The hash and number of the last best block processed by the BlockchainService.
    ///
    /// This is used to detect when the BlockchainService gets out of syncing mode and should therefore
    /// run some initialisation tasks. Also used to detect reorgs.
    pub(crate) best_block: MinimalBlockInfo,
    /// Nonce counter for the extrinsics.
    pub(crate) nonce_counter: u32,
    /// A registry of waiters for a block number.
    pub(crate) wait_for_block_request_by_number:
        BTreeMap<BlockNumber, Vec<tokio::sync::oneshot::Sender<()>>>,
    /// A registry of waiters for a tick number.
    pub(crate) wait_for_tick_request_by_number:
        BTreeMap<TickNumber, Vec<tokio::sync::oneshot::Sender<Result<(), ApiError>>>>,
    /// The Provider ID that this node is managing.
    ///
    /// Can be a BSP or an MSP.
    /// This is initialised when the node is in sync.
    pub(crate) provider_id: Option<StorageProviderId>,
    /// A map of [`EitherBucketOrBspId`] to the Forest Storage snapshots.
    ///
    /// [`EitherBucketOrBspId`] can be a BSP or the buckets that an MSP has.
    /// Forest Storage snapshots are stored in a BTreeSet, ordered by block number and block hash.
    /// Each BSP or Bucket can have multiple Forest Storage snapshots.
    /// TODO: Remove this `allow(dead_code)` once we have implemented the Forest Storage snapshots.
    #[allow(dead_code)]
    pub(crate) forest_root_snapshots:
        BTreeMap<EitherBucketOrBspId, BTreeSet<ForestStorageSnapshotInfo>>,
    /// A lock to prevent multiple tasks from writing to the runtime Forest root (send transactions) at the same time.
    ///
    /// This is a oneshot channel instead of a regular mutex because we want to "lock" in 1
    /// thread (Blockchain Service) and unlock it at the end of the spawned task. The alternative
    /// would be to send a [`MutexGuard`].
    pub(crate) forest_root_write_lock: Option<tokio::sync::oneshot::Receiver<()>>,
    /// A persistent state store for the BlockchainService actor.
    pub(crate) persistent_state: BlockchainServiceStateStore,
    /// Pending submit proof requests. Note: this is not kept in the persistent state because of
    /// various edge cases when restarting the node, all originating from the "dynamic" way of
    /// computing the next challenges tick. This case is handled separately.
    pub(crate) pending_submit_proof_requests: BTreeSet<SubmitProofRequest>,
    /// Notify period value to know when to trigger the NotifyPeriod event.
    ///
    /// This is meant to be used for periodic, low priority tasks.
    pub(crate) notify_period: Option<u32>,
}

/// Event loop for the BlockchainService actor.
pub struct BlockchainServiceEventLoop<FSH>
where
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    receiver: sc_utils::mpsc::TracingUnboundedReceiver<BlockchainServiceCommand>,
    actor: BlockchainService<FSH>,
}

/// Merged event loop message for the BlockchainService actor.
enum MergedEventLoopMessage<Block>
where
    Block: cumulus_primitives_core::BlockT,
{
    Command(BlockchainServiceCommand),
    BlockImportNotification(BlockImportNotification<Block>),
    FinalityNotification(FinalityNotification<Block>),
}

/// Implement the ActorEventLoop trait for the BlockchainServiceEventLoop.
impl<FSH> ActorEventLoop<BlockchainService<FSH>> for BlockchainServiceEventLoop<FSH>
where
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    fn new(
        actor: BlockchainService<FSH>,
        receiver: sc_utils::mpsc::TracingUnboundedReceiver<BlockchainServiceCommand>,
    ) -> Self {
        Self { actor, receiver }
    }

    async fn run(mut self) {
        info!(target: LOG_TARGET, "💾 StorageHub's Blockchain Service starting up!");

        // Import notification stream to be notified of new blocks.
        // The behaviour of this stream is:
        // 1. While the node is syncing to the tip of the chain (initial sync, i.e. it just started
        // or got behind due to connectivity issues), it will only notify us of re-orgs.
        // 2. Once the node is synced, it will notify us of every new block.
        let block_import_notification_stream = self.actor.client.import_notification_stream();

        // Finality notification stream to be notified of blocks being finalised.
        let finality_notification_stream = self.actor.client.finality_notification_stream();

        // Merging notification streams with command stream.
        let mut merged_stream = stream::select_all(vec![
            self.receiver.map(MergedEventLoopMessage::Command).boxed(),
            block_import_notification_stream
                .map(MergedEventLoopMessage::BlockImportNotification)
                .boxed(),
            finality_notification_stream
                .map(MergedEventLoopMessage::FinalityNotification)
                .boxed(),
        ]);

        // Process incoming messages.
        while let Some(notification) = merged_stream.next().await {
            match notification {
                MergedEventLoopMessage::Command(command) => {
                    self.actor.handle_message(command).await;
                }
                MergedEventLoopMessage::BlockImportNotification(notification) => {
                    self.actor
                        .handle_block_import_notification(notification)
                        .await;
                }
                MergedEventLoopMessage::FinalityNotification(notification) => {
                    self.actor.handle_finality_notification(notification).await;
                }
            };
        }
    }
}

/// Implement the Actor trait for the BlockchainService actor.
impl<FSH> Actor for BlockchainService<FSH>
where
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    type Message = BlockchainServiceCommand;
    type EventLoop = BlockchainServiceEventLoop<FSH>;
    type EventBusProvider = BlockchainServiceEventBusProvider;

    fn handle_message(
        &mut self,
        message: Self::Message,
    ) -> impl std::future::Future<Output = ()> + Send {
        async {
            match message {
                BlockchainServiceCommand::SendExtrinsic {
                    call,
                    options,
                    callback,
                } => match self.send_extrinsic(call, options).await {
                    Ok(output) => {
                        debug!(target: LOG_TARGET, "Extrinsic sent successfully: {:?}", output);
                        match callback.send(Ok(SubmittedTransaction::new(
                            output.receiver,
                            output.hash,
                            output.nonce,
                        ))) {
                            Ok(_) => {
                                trace!(target: LOG_TARGET, "Receiver sent successfully");
                            }
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!(target: LOG_TARGET, "Failed to send extrinsic: {:?}", e);

                        match callback.send(Err(e)) {
                            Ok(_) => {
                                trace!(target: LOG_TARGET, "RPC error sent successfully");
                            }
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send error message through channel: {:?}", e);
                            }
                        }
                    }
                },
                BlockchainServiceCommand::GetExtrinsicFromBlock {
                    block_hash,
                    extrinsic_hash,
                    callback,
                } => {
                    match self
                        .get_extrinsic_from_block(block_hash, extrinsic_hash)
                        .await
                    {
                        Ok(extrinsic) => {
                            debug!(target: LOG_TARGET, "Extrinsic retrieved successfully: {:?}", extrinsic);
                            match callback.send(Ok(extrinsic)) {
                                Ok(_) => {
                                    trace!(target: LOG_TARGET, "Receiver sent successfully");
                                }
                                Err(e) => {
                                    error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!(target: LOG_TARGET, "Failed to retrieve extrinsic: {:?}", e);
                            match callback.send(Err(e)) {
                                Ok(_) => {
                                    trace!(target: LOG_TARGET, "Receiver sent successfully");
                                }
                                Err(e) => {
                                    error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                                }
                            }
                        }
                    }
                }
                BlockchainServiceCommand::UnwatchExtrinsic {
                    subscription_id,
                    callback,
                } => match self.unwatch_extrinsic(subscription_id).await {
                    Ok(output) => {
                        debug!(target: LOG_TARGET, "Extrinsic unwatched successfully: {:?}", output);
                        match callback.send(Ok(())) {
                            Ok(_) => {
                                trace!(target: LOG_TARGET, "Receiver sent successfully");
                            }
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!(target: LOG_TARGET, "Failed to unwatch extrinsic: {:?}", e);
                        match callback.send(Err(e)) {
                            Ok(_) => {
                                trace!(target: LOG_TARGET, "Receiver sent successfully");
                            }
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                            }
                        }
                    }
                },
                BlockchainServiceCommand::GetBestBlockInfo { callback } => {
                    let best_block_info = self.best_block;
                    match callback.send(best_block_info) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "Best block info sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send best block info: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::WaitForBlock {
                    block_number,
                    callback,
                } => {
                    let current_block_number = self.client.info().best_number;

                    let (tx, rx) = tokio::sync::oneshot::channel();

                    if current_block_number >= block_number {
                        match tx.send(()) {
                            Ok(_) => {}
                            Err(_) => {
                                error!(target: LOG_TARGET, "Failed to notify task about waiting block number. \nThis should never happen, in this same code we have both the sender and receiver of the oneshot channel, so it should always be possible to send the message.");
                            }
                        }
                    } else {
                        self.wait_for_block_request_by_number
                            .entry(block_number)
                            .or_insert_with(Vec::new)
                            .push(tx);
                    }

                    match callback.send(rx) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "Block message receiver sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send block message receiver: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::WaitForTick {
                    tick_number,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    // Current Tick should always return a value, unless there's an internal API error.
                    let current_tick_result = self
                        .client
                        .runtime_api()
                        .get_current_tick(current_block_hash);

                    let (tx, rx) = tokio::sync::oneshot::channel();

                    match current_tick_result {
                        Ok(current_tick) => {
                            // If there is no API error, and the current tick is greater than or equal to the tick number
                            // we are waiting for, we notify the task that the tick has been reached.
                            if current_tick >= tick_number {
                                match tx.send(Ok(())) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        error!(target: LOG_TARGET, "Failed to notify task about tick reached: {:?}. \nThis should never happen, in this same code we have both the sender and receiver of the oneshot channel, so it should always be possible to send the message.", e);
                                    }
                                }
                            } else {
                                // If the current tick is less than the tick number we are waiting for, we insert it in
                                // the waiting queue.
                                self.wait_for_tick_request_by_number
                                    .entry(tick_number)
                                    .or_insert_with(Vec::new)
                                    .push(tx);
                            }
                        }
                        Err(e) => {
                            // If there is an API error, we notify the task about it immediately.
                            match tx.send(Err(e)) {
                                Ok(_) => {}
                                Err(e) => {
                                    error!(target: LOG_TARGET, "Failed to notify API error to task querying current tick: {:?}. \nThis should never happen, in this same code we have both the sender and receiver of the oneshot channel, so it should always be possible to send the message.", e);
                                }
                            }
                        }
                    }

                    match callback.send(rx) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "Tick message receiver sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send tick message receiver: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryEarliestChangeCapacityBlock { bsp_id, callback } => {
                    let current_block_hash = self.client.info().best_hash;

                    let earliest_block_to_change_capacity = self
                        .client
                        .runtime_api()
                        .query_earliest_change_capacity_block(current_block_hash, &bsp_id)
                        .unwrap_or_else(|_| {
                            error!(target: LOG_TARGET, "Failed to query earliest block to change capacity");
                            Err(QueryEarliestChangeCapacityBlockError::InternalError)
                        });

                    match callback.send(earliest_block_to_change_capacity) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "Earliest block to change capacity result sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send earliest block to change capacity: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::IsStorageRequestOpenToVolunteers {
                    file_key,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let is_open = self
                        .client
                        .runtime_api()
                        .is_storage_request_open_to_volunteers(current_block_hash, file_key)
                        .unwrap_or_else(|_| {
                            Err(IsStorageRequestOpenToVolunteersError::InternalError)
                        });

                    match callback.send(is_open) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "Storage request open to volunteers result sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send storage request open to volunteers: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryFileEarliestVolunteerTick {
                    bsp_id,
                    file_key,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let earliest_block_to_volunteer = self
                        .client
                        .runtime_api()
                        .query_earliest_file_volunteer_tick(
                            current_block_hash,
                            bsp_id.into(),
                            file_key,
                        )
                        .unwrap_or_else(|_| {
                            Err(QueryFileEarliestVolunteerTickError::InternalError)
                        });

                    match callback.send(earliest_block_to_volunteer) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "Earliest block to volunteer result sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send earliest block to volunteer: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::GetNodePublicKey { callback } => {
                    let pub_key = Self::caller_pub_key(self.keystore.clone());
                    match callback.send(pub_key) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "Node's public key sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send node's public key: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryBspConfirmChunksToProveForFile {
                    bsp_id,
                    file_key,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let chunks_to_prove = self
                        .client
                        .runtime_api()
                        .query_bsp_confirm_chunks_to_prove_for_file(
                            current_block_hash,
                            bsp_id.into(),
                            file_key,
                        )
                        .unwrap_or_else(|_| {
                            Err(QueryBspConfirmChunksToProveForFileError::InternalError)
                        });

                    match callback.send(chunks_to_prove) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "Chunks to prove file sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send chunks to prove file: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryMspConfirmChunksToProveForFile {
                    msp_id,
                    file_key,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let chunks_to_prove = self
                        .client
                        .runtime_api()
                        .query_msp_confirm_chunks_to_prove_for_file(
                            current_block_hash,
                            msp_id.into(),
                            file_key,
                        )
                        .unwrap_or_else(|_| {
                            Err(QueryMspConfirmChunksToProveForFileError::InternalError)
                        });

                    match callback.send(chunks_to_prove) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "Chunks to prove file sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send chunks to prove file: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryProviderMultiaddresses {
                    provider_id,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let multiaddresses = self
                        .client
                        .runtime_api()
                        .query_provider_multiaddresses(current_block_hash, &provider_id)
                        .unwrap_or_else(|_| {
                            error!(target: LOG_TARGET, "Failed to query provider multiaddresses");
                            Err(QueryProviderMultiaddressesError::InternalError)
                        })
                        .map(convert_raw_multiaddresses_to_multiaddr);

                    match callback.send(multiaddresses) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "Provider multiaddresses sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send provider multiaddresses: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryChallengesFromSeed {
                    seed,
                    provider_id,
                    count,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let challenges = self.client.runtime_api().get_challenges_from_seed(
                        current_block_hash,
                        &seed,
                        &provider_id,
                        count,
                    );

                    match callback.send(challenges) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "Challenges sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send challenges: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryForestChallengesFromSeed {
                    seed,
                    provider_id,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let challenges = self.client.runtime_api().get_forest_challenges_from_seed(
                        current_block_hash,
                        &seed,
                        &provider_id,
                    );

                    match callback.send(challenges) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "Challenges sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send challenges: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryLastTickProviderSubmittedProof {
                    provider_id,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let last_tick = self
                        .client
                        .runtime_api()
                        .get_last_tick_provider_submitted_proof(current_block_hash, &provider_id)
                        .unwrap_or_else(|_| Err(GetProofSubmissionRecordError::InternalApiError));

                    match callback.send(last_tick) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "Last tick sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send last tick provider submitted proof: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryChallengePeriod {
                    provider_id,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let challenge_period = self
                        .client
                        .runtime_api()
                        .get_challenge_period(current_block_hash, &provider_id)
                        .unwrap_or_else(|_| {
                            error!(target: LOG_TARGET, "Failed to query challenge period for provider [{:?}]", provider_id);
                            Err(GetChallengePeriodError::InternalApiError)
                        });

                    match callback.send(challenge_period) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "Challenge period sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send challenge period: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryNextChallengeTickForProvider {
                    provider_id,
                    callback,
                } => {
                    let next_challenge_tick =
                        self.get_next_challenge_tick_for_provider(&provider_id);

                    match callback.send(next_challenge_tick) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "Next challenge tick sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send next challenge tick: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryLastCheckpointChallengeTick { callback } => {
                    let current_block_hash = self.client.info().best_hash;

                    let last_checkpoint_tick = self
                        .client
                        .runtime_api()
                        .get_last_checkpoint_challenge_tick(current_block_hash);

                    match callback.send(last_checkpoint_tick) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "Last checkpoint tick sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send last checkpoint challenge tick: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryLastCheckpointChallenges { tick, callback } => {
                    let current_block_hash = self.client.info().best_hash;

                    let checkpoint_challenges = self
                        .client
                        .runtime_api()
                        .get_checkpoint_challenges(current_block_hash, tick)
                        .unwrap_or_else(|_| Err(GetCheckpointChallengesError::InternalApiError));

                    match callback.send(checkpoint_challenges) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "Checkpoint challenges sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send checkpoint challenges: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryProviderForestRoot {
                    provider_id,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let bsp_info = self
                        .client
                        .runtime_api()
                        .get_bsp_info(current_block_hash, &provider_id)
                        .unwrap_or_else(|_| Err(GetBspInfoError::InternalApiError));

                    let root = bsp_info.map(|bsp_info| bsp_info.root);

                    match callback.send(root) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "BSP root sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send BSP root: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryStorageProviderCapacity {
                    provider_id,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let capacity = self
                        .client
                        .runtime_api()
                        .query_storage_provider_capacity(current_block_hash, &provider_id)
                        .unwrap_or_else(|_| Err(QueryStorageProviderCapacityError::InternalError));

                    match callback.send(capacity) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "Storage provider capacity sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send storage provider capacity: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryAvailableStorageCapacity {
                    provider_id,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let capacity = self
                        .client
                        .runtime_api()
                        .query_available_storage_capacity(current_block_hash, &provider_id)
                        .unwrap_or_else(|_| Err(QueryAvailableStorageCapacityError::InternalError));

                    match callback.send(capacity) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "Available storage capacity sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send available storage capacity: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueueConfirmBspRequest { request, callback } => {
                    let state_store_context = self.persistent_state.open_rw_context_with_overlay();
                    state_store_context
                        .pending_confirm_storing_request_deque()
                        .push_back(request);
                    state_store_context.commit();
                    // We check right away if we can process the request so we don't waste time.
                    self.check_pending_forest_root_writes();
                    match callback.send(Ok(())) {
                        Ok(_) => {}
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueueMspRespondStorageRequest { request, callback } => {
                    let state_store_context = self.persistent_state.open_rw_context_with_overlay();
                    state_store_context
                        .pending_msp_respond_storage_request_deque()
                        .push_back(request);
                    state_store_context.commit();
                    // We check right away if we can process the request so we don't waste time.
                    self.check_pending_forest_root_writes();
                    match callback.send(Ok(())) {
                        Ok(_) => {}
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueueSubmitProofRequest { request, callback } => {
                    // The strategy used here is to replace the request in the set with the new request.
                    // This is because new insertions are presumed to be done with more information of the current state of the chain,
                    // so we want to make sure that the request is the most up-to-date one.
                    if let Some(replaced_request) =
                        self.pending_submit_proof_requests.replace(request.clone())
                    {
                        trace!(target: LOG_TARGET, "Replacing pending submit proof request {:?} with {:?}", replaced_request, request);
                    }

                    // We check right away if we can process the request so we don't waste time.
                    self.check_pending_forest_root_writes();
                    match callback.send(Ok(())) {
                        Ok(_) => {}
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueueStopStoringForInsolventUserRequest {
                    request,
                    callback,
                } => {
                    let state_store_context = self.persistent_state.open_rw_context_with_overlay();
                    state_store_context
                        .pending_stop_storing_for_insolvent_user_request_deque()
                        .push_back(request);
                    state_store_context.commit();
                    // We check right away if we can process the request so we don't waste time.
                    self.check_pending_forest_root_writes();
                    match callback.send(Ok(())) {
                        Ok(_) => {}
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryStorageProviderId {
                    maybe_node_pub_key,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let node_pub_key = maybe_node_pub_key
                        .unwrap_or_else(|| Self::caller_pub_key(self.keystore.clone()));

                    let provider_id = self
                        .client
                        .runtime_api()
                        .get_storage_provider_id(current_block_hash, &node_pub_key.into())
                        .map_err(|_| anyhow!("Internal API error"));

                    match callback.send(provider_id) {
                        Ok(_) => {}
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send storage provider ID: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryUsersWithDebt {
                    provider_id,
                    min_debt,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let users_with_debt = self
                        .client
                        .runtime_api()
                        .get_users_with_debt_over_threshold(
                            current_block_hash,
                            &provider_id,
                            min_debt,
                        )
                        .unwrap_or_else(|e| {
                            error!(target: LOG_TARGET, "{}", e);
                            Err(GetUsersWithDebtOverThresholdError::InternalApiError)
                        });

                    match callback.send(users_with_debt) {
                        Ok(_) => {}
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send back users with debt: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryWorstCaseScenarioSlashableAmount {
                    provider_id,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let worst_case_scenario_slashable_amount = self
                        .client
                        .runtime_api()
                        .get_worst_case_scenario_slashable_amount(current_block_hash, provider_id)
                        .map_err(|_| anyhow!("Internal API error"));

                    match callback.send(worst_case_scenario_slashable_amount) {
                        Ok(_) => {}
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send back slashable amount: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QuerySlashAmountPerMaxFileSize { callback } => {
                    // Get the current block hash.
                    let current_block_hash = self.client.info().best_hash;

                    let slash_amount_per_max_file_size = self
                        .client
                        .runtime_api()
                        .get_slash_amount_per_max_file_size(current_block_hash)
                        .map_err(|_| anyhow!("Internal API error"));

                    match callback.send(slash_amount_per_max_file_size) {
                        Ok(_) => {}
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send back `SlashAmountPerMaxFileSize`: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryMspIdOfBucketId {
                    bucket_id,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let msp_id = self
                        .client
                        .runtime_api()
                        .query_msp_id_of_bucket_id(current_block_hash, &bucket_id)
                        .unwrap_or_else(|e| {
                            error!(target: LOG_TARGET, "{}", e);
                            Err(QueryMspIdOfBucketIdError::BucketNotFound)
                        });

                    match callback.send(msp_id) {
                        Ok(_) => {}
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send back MSP ID: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::ReleaseForestRootWriteLock {
                    forest_root_write_tx,
                    callback,
                } => {
                    // Release the forest root write "lock".
                    let forest_root_write_result = forest_root_write_tx.send(()).map_err(|e| {
                        error!(target: LOG_TARGET, "CRITICAL❗️❗️ This is a bug! Failed to release forest root write lock. This is a critical bug. Please report it to the StorageHub team. \nError while sending the release message: {:?}", e);
                        anyhow!(
                            "CRITICAL❗️❗️ This is a bug! Failed to release forest root write lock. This is a critical bug. Please report it to the StorageHub team."
                        )
                    });

                    // Check if there are any pending requests to use the forest root write lock.
                    // If so, we give them the lock right away.
                    if forest_root_write_result.is_ok() {
                        self.check_pending_forest_root_writes();
                    }

                    match callback.send(forest_root_write_result) {
                        Ok(_) => {}
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send forest write lock release result: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueueFileDeletionRequest { request, callback } => {
                    let state_store_context = self.persistent_state.open_rw_context_with_overlay();
                    state_store_context
                        .pending_file_deletion_request_deque()
                        .push_back(request);
                    state_store_context.commit();
                    // We check right away if we can process the request so we don't waste time.
                    self.check_pending_forest_root_writes();
                    match callback.send(Ok(())) {
                        Ok(_) => {}
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                        }
                    }
                }
            }
        }
    }

    fn get_event_bus_provider(&self) -> &Self::EventBusProvider {
        &self.event_bus_provider
    }
}

impl<FSH> BlockchainService<FSH>
where
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    /// Create a new [`BlockchainService`].
    pub fn new(
        client: Arc<ParachainClient>,
        keystore: KeystorePtr,
        rpc_handlers: Arc<RpcHandlers>,
        forest_storage_handler: FSH,
        rocksdb_root_path: impl Into<PathBuf>,
        notify_period: Option<u32>,
    ) -> Self {
        Self {
            event_bus_provider: BlockchainServiceEventBusProvider::new(),
            client,
            keystore,
            rpc_handlers,
            forest_storage_handler,
            best_block: MinimalBlockInfo::default(),
            nonce_counter: 0,
            wait_for_block_request_by_number: BTreeMap::new(),
            wait_for_tick_request_by_number: BTreeMap::new(),
            provider_id: None,
            forest_root_snapshots: BTreeMap::new(),
            forest_root_write_lock: None,
            persistent_state: BlockchainServiceStateStore::new(rocksdb_root_path.into()),
            pending_submit_proof_requests: BTreeSet::new(),
            notify_period,
        }
    }

    async fn handle_block_import_notification<Block>(
        &mut self,
        notification: BlockImportNotification<Block>,
    ) where
        Block: cumulus_primitives_core::BlockT<Hash = H256>,
    {
        let last_block_processed = self.best_block;

        // Check if this new imported block is the new best, and if it causes a reorg.
        let new_block_notification_kind = self.register_best_block_and_check_reorg(&notification);

        // Get the new best block info, and the `TreeRoute`, i.e. the blocks from the old best block to the new best block.
        // A new non-best block is ignored and not processed.
        let (block_info, tree_route) = match new_block_notification_kind {
            NewBlockNotificationKind::NewBestBlock {
                last_best_block_processed: _,
                new_best_block,
                tree_route,
            } => (new_best_block, tree_route),
            NewBlockNotificationKind::NewNonBestBlock(_) => return,
            NewBlockNotificationKind::Reorg {
                old_best_block: _,
                new_best_block,
                tree_route,
            } => (new_best_block, tree_route),
        };
        let MinimalBlockInfo {
            number: block_number,
            hash: block_hash,
        } = block_info;

        info!(target: LOG_TARGET, "📥 Block import notification (#{}): {}", block_number, block_hash);

        // Get provider IDs linked to keys in this node's keystore and update the nonce.
        self.pre_block_processing_checks(&block_hash);

        // If this is the first block import notification, we might need to catch up.
        // Check if we just came out of syncing mode.
        // We use saturating_sub because in a reorg, there is a potential scenario where the last
        // block processed is higher than the current block number.
        if block_number.saturating_sub(last_block_processed.number) > SYNC_MODE_MIN_BLOCKS_BEHIND {
            self.handle_initial_sync(notification).await;
        }

        self.process_block_import(&block_hash, &block_number, tree_route)
            .await;
    }

    fn pre_block_processing_checks(&mut self, block_hash: &H256) {
        // We query the [`BlockchainService`] account nonce at this height
        // and update our internal counter if it's smaller than the result.
        self.sync_nonce(&block_hash);

        // Get Provider ID linked to keys in this node's keystore.
        self.get_provider_id(&block_hash);
    }

    /// Handle the situation after the node comes out of syncing mode (i.e. hasn't processed many of the last blocks).
    async fn handle_initial_sync<Block>(&mut self, notification: BlockImportNotification<Block>)
    where
        Block: cumulus_primitives_core::BlockT<Hash = H256>,
    {
        let block_hash: H256 = notification.hash;
        let block_number: BlockNumber = (*notification.header.number()).saturated_into();

        // If this is the first block import notification, we might need to catch up.
        info!(target: LOG_TARGET, "🥱 Handling coming out of sync mode (synced to #{}: {})", block_number, block_hash);

        // Check if there was an ongoing process confirm storing task.
        let state_store_context = self.persistent_state.open_rw_context_with_overlay();

        // Check if there was an ongoing process confirm storing task.
        // Note: This would only exist if the node was running as a BSP.
        let maybe_ongoing_process_confirm_storing_request = state_store_context
            .access_value(&OngoingProcessConfirmStoringRequestCf)
            .read();

        // If there was an ongoing process confirm storing task, we need to re-queue the requests.
        if let Some(process_confirm_storing_request) = maybe_ongoing_process_confirm_storing_request
        {
            for request in process_confirm_storing_request.confirm_storing_requests {
                state_store_context
                    .pending_confirm_storing_request_deque()
                    .push_back(request);
            }
        }

        // Check if there was an ongoing process msp respond storage request task.
        // Note: This would only exist if the node was running as an MSP.
        let maybe_ongoing_process_msp_respond_storage_request = state_store_context
            .access_value(&OngoingProcessMspRespondStorageRequestCf)
            .read();

        // If there was an ongoing process msp respond storage request task, we need to re-queue the requests.
        if let Some(process_msp_respond_storage_request) =
            maybe_ongoing_process_msp_respond_storage_request
        {
            for request in process_msp_respond_storage_request.respond_storing_requests {
                state_store_context
                    .pending_msp_respond_storage_request_deque()
                    .push_back(request);
            }
        }

        // Check if there was an ongoing process stop storing task.
        let maybe_ongoing_process_stop_storing_for_insolvent_user_request = state_store_context
            .access_value(&OngoingProcessStopStoringForInsolventUserRequestCf)
            .read();

        // If there was an ongoing process stop storing task, we need to re-queue the requests.
        if let Some(process_stop_storing_for_insolvent_user_request) =
            maybe_ongoing_process_stop_storing_for_insolvent_user_request
        {
            state_store_context
                .pending_stop_storing_for_insolvent_user_request_deque()
                .push_back(StopStoringForInsolventUserRequest::new(
                    process_stop_storing_for_insolvent_user_request.who,
                ));
        }

        state_store_context.commit();

        // Initialise the Provider.
        match self.provider_id {
            Some(StorageProviderId::BackupStorageProvider(bsp_id)) => {
                self.proof_submission_catch_up(&block_hash, &bsp_id);
                // TODO: Send events to check that this node has a Forest Storage for the BSP that it manages.
                // TODO: Catch up to Forest root writes in the BSP Forest.
            }
            Some(StorageProviderId::MainStorageProvider(_msp_id)) => {
                // TODO: Send events to check that this node has a Forest Storage for each Bucket this MSP manages.
                // TODO: Catch up to Forest root writes in the Bucket's Forests.
            }
            None => {
                warn!(target: LOG_TARGET, "No Provider ID found. This node is not managing a Provider.");
            }
        }
    }

    async fn process_block_import<Block>(
        &mut self,
        block_hash: &H256,
        block_number: &BlockNumber,
        tree_route: TreeRoute<Block>,
    ) where
        Block: cumulus_primitives_core::BlockT<Hash = H256>,
    {
        trace!(target: LOG_TARGET, "📠 Processing block import #{}: {}", block_number, block_hash);

        // Before triggering any task, we make sure to be caught up to the Forest roots on-chain.
        self.forest_root_changes_catchup(&tree_route).await;

        // Trigger catch up of proofs if the block is a multiple of `CHECK_FOR_PENDING_PROOFS_PERIOD`.
        // This is only relevant if this node is managing a BSP.
        if let Some(StorageProviderId::BackupStorageProvider(bsp_id)) = &self.provider_id {
            if block_number % CHECK_FOR_PENDING_PROOFS_PERIOD == BlockNumber::zero() {
                self.proof_submission_catch_up(block_hash, bsp_id);
            }
        }

        // Notify all tasks waiting for this block number (or lower).
        self.notify_import_block_number(&block_number);

        // Notify all tasks waiting for this tick number (or lower).
        // It is not guaranteed that the tick number will increase at every block import.
        self.notify_tick_number(&block_hash);

        // Process pending requests that update the forest root.
        self.check_pending_forest_root_writes();

        // Check that trigger an event every X amount of blocks (specified in config).
        self.check_for_notify(&block_number);

        let state_store_context = self.persistent_state.open_rw_context_with_overlay();
        // Get events from storage.
        // TODO: Handle the `pallet-cr-randomness` events here.
        match get_events_at_block(&self.client, block_hash) {
            Ok(block_events) => {
                // Process the events.
                for ev in block_events {
                    match ev.event.clone() {
                        // New storage request event coming from pallet-file-system.
                        RuntimeEvent::FileSystem(
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
                        ) => self.emit(NewStorageRequest {
                            who,
                            file_key: FileKey::from(file_key.as_ref()),
                            bucket_id,
                            location,
                            fingerprint: fingerprint.as_ref().into(),
                            size,
                            user_peer_ids: peer_ids,
                            expires_at,
                        }),
                        // A Provider's challenge cycle has been initialised.
                        RuntimeEvent::ProofsDealer(
                            pallet_proofs_dealer::Event::NewChallengeCycleInitialised {
                                current_tick: _,
                                next_challenge_deadline: _,
                                provider: provider_id,
                                maybe_provider_account,
                            },
                        ) => {
                            // This node only cares if the Provider account matches one of the accounts in the keystore.
                            if let Some(account) = maybe_provider_account {
                                let account: Vec<u8> =
                                    <sp_runtime::AccountId32 as AsRef<[u8; 32]>>::as_ref(&account)
                                        .to_vec();
                                if self.keystore.has_keys(&[(account.clone(), BCSV_KEY_TYPE)]) {
                                    // If so, add the Provider ID to the list of Providers that this node is monitoring.
                                    info!(target: LOG_TARGET, "🔑 New Provider ID to monitor [{:?}] for account [{:?}]", provider_id, account);

                                    // Managing more than one Provider is not supported, so if this node is already managing another Provider, emit a warning
                                    // and stop managing it, in favour of the new Provider.
                                    if let Some(managed_provider) = &self.provider_id {
                                        let managed_provider_id = match managed_provider {
                                            StorageProviderId::BackupStorageProvider(bsp_id) => {
                                                bsp_id
                                            }
                                            StorageProviderId::MainStorageProvider(msp_id) => {
                                                msp_id
                                            }
                                        };
                                        if managed_provider_id != &provider_id {
                                            warn!(target: LOG_TARGET, "🔄 This node is already managing a Provider. Stopping managing Provider ID {:?} in favour of Provider ID {:?}", managed_provider, provider_id);
                                        }
                                    }

                                    // Only BSPs can be challenged, therefore this is a BSP.
                                    self.provider_id =
                                        Some(StorageProviderId::BackupStorageProvider(provider_id));
                                }
                            }
                        }
                        // New challenge seed event coming from pallet-proofs-dealer.
                        RuntimeEvent::ProofsDealer(
                            pallet_proofs_dealer::Event::NewChallengeSeed {
                                challenges_ticker,
                                seed: _,
                            },
                        ) => {
                            // This event is relevant in case the Provider managed is a BSP.
                            if let Some(StorageProviderId::BackupStorageProvider(bsp_id)) =
                                &self.provider_id
                            {
                                // Check if the challenges tick is one that this BSP has to submit a proof for.
                                if self.should_provider_submit_proof(
                                    &block_hash,
                                    bsp_id,
                                    &challenges_ticker,
                                ) {
                                    self.proof_submission_catch_up(&block_hash, bsp_id);
                                } else {
                                    trace!(target: LOG_TARGET, "Challenges tick is not the next one to be submitted for Provider [{:?}]", bsp_id);
                                }
                            }
                        }
                        // A provider has been marked as slashable.
                        RuntimeEvent::ProofsDealer(
                            pallet_proofs_dealer::Event::SlashableProvider {
                                provider,
                                next_challenge_deadline,
                            },
                        ) => self.emit(SlashableProvider {
                            provider,
                            next_challenge_deadline,
                        }),
                        // The last chargeable info of a provider has been updated
                        RuntimeEvent::PaymentStreams(
                            pallet_payment_streams::Event::LastChargeableInfoUpdated {
                                provider_id,
                                last_chargeable_tick,
                                last_chargeable_price_index,
                            },
                        ) => {
                            if let Some(managed_provider_id) = &self.provider_id {
                                // We only emit the event if the Provider ID is the one that this node is managing.
                                // It's irrelevant if the Provider ID is a MSP or a BSP.
                                let managed_provider_id = match managed_provider_id {
                                    StorageProviderId::BackupStorageProvider(bsp_id) => bsp_id,
                                    StorageProviderId::MainStorageProvider(msp_id) => msp_id,
                                };
                                if provider_id == *managed_provider_id {
                                    self.emit(LastChargeableInfoUpdated {
                                        provider_id: provider_id,
                                        last_chargeable_tick: last_chargeable_tick,
                                        last_chargeable_price_index: last_chargeable_price_index,
                                    })
                                }
                            }
                        }
                        // A user has been flagged as without funds in the runtime
                        RuntimeEvent::PaymentStreams(
                            pallet_payment_streams::Event::UserWithoutFunds { who },
                        ) => {
                            self.emit(UserWithoutFunds { who });
                        }
                        // A file was correctly deleted from a user without funds
                        RuntimeEvent::FileSystem(
                            pallet_file_system::Event::SpStopStoringInsolventUser {
                                sp_id,
                                file_key,
                                owner,
                                location,
                                new_root,
                            },
                        ) => {
                            if let Some(managed_provider_id) = &self.provider_id {
                                // We only emit the event if the Provider ID is the one that this node is managing.
                                // It's irrelevant if the Provider ID is a MSP or a BSP.
                                let managed_provider_id = match managed_provider_id {
                                    StorageProviderId::BackupStorageProvider(bsp_id) => bsp_id,
                                    StorageProviderId::MainStorageProvider(msp_id) => msp_id,
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
                        RuntimeEvent::FileSystem(
                            pallet_file_system::Event::MoveBucketRejected { bucket_id, msp_id },
                        ) => {
                            // This event is relevant in case the Provider managed is a BSP.
                            if let Some(StorageProviderId::BackupStorageProvider(_)) =
                                &self.provider_id
                            {
                                self.emit(MoveBucketRejected { bucket_id, msp_id });
                            }
                        }
                        RuntimeEvent::FileSystem(
                            pallet_file_system::Event::MoveBucketAccepted {
                                bucket_id,
                                msp_id,
                                value_prop_id,
                            },
                        ) => {
                            match self.provider_id {
                                // As a BSP, this node is interested in the event to allow the new MSP to request files from it.
                                Some(StorageProviderId::BackupStorageProvider(_)) => {
                                    self.emit(MoveBucketAccepted { bucket_id, msp_id });
                                }
                                // As an MSP, this node is interested in the event only if this node is the new MSP.
                                Some(StorageProviderId::MainStorageProvider(own_msp_id))
                                    if own_msp_id == msp_id =>
                                {
                                    self.emit(StartMovedBucketDownload {
                                        bucket_id,
                                        value_prop_id,
                                    });
                                }
                                // Otherwise, ignore the event.
                                _ => {}
                            }
                        }
                        RuntimeEvent::FileSystem(
                            pallet_file_system::Event::MoveBucketRequestExpired { bucket_id },
                        ) => {
                            // This event is relevant in case the Provider managed is a BSP.
                            if let Some(StorageProviderId::BackupStorageProvider(_)) =
                                &self.provider_id
                            {
                                self.emit(MoveBucketExpired { bucket_id });
                            }
                        }
                        RuntimeEvent::FileSystem(
                            pallet_file_system::Event::BspConfirmStoppedStoring {
                                bsp_id,
                                file_key,
                                new_root,
                            },
                        ) => {
                            // This event is relevant in case the Provider managed is a BSP.
                            if let Some(StorageProviderId::BackupStorageProvider(managed_bsp_id)) =
                                &self.provider_id
                            {
                                if managed_bsp_id == &bsp_id {
                                    self.emit(BspConfirmStoppedStoring {
                                        bsp_id,
                                        file_key: file_key.into(),
                                        new_root,
                                    });
                                }
                            }
                        }
                        RuntimeEvent::FileSystem(
                            pallet_file_system::Event::AcceptedBspVolunteer {
                                bsp_id,
                                bucket_id,
                                location,
                                fingerprint,
                                multiaddresses,
                                owner,
                                size,
                            },
                        ) if owner
                            == AccountId32::from(Self::caller_pub_key(self.keystore.clone())) =>
                        {
                            // This event should only be of any use if a node is run by as a user.
                            if self.provider_id.is_none() {
                                log::info!(
                                    target: LOG_TARGET,
                                    "AcceptedBspVolunteer event for BSP ID: {:?}",
                                    bsp_id
                                );

                                // We try to convert the types coming from the runtime into our expected types.
                                let fingerprint: Fingerprint = fingerprint.as_bytes().into();

                                let multiaddress_vec: Vec<Multiaddr> =
                                    convert_raw_multiaddresses_to_multiaddr(multiaddresses);

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
                        RuntimeEvent::FileSystem(
                            pallet_file_system::Event::FileDeletionRequest {
                                user,
                                file_key,
                                file_size,
                                bucket_id,
                                msp_id,
                                proof_of_inclusion,
                            },
                        ) => {
                            // This event is relevant in case the Provider managed is an MSP.
                            if let Some(StorageProviderId::MainStorageProvider(managed_msp_id)) =
                                &self.provider_id
                            {
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
                        }
                        // Ignore all other events.
                        _ => {}
                    }
                }
            }
            Err(e) => {
                // TODO: Handle case where the storage cannot be decoded.
                // TODO: This would happen if we're parsing a block authored with an older version of the runtime, using
                // TODO: a node that has a newer version of the runtime, therefore the EventsVec type is different.
                // TODO: Consider using runtime APIs for getting old data of previous blocks, and this just for current blocks.
                error!(target: LOG_TARGET, "Failed to get events storage element: {:?}", e);
            }
        }
        state_store_context
            .access_value(&LastProcessedBlockNumberCf)
            .write(block_number);
        state_store_context.commit();
    }

    /// Handle a finality notification.
    async fn handle_finality_notification<Block>(
        &mut self,
        notification: FinalityNotification<Block>,
    ) where
        Block: cumulus_primitives_core::BlockT<Hash = H256>,
    {
        let block_hash: H256 = notification.hash;
        let block_number: BlockNumber = (*notification.header.number()).saturated_into();

        info!(target: LOG_TARGET, "📨 Finality notification #{}: {}", block_number, block_hash);

        // Get events from storage.
        match get_events_at_block(&self.client, &block_hash) {
            Ok(block_events) => {
                // Process the events.
                for ev in block_events {
                    match ev.event.clone() {
                        RuntimeEvent::ProofsDealer(
                            pallet_proofs_dealer::Event::MutationsAppliedForProvider {
                                provider_id,
                                mutations,
                                old_root: _,
                                new_root,
                            },
                        ) => {
                            // This event is relevant in case the Provider managed is a BSP.
                            if let Some(StorageProviderId::BackupStorageProvider(managed_bsp_id)) =
                                &self.provider_id
                            {
                                // We only emit the event if the Provider ID is the one that this node is managing.
                                if provider_id == *managed_bsp_id {
                                    self.emit(FinalisedTrieRemoveMutationsApplied {
                                        provider_id,
                                        mutations: mutations.clone().into(),
                                        new_root,
                                    })
                                }
                            }
                        }
                        RuntimeEvent::FileSystem(
                            pallet_file_system::Event::MspStoppedStoringBucket {
                                msp_id,
                                owner,
                                bucket_id,
                            },
                        ) => {
                            // This event is relevant in case the Provider managed is an MSP.
                            if let Some(StorageProviderId::MainStorageProvider(managed_msp_id)) =
                                &self.provider_id
                            {
                                if msp_id == *managed_msp_id {
                                    self.emit(FinalisedMspStoppedStoringBucket {
                                        msp_id,
                                        owner,
                                        bucket_id,
                                    })
                                }
                            }
                        }
                        RuntimeEvent::FileSystem(
                            pallet_file_system::Event::BspConfirmStoppedStoring {
                                bsp_id,
                                file_key,
                                new_root,
                            },
                        ) => {
                            // This event is relevant in case the Provider managed is a BSP.
                            if let Some(StorageProviderId::BackupStorageProvider(managed_bsp_id)) =
                                &self.provider_id
                            {
                                if managed_bsp_id == &bsp_id {
                                    self.emit(FinalisedBspConfirmStoppedStoring {
                                        bsp_id,
                                        file_key: file_key.into(),
                                        new_root,
                                    });
                                }
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
                            // This event is relevant in case the Provider managed is an MSP.
                            if let Some(StorageProviderId::MainStorageProvider(managed_msp_id)) =
                                &self.provider_id
                            {
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
                        }
                        RuntimeEvent::FileSystem(
                            pallet_file_system::Event::MoveBucketRequested {
                                who: _,
                                bucket_id,
                                new_msp_id,
                                new_value_prop_id,
                            },
                        ) => {
                            match self.provider_id {
                                // As a BSP, this node is interested in the event to allow the new MSP to request files from it.
                                Some(StorageProviderId::BackupStorageProvider(_)) => {
                                    self.emit(MoveBucketRequested {
                                        bucket_id,
                                        new_msp_id,
                                    });
                                }
                                // As an MSP, this node is interested in the event only if this node is the new MSP.
                                Some(StorageProviderId::MainStorageProvider(msp_id))
                                    if msp_id == new_msp_id =>
                                {
                                    self.emit(MoveBucketRequestedForMsp {
                                        bucket_id,
                                        value_prop_id: new_value_prop_id,
                                    });
                                }
                                // Otherwise, ignore the event.
                                _ => {}
                            }
                        }
                        // Ignore all other events.
                        _ => {}
                    }
                }
            }
            Err(e) => {
                // TODO: Handle case where the storage cannot be decoded.
                // TODO: This would happen if we're parsing a block authored with an older version of the runtime, using
                // TODO: a node that has a newer version of the runtime, therefore the EventsVec type is different.
                // TODO: Consider using runtime APIs for getting old data of previous blocks, and this just for current blocks.
                error!(target: LOG_TARGET, "Failed to get events storage element: {:?}", e);
            }
        }
    }
}
