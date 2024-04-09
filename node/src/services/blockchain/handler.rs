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

use futures::prelude::*;
use sc_tracing::tracing::info;
use storage_hub_infra::actor::{Actor, ActorEventLoop};

use super::events::BlockchainServiceEventBusProvider;

const LOG_TARGET: &str = "blockchain-service";

#[derive(Debug)]
pub enum BlockchainServiceCommand {}

#[derive(Debug)]
pub struct BlockchainService {
    event_bus_provider: BlockchainServiceEventBusProvider,
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

        // TODO: listen to produced blocks and process them into defined events

        while let Some(message) = self.receiver.next().await {
            self.actor.handle_message(message).await;
        }
    }
}

impl BlockchainService {
    /// Create a new [`BlockchainService`].
    pub fn new() -> Self {
        Self {
            event_bus_provider: BlockchainServiceEventBusProvider::new(),
        }
    }
}
