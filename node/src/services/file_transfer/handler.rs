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

use std::{collections::HashSet, sync::Arc};

use futures::prelude::*;
use futures::stream::select;
use libp2p_identity::PeerId;
use prost::Message;
use sc_network::{
    request_responses::{IncomingRequest, OutgoingResponse},
    IfDisconnected, NetworkPeers, NetworkRequest, ProtocolName, ReputationChange,
};
use sc_tracing::tracing::{debug, error, info, warn};
use storage_hub_infra::{
    actor::{Actor, ActorEventLoop},
    types::Key,
};
use tokio::sync::Mutex;

use crate::{
    service::ParachainNetworkService, services::file_transfer::events::RemoteUploadRequest,
};

use super::{
    commands::{FileTransferServiceCommand, RequestError},
    events::{FileTransferServiceEventBusProvider, RemoteDownloadRequest},
    schema,
};

const LOG_TARGET: &str = "file-transfer-service";

pub struct FileTransferService {
    protocol_name: ProtocolName,
    request_receiver: async_channel::Receiver<IncomingRequest>,
    network: Arc<ParachainNetworkService>,
    peer_file_registry: HashSet<(PeerId, Key)>,
    event_bus_provider: FileTransferServiceEventBusProvider,
}

impl Actor for FileTransferService {
    type Message = FileTransferServiceCommand;
    type EventLoop = FileTransferServiceEventLoop;
    type EventBusProvider = FileTransferServiceEventBusProvider;

    fn handle_message(
        &mut self,
        message: Self::Message,
    ) -> impl std::future::Future<Output = ()> + Send {
        async {
            match message {
                FileTransferServiceCommand::UploadRequest {
                    peer_id,
                    file_key,
                    chunk_with_proof,
                    callback,
                } => {
                    let request = schema::v1::provider::request::Request::RemoteUploadDataRequest(
                        schema::v1::provider::RemoteUploadDataRequest {
                            file_key: bincode::serialize(&file_key)
                                .expect("Failed to serialize file key."),
                            file_chunk_with_proof: bincode::serialize(&chunk_with_proof)
                                .expect("Failed to serialize file chunk proof."),
                        },
                    );

                    // Serialize the request
                    let mut request_data = Vec::new();
                    request.encode(&mut request_data);

                    let (tx, rx) = futures::channel::oneshot::channel();
                    self.network.start_request(
                        peer_id,
                        self.protocol_name.clone(),
                        request_data,
                        None,
                        tx,
                        IfDisconnected::ImmediateError,
                    );

                    match callback.send(rx) {
                        Ok(()) => {}
                        Err(_) => error!(
                            target: LOG_TARGET,
                            "Failed to send the response back. Looks like the requester task is gone."
                        ),
                    }
                }
                FileTransferServiceCommand::DownloadRequest {
                    peer_id,
                    file_key,
                    chunk_id,
                    callback,
                } => {
                    let request = schema::v1::provider::request::Request::RemoteDownloadDataRequest(
                        schema::v1::provider::RemoteDownloadDataRequest {
                            file_key: bincode::serialize(&file_key)
                                .expect("Failed to serialize file key."),
                            file_chunk_id: chunk_id,
                        },
                    );

                    // Serialize the request
                    let mut request_data = Vec::new();
                    request.encode(&mut request_data);

                    let (tx, rx) = futures::channel::oneshot::channel();
                    self.network.start_request(
                        peer_id,
                        self.protocol_name.clone(),
                        request_data,
                        None,
                        tx,
                        IfDisconnected::ImmediateError,
                    );

                    match callback.send(rx) {
                        Ok(()) => {}
                        Err(_) => error!(
                            target: LOG_TARGET,
                            "Failed to send the response back. Looks like the requester task is gone."
                        ),
                    }
                }
                FileTransferServiceCommand::AddKnownAddress {
                    peer_id,
                    multiaddress,
                    callback,
                } => {
                    self.network.add_known_address(peer_id, multiaddress);
                    // `add_known_address()` method doesn't return anything.
                    match callback.send(Ok(())) {
                        Ok(()) => {}
                        Err(_) => error!(
                            target: LOG_TARGET,
                            "Failed to send the response back. Looks like the requester task is gone."
                        ),
                    }
                }
                FileTransferServiceCommand::RegisterNewFile {
                    peer_id,
                    file_key,
                    callback,
                } => {
                    let result = match self.peer_file_registry.insert((peer_id, file_key)) {
                        true => Ok(()),
                        false => Err(RequestError::FileAlreadyRegisteredForPeer),
                    };

                    match callback.send(result) {
                        Ok(()) => {}
                        Err(_) => error!(
                            target: LOG_TARGET,
                            "Failed to send the response back. Looks like the requester task is gone."
                        ),
                    }
                }
                FileTransferServiceCommand::UnregisterFile {
                    peer_id,
                    file_key,
                    callback,
                } => {
                    let result = match self.peer_file_registry.remove(&(peer_id, file_key)) {
                        true => Ok(()),
                        false => Err(RequestError::FileNotRegisteredForPeer),
                    };
                    match callback.send(result) {
                        Ok(()) => {}
                        Err(_) => error!(
                            target: LOG_TARGET,
                            "Failed to send the response back. Looks like the requester task is gone."
                        ),
                    }
                }
            };
        }
    }

    fn get_event_bus_provider(&self) -> &Self::EventBusProvider {
        &self.event_bus_provider
    }
}

/// Event loop for the FileTransferService actor.
pub struct FileTransferServiceEventLoop {
    receiver: sc_utils::mpsc::TracingUnboundedReceiver<FileTransferServiceCommand>,
    actor: FileTransferService,
}

enum MergedEventLoopMessage {
    Command(FileTransferServiceCommand),
    Request(IncomingRequest),
}

/// Since this actor is a network service, it needs to handle both incoming network events and
/// messages from other actors, hence the need for a custom `ActorEventLoop`.
impl ActorEventLoop<FileTransferService> for FileTransferServiceEventLoop {
    fn new(
        actor: FileTransferService,
        receiver: sc_utils::mpsc::TracingUnboundedReceiver<FileTransferServiceCommand>,
    ) -> Self {
        Self { actor, receiver }
    }

    async fn run(mut self) {
        info!(target: LOG_TARGET, "FileTransferService starting up!");

        let mut merged_stream = select(
            self.receiver.map(MergedEventLoopMessage::Command),
            self.actor
                .request_receiver
                .clone()
                .map(MergedEventLoopMessage::Request),
        );

        loop {
            match merged_stream.next().await {
                Some(MergedEventLoopMessage::Command(command)) => {
                    self.actor.handle_message(command).await;
                }
                Some(MergedEventLoopMessage::Request(request)) => {
                    let IncomingRequest {
                        peer,
                        payload,
                        pending_response,
                    } = request;

                    self.actor.handle_request(peer, payload, pending_response);
                }
                None => {
                    warn!(target: LOG_TARGET, "FileTransferService event loop terminated.");
                    break;
                }
            }
        }
    }
}

impl FileTransferService {
    /// Create a new [`FileTransferService`].
    pub fn new(
        protocol_name: ProtocolName,
        request_receiver: async_channel::Receiver<IncomingRequest>,
        network: Arc<ParachainNetworkService>,
    ) -> Self {
        Self {
            protocol_name,
            request_receiver,
            network,
            peer_file_registry: HashSet::new(),
            event_bus_provider: FileTransferServiceEventBusProvider::new(),
        }
    }

    fn handle_request(
        &mut self,
        peer: PeerId,
        payload: Vec<u8>,
        pending_response: futures::channel::oneshot::Sender<OutgoingResponse>,
    ) {
        let request = match schema::v1::provider::Request::decode(&payload[..]) {
            Ok(request) => request,
            Err(e) => {
                error!(
                    target: LOG_TARGET,
                    "Failed to decode provider client request from {}: {:?}", peer, e
                );

                self.handle_bad_request(pending_response);

                return;
            }
        };

        match &request.request {
            Some(schema::v1::provider::request::Request::RemoteUploadDataRequest(r)) => {
                let file_key = match bincode::deserialize(&r.file_key) {
                    Ok(file_key) => file_key,
                    Err(e) => {
                        error!(
                            target: LOG_TARGET,
                            "Failed to deserialize file key from provider client request from {}: {:?}",
                            peer,
                            e
                        );

                        self.handle_bad_request(pending_response);

                        return;
                    }
                };
                let chunk_with_proof = match bincode::deserialize(&r.file_chunk_with_proof) {
                    Ok(chunk_with_proof) => chunk_with_proof,
                    Err(e) => {
                        error!(
                            target: LOG_TARGET,
                            "Failed to deserialize file chunk with proof from provider client request from {}: {:?}",
                            peer,
                            e
                        );

                        self.handle_bad_request(pending_response);

                        return;
                    }
                };
                self.emit(RemoteUploadRequest {
                    file_key,
                    chunk_with_proof,
                    maybe_pending_response: Arc::new(Mutex::new(Some(pending_response))),
                });
            }
            Some(schema::v1::provider::request::Request::RemoteDownloadDataRequest(r)) => {
                let file_key = match bincode::deserialize(&r.file_key) {
                    Ok(file_key) => file_key,
                    Err(e) => {
                        error!(
                            target: LOG_TARGET,
                            "Failed to deserialize file key from provider client request from {}: {:?}",
                            peer,
                            e
                        );

                        self.handle_bad_request(pending_response);

                        return;
                    }
                };
                let chunk_id = r.file_chunk_id;
                self.emit(RemoteDownloadRequest {
                    file_key,
                    chunk_id,
                    maybe_pending_response: Arc::new(Mutex::new(Some(pending_response))),
                });
            }
            None => {
                error!(
                    target: LOG_TARGET,
                    "Received provider client request from {} with no request", peer
                );

                self.handle_bad_request(pending_response);

                return;
            }
        };
    }

    fn handle_bad_request(
        &self,
        pending_response: futures::channel::oneshot::Sender<OutgoingResponse>,
    ) {
        let reputation_changes = vec![ReputationChange::new(-(1 << 12), "bad request")];

        let response = OutgoingResponse {
            result: Err(()),
            reputation_changes,
            sent_feedback: None,
        };

        if pending_response.send(response).is_err() {
            debug!(target: LOG_TARGET, "Failed to send request response back");
        }
    }
}
