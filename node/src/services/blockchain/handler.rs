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

use codec::Decode;
use frame_support::{StorageHasher, Twox128};
use frame_system::EventRecord;
use futures::prelude::*;
use sc_client_api::BlockchainEvents;
use sc_client_api::StorageKey;
use sc_client_api::StorageProvider;
use sc_tracing::tracing::info;
use storage_hub_infra::actor::{Actor, ActorEventLoop};

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

impl BlockchainService {
    /// Create a new [`BlockchainService`].
    pub fn new(client: Arc<ParachainClient>) -> Self {
        Self {
            client,
            event_bus_provider: BlockchainServiceEventBusProvider::new(),
        }
    }
}
