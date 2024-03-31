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
use libp2p_identity::PeerId;
use log::{debug, trace};
use prost::Message;
use sc_client_api::{BlockBackend, ProofProvider};
use sc_network::{
    request_responses::{IncomingRequest, OutgoingResponse, ProtocolConfig},
    ReputationChange,
};
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc};

use super::schema;

const LOG_TARGET: &str = "provider-requests-handler";

/// Max number of queued requests.
const MAX_PROVIDER_REQUESTS_QUEUE: usize = 500;

/// Handler for incoming provider requests from a remote peer.
pub struct ProviderRequestsHandler<B, Client> {
    request_receiver: async_channel::Receiver<IncomingRequest>,
    /// Blockchain client.
    _client: Arc<Client>,
    _block: PhantomData<B>,
}

impl<B, Client> ProviderRequestsHandler<B, Client>
where
    B: Block,
    Client: BlockBackend<B> + ProofProvider<B> + Send + Sync + 'static,
{
    /// Create a new [`ProviderRequestHandler`].
    pub fn new(fork_id: Option<&str>, client: Arc<Client>) -> (Self, ProtocolConfig) {
        let (tx, request_receiver) = async_channel::bounded(MAX_PROVIDER_REQUESTS_QUEUE);

        let mut protocol_config = super::generate_protocol_config(
            client
                .block_hash(0u32.into())
                .ok()
                .flatten()
                .expect("Genesis block exists; qed"),
            fork_id,
        );
        protocol_config.inbound_queue = Some(tx);

        (
            Self {
                _client: client,
                request_receiver,
                _block: PhantomData::default(),
            },
            protocol_config,
        )
    }

    /// Run [`ProviderRequestsHandler`].
    pub async fn run(mut self) {
        while let Some(request) = self.request_receiver.next().await {
            let IncomingRequest {
                peer,
                payload,
                pending_response,
            } = request;

            match self.handle_request(peer, payload) {
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
        }
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
