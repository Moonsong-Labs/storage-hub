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

//! Helper for incoming provider client requests.
//!
//! Handle (i.e. answer) incoming provider client requests from a remote peer received via
//! `crate::request_responses::RequestResponsesBehaviour` with
//! [`LightClientRequestHandler`](handler::LightClientRequestHandler).

use std::sync::Arc;

use anyhow::Result;
use codec::{Decode, Encode};
use frame_support::{StorageHasher, Twox128};
use frame_system::EventRecord;
use futures::{prelude::*, stream::select};
use polkadot_runtime_common::BlockHashCount;
use sc_client_api::{
    BlockImportNotification, BlockchainEvents, HeaderBackend, StorageKey, StorageProvider,
};
use sc_service::RpcHandlers;
use sc_tracing::tracing::info;
use sp_core::{Blake2Hasher, Hasher, H256};
use sp_keystore::Keystore;
use sp_runtime::{
    generic::{self, SignedPayload},
    SaturatedConversion,
};
use storage_hub_infra::actor::{Actor, ActorEventLoop};
use storage_hub_runtime::{RuntimeEvent, SignedExtra, UncheckedExtrinsic};

use crate::{service::ParachainClient, services::blockchain::events::NewStorageRequest};

use super::{events::BlockchainServiceEventBusProvider, KEY_TYPE};

const LOG_TARGET: &str = "blockchain-service";

type EventsVec = Vec<
    Box<
        EventRecord<
            <storage_hub_runtime::Runtime as frame_system::Config>::RuntimeEvent,
            <storage_hub_runtime::Runtime as frame_system::Config>::Hash,
        >,
    >,
>;

#[derive(Debug)]
pub enum BlockchainServiceCommand {
    SendExtrinsic {
        call: storage_hub_runtime::RuntimeCall,
    },
}

pub struct BlockchainService {
    event_bus_provider: BlockchainServiceEventBusProvider,
    client: Arc<ParachainClient>,
    keystore: Arc<dyn Keystore>,
    rpc_handlers: Arc<RpcHandlers>,
    nonce_counter: u32,
}

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
                BlockchainServiceCommand::SendExtrinsic { call } => {
                    match self.send_extrinsic(call).await {
                        Ok(output) => {
                            info!(target: LOG_TARGET, "Extrinsic sent successfully: {:?}", output);
                        }
                        Err(e) => {
                            info!(target: LOG_TARGET, "Failed to send extrinsic: {:?}", e);
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

enum MergedEventLoopMessage<Block>
where
    Block: cumulus_primitives_core::BlockT,
{
    Command(BlockchainServiceCommand),
    BlockNotification(BlockImportNotification<Block>),
}

impl ActorEventLoop<BlockchainService> for BlockchainServiceEventLoop {
    fn new(
        actor: BlockchainService,
        receiver: sc_utils::mpsc::TracingUnboundedReceiver<BlockchainServiceCommand>,
    ) -> Self {
        Self { actor, receiver }
    }

    async fn run(mut self) {
        info!(target: LOG_TARGET, "FileTransferService starting up!");

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
pub struct RpcTransactionOutput {
    /// The output string of the transaction if any.
    pub result: String,
    /// An async receiver if data will be returned via a callback.
    pub receiver: tokio::sync::mpsc::Receiver<String>,
}

impl std::fmt::Debug for RpcTransactionOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "RpcTransactionOutput {{ result: {:?}, receiver }}",
            self.result
        )
    }
}

impl BlockchainService {
    /// Create a new [`BlockchainService`].
    pub fn new(
        client: Arc<ParachainClient>,
        rpc_handlers: Arc<RpcHandlers>,
        keystore: Arc<dyn Keystore>,
    ) -> Self {
        Self {
            client,
            rpc_handlers,
            keystore,
            event_bus_provider: BlockchainServiceEventBusProvider::new(),
            nonce_counter: 0,
        }
    }

    /// Handle a block import notification.
    async fn handle_block_notification<Block>(
        &mut self,
        notification: BlockImportNotification<Block>,
    ) where
        Block: cumulus_primitives_core::BlockT<Hash = H256>,
    {
        info!(target: LOG_TARGET, "Import notification: {}", notification.hash);

        // Would be cool to be able to do this...
        // let events_storage_key = frame_system::Events::<storage_hub_runtime::Runtime>::hashed_key();

        // Get the storage key for the events storage.
        let events_storage_key = [
            Twox128::hash(b"System").to_vec(),
            Twox128::hash(b"Events").to_vec(),
        ]
        .concat();

        // Get the events storage.
        let raw_storage_opt = self
            .client
            .storage(notification.hash, &StorageKey(events_storage_key))
            .expect("Failed to get Events storage element");

        // Decode the events storage.
        if let Some(raw_storage) = raw_storage_opt {
            // TODO: Handle case where the storage cannot be decoded.
            // TODO: This would happen if we're parsing a block authored with an older version of the runtime, using
            // TODO: a node that has a newer version of the runtime, therefore the EventsVec type is different.
            // TODO: Consider using runtime APIs for getting old data of previous blocks, and this just for current blocks.
            let block_events = EventsVec::decode(&mut raw_storage.0.as_slice())
                .expect("Failed to decode Events storage element");

            for event in block_events.iter() {
                // Filter events of interest and send internal events to tasks listening.
                match event.event.clone() {
                    // New storage request event coming from pallet-file-system.
                    RuntimeEvent::FileSystem(pallet_file_system::Event::NewStorageRequest {
                        who,
                        location,
                        fingerprint,
                        size,
                        multiaddresses,
                    }) => {
                        self.emit(NewStorageRequest {
                            who,
                            location,
                            fingerprint,
                            size,
                            multiaddresses,
                        });
                    }
                    // Ignore all other events.
                    _ => {}
                }
            }
        }
    }

    /// Send an extrinsic to this node using an RPC call.
    async fn send_extrinsic(
        &mut self,
        call: impl Into<storage_hub_runtime::RuntimeCall>,
    ) -> Result<RpcTransactionOutput> {
        info!(target: LOG_TARGET, "Sending extrinsic to the runtime");

        // Get the nonce for the caller and increment it for the next transaction.
        let nonce = self.nonce_counter;
        self.nonce_counter += 1;

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
                    "method": "author_submitExtrinsic",
                    "params": ["0x{}"],
                    "id": {}
                }}"#,
                array_bytes::bytes2hex("", &extrinsic.encode()),
                id_hash
            ))
            .await
            .expect("Sending query failed even when it is correctly formatted as JSON-RPC; qed");

        parse_rpc_result(result, rx)
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

        // Getting signer public key.
        let caller_pub_key = self.keystore.sr25519_public_keys(KEY_TYPE).pop().expect(
            format!(
                "There should be at least one sr25519 key in the keystore with key type '{:?}' ; qed",
                KEY_TYPE
            )
            .as_str(),
        );

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
}

/// Parse the result of an RPC call.
pub(crate) fn parse_rpc_result(
    result: String,
    receiver: tokio::sync::mpsc::Receiver<String>,
) -> Result<RpcTransactionOutput> {
    let json: serde_json::Value =
        serde_json::from_str(&result).expect("the result can only be a JSONRPC string; qed");
    let error = json
        .as_object()
        .expect("JSON result is always an object; qed")
        .get("error");

    if let Some(error) = error {
        return Err(anyhow::anyhow!("Error in RPC call: {}", error.to_string()));
    }

    Ok(RpcTransactionOutput { result, receiver })
}
