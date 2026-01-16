//! HTTP server for receiving file chunks from trusted backends

use std::{panic::AssertUnwindSafe, sync::Arc};

use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use futures::FutureExt;
use sc_tracing::tracing::{error, info, warn};
use shc_actors_framework::actor::ActorHandle;
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface,
    types::{MspRespondStorageRequest, RespondStorageRequest},
    BlockchainService,
};
use shc_common::{traits::StorageEnableRuntime, types::ChunkId};
use shc_file_transfer_service::{
    commands::FileTransferServiceCommandInterface, FileTransferService,
};
use shc_forest_manager::traits::ForestStorageHandler;
use tokio::{
    net::TcpListener,
    sync::{mpsc, RwLock},
};
use tokio_stream::wrappers::ReceiverStream;

use crate::{trusted_file_transfer::files::process_chunk_stream, types::FileStorageT};

pub(crate) const LOG_TARGET: &str = "trusted-file-transfer-server";

/// Configuration for the trusted file transfer HTTP server
#[derive(Debug, Clone)]
pub struct Config {
    /// Host to bind the server to
    pub host: String,
    /// Port to bind the server to
    pub port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 7070,
        }
    }
}

/// Global context for the trusted file transfer server
pub struct Context<FL, FSH, Runtime>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    pub file_storage: Arc<RwLock<FL>>,
    pub blockchain: ActorHandle<BlockchainService<FSH, Runtime>>,
    pub file_transfer: ActorHandle<FileTransferService<Runtime>>,
}

impl<FL, FSH, Runtime> Clone for Context<FL, FSH, Runtime>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> Self {
        Self {
            file_storage: Arc::clone(&self.file_storage),
            blockchain: self.blockchain.clone(),
            file_transfer: self.file_transfer.clone(),
        }
    }
}

impl<FL, FSH, Runtime> Context<FL, FSH, Runtime>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    pub fn new(
        file_storage: Arc<RwLock<FL>>,
        blockchain: ActorHandle<BlockchainService<FSH, Runtime>>,
        file_transfer: ActorHandle<FileTransferService<Runtime>>,
    ) -> Self {
        Self {
            file_storage,
            blockchain,
            file_transfer,
        }
    }
}

/// Spawn the trusted file transfer HTTP server
pub async fn spawn_server<FL, FSH, Runtime>(
    config: Config,
    file_storage: Arc<RwLock<FL>>,
    blockchain: ActorHandle<BlockchainService<FSH, Runtime>>,
    file_transfer: ActorHandle<FileTransferService<Runtime>>,
) -> anyhow::Result<()>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    let context = Context::new(file_storage, blockchain, file_transfer);

    let app = Router::new()
        .route("/upload/{file_key}", post(upload_file))
        .route("/download/{file_key}", get(download_file))
        .route_layer(DefaultBodyLimit::disable())
        .with_state(context);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await.map_err(|e| {
        anyhow::anyhow!(
            "Failed to bind trusted file transfer server to {}: {}",
            addr,
            e
        )
    })?;

    info!(
        target: LOG_TARGET,
        host = %config.host,
        port = config.port,
        "👂 Trusted file transfer HTTP server listening"
    );

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            error!(
                target: LOG_TARGET,
                error = %e,
                "Trusted file transfer HTTP server error"
            );
        }
    });

    Ok(())
}

/// HTTP endpoint handler for receiving a file as chunks
///
/// The stream format is:
/// [ChunkId: 8 bytes (u64, little-endian)][Chunk data: FILE_CHUNK_SIZE bytes]...
/// [ChunkId: 8 bytes (u64, little-endian)][Chunk data: remaining bytes for last chunk]
async fn upload_file<FL, FSH, Runtime>(
    State(context): State<Context<FL, FSH, Runtime>>,
    Path(file_key): Path<String>,
    body: Body,
) -> Response
where
    FL: FileStorageT,
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    let result = AssertUnwindSafe(async { upload_file_inner(context, file_key, body).await })
        .catch_unwind()
        .await;

    match result {
        Ok(response) => response,
        Err(panic_info) => {
            let panic_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "Unknown panic".to_string()
            };

            error!(
                target: LOG_TARGET,
                panic = %panic_msg,
                "Panic caught in trusted file transfer handler"
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error: handler panicked".to_string(),
            )
                .into_response()
        }
    }
}

async fn upload_file_inner<FL, FSH, Runtime>(
    context: Context<FL, FSH, Runtime>,
    file_key: String,
    body: Body,
) -> Response
where
    FL: FileStorageT,
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    // Validate file_key is a hex string
    let key = file_key.trim_start_matches("0x");
    let file_key_bytes = match hex::decode(key) {
        Ok(bytes) => bytes,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Invalid file key hex encoding: {e}"),
            )
                .into_response();
        }
    };

    if file_key_bytes.len() != 32 {
        return (
            StatusCode::BAD_REQUEST,
            format!(
                "Invalid file key length. Expected 32 bytes, got {}",
                file_key_bytes.len()
            ),
        )
            .into_response();
    }

    // Convert file_key_bytes to H256
    let file_key_hash = sp_core::H256::from_slice(&file_key_bytes);

    // Process the streamed chunks
    let stream = body.into_data_stream();
    match process_chunk_stream(&context.file_storage, &file_key_hash, stream).await {
        Ok(_) => {
            if let Err(e) = handle_file_complete(&context, &file_key_hash).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error handling file completion: {}", e),
                )
                    .into_response();
            }
            (StatusCode::OK, ()).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error processing chunks: {}", e),
        )
            .into_response(),
    }
}

async fn handle_file_complete<FL, FSH, Runtime>(
    context: &Context<FL, FSH, Runtime>,
    file_key: &sp_core::H256,
) -> anyhow::Result<()>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    if let Err(e) = context
        .file_transfer
        .unregister_file((*file_key).into())
        .await
    {
        warn!(
            target: LOG_TARGET,
            file_key = %file_key,
            error = %e,
            "Failed to unregister file from file transfer service"
        );
    }

    context
        .blockchain
        .queue_msp_respond_storage_request(RespondStorageRequest::new(
            *file_key,
            MspRespondStorageRequest::Accept,
        ))
        .await;
    Ok(())
}

/// HTTP endpoint handler for downloading a file as chunks
///
/// The stream format is:
/// [ChunkId: 8 bytes (u64, little-endian)][Chunk data: FILE_CHUNK_SIZE bytes]...
/// [ChunkId: 8 bytes (u64, little-endian)][Chunk data: remaining bytes for last chunk]
async fn download_file<FL, FSH, Runtime>(
    State(context): State<Context<FL, FSH, Runtime>>,
    Path(file_key): Path<String>,
) -> Response
where
    FL: FileStorageT,
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    let result = AssertUnwindSafe(async { download_file_inner(context, file_key).await })
        .catch_unwind()
        .await;

    match result {
        Ok(response) => response,
        Err(panic_info) => {
            let panic_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "Unknown panic".to_string()
            };

            error!(
                target: LOG_TARGET,
                panic = %panic_msg,
                "Panic caught in trusted file transfer download handler"
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error: handler panicked".to_string(),
            )
                .into_response()
        }
    }
}

async fn download_file_inner<FL, FSH, Runtime>(
    context: Context<FL, FSH, Runtime>,
    file_key_str: String,
) -> Response
where
    FL: FileStorageT,
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    // Parse file key from hex string
    let file_key = match sp_core::H256::from_slice(&hex::decode(&file_key_str).unwrap_or_default())
    {
        key if key != sp_core::H256::zero() => key,
        _ => {
            warn!(
                target: LOG_TARGET,
                file_key = %file_key_str,
                "Invalid file key hex format"
            );
            return (
                StatusCode::BAD_REQUEST,
                format!("Invalid file key format: {}", file_key_str),
            )
                .into_response();
        }
    };

    info!(
        target: LOG_TARGET,
        file_key = %file_key,
        "Download request received for file"
    );

    // Get file metadata to determine chunk count
    let file_storage = context.file_storage.read().await;
    let metadata = match file_storage.get_metadata(&file_key) {
        Ok(Some(metadata)) => metadata,
        Ok(None) => {
            warn!(
                target: LOG_TARGET,
                file_key = %file_key,
                "File not found"
            );
            return (
                StatusCode::NOT_FOUND,
                format!("File not found: {:x}", file_key),
            )
                .into_response();
        }
        Err(e) => {
            error!(
                target: LOG_TARGET,
                file_key = %file_key,
                error = %e,
                "Failed to get file metadata"
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get file metadata: {}", e),
            )
                .into_response();
        }
    };

    let chunks_count = metadata.chunks_count();

    // Release the read lock before starting the streaming operation
    drop(file_storage);

    info!(
        target: LOG_TARGET,
        file_key = %file_key,
        chunks_count = chunks_count,
        "Streaming file chunks"
    );

    // Create a bounded channel for streaming chunks
    // Buffer size: 1024 chunks = ~1MB in memory
    // TODO: This logic is quite similar to save_file_to_disk. Maybe there's room
    // for removing duplication or replace the backend dowload flow with one
    // that uses this endpoint
    const QUEUE_BUFFER_SIZE: usize = 1024;
    let (tx, rx) = mpsc::channel::<Result<bytes::Bytes, std::io::Error>>(QUEUE_BUFFER_SIZE);

    let file_storage_arc = context.file_storage.clone();
    let batch_size = QUEUE_BUFFER_SIZE as u64;

    // Spawn producer task to read chunks from storage and send through channel
    tokio::spawn(async move {
        let mut current_chunk: u64 = 0;

        while current_chunk < chunks_count {
            let batch_end = std::cmp::min(chunks_count, current_chunk.saturating_add(batch_size));

            // Read a batch of chunks under a single read lock
            let mut batch = Vec::with_capacity((batch_end - current_chunk) as usize);
            {
                let read_storage = file_storage_arc.read().await;
                for chunk_idx in current_chunk..batch_end {
                    let chunk_id = ChunkId::new(chunk_idx);

                    match read_storage.get_chunk(&file_key, &chunk_id) {
                        Ok(chunk_data) => {
                            // Encode chunk ID as little-endian u64
                            let chunk_id_bytes = chunk_idx.to_le_bytes();
                            batch.push((chunk_id_bytes, chunk_data));
                        }
                        Err(e) => {
                            error!(
                                target: LOG_TARGET,
                                file_key = %file_key,
                                chunk_id = chunk_idx,
                                error = %e,
                                "Failed to get chunk"
                            );
                            // Send error and stop producing
                            let _ = tx
                                .send(Err(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    format!("Error reading chunk {}: {:?}", chunk_idx, e),
                                )))
                                .await;
                            return;
                        }
                    }
                }
            } // Read lock released here

            // Send the batch, backpressure ensured by bounded channel
            for (chunk_id_bytes, chunk_data) in batch {
                // Send chunk ID bytes
                if tx
                    .send(Ok(bytes::Bytes::from(chunk_id_bytes.to_vec())))
                    .await
                    .is_err()
                {
                    // Consumer dropped (client disconnected) - stop producing
                    return;
                }

                // Send chunk data bytes
                if tx.send(Ok(bytes::Bytes::from(chunk_data))).await.is_err() {
                    // Consumer dropped (client disconnected) - stop producing
                    return;
                }
            }

            current_chunk = batch_end;
        }
    });

    // Convert channel receiver to stream and wrap in response body
    let stream = ReceiverStream::new(rx);
    let body = Body::from_stream(stream);

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/octet-stream")
        .body(body)
        .unwrap()
}
