use anyhow::Result;
use prost::Message;
use sc_network::{Multiaddr, PeerId, ProtocolName, RequestFailure};
use storage_hub_infra::{
    actor::ActorHandle,
    types::{ChunkId, FileProof, Key},
};

use crate::services::FileTransferService;

use super::schema;

/// Messages understood by the FileTransfer service actor
pub enum FileTransferServiceCommand {
    UploadRequest {
        peer_id: PeerId,
        file_key: Key,
        chunk_with_proof: FileProof,
        callback: tokio::sync::oneshot::Sender<
            futures::channel::oneshot::Receiver<Result<(Vec<u8>, ProtocolName), RequestFailure>>,
        >,
    },
    DownloadRequest {
        peer_id: PeerId,
        file_key: Key,
        chunk_id: ChunkId,
        callback: tokio::sync::oneshot::Sender<
            futures::channel::oneshot::Receiver<Result<(Vec<u8>, ProtocolName), RequestFailure>>,
        >,
    },
    AddKnownAddress {
        peer_id: PeerId,
        multiaddress: Multiaddr,
        callback: tokio::sync::oneshot::Sender<Result<(), RequestError>>,
    },
    RegisterNewFile {
        peer_id: PeerId,
        file_key: Key,
        callback: tokio::sync::oneshot::Sender<Result<(), RequestError>>,
    },
    UnregisterFile {
        peer_id: PeerId,
        file_key: Key,
        callback: tokio::sync::oneshot::Sender<Result<(), RequestError>>,
    },
}

#[derive(Debug)]
pub enum RequestError {
    /// The request failed. More details are provided in the `RequestFailure`.
    RequestFailure(RequestFailure),
    /// The response was not a valid protobuf message.
    DecodeError(prost::DecodeError),
    /// The response was decoded successfully, but it was not the expected response.
    UnexpectedResponse,
    /// File is already stored in for this Peer in the registry
    FileAlreadyRegisteredForPeer,
    /// File not found in for this Peer in the registry
    FileNotRegisteredForPeer,
}

/// Allows our ActorHandle to implement
/// the specific methods for each kind of message.
pub trait FileTransferServiceInterface {
    async fn upload_request(
        &self,
        peer_id: PeerId,
        file_key: Key,
        data: FileProof,
    ) -> Result<schema::v1::provider::RemoteUploadDataResponse, RequestError>;

    async fn download_request(
        &self,
        peer_id: PeerId,
        file_key: Key,
        chunk_id: ChunkId,
    ) -> Result<schema::v1::provider::RemoteDownloadDataResponse, RequestError>;

    async fn add_known_address(
        &self,
        peer_id: PeerId,
        multiaddress: Multiaddr,
    ) -> Result<(), RequestError>;

    async fn register_new_file(&self, peer_id: PeerId, file_key: Key) -> Result<(), RequestError>;

    async fn unregister_file(&self, peer_id: PeerId, file_key: Key) -> Result<(), RequestError>;
}

impl FileTransferServiceInterface for ActorHandle<FileTransferService> {
    /// Request an upload of a file chunk to a peer.
    /// This returns after receiving a response from the network.
    async fn upload_request(
        &self,
        peer_id: PeerId,
        file_key: Key,
        chunk_with_proof: FileProof,
    ) -> Result<schema::v1::provider::RemoteUploadDataResponse, RequestError> {
        let (callback, file_transfer_rx) = tokio::sync::oneshot::channel();
        let command = FileTransferServiceCommand::UploadRequest {
            peer_id,
            file_key,
            chunk_with_proof,
            callback,
        };
        self.send(command).await;

        // First we wait for the response from the FileTransferService.
        // The response is another oneshot channel to wait for the response from the network.
        let network_rx = file_transfer_rx.await.expect("Failed to receive response from FileTransferService. Probably means FileTransferService has crashed.");

        // Now we wait on the actual response from the network.
        let response = network_rx.await.expect(
            "Failed to receive response from the NetworkService. Probably means the NetworkService has crashed.",
        );

        match response {
            Ok((data, _protocol_name)) => {
                let response = schema::v1::provider::Response::decode(&data[..]);
                match response {
                    Ok(response) => match response.response {
                        Some(
                            schema::v1::provider::response::Response::RemoteUploadDataResponse(
                                response,
                            ),
                        ) => Ok(response),
                        _ => Err(RequestError::UnexpectedResponse),
                    },
                    Err(error) => Err(RequestError::DecodeError(error)),
                }
            }
            Err(error) => Err(RequestError::RequestFailure(error)),
        }
    }

    /// Request a download of a file chunk to a peer.
    /// This returns after receiving a response from the network.
    async fn download_request(
        &self,
        peer_id: PeerId,
        file_key: Key,
        chunk_id: ChunkId,
    ) -> Result<schema::v1::provider::RemoteDownloadDataResponse, RequestError> {
        let (callback, file_transfer_rx) = tokio::sync::oneshot::channel();
        let command = FileTransferServiceCommand::DownloadRequest {
            peer_id,
            file_key,
            chunk_id,
            callback,
        };
        self.send(command).await;

        // First we wait for the response from the FileTransferService.
        // The response is another oneshot channel to wait for the response from the network.
        let network_rx = file_transfer_rx.await.expect("Failed to received response from FileTransferService. Probably means FileTransferService has crashed.");

        // Now we wait on the actual response from the network.
        let response = network_rx.await.expect(
            "Failed to receive response from the NetworkService. Probably means the NetworkService has crashed.",
        );

        match response {
            Ok((data, _protocol_name)) => {
                let response = schema::v1::provider::Response::decode(&data[..]);
                match response {
                    Ok(response) => match response.response {
                        Some(
                            schema::v1::provider::response::Response::RemoteDownloadDataResponse(
                                response,
                            ),
                        ) => Ok(response),
                        _ => Err(RequestError::UnexpectedResponse),
                    },
                    Err(error) => Err(RequestError::DecodeError(error)),
                }
            }
            Err(error) => Err(RequestError::RequestFailure(error)),
        }
    }

    /// Tell the FileTransferService to register a multiaddress as known for a specified PeerId.
    /// This returns as soon as the message has been dispatched (not processed) to the service.
    async fn add_known_address(
        &self,
        peer_id: PeerId,
        multiaddress: Multiaddr,
    ) -> Result<(), RequestError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let command = FileTransferServiceCommand::AddKnownAddress {
            peer_id,
            multiaddress,
            callback,
        };
        self.send(command).await;
        rx.await.expect("Failed to add known multiaddress to peer")
    }

    /// Tell the FileTransferService to start listening for new upload requests from peer_id
    /// on file file_key.
    /// This returns as soon as the message has been dispatched (not processed) to the service.
    async fn register_new_file(&self, peer_id: PeerId, file_key: Key) -> Result<(), RequestError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let command = FileTransferServiceCommand::RegisterNewFile {
            peer_id,
            file_key,
            callback,
        };
        self.send(command).await;
        rx.await.expect("Failed to register new file")
    }

    /// Tell the FileTransferService to no longer listen for upload requests from peer_id on file
    /// file_key.
    /// This returns as soon as the message has been dispatched (not processed) to the service.
    async fn unregister_file(&self, peer_id: PeerId, file_key: Key) -> Result<(), RequestError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let command = FileTransferServiceCommand::UnregisterFile {
            peer_id,
            file_key,
            callback,
        };
        self.send(command).await;
        rx.await.expect("Failed to unregister file")
    }
}
