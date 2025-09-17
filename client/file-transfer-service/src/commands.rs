use anyhow::Result;
use async_trait::async_trait;
use prost::Message;
use std::collections::HashSet;
use thiserror::Error;

use sc_network::{config::OutgoingResponse, Multiaddr, PeerId, ProtocolName, RequestFailure};
use sc_tracing::tracing::error;

use shc_actors_derive::actor_command;
use shc_actors_framework::actor::ActorHandle;
use shc_common::{
    traits::StorageEnableRuntime,
    types::{BucketId, ChunkId, DownloadRequestId, FileKey, FileKeyProof, UploadRequestId},
};

use super::{schema, FileTransferService};

const LOG_TARGET: &str = "file-transfer-service";

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

/// Messages understood by the FileTransfer service actor
#[actor_command(
    service = FileTransferService<Runtime: StorageEnableRuntime>,
    default_mode = "ImmediateResponse",
    default_error_type = RequestError,
)]
pub enum FileTransferServiceCommand<Runtime: StorageEnableRuntime> {
    #[command(
        mode = "AsyncResponse", 
        success_type = (Vec<u8>, ProtocolName),
        inner_channel_type = futures::channel::oneshot::Receiver,
        error_type = RequestFailure
    )]
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
        bucket_id: Option<BucketId<Runtime>>,
    },
    #[command(
        mode = "AsyncResponse", 
        success_type = (Vec<u8>, ProtocolName),
        inner_channel_type = futures::channel::oneshot::Receiver,
        error_type = RequestFailure
    )]
    ReceiveBackendFileChunksRequest {
        /// File key of the file we are receiving via backend.
        file_key: FileKey,
        /// File key proof to be processed locally. This contains 1 or more chunks of the file
        /// and the Merkle proof of them.
        file_key_proof: FileKeyProof,
    },
    UploadResponse {
        /// The request ID used to send back the response through the FileTransferService
        request_id: UploadRequestId,
        /// Whether the file is complete
        file_complete: bool,
    },
    #[command(
        mode = "AsyncResponse", 
        success_type = (Vec<u8>, ProtocolName),
        error_type = RequestFailure,
        inner_channel_type = futures::channel::oneshot::Receiver
    )]
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
        bucket_id: Option<BucketId<Runtime>>,
    },
    DownloadResponse {
        request_id: DownloadRequestId,
        file_key_proof: FileKeyProof,
    },
    AddKnownAddress {
        peer_id: PeerId,
        multiaddress: Multiaddr,
    },
    RegisterNewFile {
        peer_id: PeerId,
        file_key: FileKey,
    },
    UnregisterFile {
        file_key: FileKey,
    },
    #[command(
        mode = "ImmediateResponse",
        success_type = bool
    )]
    /// Query whether this node is currently expecting to receive the given file key.
    ///
    /// Returns true if the file key has been registered (i.e., is allowed from at least one peer).
    IsFileExpected {
        /// File key to check for expectation/registration.
        file_key: FileKey,
    },
    RegisterNewBucketPeer {
        peer_id: PeerId,
        bucket_id: BucketId<Runtime>,
    },
    ScheduleUnregisterBucket {
        bucket_id: BucketId<Runtime>,
        grace_period_seconds: Option<u64>,
    },
}

#[async_trait]
pub trait FileTransferServiceCommandInterfaceExt {
    fn parse_remote_upload_data_response(
        &self,
        data: &Vec<u8>,
    ) -> Result<schema::v1::provider::RemoteUploadDataResponse, RequestError>;

    fn parse_remote_download_data_response(
        &self,
        data: &Vec<u8>,
    ) -> Result<schema::v1::provider::RemoteDownloadDataResponse, RequestError>;

    async fn extract_peer_ids_and_register_known_addresses(
        &self,
        multiaddresses: Vec<Multiaddr>,
    ) -> Vec<PeerId>;
}

#[async_trait]
impl<Runtime: StorageEnableRuntime> FileTransferServiceCommandInterfaceExt
    for ActorHandle<FileTransferService<Runtime>>
{
    fn parse_remote_upload_data_response(
        &self,
        data: &Vec<u8>,
    ) -> Result<schema::v1::provider::RemoteUploadDataResponse, RequestError> {
        let response = schema::v1::provider::Response::decode(&data[..]);
        match response {
            Ok(response) => match response.response {
                Some(schema::v1::provider::response::Response::RemoteUploadDataResponse(
                    response,
                )) => Ok(response),
                _ => Err(RequestError::UnexpectedResponse),
            },
            Err(e) => Err(RequestError::DecodeError(e)),
        }
    }

    fn parse_remote_download_data_response(
        &self,
        data: &Vec<u8>,
    ) -> Result<schema::v1::provider::RemoteDownloadDataResponse, RequestError> {
        let response = schema::v1::provider::Response::decode(&data[..]);
        match response {
            Ok(response) => match response.response {
                Some(schema::v1::provider::response::Response::RemoteDownloadDataResponse(
                    response,
                )) => Ok(response),
                _ => Err(RequestError::UnexpectedResponse),
            },
            Err(e) => Err(RequestError::DecodeError(e)),
        }
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
