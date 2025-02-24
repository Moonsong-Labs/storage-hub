use anyhow::Result;
use async_trait::async_trait;
use prost::Message;
use std::collections::HashSet;
use thiserror::Error;

use codec::Encode;
use sc_network::{config::OutgoingResponse, Multiaddr, PeerId, ProtocolName, RequestFailure};
use sc_tracing::tracing::error;

use shc_actors_framework::actor::ActorHandle;
use shc_common::types::{
    BucketId, ChunkId, DownloadRequestId, FileKey, FileKeyProof, UploadRequestId,
};

use super::{schema, FileTransferService};

const LOG_TARGET: &str = "file-transfer-service";

/// Messages understood by the FileTransfer service actor
pub enum FileTransferServiceCommand {
    UploadRequest {
        /// Peer ID to upload the file to. This Peer ID must be registered as a known address
        /// before the upload request can be made.
        peer_id: PeerId,
        /// File key of the file we are uploading.
        file_key: FileKey,
        /// File key proof of the file we are uploading. This contains 1 or more chunks of the file
        /// and the Merkle proof of them.
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
    UploadResponse {
        /// The request ID used to send back the response through the FileTransferService
        request_id: UploadRequestId,
        /// Whether the file is complete
        file_complete: bool,
        /// The request ID used to send back the response through the FileTransferService
        callback: tokio::sync::oneshot::Sender<Result<(), RequestError>>,
    },
    DownloadRequest {
        /// Peer ID to download the file from. This Peer ID must be registered as a known address
        /// before the download request can be made.
        peer_id: PeerId,
        /// File key of the file to download.
        file_key: FileKey,
        /// A set of chunk IDs for batched download requests.
        chunk_ids: HashSet<ChunkId>,
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
    ScheduleUnregisterBucket {
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
    /// Upload request id was not found in internal mapping
    #[error("UploadRequestId not found in internal mapping")]
    UploadRequestIdNotFound,
    /// Failed to return response from Download request
    #[error("Failed to return download response: {0:?}")]
    DownloadResponseFailure(OutgoingResponse),
    /// Failed to return response from Upload request
    #[error("Failed to return upload response: {0:?}")]
    UploadResponseFailure(OutgoingResponse),
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

    async fn upload_response(
        &self,
        file_complete: bool,
        request_id: UploadRequestId,
    ) -> Result<(), RequestError>;

    async fn download_request(
        &self,
        peer_id: PeerId,
        file_key: FileKey,
        chunk_ids: std::collections::HashSet<ChunkId>,
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

    async fn schedule_unregister_bucket(
        &self,
        bucket_id: BucketId,
        grace_period_seconds: Option<u64>,
    ) -> Result<(), RequestError>;

    async fn extract_peer_ids_and_register_known_addresses(
        &self,
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

    /// Respond to an upload request with the file completion status.
    /// This returns after the message has been processed by the service.
    async fn upload_response(
        &self,
        file_complete: bool,
        request_id: UploadRequestId,
    ) -> Result<(), RequestError> {
        let (callback, rx) = tokio::sync::oneshot::channel();

        let command = FileTransferServiceCommand::UploadResponse {
            request_id,
            file_complete,
            callback,
        };

        self.send(command).await;

        rx.await
            .expect("Failed to receive response from FileTransferService")
    }

    /// Request to download a batch of file chunks from a peer.
    /// This returns after receiving and processing the network response.
    async fn download_request(
        &self,
        peer_id: PeerId,
        file_key: FileKey,
        chunk_ids: std::collections::HashSet<ChunkId>,
        bucket_id: Option<BucketId>,
    ) -> Result<schema::v1::provider::RemoteDownloadDataResponse, RequestError> {
        let (callback, file_transfer_rx) = tokio::sync::oneshot::channel();
        let command = FileTransferServiceCommand::DownloadRequest {
            peer_id,
            file_key,
            chunk_ids,
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

    /// Respond to a download request of a file chunk with a [`FileKeyProof`].
    /// This returns after the message has been processed by the service.
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

    /// Tell the FileTransferService to register a multiaddress as known for a specified [`PeerId`].
    /// This returns after the message has been processed by the service.
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

    /// Tell the FileTransferService to start listening for new upload requests from [`peer_id`]
    /// on file [`file_key`].
    /// This returns after the message has been processed by the service.
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

    /// Tell the FileTransferService to no longer listen for upload requests from [`peer_id`] on
    /// file [`file_key`].
    /// This returns as soon as the message has been dispatched (not processed) to the service.
    async fn unregister_file(&self, file_key: FileKey) -> Result<(), RequestError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let command = FileTransferServiceCommand::UnregisterFile { file_key, callback };
        self.send(command).await;
        rx.await.expect("Failed to unregister file")
    }

    /// Tell the FileTransferService to start listening for new upload requests from [`peer_id`]
    /// on Bucket [`bucket_id`].
    /// This returns after the message has been processed by the service.
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

    /// Same as [`unregister_bucket`] but the unregistering is delayed for a specified amount of
    /// time.
    /// This returns after the message has been processed by the service.
    async fn schedule_unregister_bucket(
        &self,
        bucket_id: BucketId,
        grace_period_seconds: Option<u64>,
    ) -> Result<(), RequestError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let command = FileTransferServiceCommand::ScheduleUnregisterBucket {
            bucket_id,
            grace_period_seconds,
            callback,
        };
        self.send(command).await;
        rx.await.expect("Failed to schedule unregister bucket")
    }

    /// Helper function to register known addresses and extract their peer ids.
    async fn extract_peer_ids_and_register_known_addresses(
        &self,
        multiaddresses: Vec<Multiaddr>,
    ) -> Vec<PeerId> {
        let mut peer_ids = Vec::new();
        for multiaddress in multiaddresses {
            if let Some(peer_id) = PeerId::try_from_multiaddr(&multiaddress) {
                if let Err(e) = self.add_known_address(peer_id, multiaddress.clone()).await {
                    error!(
                        target: LOG_TARGET,
                        "Failed to add known address {:?} for peer {:?}: {:?}",
                        multiaddress,
                        peer_id,
                        e
                    );
                }
                peer_ids.push(peer_id);
            }
        }
        peer_ids
    }
}
