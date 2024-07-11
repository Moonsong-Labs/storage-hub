// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::{collections::BTreeMap, str::FromStr, sync::Arc};

use anyhow::Result;
use codec::{Decode, Encode};
use cumulus_primitives_core::BlockT;
use frame_support::{StorageHasher, Twox128};
use futures::{prelude::*, stream::select};
use lazy_static::lazy_static;
use log::{debug, trace, warn};
use polkadot_runtime_common::BlockHashCount;
use sc_client_api::{
    BlockBackend, BlockImportNotification, BlockchainEvents, HeaderBackend, StorageKey,
    StorageProvider,
};
use sc_network::Multiaddr;
use sc_service::RpcHandlers;
use sc_tracing::tracing::{error, info};
use serde_json::Number;
use shc_actors_framework::actor::{Actor, ActorEventLoop};
use shc_common::types::Fingerprint;
use shp_file_key_verifier::types::FileKey;
use sp_api::ProvideRuntimeApi;
use sp_core::{Blake2Hasher, Hasher, H256};
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::{
    generic::{self, SignedPayload},
    traits::Header,
    AccountId32, SaturatedConversion,
};
use storage_hub_runtime::{RuntimeEvent, SignedExtra, UncheckedExtrinsic};
use substrate_frame_rpc_system::AccountNonceApi;

use pallet_file_system_runtime_api::{
    FileSystemApi, QueryBspConfirmChunksToProveForFileError, QueryFileEarliestVolunteerBlockError,
};
use pallet_proofs_dealer_runtime_api::{
    GetChallengePeriodError, GetLastTickProviderSubmittedProofError, ProofsDealerApi,
};
use shc_common::types::{BlockNumber, ParachainClient, ProviderId};

use crate::{
    commands::BlockchainServiceCommand,
    events::{
        AcceptedBspVolunteer, BlockchainServiceEventBusProvider, BspConfirmedStoring,
        NewChallengeSeed, NewStorageRequest,
    },
    transaction::SubmittedTransaction,
    types::{EventsVec, Extrinsic},
    KEY_TYPE,
};

const LOG_TARGET: &str = "blockchain-service";

lazy_static! {
    // Would be cool to be able to do this...
    // let events_storage_key = frame_system::Events::<storage_hub_runtime::Runtime>::hashed_key();

    // Static and lazily initialized `events_storage_key`
    static ref EVENTS_STORAGE_KEY: Vec<u8> = {
        let key = [
            Twox128::hash(b"System").to_vec(),
            Twox128::hash(b"Events").to_vec(),
        ]
        .concat();
        key
    };
}

/// The BlockchainService actor.
///
/// This actor is responsible for sending extrinsics to the runtime and handling block import notifications.
/// For such purposes, it uses the [`ParachainClient`] to interact with the runtime, the [`RpcHandlers`] to send
/// extrinsics, and the [`Keystore`] to sign the extrinsics.
pub struct BlockchainService {
    /// The event bus provider.
    event_bus_provider: BlockchainServiceEventBusProvider,
    /// The parachain client. Used to interact with the runtime.
    client: Arc<ParachainClient>,
    /// The keystore. Used to sign extrinsics.
    keystore: KeystorePtr,
    /// The RPC handlers. Used to send extrinsics.
    rpc_handlers: Arc<RpcHandlers>,
    /// Nonce counter for the extrinsics.
    nonce_counter: u32,
    /// A registry of waiters for a block number.
    wait_for_block_request_by_number: BTreeMap<BlockNumber, Vec<tokio::sync::oneshot::Sender<()>>>,
    /// A list of Provider IDs that this node has to pay attention to submit proofs for.
    /// This could be a BSP or a list of buckets that an MSP has.
    provider_ids: Vec<ProviderId>,
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
                            error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
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
                            error!(target: LOG_TARGET, "Failed to send receiver: {:?}", e);
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
    BlockNotification(BlockImportNotification<Block>),
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
        let notification_stream = self.actor.client.import_notification_stream();

        // Merging notification stream with command stream.
        let mut merged_stream = select(
            self.receiver.map(MergedEventLoopMessage::Command),
            notification_stream.map(MergedEventLoopMessage::BlockNotification),
        );

        // Process incoming messages.
        while let Some(notification) = merged_stream.next().await {
            match notification {
                MergedEventLoopMessage::Command(command) => {
                    self.actor.handle_message(command).await;
                }
                MergedEventLoopMessage::BlockNotification(notification) => {
                    self.actor.handle_block_notification(notification).await;
                }
            };
        }
    }
}

/// The output of an RPC transaction.
pub struct RpcExtrinsicOutput {
    /// Hash of the extrinsic.
    pub hash: H256,
    /// The output string of the transaction if any.
    pub result: String,
    /// An async receiver if data will be returned via a callback.
    pub receiver: tokio::sync::mpsc::Receiver<String>,
}

impl std::fmt::Debug for RpcExtrinsicOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "RpcExtrinsicOutput {{ hash: {:?}, result: {:?}, receiver }}",
            self.hash, self.result
        )
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
            provider_ids: Vec::new(),
        }
    }

    /// Handle a block import notification.
    async fn handle_block_notification<Block>(
        &mut self,
        notification: BlockImportNotification<Block>,
    ) where
        Block: cumulus_primitives_core::BlockT<Hash = H256>,
    {
        let block_hash: H256 = notification.hash;
        let block_number: BlockNumber = (*notification.header.number()).saturated_into();

        debug!(target: LOG_TARGET, "Import notification #{}: {}", block_number, block_hash);

        // Notify all tasks waiting for this block number (or lower).
        let mut keys_to_remove = Vec::new();

        for (block_number, waiters) in self
            .wait_for_block_request_by_number
            .range_mut(..=block_number)
        {
            keys_to_remove.push(*block_number);
            for waiter in waiters.drain(..) {
                match waiter.send(()) {
                    Ok(_) => {}
                    Err(_) => {
                        error!(target: LOG_TARGET, "Failed to notify task about block number.");
                    }
                }
            }
        }

        for key in keys_to_remove {
            self.wait_for_block_request_by_number.remove(&key);
        }

        // We query the [`BlockchainService`] account nonce at this height
        // and update our internal counter if it's smaller than the result.
        let pub_key = Self::caller_pub_key(self.keystore.clone());
        let latest_nonce = self
            .client
            .runtime_api()
            .account_nonce(block_hash, pub_key.into())
            .expect("Fetching account nonce works; qed");
        if latest_nonce > self.nonce_counter {
            self.nonce_counter = latest_nonce
        }

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
                                provider: provider_id,
                                maybe_provider_account,
                            },
                        ) => {
                            // This node only cares if the Provider account matches one of the accounts in the keystore.
                            if let Some(account) = maybe_provider_account {
                                let account: Vec<u8> =
                                    <sp_runtime::AccountId32 as AsRef<[u8; 32]>>::as_ref(&account)
                                        .to_vec();
                                if self.keystore.has_keys(&[(account.clone(), KEY_TYPE)]) {
                                    // If so, add the Provider ID to the list of Providers that this node is monitoring.
                                    info!(target: LOG_TARGET, "New Provider ID to monitor [{:?}] for account [{:?}]", provider_id, account);
                                    self.provider_ids.push(provider_id);
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
                        RuntimeEvent::FileSystem(
                            pallet_file_system::Event::BspConfirmedStoring {
                                who,
                                bsp_id,
                                file_key,
                                new_root,
                            },
                            // Filter the events by the BSP id.
                        ) => {
                            // TODO: Ideally we would be filtering by BSP ID here, but since we don't have a
                            // TODO: way to properly initialise the BSP ID yet, we are just going to filter by the
                            // TODO: caller's public key.
                            if who == AccountId32::from(Self::caller_pub_key(self.keystore.clone()))
                            {
                                self.emit(BspConfirmedStoring {
                                    bsp_id,
                                    file_key: FileKey::from(file_key.as_ref()),
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

    /// Send an extrinsic to this node using an RPC call.
    async fn send_extrinsic(
        &mut self,
        call: impl Into<storage_hub_runtime::RuntimeCall>,
    ) -> Result<RpcExtrinsicOutput> {
        debug!(target: LOG_TARGET, "Sending extrinsic to the runtime");

        // Get the nonce for the caller and increment it for the next transaction.
        // TODO: Handle nonce overflow.
        let nonce = self.nonce_counter;

        // Construct the extrinsic.
        let extrinsic = self.construct_extrinsic(self.client.clone(), call, nonce);

        // Generate a unique ID for this query.
        let id_hash = Blake2Hasher::hash(&extrinsic.encode());
        // TODO: Consider storing the ID in a hashmap if later retrieval is needed.

        let (result, rx) = self
            .rpc_handlers
            .rpc_query(&format!(
                r#"{{
                    "jsonrpc": "2.0",
                    "method": "author_submitAndWatchExtrinsic",
                    "params": ["0x{}"],
                    "id": {:?}
                }}"#,
                array_bytes::bytes2hex("", &extrinsic.encode()),
                array_bytes::bytes2hex("", &id_hash.as_bytes())
            ))
            .await
            .expect("Sending query failed even when it is correctly formatted as JSON-RPC; qed");

        let json: serde_json::Value =
            serde_json::from_str(&result).expect("the result can only be a JSONRPC string; qed");
        let error = json
            .as_object()
            .expect("JSON result is always an object; qed")
            .get("error");

        if let Some(error) = error {
            // TODO: Consider how to handle a low nonce error, and retry.
            return Err(anyhow::anyhow!("Error in RPC call: {}", error.to_string()));
        }

        // Only update nonce after we are sure no errors
        // occurred submitting the extrinsic.
        self.nonce_counter += 1;

        Ok(RpcExtrinsicOutput {
            hash: id_hash,
            result,
            receiver: rx,
        })
    }

    /// Construct an extrinsic that can be applied to the runtime.
    pub fn construct_extrinsic(
        &self,
        client: Arc<ParachainClient>,
        function: impl Into<storage_hub_runtime::RuntimeCall>,
        nonce: u32,
    ) -> UncheckedExtrinsic {
        let function = function.into();
        let current_block_hash = client.info().best_hash;
        let current_block = client.info().best_number.saturated_into();
        let genesis_block = client
            .hash(0)
            .expect("Failed to get genesis block hash, always present; qed")
            .expect("Genesis block hash should never not be on-chain; qed");
        let period = BlockHashCount::get()
            .checked_next_power_of_two()
            .map(|c| c / 2)
            .unwrap_or(2) as u64;
        // TODO: Consider tipping the transaction.
        let tip = 0;
        let extra: SignedExtra = (
        frame_system::CheckNonZeroSender::<storage_hub_runtime::Runtime>::new(),
        frame_system::CheckSpecVersion::<storage_hub_runtime::Runtime>::new(),
        frame_system::CheckTxVersion::<storage_hub_runtime::Runtime>::new(),
        frame_system::CheckGenesis::<storage_hub_runtime::Runtime>::new(),
        frame_system::CheckEra::<storage_hub_runtime::Runtime>::from(generic::Era::mortal(
            period,
            current_block,
        )),
        frame_system::CheckNonce::<storage_hub_runtime::Runtime>::from(nonce),
        frame_system::CheckWeight::<storage_hub_runtime::Runtime>::new(),
        pallet_transaction_payment::ChargeTransactionPayment::<storage_hub_runtime::Runtime>::from(
            tip,
        ),
        cumulus_primitives_storage_weight_reclaim::StorageWeightReclaim::<
            storage_hub_runtime::Runtime,
        >::new(),
    );

        let raw_payload = SignedPayload::from_raw(
            function.clone(),
            extra.clone(),
            (
                (),
                storage_hub_runtime::VERSION.spec_version,
                storage_hub_runtime::VERSION.transaction_version,
                genesis_block,
                current_block_hash,
                (),
                (),
                (),
                (),
            ),
        );

        let caller_pub_key = Self::caller_pub_key(self.keystore.clone());

        // Sign the payload.
        let signature = raw_payload
            .using_encoded(|e| self.keystore.sr25519_sign(KEY_TYPE, &caller_pub_key, e))
            .expect("The payload is always valid and should be possible to sign; qed")
            .expect("They key type and public key are valid because we just extracted them from the keystore; qed");

        // Construct the extrinsic.
        UncheckedExtrinsic::new_signed(
            function.clone(),
            storage_hub_runtime::Address::Id(<sp_core::sr25519::Public as Into<
                storage_hub_runtime::AccountId,
            >>::into(caller_pub_key)),
            polkadot_primitives::Signature::Sr25519(signature.clone()),
            extra.clone(),
        )
    }

    // Getting signer public key.
    pub fn caller_pub_key(keystore: KeystorePtr) -> sp_core::sr25519::Public {
        let caller_pub_key = keystore.sr25519_public_keys(KEY_TYPE).pop().expect(
            format!(
                "There should be at least one sr25519 key in the keystore with key type '{:?}' ; qed",
                KEY_TYPE
            )
            .as_str(),
        );
        caller_pub_key
    }

    /// Get an extrinsic from a block.
    async fn get_extrinsic_from_block(
        &self,
        block_hash: H256,
        extrinsic_hash: H256,
    ) -> Result<Extrinsic> {
        // Get the block.
        let block = self
            .client
            .block(block_hash)
            .expect("Failed to get block. This shouldn't be possible for known existing block hash; qed")
            .expect("Block returned None for known existing block hash. This shouldn't be the case for a block known to have at least one transaction; qed");

        // Get the extrinsics.
        let extrinsics = block.block.extrinsics();

        // Find the extrinsic index in the block.
        let extrinsic_index = extrinsics
            .iter()
            .position(|e| {
                let hash = Blake2Hasher::hash(&e.encode());
                hash == extrinsic_hash
            })
            .expect("Extrinsic not found in block. This shouldn't be possible if we're looking into a block for which we got confirmation that the extrinsic was included; qed");

        // Get the events from storage.
        let events_in_block = self.get_events_storage_element(block_hash)?;

        // Filter the events for the extrinsic.
        // Each event record is composed of the `phase`, `event` and `topics` fields.
        // We are interested in those events whose `phase` is equal to `ApplyExtrinsic` with the index of the extrinsic.
        // For more information see: https://polkadot.js.org/docs/api/cookbook/blocks/#how-do-i-map-extrinsics-to-their-events
        let events = events_in_block
            .into_iter()
            .filter(|ev| ev.phase == frame_system::Phase::ApplyExtrinsic(extrinsic_index as u32))
            .collect();

        // Construct the extrinsic.
        Ok(Extrinsic {
            hash: extrinsic_hash,
            block_hash,
            events,
        })
    }

    /// Unwatch an extrinsic.
    async fn unwatch_extrinsic(&self, subscription_id: Number) -> Result<String> {
        let (result, _rx) = self
            .rpc_handlers
            .rpc_query(&format!(
                r#"{{
                    "jsonrpc": "2.0",
                    "method": "author_unwatchExtrinsic",
                    "params": [{}],
                    "id": {}
                }}"#,
                subscription_id, subscription_id
            ))
            .await
            .expect("Sending query failed even when it is correctly formatted as JSON-RPC; qed");

        let json: serde_json::Value =
            serde_json::from_str(&result).expect("the result can only be a JSONRPC string; qed");
        let unwatch_result = json
            .as_object()
            .expect("JSON result is always an object; qed")
            .get("result");

        if let Some(unwatch_result) = unwatch_result {
            if unwatch_result
                .as_bool()
                .expect("Result is always a boolean; qed")
            {
                debug!(target: LOG_TARGET, "Extrinsic unwatched successfully");
            } else {
                return Err(anyhow::anyhow!("Failed to unwatch extrinsic"));
            }
        } else {
            return Err(anyhow::anyhow!("Failed to unwatch extrinsic"));
        }

        Ok(result)
    }

    /// Get the events storage element in a block.
    fn get_events_storage_element(&self, block_hash: H256) -> Result<EventsVec> {
        // Get the events storage.
        let raw_storage_opt = self
            .client
            .storage(block_hash, &StorageKey(EVENTS_STORAGE_KEY.clone()))
            .expect("Failed to get Events storage element");

        // Decode the events storage.
        if let Some(raw_storage) = raw_storage_opt {
            let block_events = EventsVec::decode(&mut raw_storage.0.as_slice())
                .expect("Failed to decode Events storage element");

            return Ok(block_events);
        } else {
            return Err(anyhow::anyhow!("Failed to get Events storage element"));
        }
    }

    /// Check if the challenges tick is one that this provider has to submit a proof for,
    /// and if so, emit a `NewChallengeSeed` event.
    fn should_provider_submit_proof(
        &self,
        block_hash: &H256,
        provider_id: &ProviderId,
        current_tick: &BlockNumber,
    ) -> bool {
        let last_tick_provided = match self
            .client
            .runtime_api()
            .get_last_tick_provider_submitted_proof(*block_hash, provider_id)
        {
            Ok(last_tick_provided_result) => match last_tick_provided_result {
                Ok(last_tick_provided) => last_tick_provided,
                Err(e) => match e {
                    GetLastTickProviderSubmittedProofError::ProviderNotRegistered => {
                        debug!(target: LOG_TARGET, "Provider [{:?}] is not registered", provider_id);
                        return false;
                    }
                    GetLastTickProviderSubmittedProofError::ProviderNeverSubmittedProof => {
                        debug!(target: LOG_TARGET, "Provider [{:?}] does not have an initialised challenge cycle", provider_id);
                        return false;
                    }
                },
            },
            Err(e) => {
                error!(target: LOG_TARGET, "Runtime API error while getting last tick Provider [{:?}] submitted a proof for: {:?}", provider_id, e);
                return false;
            }
        };
        let provider_challenge_period = match self
            .client
            .runtime_api()
            .get_challenge_period(block_hash.clone(), provider_id)
        {
            Ok(provider_challenge_period_result) => match provider_challenge_period_result {
                Ok(provider_challenge_period) => provider_challenge_period,
                Err(e) => match e {
                    GetChallengePeriodError::ProviderNotRegistered => {
                        debug!(target: LOG_TARGET, "Provider [{:?}] is not registered", provider_id);
                        return false;
                    }
                },
            },
            Err(e) => {
                debug!(target: LOG_TARGET, "Runtime API error while getting challenge period for Provider [{:?}]: {:?}", provider_id, e);
                return false;
            }
        };
        current_tick == &last_tick_provided.saturating_add(provider_challenge_period)
    }
}
