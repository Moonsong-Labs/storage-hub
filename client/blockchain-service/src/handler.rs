use anyhow::anyhow;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    str::FromStr,
    sync::Arc,
};

use futures::prelude::*;
use log::{debug, trace, warn};
use pallet_storage_providers_runtime_api::{GetBspInfoError, StorageProvidersApi};
use sc_client_api::{
    BlockImportNotification, BlockchainEvents, FinalityNotification, HeaderBackend,
};
use sc_network::Multiaddr;
use sc_service::RpcHandlers;
use sc_tracing::tracing::{error, info};
use shc_actors_framework::actor::{Actor, ActorEventLoop};
use shc_common::types::{Fingerprint, RandomnessOutput, TrieRemoveMutation, BCSV_KEY_TYPE};
use shp_file_metadata::FileKey;
use sp_api::ProvideRuntimeApi;
use sp_core::H256;
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::{traits::Header, AccountId32, SaturatedConversion};
use storage_hub_runtime::RuntimeEvent;

use pallet_file_system_runtime_api::{
    FileSystemApi, QueryBspConfirmChunksToProveForFileError, QueryFileEarliestVolunteerBlockError,
};
use pallet_payment_streams_runtime_api::{GetUsersWithDebtOverThresholdError, PaymentStreamsApi};
use pallet_proofs_dealer_runtime_api::{
    GetCheckpointChallengesError, GetLastTickProviderSubmittedProofError, ProofsDealerApi,
};
use shc_common::types::{BlockNumber, ParachainClient, ProviderId};

use crate::{
    commands::BlockchainServiceCommand,
    events::{
        AcceptedBspVolunteer, BlockchainServiceEventBusProvider, FinalisedMutationsApplied,
        NewChallengeSeed, NewStorageRequest, ProofAccepted, SlashableProvider,
    },
    transaction::SubmittedTransaction,
};

pub(crate) const LOG_TARGET: &str = "blockchain-service";

pub struct SubmitProofRequest {
    pub provider_id: ProviderId,
    pub tick: BlockNumber,
    pub seed: RandomnessOutput,
    pub forest_challenges: Vec<H256>,
    pub checkpoint_challenges: Vec<(H256, Option<TrieRemoveMutation>)>,
}

impl SubmitProofRequest {
    pub fn new(
        new_challenge_seed_event: NewChallengeSeed,
        forest_challenges: Vec<H256>,
        checkpoint_challenges: Vec<(H256, Option<TrieRemoveMutation>)>,
    ) -> Self {
        Self {
            provider_id: new_challenge_seed_event.provider_id,
            tick: new_challenge_seed_event.tick,
            seed: new_challenge_seed_event.seed,
            forest_challenges,
            checkpoint_challenges,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfirmStoringRequest {
    pub file_key: H256,
    pub try_count: usize,
}

impl ConfirmStoringRequest {
    pub fn new(file_key: H256) -> Self {
        Self {
            file_key,
            try_count: 0,
        }
    }

    pub fn increment_try_count(&mut self) {
        self.try_count += 1;
    }
}

/// The BlockchainService actor.
///
/// This actor is responsible for sending extrinsics to the runtime and handling block import notifications.
/// For such purposes, it uses the [`ParachainClient`] to interact with the runtime, the [`RpcHandlers`] to send
/// extrinsics, and the [`Keystore`] to sign the extrinsics.
pub struct BlockchainService {
    /// The event bus provider.
    pub(crate) event_bus_provider: BlockchainServiceEventBusProvider,
    /// The parachain client. Used to interact with the runtime.
    pub(crate) client: Arc<ParachainClient>,
    /// The keystore. Used to sign extrinsics.
    pub(crate) keystore: KeystorePtr,
    /// The RPC handlers. Used to send extrinsics.
    pub(crate) rpc_handlers: Arc<RpcHandlers>,
    /// Nonce counter for the extrinsics.
    pub(crate) nonce_counter: u32,
    /// A registry of waiters for a block number.
    pub(crate) wait_for_block_request_by_number:
        BTreeMap<BlockNumber, Vec<tokio::sync::oneshot::Sender<()>>>,
    /// A list of Provider IDs that this node has to pay attention to submit proofs for.
    /// This could be a BSP or a list of buckets that an MSP has.
    pub(crate) provider_ids: BTreeSet<ProviderId>,
    /// A lock to prevent multiple tasks from writing to the runtime forest root (send transactions) at the same time.
    /// This is an [`Arc<Mutex<Option<T>>>`] (in this case [`oneshot::Sender<()>`]) instead of just
    /// T so that we can keep using the current actors event bus (emit) which requires Clone on the
    /// event. Clone is required because there is no constraint on the number of listeners that can
    /// subscribe to the event (and each is guaranteed to receive all emitted events).
    /// Also, this is a oneshot channel instead of a regular mutex because we want to "lock" in 1
    /// thread (blockchain service) and unlock it at the end of the spawned task. The alternative
    /// would be to send a [`MutexGuard`].
    pub(crate) forest_root_write_lock: Option<tokio::sync::oneshot::Receiver<()>>,
    /// A list of [SubmitProofRequest] that are waiting to be processed (sent to the runtime).
    pub(crate) pending_submit_proof: VecDeque<SubmitProofRequest>,
    /// A list of [ConfirmStoringRequest] that are waiting to be processed (sent to the runtime).
    pub(crate) pending_confirm_storing: VecDeque<ConfirmStoringRequest>,
}

/// Event loop for the BlockchainService actor.
pub struct BlockchainServiceEventLoop {
    receiver: sc_utils::mpsc::TracingUnboundedReceiver<BlockchainServiceCommand>,
    actor: BlockchainService,
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
impl ActorEventLoop<BlockchainService> for BlockchainServiceEventLoop {
    fn new(
        actor: BlockchainService,
        receiver: sc_utils::mpsc::TracingUnboundedReceiver<BlockchainServiceCommand>,
    ) -> Self {
        Self { actor, receiver }
    }

    async fn run(mut self) {
        info!(target: LOG_TARGET, "BlockchainService starting up!");

        // Import notification stream to be notified of new blocks.
        // This will notify us when sync to the latest block, or if there is a re-org.
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
impl Actor for BlockchainService {
    type Message = BlockchainServiceCommand;
    type EventLoop = BlockchainServiceEventLoop;
    type EventBusProvider = BlockchainServiceEventBusProvider;

    fn handle_message(
        &mut self,
        message: Self::Message,
    ) -> impl std::future::Future<Output = ()> + Send {
        async {
            match message {
                BlockchainServiceCommand::SendExtrinsic { call, callback } => {
                    match self.send_extrinsic(call).await {
                        Ok(output) => {
                            debug!(target: LOG_TARGET, "Extrinsic sent successfully: {:?}", output);
                            match callback
                                .send(Ok(SubmittedTransaction::new(output.receiver, output.hash)))
                            {
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
                    }
                }
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
                                error!(target: LOG_TARGET, "Failed to notify task about waiting block number.");
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
                            trace!(target: LOG_TARGET, "Receiver sent successfully");
                        }
                        Err(e) => {
                            error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
                        }
                    }
                }
                BlockchainServiceCommand::QueryFileEarliestVolunteerBlock {
                    bsp_id,
                    file_key,
                    callback,
                } => {
                    let current_block_hash = self.client.info().best_hash;

                    let earliest_block_to_volunteer = self
                        .client
                        .runtime_api()
                        .query_earliest_file_volunteer_block(
                            current_block_hash,
                            bsp_id.into(),
                            file_key,
                        )
                        .unwrap_or_else(|_| {
                            Err(QueryFileEarliestVolunteerBlockError::InternalError)
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
                            Err(GetLastTickProviderSubmittedProofError::InternalApiError)
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
                BlockchainServiceCommand::QueueConfirmBspRequest { request, callback } => {
                    self.pending_confirm_storing.push_back(request);
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
                    self.pending_submit_proof.push_back(request);
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
            }
        }
    }

    fn get_event_bus_provider(&self) -> &Self::EventBusProvider {
        &self.event_bus_provider
    }
}

impl BlockchainService {
    /// Create a new [`BlockchainService`].
    pub fn new(
        client: Arc<ParachainClient>,
        rpc_handlers: Arc<RpcHandlers>,
        keystore: KeystorePtr,
    ) -> Self {
        Self {
            client,
            rpc_handlers,
            keystore,
            event_bus_provider: BlockchainServiceEventBusProvider::new(),
            nonce_counter: 0,
            wait_for_block_request_by_number: BTreeMap::new(),
            provider_ids: BTreeSet::new(),
            forest_root_write_lock: None,
            pending_submit_proof: VecDeque::new(),
            pending_confirm_storing: VecDeque::new(),
        }
    }

    /// Handle a block import notification.
    async fn handle_block_import_notification<Block>(
        &mut self,
        notification: BlockImportNotification<Block>,
    ) where
        Block: cumulus_primitives_core::BlockT<Hash = H256>,
    {
        let block_hash: H256 = notification.hash;
        let block_number: BlockNumber = (*notification.header.number()).saturated_into();

        debug!(target: LOG_TARGET, "Import notification #{}: {}", block_number, block_hash);

        // Notify all tasks waiting for this block number (or lower).
        self.notify_import_block_number(block_number);

        // We query the [`BlockchainService`] account nonce at this height
        // and update our internal counter if it's smaller than the result.
        self.check_nonce(block_hash);

        // Get provider IDs linked to keys in this node's keystore.
        self.get_provider_ids(block_hash);

        // Process pending requests that update the forest root.
        self.check_pending_forest_root_writes();

        // Get events from storage.
        match self.get_events_storage_element(block_hash) {
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
                            },
                        ) => self.emit(NewStorageRequest {
                            who,
                            file_key: FileKey::from(file_key.as_ref()),
                            bucket_id,
                            location,
                            fingerprint: fingerprint.as_ref().into(),
                            size,
                            user_peer_ids: peer_ids,
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
                                    info!(target: LOG_TARGET, "New Provider ID to monitor [{:?}] for account [{:?}]", provider_id, account);
                                    self.provider_ids.insert(provider_id);
                                }
                            }
                        }
                        // New challenge seed event coming from pallet-proofs-dealer.
                        RuntimeEvent::ProofsDealer(
                            pallet_proofs_dealer::Event::NewChallengeSeed {
                                challenges_ticker,
                                seed,
                            },
                        ) => {
                            // For each Provider ID this node monitors...
                            for provider_id in &self.provider_ids {
                                // ...check if the challenges tick is one that this provider has to submit a proof for.
                                if self.should_provider_submit_proof(
                                    &block_hash,
                                    provider_id,
                                    &challenges_ticker,
                                ) {
                                    self.emit(NewChallengeSeed {
                                        provider_id: *provider_id,
                                        tick: challenges_ticker,
                                        seed,
                                    })
                                } else {
                                    trace!(target: LOG_TARGET, "Challenges tick is not the next one to be submitted for Provider [{:?}]", provider_id);
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
                        // A new proof has been submitted and accepted.
                        RuntimeEvent::ProofsDealer(
                            pallet_proofs_dealer::Event::ProofAccepted { provider, proof },
                        ) => self.emit(ProofAccepted {
                            provider_id: provider,
                            proofs: proof.key_proofs,
                        }),
                        // This event should only be of any use if a node is run by as a user.
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
                            // We try to convert the types coming from the runtime into our expected types.
                            let fingerprint: Fingerprint = fingerprint.as_bytes().into();
                            // Here the Multiaddresses come as a BoundedVec of BoundedVecs of bytes,
                            // and we need to convert them. Returns if any of the provided multiaddresses are invalid.
                            let mut multiaddress_vec: Vec<Multiaddr> = Vec::new();
                            for raw_multiaddr in multiaddresses.into_iter() {
                                let multiaddress = match std::str::from_utf8(&raw_multiaddr) {
                                    Ok(s) => match Multiaddr::from_str(s) {
                                        Ok(multiaddr) => multiaddr,
                                        Err(e) => {
                                            error!(target: LOG_TARGET, "Failed to parse Multiaddress from string in AcceptedBspVolunteer event. bsp: {:?}, file owner: {:?}, file fingerprint: {:?}\n Error: {:?}", bsp_id, owner, fingerprint, e);
                                            return;
                                        }
                                    },
                                    Err(e) => {
                                        error!(target: LOG_TARGET, "Failed to parse Multiaddress from bytes in AcceptedBspVolunteer event. bsp: {:?}, file owner: {:?}, file fingerprint: {:?}\n Error: {:?}", bsp_id, owner, fingerprint, e);
                                        return;
                                    }
                                };

                                multiaddress_vec.push(multiaddress);
                            }

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

    /// Handle a finality notification.
    async fn handle_finality_notification<Block>(
        &mut self,
        notification: FinalityNotification<Block>,
    ) where
        Block: cumulus_primitives_core::BlockT<Hash = H256>,
    {
        let block_hash: H256 = notification.hash;
        let block_number: BlockNumber = (*notification.header.number()).saturated_into();

        debug!(target: LOG_TARGET, "Finality notification #{}: {}", block_number, block_hash);

        // Get events from storage.
        match self.get_events_storage_element(block_hash) {
            Ok(block_events) => {
                // Process the events.
                for ev in block_events {
                    match ev.event.clone() {
                        // New storage request event coming from pallet-file-system.
                        RuntimeEvent::ProofsDealer(
                            pallet_proofs_dealer::Event::MutationsApplied {
                                provider,
                                mutations,
                                new_root,
                            },
                        ) => {
                            // Check if the provider ID is one of the provider IDs this node is tracking.
                            if self.provider_ids.contains(&provider) {
                                self.emit(FinalisedMutationsApplied {
                                    provider_id: provider,
                                    mutations: mutations.clone(),
                                    new_root,
                                })
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
