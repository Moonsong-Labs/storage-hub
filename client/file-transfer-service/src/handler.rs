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

use codec::{Decode, Encode};
use futures::stream::{self, StreamExt};
use prost::Message;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    sync::Arc,
};
use tokio::time::{interval, Duration};

use sc_network::{
    request_responses::{IncomingRequest, OutgoingResponse, RequestFailure},
    service::traits::NetworkService,
    IfDisconnected, NetworkPeers, NetworkRequest, ProtocolName, ReputationChange,
};
use sc_network_types::PeerId;
use sc_tracing::tracing::{debug, error, info, trace, warn};

use shc_actors_framework::actor::{Actor, ActorEventLoop};
use shc_common::{
    traits::StorageEnableRuntime,
    types::{
        BucketId, DownloadRequestId, FileKey, FileKeyProof, UploadRequestId,
        BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE, FILE_CHUNK_SIZE,
    },
};
use shp_file_metadata::ChunkId;

use super::{
    commands::{FileTransferServiceCommand, RequestError},
    events::{
        FileTransferServiceEventBusProvider, RemoteDownloadRequest, RemoteUploadRequest,
        RetryBucketMoveDownload,
    },
    schema,
};

const LOG_TARGET: &str = "file-transfer-service";

/// Time interval between bucket move download retry attempts (in seconds)
const BUCKET_MOVE_RETRY_INTERVAL_SECONDS: u64 = 3 * 60 * 60; // 3 hours

#[derive(Eq)]
pub struct BucketIdWithExpiration<Runtime: StorageEnableRuntime> {
    bucket_id: BucketId<Runtime>,
    expiration: chrono::DateTime<chrono::Utc>,
}

impl<Runtime: StorageEnableRuntime> BucketIdWithExpiration<Runtime> {
    pub fn new(bucket_id: BucketId<Runtime>, grace_period_seconds: u64) -> Self {
        let expiration = chrono::Utc::now()
            + chrono::Duration::seconds(grace_period_seconds.try_into().unwrap_or(0));
        Self {
            bucket_id,
            expiration,
        }
    }
}

impl<Runtime: StorageEnableRuntime> PartialEq for BucketIdWithExpiration<Runtime> {
    fn eq(&self, other: &Self) -> bool {
        self.expiration.eq(&other.expiration) && self.bucket_id.eq(&other.bucket_id)
    }
}

impl<Runtime: StorageEnableRuntime> PartialOrd for BucketIdWithExpiration<Runtime> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<Runtime: StorageEnableRuntime> Ord for BucketIdWithExpiration<Runtime> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Order by expiration
        self.expiration.cmp(&other.expiration)
    }
}

pub struct FileTransferService<Runtime: StorageEnableRuntime> {
    /// Protocol name used by substrate network for the file transfer service.
    protocol_name: ProtocolName,
    /// Receiver for incoming requests.
    request_receiver: async_channel::Receiver<IncomingRequest>,
    /// Substrate network service that gives access to p2p operations.
    network: Arc<dyn NetworkService>,
    /// Registry of (peer, file key) pairs for which we accept requests.
    peer_file_allow_list: HashSet<(PeerId, FileKey)>,
    /// Registry of peers by file key, used for cleanup.
    peers_by_file: HashMap<FileKey, Vec<PeerId>>,
    /// Registry of (peer, bucket id) pairs for which we accept requests.
    peer_bucket_allow_list: HashSet<(PeerId, BucketId<Runtime>)>,
    /// Registry of peers by bucket id, used for cleanup.
    peers_by_bucket: HashMap<BucketId<Runtime>, Vec<PeerId>>,
    /// Mapping from bucket id to the grace period time.
    bucket_allow_list_grace_period_time: BTreeSet<BucketIdWithExpiration<Runtime>>,
    /// The event bus provider for the file transfer service.
    /// Part of the actor framework, allows for emitting events.
    event_bus_provider: FileTransferServiceEventBusProvider<Runtime>,
    /// Mapping from RequestId to a download pending response channel
    download_pending_responses:
        HashMap<DownloadRequestId, futures::channel::oneshot::Sender<OutgoingResponse>>,
    /// Counter for generating unique download request IDs
    download_pending_response_nonce: DownloadRequestId,
    /// Mapping from RequestId to an upload pending response channel
    upload_pending_responses:
        HashMap<UploadRequestId, futures::channel::oneshot::Sender<OutgoingResponse>>,
    /// Counter for generating unique upload request IDs
    upload_pending_response_nonce: UploadRequestId,
    /// Timestamp of the last bucket move retry check
    last_bucket_move_retry: Option<chrono::DateTime<chrono::Utc>>,
}

impl<Runtime: StorageEnableRuntime> Actor for FileTransferService<Runtime> {
    type Message = FileTransferServiceCommand<Runtime>;
    type EventLoop = FileTransferServiceEventLoop<Runtime>;
    type EventBusProvider = FileTransferServiceEventBusProvider<Runtime>;

    fn handle_message(
        &mut self,
        message: Self::Message,
    ) -> impl std::future::Future<Output = ()> + Send {
        async {
            match message {
                FileTransferServiceCommand::UploadRequest {
                    peer_id,
                    file_key,
                    file_key_proof,
                    bucket_id,
                    callback,
                } => {
                    let request = schema::v1::provider::request::Request::RemoteUploadDataRequest(
                        schema::v1::provider::RemoteUploadDataRequest {
                            file_key: file_key.encode(),
                            file_key_proof: file_key_proof.encode(),
                            bucket_id: bucket_id.map(|id| id.encode()),
                        },
                    );

                    // Serialize the request
                    let mut request_data = Vec::new();
                    request.encode(&mut request_data);

                    let (tx, rx) = futures::channel::oneshot::channel();
                    self.network.start_request(
                        peer_id.into(),
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
                // TODO: Remove this handler once legacy upload is deprecated
                FileTransferServiceCommand::ReceiveBackendFileChunksRequest {
                    file_key,
                    file_key_proof,
                    callback,
                } => {
                    let request = schema::v1::provider::request::Request::RemoteUploadDataRequest(
                        schema::v1::provider::RemoteUploadDataRequest {
                            file_key: file_key.encode(),
                            file_key_proof: file_key_proof.encode(),
                            bucket_id: None,
                        },
                    );

                    // Serialize the request
                    let mut request_data = Vec::new();
                    request.encode(&mut request_data);

                    // Directly handle the request locally, using the local peer ID
                    let local_peer = self.network.local_peer_id();
                    let (tx_local, rx_local) = futures::channel::oneshot::channel();
                    self.handle_request(local_peer, request_data, tx_local)
                        .await;

                    // Map the local response to the expected result shape
                    let (tx_net, rx_net) = futures::channel::oneshot::channel();
                    let protocol = self.protocol_name.clone();
                    tokio::spawn(async move {
                        let mapped: Result<(Vec<u8>, ProtocolName), RequestFailure> =
                            match rx_local.await {
                                Ok(out) => match out.result {
                                    Ok(bytes) => Ok((bytes, protocol)),
                                    Err(()) => Err(RequestFailure::Refused),
                                },
                                Err(_) => Err(RequestFailure::NotConnected),
                            };
                        let _ = tx_net.send(mapped);
                    });

                    match callback.send(rx_net) {
                        Ok(()) => {}
                        Err(_) => error!(
                            target: LOG_TARGET,
                            "Failed to send the response back. Looks like the requester task is gone."
                        ),
                    }
                }
                FileTransferServiceCommand::UploadResponse {
                    request_id,
                    file_complete,
                    callback,
                } => {
                    trace!(target: LOG_TARGET, "Received upload response for request id {:?}", request_id);
                    trace!(target: LOG_TARGET, "File complete: {:?}", file_complete);

                    let response =
                        schema::v1::provider::response::Response::RemoteUploadDataResponse(
                            schema::v1::provider::RemoteUploadDataResponse {
                                success: true,
                                file_complete,
                            },
                        );

                    let mut response_data = Vec::new();
                    response.encode(&mut response_data);

                    let outgoing_response = OutgoingResponse {
                        result: Ok(response_data),
                        reputation_changes: Vec::new(),
                        sent_feedback: None,
                    };

                    // Tries to find the sender half of the response channel
                    let maybe_pending_response =
                        self.upload_pending_responses.remove(&request_id).take();

                    // Tries to send back the upload response and then gets the request callback result.
                    let request_callback_result = match maybe_pending_response {
                        Some(pending_response_sender) => {
                            // Tries to send upload response back
                            let pending_response_result =
                                pending_response_sender.send(outgoing_response);

                            // Checks if response was sent back
                            match pending_response_result {
                                Ok(()) => {
                                    trace!(target: LOG_TARGET, "Upload response sent back successfully");
                                    callback.send(Ok(()))
                                }
                                Err(e) => {
                                    error!(
                                        target: LOG_TARGET,
                                        "Failed to return Upload Response {:?}", e
                                    );
                                    callback.send(Err(RequestError::UploadResponseFailure(e)))
                                }
                            }
                        }
                        None => {
                            error!(target: LOG_TARGET, "No pending response channel found for request id {:?}", request_id);
                            callback.send(Err(RequestError::UploadRequestIdNotFound))
                        }
                    };

                    // Checks that request callback was returned correctly.
                    match request_callback_result {
                        Ok(()) => {}
                        Err(_) => error!(
                            target: LOG_TARGET,
                            "Failed to send the response back. Looks like the requester task is gone."
                        ),
                    };
                }
                FileTransferServiceCommand::DownloadRequest {
                    peer_id,
                    file_key,
                    chunk_ids,
                    bucket_id,
                    callback,
                } => {
                    // Calculate max chunks based on packet size and chunk size
                    let max_chunks =
                        (BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE as u64) / (FILE_CHUNK_SIZE as u64);
                    if chunk_ids.len() > max_chunks as usize {
                        warn!(
                            target: LOG_TARGET,
                            "Requested batch size {} exceeds maximum allowed {} based on packet size limit.",
                            chunk_ids.len(),
                            max_chunks
                        );
                    }

                    // Convert HashSet to Vec only for protobuf encoding
                    let chunk_ids_u64: Vec<u64> =
                        chunk_ids.iter().map(|chunk_id| chunk_id.as_u64()).collect();

                    let request = schema::v1::provider::request::Request::RemoteDownloadDataRequest(
                        schema::v1::provider::RemoteDownloadDataRequest {
                            file_key: file_key.encode(),
                            file_chunk_ids: chunk_ids_u64,
                            bucket_id: bucket_id.map(|id| id.encode()),
                        },
                    );

                    let mut request_data = Vec::new();
                    request.encode(&mut request_data);

                    let (tx, rx) = futures::channel::oneshot::channel();
                    self.network.start_request(
                        peer_id.into(),
                        self.protocol_name.clone(),
                        request_data,
                        None,
                        tx,
                        IfDisconnected::ImmediateError,
                    );

                    match callback.send(rx) {
                        Ok(()) => {}
                        Err(_) => {
                            error!(target: LOG_TARGET, "Failed to send the response back. Looks like the requester task is gone.")
                        }
                    }
                }
                FileTransferServiceCommand::DownloadResponse {
                    request_id,
                    file_key_proof,
                    callback,
                } => {
                    let response =
                        schema::v1::provider::response::Response::RemoteDownloadDataResponse(
                            schema::v1::provider::RemoteDownloadDataResponse {
                                file_key_proof: file_key_proof.encode(),
                            },
                        );

                    let mut response_data = Vec::new();
                    response.encode(&mut response_data);

                    let outgoing_response = OutgoingResponse {
                        sent_feedback: None,
                        result: Ok(response_data),
                        reputation_changes: Vec::new(),
                    };

                    // Tries to find the sender half of the response channel
                    let maybe_pending_response =
                        self.download_pending_responses.remove(&request_id).take();

                    // Tries to send back the download response and then gets the request callback result.
                    let request_callback_result = match maybe_pending_response {
                        Some(pending_response_sender) => {
                            // Tries to send download response back
                            let pending_response_result =
                                pending_response_sender.send(outgoing_response);

                            // Checks if response was sent back
                            match pending_response_result {
                                Ok(()) => callback.send(Ok(())),
                                Err(e) => {
                                    error!(
                                        target: LOG_TARGET,
                                        "Failed to return Download Response {:?}", e
                                    );
                                    callback.send(Err(RequestError::DownloadResponseFailure(e)))
                                }
                            }
                        }
                        None => {
                            error!(target: LOG_TARGET, "No pending response channel found for request id {:?}", request_id);
                            callback.send(Err(RequestError::DownloadRequestIdNotFound))
                        }
                    };

                    // Checks that request callback was returned correctly.
                    match request_callback_result {
                        Ok(()) => {}
                        Err(_) => error!(
                            target: LOG_TARGET,
                            "Failed to send the response back. Looks like the requester task is gone."
                        ),
                    };
                }
                FileTransferServiceCommand::AddKnownAddress {
                    peer_id,
                    multiaddress,
                    callback,
                } => {
                    self.network.add_known_address(peer_id.into(), multiaddress);
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
                    if !self.peer_file_allow_list.insert((peer_id, file_key)) {
                        trace!(target: LOG_TARGET, "File already registered for peer id {} and file key {:?}", peer_id, file_key);
                    }

                    self.peers_by_file
                        .entry(file_key)
                        .or_insert_with(Vec::new)
                        .push(peer_id);

                    match callback.send(Ok(())) {
                        Ok(()) => {}
                        Err(_) => error!(
                            target: LOG_TARGET,
                            "Failed to send the response back. Looks like the requester task is gone."
                        ),
                    }
                }
                FileTransferServiceCommand::UnregisterFile { file_key, callback } => {
                    let result = match self.peers_by_file.get(&file_key) {
                        Some(peers) => {
                            for peer_id in peers {
                                self.peer_file_allow_list.remove(&(*peer_id, file_key));
                            }
                            self.peers_by_file.remove(&file_key);
                            Ok(())
                        }
                        None => Err(RequestError::FileNotRegistered),
                    };
                    match callback.send(result) {
                        Ok(()) => {}
                        Err(_) => error!(
                            target: LOG_TARGET,
                            "Failed to send the response back. Looks like the requester task is gone."
                        ),
                    }
                }
                FileTransferServiceCommand::IsFileExpected { file_key, callback } => {
                    let is_expected = self.peers_by_file.contains_key(&file_key);
                    match callback.send(Ok(is_expected)) {
                        Ok(()) => {}
                        Err(_) => error!(
                            target: LOG_TARGET,
                            "Failed to send the response back. Looks like the requester task is gone."
                        ),
                    }
                }
                FileTransferServiceCommand::RegisterNewBucketPeer {
                    peer_id,
                    bucket_id,
                    callback,
                } => {
                    info!(target: LOG_TARGET, "Registering new bucket peer {:?} for bucket {:?}", peer_id, bucket_id);
                    let result = match self.peer_bucket_allow_list.insert((peer_id, bucket_id)) {
                        true => Ok(()),
                        false => Err(RequestError::BucketAlreadyRegisteredForPeer),
                    };

                    self.peers_by_bucket
                        .entry(bucket_id)
                        .or_insert_with(Vec::new)
                        .push(peer_id);

                    match callback.send(result) {
                        Ok(()) => {}
                        Err(_) => error!(
                            target: LOG_TARGET,
                            "Failed to send the response back. Looks like the requester task is gone."
                        ),
                    }
                }
                FileTransferServiceCommand::ScheduleUnregisterBucket {
                    bucket_id,
                    grace_period_seconds,
                    callback,
                } => {
                    let result = match grace_period_seconds {
                        Some(grace_period_seconds) => {
                            self.bucket_allow_list_grace_period_time.insert(
                                BucketIdWithExpiration::new(bucket_id, grace_period_seconds),
                            );
                            Ok(())
                        }
                        None => self.unregister_bucket(bucket_id),
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
pub struct FileTransferServiceEventLoop<Runtime: StorageEnableRuntime> {
    receiver: sc_utils::mpsc::TracingUnboundedReceiver<FileTransferServiceCommand<Runtime>>,
    actor: FileTransferService<Runtime>,
}

enum MergedEventLoopMessage<Runtime: StorageEnableRuntime> {
    Command(FileTransferServiceCommand<Runtime>),
    Request(IncomingRequest),
    Tick,
}

/// Since this actor is a network service, it needs to handle both incoming network events and
/// messages from other actors, hence the need for a custom `ActorEventLoop`.
impl<Runtime: StorageEnableRuntime> ActorEventLoop<FileTransferService<Runtime>>
    for FileTransferServiceEventLoop<Runtime>
{
    fn new(
        actor: FileTransferService<Runtime>,
        receiver: sc_utils::mpsc::TracingUnboundedReceiver<FileTransferServiceCommand<Runtime>>,
    ) -> Self {
        Self { actor, receiver }
    }

    async fn run(mut self) {
        info!(target: LOG_TARGET, "💾 StorageHub's File Transfer Service starting up!");

        let ticker = interval(Duration::from_secs(1));
        let ticker_stream = stream::unfold(ticker, |mut interval| {
            Box::pin(async move {
                interval.tick().await;
                Some((MergedEventLoopMessage::Tick, interval))
            })
        });

        let mut merged_stream = stream::select(
            stream::select(
                self.receiver.map(MergedEventLoopMessage::Command),
                self.actor
                    .request_receiver
                    .clone()
                    .map(MergedEventLoopMessage::Request),
            ),
            ticker_stream,
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

                    self.actor
                        .handle_request(peer.into(), payload, pending_response)
                        .await;
                }
                Some(MergedEventLoopMessage::Tick) => {
                    // Handle expired buckets
                    self.actor.handle_expired_buckets();

                    // Check for pending bucket move downloads to retry
                    self.actor.handle_retry_bucket_move();
                }
                None => {
                    warn!(target: LOG_TARGET, "FileTransferService event loop terminated.");
                    break;
                }
            }
        }
    }
}

impl<Runtime: StorageEnableRuntime> FileTransferService<Runtime> {
    /// Create a new [`FileTransferService`].
    pub fn new(
        protocol_name: ProtocolName,
        request_receiver: async_channel::Receiver<IncomingRequest>,
        network: Arc<dyn NetworkService>,
    ) -> Self {
        Self {
            protocol_name,
            request_receiver,
            network,
            peer_file_allow_list: HashSet::new(),
            peers_by_file: HashMap::new(),
            peer_bucket_allow_list: HashSet::new(),
            peers_by_bucket: HashMap::new(),
            bucket_allow_list_grace_period_time: BTreeSet::new(),
            event_bus_provider: FileTransferServiceEventBusProvider::new(),
            download_pending_responses: HashMap::new(),
            download_pending_response_nonce: DownloadRequestId::new(0),
            upload_pending_responses: HashMap::new(),
            upload_pending_response_nonce: UploadRequestId::new(0),
            last_bucket_move_retry: None,
        }
    }

    async fn handle_request(
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
                let file_key = match FileKey::decode(&mut r.file_key.as_slice()) {
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

                let file_key_proof = match FileKeyProof::decode(&mut r.file_key_proof.as_slice()) {
                    Ok(file_key_proof) => file_key_proof,
                    Err(e) => {
                        error!(
                            target: LOG_TARGET,
                            "Failed to deserialize file key proof from provider client request from {}: {:?}",
                            peer,
                            e
                        );

                        self.handle_bad_request(pending_response);

                        return;
                    }
                };

                let bucket_id = match &r.bucket_id {
                    Some(bucket_id) => match BucketId::<Runtime>::decode(&mut bucket_id.as_slice())
                    {
                        Ok(bucket_id) => Some(bucket_id),
                        Err(e) => {
                            error!(
                                target: LOG_TARGET,
                                "Failed to deserialize bucket id from provider client request from {}: {:?}",
                                peer,
                                e
                            );

                            self.handle_bad_request(pending_response);

                            return;
                        }
                    },
                    None => None,
                };

                // Check if the peer is allowed to upload this file
                if !self.is_allowed(peer, file_key, bucket_id) {
                    debug!(
                        target: LOG_TARGET,
                        "Received unexpected upload request from {} for file key {:?}",
                        peer,
                        file_key
                    );

                    self.handle_bad_request(pending_response);
                    return;
                }

                // Generate a new request ID for this upload request
                let request_id = self.upload_pending_response_nonce.next();

                // Store the pending response channel with this ID
                self.upload_pending_responses
                    .insert(request_id.clone(), pending_response);

                // Emit the RemoteUploadRequest event
                self.emit(RemoteUploadRequest {
                    peer,
                    file_key,
                    file_key_proof,
                    bucket_id,
                    request_id,
                });
            }
            Some(schema::v1::provider::request::Request::RemoteDownloadDataRequest(r)) => {
                // TODO: Respond to the pending_response with some criteria of what is a valid download request.
                let file_key = match FileKey::decode(&mut r.file_key.as_slice()) {
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

                let bucket_id = match r.bucket_id {
                    Some(ref bucket_id) => {
                        BucketId::<Runtime>::decode(&mut bucket_id.as_slice()).ok()
                    }
                    None => None,
                };

                if !self.is_allowed(peer, file_key, bucket_id) {
                    warn!(
                        target: LOG_TARGET,
                        "Received unexpected download request from {} for file key {:?} (bucket {:?})",
                        peer, file_key, bucket_id
                    );

                    self.handle_bad_request(pending_response);

                    return;
                }

                let chunk_ids = r
                    .file_chunk_ids
                    .iter()
                    .map(|id| ChunkId::new(*id))
                    .collect();
                let request_id = self.download_pending_response_nonce.next();
                self.download_pending_responses
                    .insert(request_id.clone(), pending_response);

                self.emit(RemoteDownloadRequest {
                    file_key,
                    chunk_ids,
                    request_id,
                    bucket_id,
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

    fn is_allowed(
        &self,
        peer: PeerId,
        file_key: FileKey,
        bucket_id: Option<BucketId<Runtime>>,
    ) -> bool {
        // Always accept local requests
        if peer == self.network.local_peer_id() {
            return true;
        }

        if self.peer_file_allow_list.contains(&(peer, file_key)) {
            return true;
        }

        if let Some(bucket_id) = bucket_id {
            self.peer_bucket_allow_list.contains(&(peer, bucket_id))
        } else {
            false
        }
    }

    fn handle_bad_request(
        &self,
        pending_response: futures::channel::oneshot::Sender<OutgoingResponse>,
    ) {
        debug!(target: LOG_TARGET, "Bad request received. Lowering reputation.");
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

    fn unregister_bucket(&mut self, bucket_id: BucketId<Runtime>) -> Result<(), RequestError> {
        let result = match self.peers_by_bucket.get(&bucket_id) {
            Some(peers) => {
                for peer_id in peers {
                    self.peer_bucket_allow_list.remove(&(*peer_id, bucket_id));
                }
                Ok(())
            }
            None => Err(RequestError::BucketNotRegisteredForPeer),
        };

        result
    }

    fn handle_expired_buckets(&mut self) {
        // Return early if there are no buckets to unregister.
        if self.bucket_allow_list_grace_period_time.is_empty() {
            return;
        }

        // Get the current time.
        let now = chrono::Utc::now();

        // At this point we know there must be at least one bucket in the allow list.
        let mut bucket_to_check = self.bucket_allow_list_grace_period_time.first();
        while bucket_to_check.map_or(false, |bucket| bucket.expiration < now) {
            // Remove the bucket from the allow list.
            let bucket_to_remove = self
                .bucket_allow_list_grace_period_time
                .pop_first()
                .expect("Bucket allow list is not empty; qed");

            // Try to unregister the bucket.
            if let Err(e) = self.unregister_bucket(bucket_to_remove.bucket_id) {
                error!(target: LOG_TARGET, "Failed to unregister expired bucket {:?}: {:?}", bucket_to_remove.bucket_id, e);
            }

            // Update the expiration to check.
            bucket_to_check = self.bucket_allow_list_grace_period_time.first();
        }
    }

    fn handle_retry_bucket_move(&mut self) {
        let now = chrono::Utc::now();

        // Check if it's time to retry bucket move downloads
        let should_retry = match self.last_bucket_move_retry {
            // If we've never retried or enough time has passed
            None => true,
            Some(last_retry) => {
                let duration_since_last_retry = now.signed_duration_since(last_retry);
                duration_since_last_retry.num_seconds() as u64 >= BUCKET_MOVE_RETRY_INTERVAL_SECONDS
            }
        };

        if should_retry {
            // Update the last retry timestamp
            self.last_bucket_move_retry = Some(now);

            // Emit the retry event
            info!(
                target: LOG_TARGET,
                "Emitting RetryBucketMoveDownload event to check for pending bucket downloads"
            );
            self.emit(RetryBucketMoveDownload);
        }
    }
}
