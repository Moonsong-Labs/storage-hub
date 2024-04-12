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

use codec::{Decode, Encode};
use frame_support::{StorageHasher, Twox128};
use frame_system::EventRecord;
use futures::prelude::*;
use polkadot_runtime_common::BlockHashCount;
use sc_client_api::{BlockchainEvents, HeaderBackend, StorageKey, StorageProvider};
use sc_service::RpcHandlers;
use sc_tracing::tracing::info;
use serde::Deserialize;
use sp_keyring::Sr25519Keyring;
use sp_runtime::{
    generic::{self, SignedPayload},
    SaturatedConversion,
};
use storage_hub_infra::actor::{Actor, ActorEventLoop};
use storage_hub_runtime::{SignedExtra, UncheckedExtrinsic};

use crate::service::ParachainClient;

use super::events::BlockchainServiceEventBusProvider;

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
pub enum BlockchainServiceCommand {}

pub struct BlockchainService {
    event_bus_provider: BlockchainServiceEventBusProvider,
    client: Arc<ParachainClient>,
    rpc_handlers: Arc<RpcHandlers>,
}

impl Actor for BlockchainService {
    type Message = BlockchainServiceCommand;
    type EventLoop = BlockchainServiceEventLoop;
    type EventBusProvider = BlockchainServiceEventBusProvider;

    fn handle_message(
        &mut self,
        _message: Self::Message,
    ) -> impl std::future::Future<Output = ()> + Send {
        async {}
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
        let mut notification_stream = self.actor.client.import_notification_stream();
        while let Some(notification) = notification_stream.next().await {
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
                .actor
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
                    info!(target: LOG_TARGET, "Event: {:?}", event);

                    // TODO: Filter events of interest and send internal events to tasks listening.
                }
            }
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

/// An error for when the RPC call fails.
#[derive(Deserialize, Debug)]
pub struct RpcTransactionError {
    /// A Number that indicates the error type that occurred.
    pub code: i64,
    /// A String providing a short description of the error.
    pub message: String,
    /// A Primitive or Structured value that contains additional information about the error.
    pub data: Option<serde_json::Value>,
}

impl std::fmt::Display for RpcTransactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl BlockchainService {
    /// Create a new [`BlockchainService`].
    pub fn new(client: Arc<ParachainClient>, rpc_handlers: Arc<RpcHandlers>) -> Self {
        Self {
            client,
            rpc_handlers,
            event_bus_provider: BlockchainServiceEventBusProvider::new(),
        }
    }

    /// Send an extrinsic to this node using an RPC call.
    pub async fn send_extrinsic(
        &self,
        call: impl Into<storage_hub_runtime::RuntimeCall>,
        caller: Sr25519Keyring,
        nonce: u32,
    ) -> Result<RpcTransactionOutput, RpcTransactionError> {
        let extrinsic = construct_extrinsic(self.client.clone(), call, caller, nonce);

        // TODO: Consider using a unique ID for each RPC call and keeping track of them.
        let id = 0;
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
                id
            ))
            .await
            .expect("Sending query faild even when it is correctly formatted as JSON-RPC; qed");

        parse_rpc_result(result, rx)
    }
}

/// Construct an extrinsic that can be applied to the runtime.
// TODO: Review thoroughly this function.
pub fn construct_extrinsic(
    client: Arc<ParachainClient>,
    function: impl Into<storage_hub_runtime::RuntimeCall>,
    caller: Sr25519Keyring,
    nonce: u32,
) -> UncheckedExtrinsic {
    let function = function.into();
    let current_block_hash = client.info().best_hash;
    let current_block = client.info().best_number.saturated_into();
    let genesis_block = client.hash(0).unwrap().unwrap();
    let period = BlockHashCount::get()
        .checked_next_power_of_two()
        .map(|c| c / 2)
        .unwrap_or(2) as u64;
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
    let signature = raw_payload.using_encoded(|e| caller.sign(e));
    UncheckedExtrinsic::new_signed(
        function.clone(),
        storage_hub_runtime::Address::Id(caller.public().into()),
        polkadot_primitives::Signature::Sr25519(signature.clone()),
        extra.clone(),
    )
}

/// Parse the result of an RPC call.
// TODO: Review thoroughly this function.
pub(crate) fn parse_rpc_result(
    result: String,
    receiver: tokio::sync::mpsc::Receiver<String>,
) -> Result<RpcTransactionOutput, RpcTransactionError> {
    let json: serde_json::Value =
        serde_json::from_str(&result).expect("the result can only be a JSONRPC string; qed");
    let error = json
        .as_object()
        .expect("JSON result is always an object; qed")
        .get("error");

    if let Some(error) = error {
        return Err(serde_json::from_value(error.clone())
            .expect("the JSONRPC result's error is always valid; qed"));
    }

    Ok(RpcTransactionOutput { result, receiver })
}
