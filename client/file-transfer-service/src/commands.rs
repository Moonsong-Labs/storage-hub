use anyhow::Result;
use async_trait::async_trait;
use prost::Message;
use thiserror::Error;

use sc_network::{config::OutgoingResponse, Multiaddr, PeerId, ProtocolName, RequestFailure};
use sc_tracing::tracing::error;

use super::{schema, FileTransferService};
use codec::Encode;
use shc_actors_framework::actor::ActorHandle;
use shc_common::types::{BucketId, ChunkId, DownloadRequestId, FileKey, FileKeyProof};

const LOG_TARGET: &str = "file-transfer-service";

/// Messages understood by the FileTransfer service actor
pub enum FileTransferServiceCommand {
    UploadRequest {
        peer_id: PeerId,
        file_key: FileKey,
        file_key_proof: FileKeyProof,
        /// Bucket ID is only required for Bucket operations.
        /// Since the FileTransferService is not aware of which files are in which buckets,
        /// it needs to be provided by the caller to pass the allow list check.
        /// Note: The task that handles the event is responsible for checking if the file is
        /// part of the specified bucket.
        bucket_id: Option<BucketId>,
        callback: tokio::sync::oneshot::Sender<
            futures::channel::oneshot::Receiver<Result<(Vec<u8>, ProtocolName), RequestFailure>>,
        >,
    },
    DownloadRequest {
        peer_id: PeerId,
        file_key: FileKey,
        chunk_id: ChunkId,
        /// Bucket ID is only required for Bucket operations.
        /// Since the FileTransferService is not aware of which files are in which buckets,
        /// it needs to be provided by the caller to pass the allow list check.
        /// Note: The task that handles the event is responsible for checking if the file is
        /// part of the specified bucket.
        bucket_id: Option<BucketId>,
        callback: tokio::sync::oneshot::Sender<
            futures::channel::oneshot::Receiver<Result<(Vec<u8>, ProtocolName), RequestFailure>>,
        >,
    },
    DownloadResponse {
        request_id: DownloadRequestId,
        file_key_proof: FileKeyProof,
        callback: tokio::sync::oneshot::Sender<Result<(), RequestError>>,
    },
    AddKnownAddress {
        peer_id: PeerId,
        multiaddress: Multiaddr,
        callback: tokio::sync::oneshot::Sender<Result<(), RequestError>>,
    },
    RegisterNewFile {
        peer_id: PeerId,
        file_key: FileKey,
        callback: tokio::sync::oneshot::Sender<Result<(), RequestError>>,
    },
    UnregisterFile {
        file_key: FileKey,
        callback: tokio::sync::oneshot::Sender<Result<(), RequestError>>,
    },
    RegisterNewBucketPeer {
        peer_id: PeerId,
        bucket_id: BucketId,
        callback: tokio::sync::oneshot::Sender<Result<(), RequestError>>,
    },
    UnregisterBucket {
        bucket_id: BucketId,
        grace_period_seconds: Option<u64>,
        callback: tokio::sync::oneshot::Sender<Result<(), RequestError>>,
    },
}

#[derive(Debug, Error)]
pub enum RequestError {
    /// The request failed. More details are provided in the `RequestFailure`.
    #[error("Request failed: {0}")]
    RequestFailure(#[from] RequestFailure),
    /// The response was not a valid protobuf message.
    #[error("Failed to decode response: {0}")]
    DecodeError(prost::DecodeError),
    /// The response was decoded successfully, but it was not the expected response.
    #[error("Unexpected response")]
    UnexpectedResponse,
    /// File is already stored in for this Peer in the registry.
    #[error("File already registered for peer")]
    FileAlreadyRegisteredForPeer,
    /// File not found in the registry.
    #[error("File not found in registry")]
    FileNotRegistered,
    /// Download request id was not found in internal mapping
    #[error("DownloadRequestId not found in internal mapping")]
    DownloadRequestIdNotFound,
    /// Failed to return response from Download request
    #[error("Failed to return download response: {0:?}")]
    DownloadResponseFailure(OutgoingResponse),
    /// Bucket already registered for peer
    #[error("Bucket already registered for peer")]
    BucketAlreadyRegisteredForPeer,
    /// Bucket not registered for peer
    #[error("Bucket not registered for peer")]
    BucketNotRegisteredForPeer,
}

/// Allows our ActorHandle to implement
/// the specific methods for each kind of message.
#[async_trait]
pub trait FileTransferServiceInterface {
    async fn upload_request(
        &self,
        peer_id: PeerId,
        file_key: FileKey,
        file_key_proof: FileKeyProof,
        bucket_id: Option<BucketId>,
    ) -> Result<schema::v1::provider::RemoteUploadDataResponse, RequestError>;

    async fn download_request(
        &self,
        peer_id: PeerId,
        file_key: FileKey,
        chunk_id: ChunkId,
        bucket_id: Option<BucketId>,
    ) -> Result<schema::v1::provider::RemoteDownloadDataResponse, RequestError>;

    async fn download_response(
        &self,
        file_key_proof: FileKeyProof,
        request_id: DownloadRequestId,
    ) -> Result<schema::v1::provider::RemoteDownloadDataResponse, RequestError>;

    async fn add_known_address(
        &self,
        peer_id: PeerId,
        multiaddress: Multiaddr,
    ) -> Result<(), RequestError>;

    async fn register_new_file_peer(
        &self,
        peer_id: PeerId,
        file_key: FileKey,
    ) -> Result<(), RequestError>;

    async fn unregister_file(&self, file_key: FileKey) -> Result<(), RequestError>;

    async fn register_new_bucket_peer(
        &self,
        peer_id: PeerId,
        bucket_id: BucketId,
    ) -> Result<(), RequestError>;

    async fn unregister_bucket(&self, bucket_id: BucketId) -> Result<(), RequestError>;

    async fn unregister_bucket_with_grace_period(
        &self,
        bucket_id: BucketId,
        grace_period_seconds: u64,
    ) -> Result<(), RequestError>;

    async fn extract_peer_ids_and_register_known_addresses(
        &mut self,
        multiaddresses: Vec<Multiaddr>,
    ) -> Vec<PeerId>;
}

#[async_trait]
impl FileTransferServiceInterface for ActorHandle<FileTransferService> {
    /// Request an upload of a file chunk to a peer.
    /// This returns after receiving a response from the network.
    async fn upload_request(
        &self,
        peer_id: PeerId,
        file_key: FileKey,
        file_key_proof: FileKeyProof,
        bucket_id: Option<BucketId>,
    ) -> Result<schema::v1::provider::RemoteUploadDataResponse, RequestError> {
        let (callback, file_transfer_rx) = tokio::sync::oneshot::channel();
        let command = FileTransferServiceCommand::UploadRequest {
            peer_id,
            file_key,
            file_key_proof,
            bucket_id,
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
        file_key: FileKey,
        chunk_id: ChunkId,
        bucket_id: Option<BucketId>,
    ) -> Result<schema::v1::provider::RemoteDownloadDataResponse, RequestError> {
        let (callback, file_transfer_rx) = tokio::sync::oneshot::channel();
        let command = FileTransferServiceCommand::DownloadRequest {
            peer_id,
            file_key,
            chunk_id,
            bucket_id,
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

    /// Responds a download request of a file chunk with a [`FileKeyProof`]
    async fn download_response(
        &self,
        file_key_proof: FileKeyProof,
        request_id: DownloadRequestId,
    ) -> Result<schema::v1::provider::RemoteDownloadDataResponse, RequestError> {
        let (callback, file_transfer_rx) = tokio::sync::oneshot::channel();

        let command = FileTransferServiceCommand::DownloadResponse {
            request_id,
            file_key_proof: file_key_proof.clone(),
            callback,
        };

        self.send(command).await;

        let result = file_transfer_rx.await.expect("Failed to received response from FileTransferService. Probably means FileTransferService has crashed.");

        match result {
            Ok(()) => {
                let response = schema::v1::provider::RemoteDownloadDataResponse {
                    file_key_proof: file_key_proof.encode(),
                };

                Ok(response)
            }
            Err(e) => Err(e),
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
    async fn register_new_file_peer(
        &self,
        peer_id: PeerId,
        file_key: FileKey,
    ) -> Result<(), RequestError> {
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
    async fn unregister_file(&self, file_key: FileKey) -> Result<(), RequestError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let command = FileTransferServiceCommand::UnregisterFile { file_key, callback };
        self.send(command).await;
        rx.await.expect("Failed to unregister file")
    }

    /// Tell the FileTransferService to start listening for new upload requests from peer_id
    /// on bucket bucket_id.
    /// This returns as soon as the message has been dispatched (not processed) to the service.
    async fn register_new_bucket_peer(
        &self,
        peer_id: PeerId,
        bucket_id: BucketId,
    ) -> Result<(), RequestError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let command = FileTransferServiceCommand::RegisterNewBucketPeer {
            peer_id,
            bucket_id,
            callback,
        };
        self.send(command).await;
        rx.await.expect("Failed to register new bucket peer")
    }

    async fn unregister_bucket_with_grace_period(
        &self,
        bucket_id: BucketId,
        grace_period_seconds: u64,
    ) -> Result<(), RequestError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let command = FileTransferServiceCommand::UnregisterBucket {
            bucket_id,
            grace_period_seconds: Some(grace_period_seconds),
            callback,
        };
        self.send(command).await;
        rx.await.expect("Failed to unregister bucket")
    }

    /// Tell the FileTransferService to no longer listen for upload requests from peer_id
    /// on bucket bucket_id.
    /// This returns as soon as the message has been dispatched (not processed) to the service.
    async fn unregister_bucket(&self, bucket_id: BucketId) -> Result<(), RequestError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let command = FileTransferServiceCommand::UnregisterBucket {
            bucket_id,
            grace_period_seconds: None,
            callback,
        };
        self.send(command).await;
        rx.await.expect("Failed to unregister bucket")
    }

    /// Helper function to register known addresses and extract their peer ids.
    async fn extract_peer_ids_and_register_known_addresses(
        &mut self,
        multiaddresses: Vec<Multiaddr>,
    ) -> Vec<PeerId> {
        let mut peer_ids = Vec::new();
        for multiaddress in &multiaddresses {
            if let Some(peer_id) = PeerId::try_from_multiaddr(&multiaddress) {
                if let Err(error) = self.add_known_address(peer_id, multiaddress.clone()).await {
                    error!(
                        target: LOG_TARGET,
                        "Failed to add known address {:?} for peer {:?} due to {:?}",
                        multiaddress,
                        peer_id,
                        error
                    );
                }
                peer_ids.push(peer_id);
            }
        }
        peer_ids
    }
}
