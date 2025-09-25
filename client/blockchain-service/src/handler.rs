use anyhow::anyhow;
use futures::prelude::*;
use std::{collections::BTreeMap, marker::PhantomData, path::PathBuf, sync::Arc};

use sc_client_api::{
    BlockImportNotification, BlockchainEvents, FinalityNotification, HeaderBackend,
};
use sc_network_types::PeerId;
use sc_service::RpcHandlers;
use sc_tracing::tracing::{debug, error, info, trace, warn};
use shc_common::traits::StorageEnableRuntime;
use sp_api::{ApiError, ProvideRuntimeApi};
use sp_blockchain::TreeRoute;
use sp_keystore::KeystorePtr;
use sp_runtime::{traits::Header, SaturatedConversion, Saturating};

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
    GetBspInfoError, QueryAvailableStorageCapacityError, QueryBucketsOfUserStoredByMspError,
    QueryEarliestChangeCapacityBlockError, QueryMspIdOfBucketIdError,
    QueryProviderMultiaddressesError, QueryStorageProviderCapacityError, StorageProvidersApi,
};
use shc_actors_framework::actor::{Actor, ActorEventLoop};
use shc_common::{
    blockchain_utils::{convert_raw_multiaddresses_to_multiaddr, get_events_at_block},
    typed_store::{CFDequeAPI, ProvidesTypedDbSingleAccess},
    types::{AccountId, BlockNumber, OpaqueBlock, ParachainClient, TickNumber},
};
use shc_forest_manager::traits::ForestStorageHandler;

use crate::{
    capacity_manager::{CapacityRequest, CapacityRequestQueue},
    commands::BlockchainServiceCommand,
    events::BlockchainServiceEventBusProvider,
    state::{BlockchainServiceStateStore, LastProcessedBlockNumberCf},
    transaction::SubmittedTransaction,
    types::{
        FileDistributionInfo, ManagedProvider, MinimalBlockInfo, NewBlockNotificationKind,
        StopStoringForInsolventUserRequest,
    },
};

pub(crate) const LOG_TARGET: &str = "blockchain-service";

/// The BlockchainService actor.
///
/// This actor is responsible for sending extrinsics to the runtime and handling block import notifications.
/// For such purposes, it uses the [`ParachainClient<RuntimeApi>`] to interact with the runtime, the [`RpcHandlers`] to send
/// extrinsics, and the [`Keystore`] to sign the extrinsics.
pub struct BlockchainService<FSH, Runtime>
where
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    /// The configuration for the BlockchainService.
    pub(crate) config: BlockchainServiceConfig<Runtime>,
    /// The event bus provider.
    pub(crate) event_bus_provider: BlockchainServiceEventBusProvider<Runtime>,
    /// The parachain client. Used to interact with the runtime.
    /// TODO: Consider not using `ParachainClient` here.
    pub(crate) client: Arc<ParachainClient<Runtime::RuntimeApi>>,
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
    pub(crate) best_block: MinimalBlockInfo<Runtime>,
    /// Nonce counter for the extrinsics.
    pub(crate) nonce_counter: u32,
    /// A registry of waiters for a block number.
    pub(crate) wait_for_block_request_by_number:
        BTreeMap<BlockNumber<Runtime>, Vec<tokio::sync::oneshot::Sender<anyhow::Result<()>>>>,
    /// A registry of waiters for a tick number.
    pub(crate) wait_for_tick_request_by_number:
        BTreeMap<TickNumber<Runtime>, Vec<tokio::sync::oneshot::Sender<Result<(), ApiError>>>>,
    /// The Provider ID that this node is managing.
    ///
    /// Can be a BSP or an MSP.
    /// This is initialised when the node is in sync.
    pub(crate) maybe_managed_provider: Option<ManagedProvider<Runtime>>,
    /// A persistent state store for the BlockchainService actor.
    pub(crate) persistent_state: BlockchainServiceStateStore,
    /// Notify period value to know when to trigger the NotifyPeriod event.
    ///
    /// This is meant to be used for periodic, low priority tasks.
    pub(crate) notify_period: Option<u32>,
    /// Efficiently manages the capacity changes of storage providers.
    ///
    /// Only required if the node is running as a provider.
    pub(crate) capacity_manager: Option<CapacityRequestQueue<Runtime>>,
    /// Whether the node is running in maintenance mode.
    pub(crate) maintenance_mode: bool,
    /// Phantom data for the Runtime type.
    _runtime: PhantomData<Runtime>,
}

#[derive(Debug, Clone)]
pub struct BlockchainServiceConfig<Runtime>
where
    Runtime: StorageEnableRuntime,
{
    /// Extrinsic retry timeout in seconds.
    pub extrinsic_retry_timeout: u64,
    /// The minimum number of blocks behind the current best block to consider the node out of sync.
    ///
    /// This triggers a catch-up of proofs and Forest root changes in the blockchain service, before
    /// continuing to process incoming events.
    pub sync_mode_min_blocks_behind: BlockNumber<Runtime>,

    /// On blocks that are multiples of this number, the blockchain service will trigger the catch
    /// up of proofs (see [`BlockchainService::proof_submission_catch_up`]).
    pub check_for_pending_proofs_period: BlockNumber<Runtime>,

    /// The maximum number of blocks from the past that will be processed for catching up the root
    /// changes (see [`BlockchainService::forest_root_changes_catchup`]). This constant determines
    /// the maximum size of the `tree_route` in the [`NewBlockNotificationKind::NewBestBlock`] enum
    /// variant.
    pub max_blocks_behind_to_catch_up_root_changes: BlockNumber<Runtime>,

    /// The peer ID of this node.
    pub peer_id: Option<PeerId>,
}

impl<Runtime> Default for BlockchainServiceConfig<Runtime>
where
    Runtime: StorageEnableRuntime,
{
    fn default() -> Self {
        Self {
            extrinsic_retry_timeout: 30,
            sync_mode_min_blocks_behind: 5u32.into(),
            check_for_pending_proofs_period: 4u32.into(),
            max_blocks_behind_to_catch_up_root_changes: 10u32.into(),
            peer_id: None,
        }
    }
}

/// Event loop for the BlockchainService actor.
pub struct BlockchainServiceEventLoop<FSH, Runtime>
where
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    receiver: sc_utils::mpsc::TracingUnboundedReceiver<BlockchainServiceCommand<Runtime>>,
    actor: BlockchainService<FSH, Runtime>,
}

/// Merged event loop message for the BlockchainService actor.
enum MergedEventLoopMessage<Runtime>
where
    Runtime: StorageEnableRuntime,
{
    Command(BlockchainServiceCommand<Runtime>),
    BlockImportNotification(BlockImportNotification<OpaqueBlock>),
    FinalityNotification(FinalityNotification<OpaqueBlock>),
}

/// Implement the ActorEventLoop trait for the BlockchainServiceEventLoop.
impl<FSH, Runtime> ActorEventLoop<BlockchainService<FSH, Runtime>>
    for BlockchainServiceEventLoop<FSH, Runtime>
where
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    fn new(
        actor: BlockchainService<FSH, Runtime>,
        receiver: sc_utils::mpsc::TracingUnboundedReceiver<BlockchainServiceCommand<Runtime>>,
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
            self.receiver
                .map(MergedEventLoopMessage::<Runtime>::Command)
                .boxed(),
            block_import_notification_stream
                .map(|n| MergedEventLoopMessage::<Runtime>::BlockImportNotification(n))
                .boxed(),
            finality_notification_stream
                .map(|n| MergedEventLoopMessage::<Runtime>::FinalityNotification(n))
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
impl<FSH, Runtime> Actor for BlockchainService<FSH, Runtime>
where
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    type Message = BlockchainServiceCommand<Runtime>;
    type EventLoop = BlockchainServiceEventLoop<FSH, Runtime>;
    type EventBusProvider = BlockchainServiceEventBusProvider<Runtime>;

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
                } => match self.send_extrinsic(call, &options).await {
                    Ok(output) => {
                        debug!(target: LOG_TARGET, "Extrinsic sent successfully: {:?}", output);
                        match callback.send(Ok(SubmittedTransaction::new(
                            output.receiver,
                            output.hash,
                            output.nonce,
                            options.timeout(),
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
                    match callback.send(Ok(best_block_info)) {
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

                    if current_block_number >= block_number.saturated_into() {
                        match tx.send(Ok(())) {
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
                BlockchainServiceCommand::WaitForNumBlocks {
                    number_of_blocks,
                    callback,
                } => {
                    let current_block_number = self.client.info().best_number;

                    let (tx, rx) = tokio::sync::oneshot::channel();

                    self.wait_for_block_request_by_number
                        .entry(number_of_blocks.saturating_add(current_block_number.into()))
                        .or_insert_with(Vec::new)
                        .push(tx);

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
                    match callback.send(Ok(pub_key)) {
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
                        .map(convert_raw_multiaddresses_to_multiaddr::<Runtime>);

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
                    if let Some(ManagedProvider::Bsp(_)) = &self.maybe_managed_provider {
                        let state_store_context =
                            self.persistent_state.open_rw_context_with_overlay();
                        state_store_context
                            .pending_confirm_storing_request_deque::<Runtime>()
                            .push_back(request);
                        state_store_context.commit();
                        // We check right away if we can process the request so we don't waste time.
                        self.bsp_assign_forest_root_write_lock();
                        match callback.send(Ok(())) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                            }
                        }
                    } else {
                        error!(target: LOG_TARGET, "Received a QueueConfirmBspRequest command while not managing a BSP. This should never happen. Please report it to the StorageHub team.");
                        match callback.send(Err(anyhow!("Received a QueueConfirmBspRequest command while not managing a BSP. This should never happen. Please report it to the StorageHub team."))) {
                        Ok(_) => {}
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                        }
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
                    self.msp_assign_forest_root_write_lock();
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
                    if let Some(ManagedProvider::Bsp(bsp_handler)) =
                        &mut self.maybe_managed_provider
                    {
                        if let Some(replaced_request) = bsp_handler
                            .pending_submit_proof_requests
                            .replace(request.clone())
                        {
                            trace!(target: LOG_TARGET, "Replacing pending submit proof request {:?} with {:?}", replaced_request, request);
                        }

                        // We check right away if we can process the request so we don't waste time.
                        self.bsp_assign_forest_root_write_lock();
                        match callback.send(Ok(())) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                            }
                        }
                    } else {
                        error!(target: LOG_TARGET, "Received a QueueSubmitProofRequest command while not managing a BSP. This should never happen. Please report it to the StorageHub team.");
                        match callback.send(Err(anyhow!("Received a QueueSubmitProofRequest command while not managing a BSP. This should never happen. Please report it to the StorageHub team."))) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                            }
                        }
                    }
                }
                BlockchainServiceCommand::QueueStopStoringForInsolventUserRequest {
                    request,
                    callback,
                } => {
                    if let Some(managed_bsp_or_msp) = &self.maybe_managed_provider {
                        let state_store_context =
                            self.persistent_state.open_rw_context_with_overlay();
                        state_store_context
                            .pending_stop_storing_for_insolvent_user_request_deque()
                            .push_back(request);
                        state_store_context.commit();

                        // We check right away if we can process the request so we don't waste time.
                        match managed_bsp_or_msp {
                            ManagedProvider::Bsp(_) => {
                                self.bsp_assign_forest_root_write_lock();

                                match callback.send(Ok(())) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                                    }
                                }
                            }
                            ManagedProvider::Msp(_) => {
                                self.msp_assign_forest_root_write_lock();

                                match callback.send(Ok(())) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                                    }
                                }
                            }
                        }
                    } else {
                        error!(target: LOG_TARGET, "Received a QueueStopStoringForInsolventUserRequest command while not managing a MSP or BSP. This should never happen. Please report it to the StorageHub team.");
                        match callback.send(Err(anyhow!("Received a QueueStopStoringForInsolventUserRequest command while not managing a MSP or BSP. This should never happen. Please report it to the StorageHub team."))) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                            }
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
                    let node_pub_key: AccountId<Runtime> = node_pub_key.into();

                    let provider_id = self
                        .client
                        .runtime_api()
                        .get_storage_provider_id(current_block_hash, &node_pub_key)
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
                BlockchainServiceCommand::IncreaseCapacity { request, callback } => {
                    // Create a new channel that will be used to notify completion
                    let (tx, rx) = tokio::sync::oneshot::channel();

                    // The capacity manager handles sending the result back to the caller so we don't need to do anything here. Whether the transaction failed or succeeded, or if the capacity request was never queued, the result will be sent back through the channel by the capacity manager.
                    self.queue_capacity_request(CapacityRequest::new(request, tx))
                        .await;

                    // Send the receiver back through the callback
                    if let Err(e) = callback.send(rx) {
                        error!(target: LOG_TARGET, "Failed to send capacity request receiver: {:?}", e);
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
                BlockchainServiceCommand::QueryBucketsOfUserStoredByMsp {
                    msp_id,
                    user,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let buckets = self
                        .client
                        .runtime_api()
                        .query_buckets_of_user_stored_by_msp(current_block_hash, &msp_id, &user)
                        .unwrap_or_else(|e| {
                            error!(target: LOG_TARGET, "{}", e);
                            Err(QueryBucketsOfUserStoredByMspError::InternalError)
                        });

                    match callback.send(buckets) {
                        Ok(_) => {}
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send back buckets: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::RegisterBspDistributing {
                    file_key,
                    bsp_id,
                    callback,
                } => {
                    if let Some(ManagedProvider::Msp(msp_handler)) =
                        &mut self.maybe_managed_provider
                    {
                        let entry = msp_handler
                            .files_to_distribute
                            .entry(file_key.clone())
                            .or_insert(FileDistributionInfo::new());
                        entry.bsps_distributing.insert(bsp_id);

                        match callback.send(Ok(())) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                            }
                        }
                    } else {
                        error!(target: LOG_TARGET, "Received a RegisterBspDistributing command while not managing a MSP. This should never happen. Please report it to the StorageHub team.");
                        match callback.send(Err(anyhow!("Received a RegisterBspDistributing command while not managing a MSP. This should never happen. Please report it to the StorageHub team."))) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                            }
                        }
                    }
                }
                BlockchainServiceCommand::UnregisterBspDistributing {
                    file_key,
                    bsp_id,
                    callback,
                } => {
                    if let Some(ManagedProvider::Msp(msp_handler)) =
                        &mut self.maybe_managed_provider
                    {
                        if let Some(entry) = msp_handler.files_to_distribute.get_mut(&file_key) {
                            entry.bsps_distributing.remove(&bsp_id);
                        }

                        match callback.send(Ok(())) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                            }
                        }
                    } else {
                        error!(target: LOG_TARGET, "Received an UnregisterBspDistributing command while not managing an MSP. This should never happen. Please report it to the StorageHub team.");
                        match callback.send(Err(anyhow!("Received an UnregisterBspDistributing command while not managing an MSP. This should never happen. Please report it to the StorageHub team."))) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                            }
                        }
                    }
                }
                BlockchainServiceCommand::ReleaseForestRootWriteLock {
                    forest_root_write_tx,
                    callback,
                } => {
                    if let Some(managed_bsp_or_msp) = &self.maybe_managed_provider {
                        // Release the forest root write "lock".
                        let forest_root_write_result = forest_root_write_tx.send(()).map_err(|e| {
                            error!(target: LOG_TARGET, "CRITICAL❗️❗️ This is a bug! Failed to release forest root write lock. This is a critical bug. Please report it to the StorageHub team. \nError while sending the release message: {:?}", e);
                            anyhow!("CRITICAL❗️❗️ This is a bug! Failed to release forest root write lock. This is a critical bug. Please report it to the StorageHub team.")
                        });

                        // Check if there are any pending requests to use the forest root write lock.
                        // If so, we give them the lock right away.
                        if forest_root_write_result.is_ok() {
                            match managed_bsp_or_msp {
                                ManagedProvider::Msp(_) => {
                                    self.msp_assign_forest_root_write_lock();
                                }
                                ManagedProvider::Bsp(_) => {
                                    self.bsp_assign_forest_root_write_lock();
                                }
                            }
                        }

                        match callback.send(forest_root_write_result) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send forest write lock release result: {:?}", e);
                            }
                        }
                    } else {
                        error!(target: LOG_TARGET, "Received a ReleaseForestRootWriteLock command while not managing a MSP or BSP. This should never happen. Please report it to the StorageHub team.");
                        match callback.send(Err(anyhow!("Received a ReleaseForestRootWriteLock command while not managing a MSP or BSP. This should never happen. Please report it to the StorageHub team."))) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                            }
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
                    self.msp_assign_forest_root_write_lock();
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

impl<FSH, Runtime> BlockchainService<FSH, Runtime>
where
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    /// Create a new [`BlockchainService`].
    pub fn new(
        config: BlockchainServiceConfig<Runtime>,
        client: Arc<ParachainClient<Runtime::RuntimeApi>>,
        keystore: KeystorePtr,
        rpc_handlers: Arc<RpcHandlers>,
        forest_storage_handler: FSH,
        rocksdb_root_path: impl Into<PathBuf>,
        notify_period: Option<u32>,
        capacity_request_queue: Option<CapacityRequestQueue<Runtime>>,
        maintenance_mode: bool,
    ) -> Self {
        Self {
            config,
            event_bus_provider: BlockchainServiceEventBusProvider::new(),
            client,
            keystore,
            rpc_handlers,
            forest_storage_handler,
            best_block: MinimalBlockInfo::default(),
            nonce_counter: 0,
            wait_for_block_request_by_number: BTreeMap::new(),
            wait_for_tick_request_by_number: BTreeMap::new(),
            maybe_managed_provider: None,
            persistent_state: BlockchainServiceStateStore::new(rocksdb_root_path.into()),
            notify_period,
            capacity_manager: capacity_request_queue,
            maintenance_mode,
            _runtime: PhantomData,
        }
    }

    async fn handle_block_import_notification(
        &mut self,
        notification: BlockImportNotification<OpaqueBlock>,
    ) {
        // If the node is running in maintenance mode, we don't process block imports.
        if self.maintenance_mode {
            trace!(target: LOG_TARGET, "🔒 Maintenance mode is enabled. Skipping processing of block import notification: {:?}", notification);
            return;
        }

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
        self.init_block_processing(&block_hash);

        // If this is the first block import notification, we might need to catch up.
        // Check if we just came out of syncing mode.
        // We use saturating_sub because in a reorg, there is a potential scenario where the last
        // block processed is higher than the current block number.
        let sync_mode_min_blocks_behind = self.config.sync_mode_min_blocks_behind;
        if block_number.saturating_sub(last_block_processed.number) > sync_mode_min_blocks_behind {
            self.handle_initial_sync(notification).await;
        }

        let block_number = block_number.saturated_into();
        self.process_block_import(&block_hash, &block_number, tree_route)
            .await;
    }

    /// Initialises the Blockchain Service with variables that should be checked and
    /// potentially updated at the start of every block processing.
    ///
    /// Steps:
    /// 1. Sync the latest nonce, used to sign extrinsics (see [`Self::sync_nonce`]).
    /// 2. Get the Provider ID linked to keys in this node's keystore, and set it as
    /// the Provider ID that this node is managing (see [`Self::sync_provider_id`]).
    fn init_block_processing(&mut self, block_hash: &Runtime::Hash) {
        // We query the [`BlockchainService`] account nonce at this height
        // and update our internal counter if it's smaller than the result.
        self.sync_nonce(&block_hash);

        // Get Provider ID linked to keys in this node's keystore and set it
        // as the Provider ID that this node is managing.
        self.sync_provider_id(&block_hash);
    }

    /// Handle the situation after the node comes out of syncing mode (i.e. hasn't processed many of the last blocks).
    async fn handle_initial_sync(&mut self, notification: BlockImportNotification<OpaqueBlock>) {
        let block_hash = notification.hash;
        let block_number = *notification.header.number();

        // If this is the first block import notification, we might need to catch up.
        info!(target: LOG_TARGET, "🥱 Handling coming out of sync mode (synced to #{}: {})", block_number, block_hash);

        // Initialise the Provider.
        match &self.maybe_managed_provider {
            Some(ManagedProvider::Bsp(_)) => {
                self.bsp_initial_sync();
            }
            Some(ManagedProvider::Msp(msp_handler)) => {
                self.msp_initial_sync(block_hash, msp_handler.msp_id);
            }
            None => {
                warn!(target: LOG_TARGET, "No Provider ID found. This node is not managing a Provider.");
            }
        }
    }

    async fn process_block_import(
        &mut self,
        block_hash: &Runtime::Hash,
        block_number: &BlockNumber<Runtime>,
        tree_route: TreeRoute<OpaqueBlock>,
    ) {
        trace!(target: LOG_TARGET, "📠 Processing block import #{}: {}", block_number, block_hash);

        // Provider-specific code to run at the start of every block import.
        match self.maybe_managed_provider {
            Some(ManagedProvider::Bsp(_)) => {
                self.bsp_init_block_processing(block_hash, block_number, tree_route.clone())
                    .await;
            }
            Some(ManagedProvider::Msp(_)) => {
                self.msp_init_block_processing(block_hash, block_number, tree_route.clone())
                    .await;
            }
            None => {
                trace!(target: LOG_TARGET, "No Provider ID found. This node is not managing a Provider.");
            }
        }

        // Notify all tasks waiting for this block number (or lower).
        self.notify_import_block_number(&block_number);

        // Notify all tasks waiting for this tick number (or lower).
        // It is not guaranteed that the tick number will increase at every block import.
        self.notify_tick_number(&block_hash);

        // Notify the capacity manager that a new block has been imported.
        self.notify_capacity_manager(&block_number).await;

        // Process pending requests that update the forest root.
        match &self.maybe_managed_provider {
            Some(ManagedProvider::Bsp(_)) => {
                self.bsp_assign_forest_root_write_lock();
            }
            Some(ManagedProvider::Msp(_)) => {
                self.msp_assign_forest_root_write_lock();
            }
            None => {
                trace!(target: LOG_TARGET, "No Provider ID found. This node is not managing a Provider.");
            }
        }
        // Check that trigger an event every X amount of blocks (specified in config).
        self.check_for_notify(&block_number);

        // Get events from storage.
        // TODO: Handle the `pallet-cr-randomness` events here, if/when we start using them.
        match get_events_at_block::<Runtime>(&self.client, block_hash) {
            Ok(block_events) => {
                for ev in block_events {
                    // Process the events applicable regardless of whether this node is managing a BSP or an MSP.

                    self.process_common_block_import_events(ev.event.clone().into());

                    // Process Provider-specific events.
                    match &self.maybe_managed_provider {
                        Some(ManagedProvider::Bsp(_)) => {
                            self.bsp_process_block_import_events(
                                block_hash,
                                ev.event.clone().into(),
                            );
                        }
                        Some(ManagedProvider::Msp(_)) => {
                            self.msp_process_block_import_events(
                                block_hash,
                                ev.event.clone().into(),
                            );
                        }
                        None => {
                            // * USER SPECIFIC EVENTS. USED ONLY FOR TESTING.
                            self.process_test_user_events(ev.event.clone().into());
                        }
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

        // Provider-specific code to run at the end of every block import.
        match self.maybe_managed_provider {
            Some(ManagedProvider::Bsp(_)) => {
                self.bsp_end_block_processing(block_hash, block_number, tree_route)
                    .await;
            }
            Some(ManagedProvider::Msp(_)) => {
                self.msp_end_block_processing(block_hash, block_number, tree_route)
                    .await;
            }
            None => {
                trace!(target: LOG_TARGET, "No Provider ID found. This node is not managing a Provider.");
            }
        }

        let state_store_context = self.persistent_state.open_rw_context_with_overlay();
        state_store_context
            .access_value(&LastProcessedBlockNumberCf::<Runtime> {
                phantom: Default::default(),
            })
            .write(block_number);
        state_store_context.commit();
    }

    /// Handle a finality notification.
    async fn handle_finality_notification(
        &mut self,
        notification: FinalityNotification<OpaqueBlock>,
    ) {
        let block_hash = notification.hash;
        let block_number = *notification.header.number();

        // If the node is running in maintenance mode, we don't process finality notifications.
        if self.maintenance_mode {
            trace!(target: LOG_TARGET, "🔒 Maintenance mode is enabled. Skipping finality notification #{}: {}", block_number, block_hash);
            return;
        }

        info!(target: LOG_TARGET, "📨 Finality notification #{}: {}", block_number, block_hash);

        // Get events from storage.
        match get_events_at_block::<Runtime>(&self.client, &block_hash) {
            Ok(block_events) => {
                for ev in block_events {
                    // Process the events applicable regardless of whether this node is managing a BSP or an MSP.
                    self.process_common_finality_events(ev.event.clone().into());

                    // Process Provider-specific events.
                    match &self.maybe_managed_provider {
                        Some(ManagedProvider::Bsp(_)) => {
                            self.bsp_process_finality_events(&block_hash, ev.event.clone().into());
                        }
                        Some(ManagedProvider::Msp(_)) => {
                            self.msp_process_finality_events(&block_hash, ev.event.clone().into());
                        }
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
