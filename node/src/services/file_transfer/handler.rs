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

use anyhow::Result;
use futures::prelude::*;
use libp2p_identity::PeerId;
use log::{debug, info, trace, warn};
use prost::Message;
use sc_network::{
    request_responses::{IncomingRequest, OutgoingResponse, ProtocolConfig},
    ReputationChange,
};
use sp_core::hexdisplay::HexDisplay;
use storage_hub_infra::actor::{Actor, ActorEventLoop};
use tokio::select;

use super::schema;

const LOG_TARGET: &str = "file-transfer-service";

/// Max number of queued requests.
const MAX_FILE_TRANSFER_REQUESTS_QUEUE: usize = 500;

pub enum FileTransferServiceCommand {}

pub struct FileTransferService {
    request_receiver: async_channel::Receiver<IncomingRequest>,
}

impl Actor for FileTransferService {
    type Message = FileTransferServiceCommand;
    type EventLoop = FileTransferServiceEventLoop;
    type EventBusProvider = ();

    fn handle_message(
        &mut self,
        _message: Self::Message,
    ) -> impl std::future::Future<Output = ()> + Send {
        async {}
    }

    fn get_event_bus_provider(&self) -> &Self::EventBusProvider {
        &()
    }
}

/// Event loop for the FileTransferService actor.
pub struct FileTransferServiceEventLoop {
    receiver: sc_utils::mpsc::TracingUnboundedReceiver<FileTransferServiceCommand>,
    actor: FileTransferService,
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

    async fn run(&mut self) {
        info!("FileTransferService starting up!");
        loop {
            select! {
                request = self.actor.request_receiver.recv() => {
                    let IncomingRequest {
                        peer,
                        payload,
                        pending_response,
                    } = match request {
                        Err(_) => {
                            warn!(target: LOG_TARGET, "P2p request channel closed. Shutting down.");
                            break;
                        },
                        Ok(request) => request,
                    };

                    match self.actor.handle_request(peer, payload) {
                        Ok(response_data) => {
                            let response = OutgoingResponse {
                                result: Ok(response_data),
                                reputation_changes: Vec::new(),
                                sent_feedback: None,
                            };

                            match pending_response.send(response) {
                                Ok(()) => trace!(
                                    target: LOG_TARGET,
                                    "Handled provider client request from {}.",
                                    peer,
                                ),
                                Err(_) => debug!(
                                    target: LOG_TARGET,
                                    "Failed to handle provider request from {}: {}",
                                    peer,
                                    HandleRequestError::SendResponse,
                                ),
                            };
                        }
                        Err(e) => {
                            debug!(
                                target: LOG_TARGET,
                                "Failed to handle provider client request from {}: {}", peer, e,
                            );

                            let reputation_changes = match e {
                                HandleRequestError::BadRequest(_) => {
                                    vec![ReputationChange::new(-(1 << 12), "bad request")]
                                }
                                _ => Vec::new(),
                            };

                            let response = OutgoingResponse {
                                result: Err(()),
                                reputation_changes,
                                sent_feedback: None,
                            };

                            if pending_response.send(response).is_err() {
                                debug!(
                                    target: LOG_TARGET,
                                    "Failed to handle provider client request from {}: {}",
                                    peer,
                                    HandleRequestError::SendResponse,
                                );
                            };
                        }
                    }
                },
                message = self.receiver.next() => {
                    let message = message.expect("All senders dropped.");
                    self.actor.handle_message(message).await;
                },
            }
        }
    }
}

impl FileTransferService {
    /// Create a new [`FileTransferServiceEventLoop`].
    pub fn new<Hash: AsRef<[u8]>>(
        genesis_hash: Hash,
        fork_id: Option<&str>,
    ) -> (Self, ProtocolConfig) {
        let (tx, request_receiver) = async_channel::bounded(MAX_FILE_TRANSFER_REQUESTS_QUEUE);

        let mut protocol_config = super::generate_protocol_config(genesis_hash, fork_id);
        protocol_config.inbound_queue = Some(tx);

        (Self { request_receiver }, protocol_config)
    }

    fn handle_request(
        &mut self,
        peer: PeerId,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, HandleRequestError> {
        let request = schema::v1::provider::Request::decode(&payload[..])?;

        let response = match &request.request {
            Some(schema::v1::provider::request::Request::RemoteUploadDataRequest(r)) => {
                self.on_remote_upload_data_request(&peer, r)?
            }
            Some(schema::v1::provider::request::Request::RemoteReadRequest(r)) => {
                self.on_remote_read_request(&peer, r)?
            }
            None => {
                return Err(HandleRequestError::BadRequest(
                    "Remote request without request data.",
                ))
            }
        };

        let mut data = Vec::new();
        response.encode(&mut data)?;

        Ok(data)
    }

    fn on_remote_upload_data_request(
        &mut self,
        peer: &PeerId,
        request: &schema::v1::provider::RemoteUploadDataRequest,
    ) -> Result<schema::v1::provider::Response, HandleRequestError> {
        trace!("Remote call request from {}.", peer,);

        // TODO actually save data.
        let response = schema::v1::provider::RemoteUploadDataResponse {
            location: request.location.clone(),
        };

        Ok(schema::v1::provider::Response {
            response: Some(
                schema::v1::provider::response::Response::RemoteUploadDataResponse(response),
            ),
        })
    }

    fn on_remote_read_request(
        &mut self,
        peer: &PeerId,
        request: &schema::v1::provider::RemoteReadRequest,
    ) -> Result<schema::v1::provider::Response, HandleRequestError> {
        if request.locations.is_empty() {
            debug!("Invalid remote read request sent by {}.", peer);
            return Err(HandleRequestError::BadRequest(
                "Remote read request without locations.",
            ));
        }

        trace!(
            "Remote read request from {} ({}).",
            peer,
            fmt_keys(request.locations.first(), request.locations.last()),
        );

        // TODO actually read data.
        let response = schema::v1::provider::RemoteReadResponse {
            data: request.locations.clone(),
        };

        Ok(schema::v1::provider::Response {
            response: Some(schema::v1::provider::response::Response::RemoteReadResponse(response)),
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum HandleRequestError {
    #[error("Failed to decode request: {0}.")]
    DecodeProto(#[from] prost::DecodeError),
    #[error("Failed to encode response: {0}.")]
    EncodeProto(#[from] prost::EncodeError),
    #[error("Failed to send response.")]
    SendResponse,
    /// A bad request has been received.
    #[error("bad request: {0}")]
    BadRequest(&'static str),
    /// Encoding or decoding of some data failed.
    #[error("codec error: {0}")]
    Codec(#[from] codec::Error),
}

fn fmt_keys(first: Option<&Vec<u8>>, last: Option<&Vec<u8>>) -> String {
    if let (Some(first), Some(last)) = (first, last) {
        if first == last {
            HexDisplay::from(first).to_string()
        } else {
            format!("{}..{}", HexDisplay::from(first), HexDisplay::from(last))
        }
    } else {
        String::from("n/a")
    }
}
