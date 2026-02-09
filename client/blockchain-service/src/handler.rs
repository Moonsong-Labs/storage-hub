use anyhow::anyhow;
use futures::prelude::*;
use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::RwLock;

use sc_client_api::{
    BlockImportNotification, BlockchainEvents, FinalityNotification, HeaderBackend,
};
use sc_network_types::PeerId;
use sc_service::RpcHandlers;
use sc_tracing::tracing::{debug, error, info, trace, warn};
use sc_transaction_pool_api::TransactionStatus;
use shc_common::traits::StorageEnableRuntime;
use sp_api::{ApiError, ProvideRuntimeApi};
use sp_blockchain::TreeRoute;
use sp_keystore::KeystorePtr;
use sp_runtime::{traits::Header, SaturatedConversion, Saturating};

use pallet_file_system_runtime_api::{
    FileSystemApi, IsStorageRequestOpenToVolunteersError, QueryBspConfirmChunksToProveForFileError,
    QueryBspsVolunteeredForFileError, QueryFileEarliestVolunteerTickError,
    QueryMspConfirmChunksToProveForFileError,
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
use shc_blockchain_service_db::{leadership::LeadershipClient, store::PendingTxStore};
use shc_common::{
    blockchain_utils::{convert_raw_multiaddresses_to_multiaddr, get_events_at_block},
    typed_store::CFDequeAPI,
    types::{AccountId, BlockNumber, OpaqueBlock, StorageHubClient, TickNumber},
};
use shc_forest_manager::traits::ForestStorageHandler;
use shc_telemetry::{observe_histogram, MetricsLink, STATUS_FAILURE, STATUS_SUCCESS};

use crate::{
    capacity_manager::{CapacityRequest, CapacityRequestQueue},
    commands::BlockchainServiceCommand,
    events::{BlockchainServiceEventBusProvider, NewStorageRequest},
    state::BlockchainServiceStateStore,
    transaction_manager::{TransactionManager, TransactionManagerConfig},
    types::{
        FileDistributionInfo, ManagedProvider, MinimalBlockInfo, MultiInstancesNodeRole,
        NewBlockNotificationKind,
    },
};

pub(crate) const LOG_TARGET: &str = "blockchain-service";

/// The BlockchainService actor.
///
/// This actor is responsible for sending extrinsics to the runtime and handling block import notifications.
/// For such purposes, it uses the [`StorageHubClient<RuntimeApi>`] to interact with the runtime, the [`RpcHandlers`] to send
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
    /// TODO: Consider not using `StorageHubClient` here.
    pub(crate) client: Arc<StorageHubClient<Runtime::RuntimeApi>>,
    /// The keystore. Used to sign extrinsics.
    pub(crate) keystore: KeystorePtr,
    /// The RPC handlers. Used to submit and watch extrinsics via the local RPC server.
    ///
    /// Wrapped in `RwLock<Option<...>>` because the RPC server is initialized after the
    /// blockchain service is spawned. The handlers start as `None` and are set via the
    /// `SetRpcHandlers` command during the startup phase of `run()`, before any block
    /// processing begins. The `RwLock` is used because the write (from `SetRpcHandlers`)
    /// and reads (from extrinsic submission) access a shared `Arc`; in practice they are
    /// sequential within the single-threaded actor loop.
    pub(crate) rpc_handlers: Arc<RwLock<Option<Arc<RpcHandlers>>>>,
    /// The Forest Storage handler.
    ///
    /// This is used to manage Forest Storage instances and update their roots when there are
    /// Forest-root-changing events on-chain, for the Storage Provider managed by this service.
    pub(crate) forest_storage_handler: FSH,
    /// The hash and number of the block currently being processed by the BlockchainService.
    ///
    /// This is used to detect when the BlockchainService gets out of syncing mode and should therefore
    /// run some initialisation tasks. Also used to detect reorgs.
    ///
    /// Note: This is updated at the START of block processing, so it doesn't indicate that
    /// processing has completed. Use `last_block_processed` for that.
    pub(crate) current_block: MinimalBlockInfo<Runtime>,
    /// The hash and number of the last block for which we've completed import processing.
    ///
    /// Unlike `current_block` which is updated at the start of block processing, this field is
    /// updated at the END of block import processing. This is important for coordinating with
    /// finality notifications, as we should only process finality for blocks that have been
    /// fully import-processed.
    pub(crate) last_block_processed: MinimalBlockInfo<Runtime>,
    /// The hash and number of the last finalised block processed by the BlockchainService.
    pub(crate) last_finalised_block_processed: MinimalBlockInfo<Runtime>,
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
    /// Tracks whether the node has caught up with the chain and completed initial sync tasks.
    ///
    /// This flag starts as `false` and is set to `true` after the first block is processed
    /// via `handle_block_import_notification`. This is the natural "sync completed" signal
    /// because `handle_block_import_notification` only fires for `NetworkBroadcast` blocks
    /// (i.e., live blocks after sync is complete), not for `NetworkInitialSync` blocks.
    ///
    /// This flag is also reset to `false` whenever a `NetworkInitialSync` block is
    /// received, indicating the node has fallen behind and needs to re-sync. This ensures
    /// `handle_initial_sync` runs again after catching up, even if the node wasn't restarted.
    ///
    /// This covers all scenarios:
    /// - Cold start when already synced: First block triggers initial sync
    /// - Cold start needing sync: All sync blocks processed, then the first post-sync block triggers initial sync
    /// - Node falling behind (without restart): Sync blocks reset the flag, then post-sync block triggers initial sync
    ///
    /// When `false`, the first `handle_block_import_notification` call will trigger
    /// `handle_initial_sync` to perform provider-specific initialization tasks:
    /// - MSP: Verify bucket forest roots, emit pending storage requests
    /// - BSP: Verify forest root, catch up on proof submissions
    pub(crate) caught_up: bool,
    /// Transaction manager for tracking pending transactions and managing nonces.
    pub(crate) transaction_manager:
        TransactionManager<Runtime::Hash, Runtime::Call, BlockNumber<Runtime>>,
    /// Channel for transaction watchers to send status updates.
    ///
    /// Watchers send TransactionStatus events for all lifecycle changes (Future, Ready, InBlock,
    /// Retracted, Finalized, Invalid, Dropped, Usurped). Terminal failure states (Invalid, Dropped)
    /// trigger immediate removal from the manager, enabling gap detection without waiting for timeout.
    pub(crate) tx_status_sender: tokio::sync::mpsc::UnboundedSender<(
        u32,
        Runtime::Hash,
        TransactionStatus<Runtime::Hash, Runtime::Hash>,
    )>,
    /// Channel for forest root write permit release notifications.
    ///
    /// When a [`ForestWritePermitGuard`][crate::types::ForestWritePermitGuard] is dropped from a task, it sends a notification through
    /// this channel and is received by the [`BlockchainServiceEventLoop::permit_release_receiver`], which will trigger reassignment
    /// of the forest root write lock via [`BlockchainService::handle_permit_released`].
    pub(crate) permit_release_sender: tokio::sync::mpsc::UnboundedSender<()>,
    /// Optional pending tx store (Postgres). When present, tx sends and cleanups are persisted.
    pub(crate) pending_tx_store: Option<PendingTxStore>,
    /// Current role of this node in the HA group.
    pub(crate) role: MultiInstancesNodeRole,
    /// Dedicated leadership connection used to hold advisory locks when DB is enabled.
    pub(crate) leadership_conn: Option<LeadershipClient>,
    /// Queue for finality notifications that arrive before their corresponding block import.
    ///
    /// This handles the race condition where finality notifications can outpace block import
    /// notifications if the block import processing is lagging behind.
    /// When a finality notification arrives for a block that hasn't been
    /// import-processed yet (`finality_block_number > last_block_processed.number`),
    /// it's queued here and processed after block import catches up.
    ///
    /// The queue is drained at the end of each block import notification, processing any
    /// queued finality notifications whose block numbers are now <= last_block_processed.number.
    pub(crate) pending_finality_notifications: VecDeque<FinalityNotification<OpaqueBlock>>,
    /// Metrics link for recording telemetry.
    ///
    /// Used for recording command lifecycle metrics (pending count, processing duration)
    /// and block processing metrics (block import/finality durations).
    pub(crate) metrics: MetricsLink,
}

#[derive(Debug, Clone)]
pub struct BlockchainServiceConfig<Runtime>
where
    Runtime: StorageEnableRuntime,
{
    /// Extrinsic retry timeout in seconds.
    pub extrinsic_retry_timeout: u64,

    /// On blocks that are multiples of this number, the blockchain service will trigger the catch
    /// up of proofs (see [`BlockchainService::proof_submission_catch_up`]).
    pub check_for_pending_proofs_period: BlockNumber<Runtime>,

    /// The peer ID of this node.
    pub peer_id: Option<PeerId>,

    /// Whether MSP nodes should distribute files to BSPs.
    ///
    /// If set to `false`, MSP distribution tasks will be disabled even if the node
    /// is otherwise configured as a distributor (e.g. has a peer_id).
    pub enable_msp_distribute_files: bool,
    /// Optional Postgres URL for the pending transactions DB. If None, DB is disabled.
    pub pending_db_url: Option<String>,

    /// Maximum number of BSP confirm storing requests to batch together.
    pub bsp_confirm_file_batch_size: u32,

    /// Maximum number of MSP respond storage requests to batch together.
    pub msp_respond_storage_batch_size: u32,
}

impl<Runtime> Default for BlockchainServiceConfig<Runtime>
where
    Runtime: StorageEnableRuntime,
{
    fn default() -> Self {
        Self {
            extrinsic_retry_timeout: 30,
            check_for_pending_proofs_period: 4u32.into(),
            peer_id: None,
            enable_msp_distribute_files: false,
            pending_db_url: None,
            bsp_confirm_file_batch_size: 20,
            msp_respond_storage_batch_size: 20,
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
    tx_status_receiver: tokio::sync::mpsc::UnboundedReceiver<(
        u32,
        Runtime::Hash,
        TransactionStatus<Runtime::Hash, Runtime::Hash>,
    )>,
    /// Receiver for forest root write permit release notifications.
    ///
    /// Receives notifications when [`ForestWritePermitGuard`][crate::types::ForestWritePermitGuard] instances are dropped,
    /// triggering the processing of pending forest write requests.
    permit_release_receiver: tokio::sync::mpsc::UnboundedReceiver<()>,
}

/// Merged event loop message for the BlockchainService actor.
enum MergedEventLoopMessage<Runtime>
where
    Runtime: StorageEnableRuntime,
{
    Command(BlockchainServiceCommand<Runtime>),
    BlockImportNotification(BlockImportNotification<OpaqueBlock>),
    SyncBlockNotification(BlockImportNotification<OpaqueBlock>),
    FinalityNotification(FinalityNotification<OpaqueBlock>),
    TxStatusUpdate(
        (
            u32,
            Runtime::Hash,
            TransactionStatus<Runtime::Hash, Runtime::Hash>,
        ),
    ),
    /// Notification that a forest root write permit has been released.
    ///
    /// Sent by `ForestWritePermitGuard::drop()` when a task with the forest write lock succeeds, fails or panics.
    /// Triggers [`BlockchainService::handle_permit_released`] to process any pending forest write requests.
    ForestRootWritePermitReleased,
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
        // Create transaction status channel and wire sender into actor
        let (tx_status_sender, tx_status_receiver) = tokio::sync::mpsc::unbounded_channel();

        // Create permit release channel for forest root write lock notifications
        let (permit_release_sender, permit_release_receiver) =
            tokio::sync::mpsc::unbounded_channel();

        let mut actor = actor;
        actor.tx_status_sender = tx_status_sender;
        actor.permit_release_sender = permit_release_sender;

        Self {
            actor,
            receiver,
            tx_status_receiver,
            permit_release_receiver,
        }
    }

    async fn run(mut self) {
        info!(target: LOG_TARGET, "💾 StorageHub's Blockchain Service starting up!");

        // Initialise pending transactions DB store if configured
        self.actor.init_pending_tx_store().await;

        // Wait for RPC handlers before proceeding with any block processing or transaction
        // resubscription. The RPC server is initialized after the blockchain service is spawned,
        // so the SetRpcHandlers command arrives through the command channel shortly after startup.
        // We drain commands here to avoid processing blocks without the ability to submit extrinsics.
        info!(target: LOG_TARGET, "⏳ Waiting for RPC handlers to be available...");
        while let Some(command) = self.receiver.next().await {
            if let BlockchainServiceCommand::SetRpcHandlers { rpc_handlers } = command {
                info!(
                    target: LOG_TARGET,
                    "✅ RPC handlers set for BlockchainService"
                );
                let mut rpc_handlers_guard = self.actor.rpc_handlers.write().await;
                *rpc_handlers_guard = Some(rpc_handlers);
                break;
            }
            // Handle any other commands that arrive before SetRpcHandlers.
            self.actor.handle_message(command).await;
        }

        // Role-specific initialisation. Now that RPC handlers are available, Leader nodes can
        // immediately resubscribe pending transactions without deferring.
        match self.actor.role {
            MultiInstancesNodeRole::Leader => {
                info!(
                    target: LOG_TARGET,
                    "🧑‍✈️ Node role is LEADER; re-subscribing pending transactions from DB"
                );
                // Re-subscribe watchers for eligible pending transactions persisted in DB.
                self.actor
                    .resubscribe_pending_transactions_on_startup()
                    .await;
            }
            MultiInstancesNodeRole::Follower => {
                info!(
                    target: LOG_TARGET,
                    "👂 Node role is FOLLOWER; initialising follower pending-tx view"
                );
                self.actor.init_follower_pending_tx_state().await;
            }
            MultiInstancesNodeRole::Standalone => {
                info!(
                    target: LOG_TARGET,
                    "📦 Node role is STANDALONE; pending transactions will not be persisted or shared across instances"
                );
            }
        }

        // Catch up on any blocks that were imported but not processed
        self.actor.catch_up_missed_blocks().await;

        // Import notification stream to be notified of new blocks.
        // The behaviour of this stream is:
        // 1. While the node is syncing to the tip of the chain (initial sync, i.e. it just started
        // or got behind due to connectivity issues), it will only notify us of re-orgs.
        // 2. Once the node is synced, it will notify us of every new block.
        let block_import_notification_stream = self.actor.client.import_notification_stream();

        // Every block notification stream.
        // Fires for all blocks including sync blocks, which is its main purpose here:
        // - We use it to process mutations during initial sync before state is pruned.
        // - We only process linear chain extensions, not reorgs. This is because, as mentioned, that's handled
        // by the `block_import_notification_stream` above.
        // - After the initial sync period, this notification stream is just ignored.
        let every_block_notification_stream = self.actor.client.every_import_notification_stream();

        // Finality notification stream to be notified of blocks being finalised.
        let finality_notification_stream = self.actor.client.finality_notification_stream();

        // Merging notification streams with command stream.
        let tx_status_stream = futures::stream::unfold(self.tx_status_receiver, |mut rx| async {
            match rx.recv().await {
                Some(item) => Some((item, rx)),
                None => None,
            }
        });

        // Stream for forest root write permit release notifications.
        let permit_release_stream =
            futures::stream::unfold(self.permit_release_receiver, |mut rx| async {
                match rx.recv().await {
                    Some(()) => Some(((), rx)),
                    None => None,
                }
            });

        let mut merged_stream = stream::select_all(vec![
            self.receiver
                .map(MergedEventLoopMessage::<Runtime>::Command)
                .boxed(),
            block_import_notification_stream
                .map(|n| MergedEventLoopMessage::<Runtime>::BlockImportNotification(n))
                .boxed(),
            every_block_notification_stream
                .map(|n| MergedEventLoopMessage::<Runtime>::SyncBlockNotification(n))
                .boxed(),
            finality_notification_stream
                .map(|n| MergedEventLoopMessage::<Runtime>::FinalityNotification(n))
                .boxed(),
            tx_status_stream
                .map(MergedEventLoopMessage::<Runtime>::TxStatusUpdate)
                .boxed(),
            permit_release_stream
                .map(|_| MergedEventLoopMessage::<Runtime>::ForestRootWritePermitReleased)
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
                MergedEventLoopMessage::SyncBlockNotification(notification) => {
                    self.actor
                        .handle_sync_block_notification(notification)
                        .await;
                }
                MergedEventLoopMessage::FinalityNotification(notification) => {
                    self.actor.handle_finality_notification(notification).await;
                }
                MergedEventLoopMessage::TxStatusUpdate((nonce, tx_hash, status)) => {
                    self.actor
                        .handle_transaction_status_update(nonce, tx_hash, status)
                        .await;
                }
                MergedEventLoopMessage::ForestRootWritePermitReleased => {
                    self.actor.handle_permit_released();
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
        // Extract command name before the match (since message will be partially moved)
        let command_name = message.command_name();
        // Clone metrics link for use in async block
        let metrics = self.metrics.clone();

        async move {
            // Start timer for command processing
            let start = std::time::Instant::now();

            // Track command success/failure for metrics
            let mut command_succeeded = true;

            match message {
                BlockchainServiceCommand::SendExtrinsic {
                    call,
                    options,
                    callback,
                } => match self.send_extrinsic(call, &options).await {
                    Ok(output) => {
                        debug!(target: LOG_TARGET, "Extrinsic sent successfully: {:?}", output);
                        match callback.send(Ok(output)) {
                            Ok(_) => {
                                trace!(target: LOG_TARGET, "Submitted extrinsic info sent successfully");
                            }
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send submitted extrinsic info: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!(target: LOG_TARGET, "Failed to send extrinsic: {:?}", e);
                        command_succeeded = false;

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
                            command_succeeded = false;
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
                BlockchainServiceCommand::GetBestBlockInfo { callback } => {
                    let best_block_info = self.last_block_processed;
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
                            command_succeeded = false;
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
                            command_succeeded = false;
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
                            command_succeeded = false;
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
                            command_succeeded = false;
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
                            command_succeeded = false;
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
                            command_succeeded = false;
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
                BlockchainServiceCommand::QueryBspVolunteeredForFile {
                    bsp_id,
                    file_key,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let bsps_volunteered = self
                        .client
                        .runtime_api()
                        .query_bsps_volunteered_for_file(current_block_hash, file_key)
                        .unwrap_or_else(|_| {
                            command_succeeded = false;
                            Err(QueryBspsVolunteeredForFileError::InternalError)
                        });

                    let volunteered = bsps_volunteered.map(|bsps| bsps.contains(&bsp_id));

                    match callback.send(volunteered) {
                        Ok(_) => {
                            trace!(target: LOG_TARGET, "BSP volunteered status sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send BSP volunteered status: {:?}", e);
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
                            command_succeeded = false;
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
                        .unwrap_or_else(|_| {
                            command_succeeded = false;
                            Err(GetProofSubmissionRecordError::InternalApiError)
                        });

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
                            command_succeeded = false;
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
                        .unwrap_or_else(|_| {
                            command_succeeded = false;
                            Err(GetCheckpointChallengesError::InternalApiError)
                        });

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
                        .unwrap_or_else(|_| {
                            command_succeeded = false;
                            Err(GetBspInfoError::InternalApiError)
                        });

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
                        .unwrap_or_else(|_| {
                            command_succeeded = false;
                            Err(QueryStorageProviderCapacityError::InternalError)
                        });

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
                        .unwrap_or_else(|_| {
                            command_succeeded = false;
                            Err(QueryAvailableStorageCapacityError::InternalError)
                        });

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
                        self.queue_confirm_storing_requests(std::iter::once(request));
                        // We check right away if we can process the request so we don't waste time.
                        self.bsp_assign_forest_root_write_lock();
                        match callback.send(Ok(())) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                            }
                        }
                    } else {
                        command_succeeded = false;
                        error!(target: LOG_TARGET, "Received a QueueConfirmBspRequest command while not managing a BSP. This should never happen. Please report it to the StorageHub team.");
                        match callback.send(Err(anyhow!("Received a QueueConfirmBspRequest command while not managing a BSP. This should never happen. Please report it to the StorageHub team."))) {
                        Ok(_) => {}
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                        }
                    }
                    }
                }
                BlockchainServiceCommand::QueueMspRespondStorageRequest { request } => {
                    if let Some(ManagedProvider::Msp(msp_handler)) =
                        &mut self.maybe_managed_provider
                    {
                        let file_key = request.file_key;

                        trace!(
                            target: LOG_TARGET,
                            "QueueMspRespondStorageRequest received for file key [{:x}]",
                            file_key
                        );

                        // Check if file key is already pending (O(1) deduplication).
                        // `insert` returns true if the key was not present (i.e., we should queue).
                        if msp_handler
                            .pending_respond_storage_request_file_keys
                            .insert(file_key)
                        {
                            msp_handler
                                .pending_respond_storage_requests
                                .push_back(request);

                            trace!(
                                target: LOG_TARGET,
                                "File key [{:x}] added to pending queue (size: {})",
                                file_key,
                                msp_handler.pending_respond_storage_requests.len()
                            );

                            // We check right away if we can process the request so we don't waste time.
                            self.msp_assign_forest_root_write_lock();
                        } else {
                            warn!(
                                target: LOG_TARGET,
                                "File key [{:x}] already pending, skipping",
                                file_key
                            );
                        }
                    } else {
                        command_succeeded = false;
                        // Log the invariant violation but don't fail - this is fire-and-forget
                        error!(
                            target: LOG_TARGET,
                            "QueueMspRespondStorageRequest received while not managing an MSP. \
                             This is an invariant violation - please report to StorageHub team."
                        );
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
                        command_succeeded = false;
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
                        command_succeeded = false;
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
                        .map_err(|_| {
                            command_succeeded = false;
                            anyhow!("Internal API error")
                        });

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
                            command_succeeded = false;
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
                            command_succeeded = false;
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
                BlockchainServiceCommand::QueryMinWaitForStopStoring { callback } => {
                    let current_block_hash = self.client.info().best_hash;

                    let min_wait = self
                        .client
                        .runtime_api()
                        .query_min_wait_for_stop_storing(current_block_hash);

                    match callback.send(min_wait) {
                        Ok(_) => {}
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send back MinWaitForStopStoring: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::HasPendingStopStoringRequest {
                    bsp_id,
                    file_key,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let has_request = self.client.runtime_api().has_pending_stop_storing_request(
                        current_block_hash,
                        bsp_id.into(),
                        file_key.into(),
                    );

                    match callback.send(has_request) {
                        Ok(_) => {}
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send back HasPendingStopStoringRequest: {:?}", e);
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
                            command_succeeded = false;
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

                        // Register BSP as one for which the file is being distributed already.
                        // Error if the BSP is already registered.
                        if !entry.bsps_distributing.insert(bsp_id) {
                            command_succeeded = false;
                            error!(target: LOG_TARGET, "BSP {:?} is already registered as distributing file [{:x}]", bsp_id, file_key);
                            match callback.send(Err(anyhow!(
                                "BSP {:?} is already registered as distributing file [{:x}]",
                                bsp_id,
                                file_key
                            ))) {
                                Ok(_) => {}
                                Err(e) => {
                                    error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                                }
                            }
                        } else {
                            match callback.send(Ok(())) {
                                Ok(_) => {}
                                Err(e) => {
                                    error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                                }
                            }
                        }
                    } else {
                        command_succeeded = false;
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
                        command_succeeded = false;
                        error!(target: LOG_TARGET, "Received an UnregisterBspDistributing command while not managing an MSP. This should never happen. Please report it to the StorageHub team.");
                        match callback.send(Err(anyhow!("Received an UnregisterBspDistributing command while not managing an MSP. This should never happen. Please report it to the StorageHub team."))) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                            }
                        }
                    }
                }
                BlockchainServiceCommand::QueryPendingStorageRequests {
                    maybe_file_keys,
                    callback,
                } => {
                    if let Some(ManagedProvider::Msp(msp_handler)) = &self.maybe_managed_provider {
                        let managed_msp_id = msp_handler.msp_id.clone();
                        let current_block_hash = self.client.info().best_hash;

                        // Query pending storage requests (not yet accepted by MSP)
                        match self
                            .client
                            .runtime_api()
                            .pending_storage_requests_by_msp(current_block_hash, managed_msp_id)
                        {
                            Ok(mut sr) => {
                                // If specific file keys provided, filter to only those keys
                                if let Some(file_keys) = maybe_file_keys {
                                    let file_keys_set: HashSet<_> = file_keys
                                        .into_iter()
                                        .map(|k| sp_core::H256::from_slice(k.as_ref()))
                                        .collect();

                                    // From the pending storage requests for this MSP, only keep the ones that
                                    // are in the provided file keys.
                                    sr.retain(|file_key, _| file_keys_set.contains(file_key));
                                }

                                let new_storage_requests: Vec<NewStorageRequest<Runtime>> = sr
                                    .into_iter()
                                    .map(|(file_key, sr)| NewStorageRequest {
                                        who: sr.owner,
                                        file_key: file_key.into(),
                                        bucket_id: sr.bucket_id,
                                        location: sr.location,
                                        fingerprint: sr.fingerprint.as_ref().into(),
                                        size: sr.size,
                                        user_peer_ids: sr.user_peer_ids,
                                        expires_at: sr.expires_at,
                                    })
                                    .collect();

                                match callback.send(Ok(new_storage_requests)) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        error!(target: LOG_TARGET, "Failed to send pending storage requests: {:?}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                command_succeeded = false;
                                error!(target: LOG_TARGET, "Failed to get pending storage requests: {:?}", e);
                                match callback
                                    .send(Err(anyhow!("Failed to get pending storage requests")))
                                {
                                    Ok(_) => {}
                                    Err(e) => {
                                        error!(target: LOG_TARGET, "Failed to send error: {:?}", e);
                                    }
                                }
                            }
                        }
                    } else {
                        command_succeeded = false;
                        error!(target: LOG_TARGET, "`QueryPendingStorageRequests` should only be called if the node is managing a MSP. Found [{:?}] instead.", self.maybe_managed_provider);
                        match callback.send(Err(anyhow!("Node is not managing an MSP"))) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send error: {:?}", e);
                            }
                        }
                    }
                }
                BlockchainServiceCommand::QueryMaxBatchConfirmStorageRequests { callback } => {
                    let current_block_hash = self.client.info().best_hash;
                    let max_batch = match self
                        .client
                        .runtime_api()
                        .get_max_batch_confirm_storage_requests(current_block_hash)
                    {
                        Ok(max) => max,
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to query max batch confirm storage requests: {:?}", e);
                            match callback.send(Err(anyhow!(
                                "Failed to query max batch confirm storage requests"
                            ))) {
                                Ok(_) => {}
                                Err(e) => {
                                    error!(target: LOG_TARGET, "Failed to send error: {:?}", e);
                                }
                            }
                            return;
                        }
                    };
                    match callback.send(Ok(max_batch)) {
                        Ok(_) => {}
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send max batch confirm storage requests: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::AddPendingVolunteerFileKey { file_key } => {
                    if let Some(ManagedProvider::Bsp(bsp_handler)) =
                        &mut self.maybe_managed_provider
                    {
                        debug!(target: LOG_TARGET, "Adding file key [{:?}] to pending volunteer tracking", file_key);
                        bsp_handler.pending_volunteer_file_keys.insert(file_key);
                    } else {
                        error!(target: LOG_TARGET, "AddPendingVolunteerFileKey received while not managing a BSP. This should never happen.");
                    }
                }
                BlockchainServiceCommand::RemovePendingVolunteerFileKey { file_key } => {
                    if let Some(ManagedProvider::Bsp(bsp_handler)) =
                        &mut self.maybe_managed_provider
                    {
                        debug!(target: LOG_TARGET, "Removing file key [{:?}] from pending volunteer tracking", file_key);
                        bsp_handler.pending_volunteer_file_keys.remove(&file_key);
                    } else {
                        error!(target: LOG_TARGET, "RemovePendingVolunteerFileKey received while not managing a BSP. This should never happen.");
                    }
                }
                BlockchainServiceCommand::SetFileKeyStatus { file_key, status } => {
                    if let Some(ManagedProvider::Msp(msp_handler)) =
                        &mut self.maybe_managed_provider
                    {
                        info!(
                            target: LOG_TARGET,
                            "Setting file key [{:x}] status to {:?}",
                            file_key,
                            status
                        );
                        msp_handler
                            .file_key_statuses
                            .insert(file_key, status.into());
                    } else {
                        command_succeeded = false;
                        // Fire-and-forget command, just log the invariant violation
                        error!(
                            target: LOG_TARGET,
                            "SetFileKeyStatus received while not managing an MSP. \
                             This is an invariant violation - please report to StorageHub team."
                        );
                    }
                }
                BlockchainServiceCommand::RemoveFileKeyStatus { file_key } => {
                    if let Some(ManagedProvider::Msp(msp_handler)) =
                        &mut self.maybe_managed_provider
                    {
                        info!(
                            target: LOG_TARGET,
                            "Removing file key [{:x}] from statuses (enabling retry)",
                            file_key
                        );
                        msp_handler.file_key_statuses.remove(&file_key);
                    } else {
                        command_succeeded = false;
                        // Fire-and-forget command, just log the invariant violation
                        error!(
                            target: LOG_TARGET,
                            "RemoveFileKeyStatus received while not managing an MSP. \
                             This is an invariant violation - please report to StorageHub team."
                        );
                    }
                }
                BlockchainServiceCommand::SetRpcHandlers { rpc_handlers } => {
                    // This is normally handled during the pre-loop startup phase in `run()`.
                    // If it arrives here, just set the handlers.
                    info!(
                        target: LOG_TARGET,
                        "Setting RPC handlers for BlockchainService"
                    );
                    let mut rpc_handlers_guard = self.rpc_handlers.write().await;
                    *rpc_handlers_guard = Some(rpc_handlers);
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
                BlockchainServiceCommand::PopConfirmStoringRequests { count, callback } => {
                    if let Some(ManagedProvider::Bsp(_)) = &self.maybe_managed_provider {
                        let popped = self.pop_confirm_storing_requests(count);
                        match callback.send(Ok(popped)) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send popped confirm storing requests: {:?}", e);
                            }
                        }
                    } else {
                        command_succeeded = false;
                        error!(target: LOG_TARGET, "`PopConfirmStoringRequests` should only be called if the node is managing a BSP. Found [{:?}] instead.", self.maybe_managed_provider);
                        match callback.send(Err(anyhow!("Node is not managing a BSP"))) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send error: {:?}", e);
                            }
                        }
                    }
                }
                BlockchainServiceCommand::FilterConfirmStoringRequests { requests, callback } => {
                    match self.filter_confirm_storing_requests(requests) {
                        Ok(result) => match callback.send(Ok(result)) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send filtered confirm storing requests: {:?}", e);
                            }
                        },
                        Err(e) => {
                            command_succeeded = false;
                            error!(target: LOG_TARGET, "FilterConfirmStoringRequests failed: {:?}", e);
                            match callback.send(Err(e)) {
                                Ok(_) => {}
                                Err(e) => {
                                    error!(target: LOG_TARGET, "Failed to send error: {:?}", e);
                                }
                            }
                        }
                    }
                }
                BlockchainServiceCommand::QueueBspRequestStopStoring { request, callback } => {
                    if let Some(ManagedProvider::Bsp(_)) = &self.maybe_managed_provider {
                        let state_store_context =
                            self.persistent_state.open_rw_context_with_overlay();
                        state_store_context
                            .pending_request_bsp_stop_storing_deque()
                            .push_back(request.clone());
                        state_store_context.commit();

                        info!(
                            target: LOG_TARGET,
                            "Queued BSP request stop storing for file key [{:?}]",
                            request.file_key
                        );

                        // We check right away if we can process the request so we don't waste time.
                        self.bsp_assign_forest_root_write_lock();
                        match callback.send(Ok(())) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                            }
                        }
                    } else {
                        command_succeeded = false;
                        error!(
                            target: LOG_TARGET,
                            "QueueBspRequestBspStopStoring received while not managing a BSP. \
                             This command is only valid for BSP nodes."
                        );
                        match callback.send(Err(anyhow!(
                            "QueueBspRequestBspStopStoring received while not managing a BSP"
                        ))) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                            }
                        }
                    }
                }
                BlockchainServiceCommand::QueueBspConfirmStopStoring { request, callback } => {
                    if let Some(ManagedProvider::Bsp(_)) = &self.maybe_managed_provider {
                        let state_store_context =
                            self.persistent_state.open_rw_context_with_overlay();
                        state_store_context
                            .pending_confirm_bsp_stop_storing_deque()
                            .push_back(request.clone());
                        state_store_context.commit();

                        info!(
                            target: LOG_TARGET,
                            "Queued BSP confirm stop storing for file key [{:?}], confirm after tick: {:?}",
                            request.file_key,
                            request.confirm_after_tick
                        );

                        // We check right away if we can process the request so we don't waste time.
                        self.bsp_assign_forest_root_write_lock();
                        match callback.send(Ok(())) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                            }
                        }
                    } else {
                        command_succeeded = false;
                        error!(
                            target: LOG_TARGET,
                            "QueueBspConfirmBspStopStoring received while not managing a BSP. \
                             This command is only valid for BSP nodes."
                        );
                        match callback.send(Err(anyhow!(
                            "QueueBspConfirmBspStopStoring received while not managing a BSP"
                        ))) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                            }
                        }
                    }
                }
            }

            // Record command completion
            let status = if command_succeeded {
                STATUS_SUCCESS
            } else {
                STATUS_FAILURE
            };
            observe_histogram!(metrics: metrics.as_ref(), command_processing_seconds, labels: &[command_name, status], start.elapsed().as_secs_f64());
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
        client: Arc<StorageHubClient<Runtime::RuntimeApi>>,
        keystore: KeystorePtr,
        rpc_handlers: Option<Arc<RpcHandlers>>,
        forest_storage_handler: FSH,
        rocksdb_root_path: impl Into<PathBuf>,
        notify_period: Option<u32>,
        capacity_request_queue: Option<CapacityRequestQueue<Runtime>>,
        maintenance_mode: bool,
        metrics: MetricsLink,
    ) -> Self {
        let genesis_hash = client.info().genesis_hash;
        Self {
            config,
            event_bus_provider: BlockchainServiceEventBusProvider::new(),
            client,
            keystore,
            rpc_handlers: Arc::new(RwLock::new(rpc_handlers)),
            forest_storage_handler,
            current_block: MinimalBlockInfo {
                number: 0u32.into(),
                hash: genesis_hash,
            },
            last_block_processed: MinimalBlockInfo {
                number: 0u32.into(),
                hash: genesis_hash,
            },
            last_finalised_block_processed: MinimalBlockInfo {
                number: 0u32.into(),
                hash: genesis_hash,
            },
            nonce_counter: 0,
            wait_for_block_request_by_number: BTreeMap::new(),
            wait_for_tick_request_by_number: BTreeMap::new(),
            maybe_managed_provider: None,
            persistent_state: BlockchainServiceStateStore::new(rocksdb_root_path.into()),
            notify_period,
            capacity_manager: capacity_request_queue,
            maintenance_mode,
            caught_up: false,
            transaction_manager: TransactionManager::new(TransactionManagerConfig::default()),
            // Temporary sender, will be replaced by the event loop during startup
            tx_status_sender: {
                let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
                tx
            },
            // Temporary sender, will be replaced by the event loop during startup
            permit_release_sender: {
                let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
                tx
            },
            pending_tx_store: None,
            role: MultiInstancesNodeRole::Standalone,
            leadership_conn: None,
            pending_finality_notifications: VecDeque::new(),
            metrics,
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

        // Skip sync blocks as they're handled entirely by `handle_sync_block_notification`.
        // This prevents:
        // 1. Double-processing of reorgs during sync (which would trigger unwanted events)
        // 2. Premature triggering of `handle_initial_sync` during any reorgs during sync
        // 3. Duplicate `register_current_block_and_check_reorg` calls
        if notification.origin == sp_consensus::BlockOrigin::NetworkInitialSync {
            return;
        }

        // Check if this new imported block is the new best, and if it causes a reorg.
        let new_block_notification_kind =
            self.register_current_block_and_check_reorg(&notification);

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

        // Start timing block processing after early returns
        let start = std::time::Instant::now();

        info!(target: LOG_TARGET, "📬 Block import notification (#{}): {}", block_number, block_hash);

        // Get provider IDs linked to keys in this node's keystore and update the nonce.
        self.init_block_processing(&block_hash);

        // Trigger initial sync tasks on the first block import notification after startup/recovery
        // or after the node fell behind and re-synced.
        if !self.caught_up {
            self.handle_initial_sync(notification).await;
            self.caught_up = true;
        }

        let block_number = block_number.saturated_into();
        self.process_block_import(&block_hash, &block_number, tree_route)
            .await;

        info!(target: LOG_TARGET, "📭 Block import notification (#{}): {} processed successfully", block_number, block_hash);

        // Record block processing duration
        observe_histogram!(metrics: self.metrics.as_ref(), block_processing_seconds, labels: &["block_import", STATUS_SUCCESS], start.elapsed().as_secs_f64());
    }

    /// Handle block notifications during network initial sync.
    ///
    /// This function is called for every imported block (via `every_import_notification_stream`),
    /// but only processes mutation events during initial sync to keep the local forest in sync
    /// before state pruning can occur.
    ///
    /// ## Why we need this
    ///
    /// During initial sync, `block_import_notification_stream` (the normal notification stream)
    /// only fires for reorgs, not for linear chain extensions. This means if we relied solely
    /// on that stream, we'd miss processing mutations for the vast majority of sync blocks.
    /// By the time we come out of sync mode, those blocks' state may have been pruned, and we
    /// would end up with a provider who's forest is out of sync with the on-chain state and who
    /// has no way of recovering other than manually applying the required changes to the forest.
    ///
    /// ## Sync handling approach
    ///
    /// Substrate uses `MAJOR_SYNC_BLOCKS = 5` to determine when a node is "major syncing":
    /// - **6+ blocks behind**: `BlockOrigin::NetworkInitialSync` → handled by this function
    /// - **≤5 blocks behind**: `BlockOrigin::NetworkBroadcast` → handled by normal flow
    ///
    /// This creates a natural hybrid approach:
    /// 1. If the node requires heavy sync (6+ blocks behind): Process only mutations block-by-block
    /// 2. If the node is already near the chain tip (≤5 blocks behind): Normal flow with full event processing
    ///
    /// ## Reorg handling during sync
    ///
    /// Reorgs during sync are handled entirely by this function.
    /// This is important because:
    /// 1. We don't want to trigger the full block import event processing during sync (as we would process unwanted events)
    /// 2. We don't want to prematurely trigger `handle_initial_sync` during any reorgs during sync
    /// 3. We still need to properly revert retracted blocks and apply enacted blocks via `forest_root_changes_catchup`
    ///
    /// When a reorg happens during sync:
    /// 1. This handler receives the notification and detects it as a reorg
    /// 2. `process_sync_reorg` properly reverts the retracted blocks' mutations,
    ///    enacts the new blocks' mutations and processes finality for them
    /// 3. Updates the last processed block to the new best
    async fn handle_sync_block_notification(
        &mut self,
        notification: BlockImportNotification<OpaqueBlock>,
    ) {
        // Only process during initial sync
        if notification.origin != sp_consensus::BlockOrigin::NetworkInitialSync {
            return;
        }

        // Reset the caught_up flag since we're receiving sync blocks, indicating the node
        // has fallen behind. This ensures handle_initial_sync runs again after catching up.
        if self.caught_up {
            info!(
                target: LOG_TARGET,
                "🔄 Node fell behind chain tip, resetting caught_up flag to re-run initial sync after catching up"
            );
            self.caught_up = false;
        }

        // Check if this new imported block is the new best, and if it causes a reorg.
        let new_block_notification_kind =
            self.register_current_block_and_check_reorg(&notification);

        match new_block_notification_kind {
            NewBlockNotificationKind::NewBestBlock { new_best_block, .. } => {
                // Process the new best block as a linear chain extension
                self.process_sync_block(&new_best_block.hash, new_best_block.number)
                    .await;
            }
            NewBlockNotificationKind::NewNonBestBlock(_) => {
                // Skip non-best blocks (uncle/stale blocks not on the canonical chain)
                return;
            }
            NewBlockNotificationKind::Reorg {
                new_best_block,
                tree_route,
                ..
            } => {
                // Process the reorg
                self.process_sync_reorg(&tree_route, new_best_block).await;
            }
        }
    }

    /// Handle a forest root write permit release notification by assigning the forest
    /// root write lock to the next pending forest write request.
    ///
    /// This method is called when a `ForestWritePermitGuard` is dropped by a task,
    /// allowing the next pending forest write request to be processed.
    fn handle_permit_released(&mut self) {
        if let Some(managed_provider) = &self.maybe_managed_provider {
            match managed_provider {
                ManagedProvider::Bsp(_) => self.bsp_assign_forest_root_write_lock(),
                ManagedProvider::Msp(_) => self.msp_assign_forest_root_write_lock(),
            }
        } else {
            error!(target: LOG_TARGET, "Tried to handle a forest root write permit release notification while not managing a MSP or BSP. This should never happen. Please report it to the StorageHub team.");
        }
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
    ///
    /// At this point, mutations have already been applied during sync via the
    /// `every_import_notification_stream` handler, so we only need to perform provider-specific initialization tasks.
    async fn handle_initial_sync(&mut self, notification: BlockImportNotification<OpaqueBlock>) {
        let block_hash = notification.hash;
        let block_number: BlockNumber<Runtime> = (*notification.header.number()).into();

        info!(target: LOG_TARGET, "🥱 Handling coming out of sync mode (synced to #{}: {})", block_number, block_hash);

        // Perform provider-specific initialization
        match &self.maybe_managed_provider {
            Some(ManagedProvider::Bsp(bsp_handler)) => {
                let bsp_id = bsp_handler.bsp_id;
                self.bsp_initial_sync(block_hash, bsp_id).await;
            }
            Some(ManagedProvider::Msp(msp_handler)) => {
                let msp_id = msp_handler.msp_id;
                self.msp_initial_sync(block_hash, msp_id).await;
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

        // Cleanup manager and DB, and handle old nonce gaps in one helper
        // TODO: Consider doing this in a spawned task to avoid blocking the main thread.
        self.cleanup_tx_manager_and_handle_nonce_gaps(*block_number, *block_hash)
            .await;

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

                    self.process_msp_and_bsp_block_import_events(ev.event.clone().into());

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

        // Update both the in-memory tracker and persistent storage
        self.last_block_processed = MinimalBlockInfo {
            number: *block_number,
            hash: *block_hash,
        };
        self.update_last_processed_block_info(self.last_block_processed);
    }

    /// Handle a finality notification.
    ///
    /// This processes finality events for the finalised block and all implicitly finalised blocks
    /// in the `tree_route`. This is important for scenarios where finality jumps multiple blocks
    /// at once (e.g., after a node restart, network partition recovery or solved finality staleness).
    ///
    /// If the finality notification is for a block that hasn't been import-processed yet
    /// (`block_number > last_block_processed.number`), it is queued for later processing. This handles
    /// the race condition where finality notifications can arrive before block import notifications.
    async fn handle_finality_notification(
        &mut self,
        notification: FinalityNotification<OpaqueBlock>,
    ) {
        let block_hash = notification.hash;
        let block_number: BlockNumber<Runtime> = (*notification.header.number()).saturated_into();

        // If the node is running in maintenance mode, we don't process finality notifications.
        if self.maintenance_mode {
            trace!(target: LOG_TARGET, "🔒 Maintenance mode is enabled. Skipping finality notification #{}: {}", block_number, block_hash);
            return;
        }

        info!(target: LOG_TARGET, "📩 Received finality notification for block #{}: 0x{:x}", block_number, block_hash);

        // Drain any pending finality notifications that can now be processed.
        // This handles notifications that were queued because they arrived before their
        // corresponding block import was processed.
        self.drain_pending_finality_notifications().await;

        // Skip if this finalised block was already processed.
        // This can happen during sync when both `handle_sync_block_notification` (via
        // `process_finality_events_if_finalised`) and this handler process the same block.
        // Finality notifications fire even during sync, but we may have
        // already processed some blocks eagerly based on `client.info().finalized_number`.
        if block_number <= self.last_finalised_block_processed.number {
            trace!(
                target: LOG_TARGET,
                "🔁 Finality notification #{} already processed (last_finalised={}), skipping",
                block_number,
                self.last_finalised_block_processed.number
            );
            return;
        }

        // If this finality notification is for a block that hasn't been fully import-processed yet,
        // queue it for later. This prevents processing finality events before the forest has
        // been updated by block import, which would cause issues like failing to delete files
        // because they're still in the forest.
        if block_number > self.last_block_processed.number {
            warn!(
                target: LOG_TARGET,
                "🛑 Finality notification for block #{} is ahead of last import-processed block #{}, deferring to queue",
                block_number, self.last_block_processed.number
            );
            self.pending_finality_notifications.push_back(notification);
            return;
        }

        // If the block number is the same as the last import-processed block number, but the hash is different,
        // it means the block has been reorged and we need to wait for the new block that replaces it before processing
        // the finality notification.
        if block_number == self.last_block_processed.number
            && block_hash != self.last_block_processed.hash
        {
            warn!(
                target: LOG_TARGET,
                "🔄 Finality notification for block #{}: finalised block 0x{:x} is a reorg of the last processed block 0x{:x}, deferring to queue",
                block_number, block_hash, self.last_block_processed.hash
            );
            self.pending_finality_notifications.push_back(notification);
            return;
        }

        // At this point, we know that the finality notification is for a block that has been import-processed,
        // and it is not a reorg of the last processed block. Therefore, we can safely process it.
        self.process_finality_notification(notification).await;
    }

    /// Drain and process any pending finality notifications that can now be processed.
    ///
    /// This is called at the start of finality notification handling to process any
    /// notifications that were queued because they arrived before their corresponding
    /// block import was processed.
    async fn drain_pending_finality_notifications(&mut self) {
        // Process notifications in order while they're <= last_block_processed
        while let Some(notification) = self.pending_finality_notifications.front() {
            let block_number: BlockNumber<Runtime> =
                (*notification.header.number()).saturated_into();

            if block_number > self.last_block_processed.number {
                // Still ahead, stop draining
                break;
            }

            if block_number == self.last_block_processed.number
                && notification.hash != self.last_block_processed.hash
            {
                // The last processed block has been reorged, stop draining
                break;
            }

            // Safe to process now
            let notification = self.pending_finality_notifications.pop_front().unwrap();

            // Skip if the finality has been already processed
            if block_number <= self.last_finalised_block_processed.number {
                warn!(
                    target: LOG_TARGET,
                    "🔁 Deferred finality notification #{} already processed, skipping",
                    block_number
                );
                continue;
            }

            info!(
                target: LOG_TARGET,
                "⏰ Processing deferred finality notification #{} (queue size: {})",
                block_number,
                self.pending_finality_notifications.len()
            );

            self.process_finality_notification(notification).await;
        }
    }

    /// Internal method to process a finality notification.
    async fn process_finality_notification(
        &mut self,
        notification: FinalityNotification<OpaqueBlock>,
    ) {
        // Start timing finality notification processing
        let start = std::time::Instant::now();

        let block_hash = notification.hash;
        let block_number: BlockNumber<Runtime> = (*notification.header.number()).saturated_into();

        info!(target: LOG_TARGET, "📇 Processing finality notification for block #{}: 0x{:x}", block_number, block_hash);

        // Process finality events for all implicitly finalised blocks in tree_route.
        // tree_route contains all blocks from (old_finalised, new_finalised_parent), i.e., the blocks
        // that were implicitly finalised when jumping from the old finalised to the new one.
        // The tree_route does not include the latest finalised block itself.
        //
        // We filter out blocks that were already processed to avoid double-processing.
        // This can happen when blocks were processed eagerly during sync via
        // `process_finality_events_if_finalised`, but the finality gadget's internal finalised state
        // was behind our `last_finalised_block_processed`.
        if !notification.tree_route.is_empty() {
            info!(
                target: LOG_TARGET,
                "📦 Processing finality events for {} implicitly finalised blocks",
                notification.tree_route.len()
            );

            let last_processed = self.last_finalised_block_processed.number;

            for intermediate_hash in notification.tree_route.iter() {
                // Get the block number for this hash to check if we already processed it
                let intermediate_number: BlockNumber<Runtime> = match self
                    .client
                    .number(*intermediate_hash)
                {
                    Ok(Some(num)) => num.saturated_into(),
                    Ok(None) => {
                        warn!(
                                target: LOG_TARGET,
                                "Could not find block number for hash {:?} in tree_route, skipping",
                                intermediate_hash
                        );
                        continue;
                    }
                    Err(e) => {
                        warn!(
                                target: LOG_TARGET,
                                "Error getting block number for hash {:?}: {:?}, skipping",
                                intermediate_hash, e
                        );
                        continue;
                    }
                };

                // Skip if already processed
                if intermediate_number <= last_processed {
                    continue;
                }

                self.process_finality_events(intermediate_hash);
            }
        }

        // Process finality events for the newly finalised block itself
        self.process_finality_events(&block_hash);

        // Cleanup the pending transaction store for the last finalised block processed.
        // Transactions with a nonce below the on-chain nonce of this block are finalised.
        // Still, we'll delete up to the last finalised block processed, to leave transactions with
        // a terminal state in the pending DB for a short period of time.
        if matches!(self.role, MultiInstancesNodeRole::Leader) {
            self.cleanup_pending_tx_store(self.last_finalised_block_processed.hash)
                .await;
        }

        // Update the last finalised block processed (in memory and persistent storage).
        self.last_finalised_block_processed = MinimalBlockInfo {
            number: block_number.saturated_into(),
            hash: block_hash,
        };
        self.update_last_finalised_block_info(self.last_finalised_block_processed);

        info!(target: LOG_TARGET, "📨 Finality notification for block #{}: 0x{:x} processed successfully", block_number, block_hash);

        // Record finality notification processing duration
        observe_histogram!(metrics: self.metrics.as_ref(), block_processing_seconds, labels: &["finalized_block", STATUS_SUCCESS], start.elapsed().as_secs_f64());
    }

    /// Queue one or more confirm storing requests to the pending deque.
    pub(crate) fn queue_confirm_storing_requests(
        &self,
        requests: impl IntoIterator<Item = crate::types::ConfirmStoringRequest<Runtime>>,
    ) {
        let state_store_context = self.persistent_state.open_rw_context_with_overlay();
        let mut deque = state_store_context.pending_confirm_storing_request_deque::<Runtime>();
        for request in requests {
            deque.push_back(request);
        }
        state_store_context.commit();
    }
}
