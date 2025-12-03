//! Trusted File Transfer Server
//!
//! HTTP server to receive streamed file chunks from trusted backends.
//! This server accepts POST requests with streamed chunks in the format:
//! [ChunkId: 8 bytes (u64, little-endian)][Chunk data: FILE_CHUNK_SIZE bytes]...
//! [ChunkId: 8 bytes (u64, little-endian)][Chunk data: remaining bytes for last chunk]
//! Note: All chunks are FILE_CHUNK_SIZE except the last one which may be smaller

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use sc_tracing::tracing::{info, warn};
use shc_actors_framework::actor::ActorHandle;
use shc_blockchain_service::commands::BlockchainServiceCommandInterface;
use shc_blockchain_service::types::{MspRespondStorageRequest, RespondStorageRequest};
use shc_blockchain_service::BlockchainService;
use shc_common::traits::StorageEnableRuntime;
use shc_common::types::{ChunkId, FILE_CHUNK_SIZE};
use shc_file_manager::traits::FileStorageWriteOutcome;
use shc_file_transfer_service::commands::FileTransferServiceCommandInterface;
use shc_file_transfer_service::FileTransferService;
use shc_forest_manager::traits::ForestStorageHandler;
use shp_file_metadata::Chunk;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;

use crate::types::FileStorageT;

const LOG_TARGET: &str = "trusted-file-transfer-server";

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
        "Trusted file transfer HTTP server listening"
    );

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            warn!(
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
) -> impl IntoResponse
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
    match process_chunk_stream(&context, &file_key_hash, body).await {
        Ok(_) => (StatusCode::OK, ()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error processing chunks: {}", e),
        )
            .into_response(),
    }
}

/// Process a stream of chunks from the request body
async fn process_chunk_stream<FL, FSH, Runtime>(
    context: &Context<FL, FSH, Runtime>,
    file_key: &sp_core::H256,
    request_body: Body,
) -> anyhow::Result<()>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    let mut request_stream = request_body.into_data_stream();
    let mut buffer = Vec::new();
    let mut last_write_outcome = FileStorageWriteOutcome::FileIncomplete;

    const CHUNK_ID_SIZE: usize = 8; // u64
    let file_chunk_size = FILE_CHUNK_SIZE as usize;

    // Process request stream, storing chunks as they are received
    while let Some(try_bytes) = request_stream.next().await {
        let bytes = try_bytes?;
        buffer.extend_from_slice(&bytes);

        while buffer.len() >= CHUNK_ID_SIZE + file_chunk_size {
            let (chunk_id, chunk_data) = get_next_chunk(&mut buffer, true)?;
            last_write_outcome = write_chunk(context, file_key, &chunk_id, &chunk_data).await?;
        }
    }

    // Check if there's remaining data for the last chunk (This is the case when data is
    // smaller than FILE_CHUNK_SIZE)
    if !buffer.is_empty() {
        let (chunk_id, chunk_data) = get_next_chunk(&mut buffer, false)?;
        last_write_outcome = write_chunk(context, file_key, &chunk_id, &chunk_data).await?;
    }

    // Verify the file is complete using the last write outcome
    if matches!(last_write_outcome, FileStorageWriteOutcome::FileComplete) {
        handle_file_complete(context, file_key).await?;
    } else {
        return Err(anyhow::anyhow!(
            "File incomplete after processing all chunks"
        ));
    }

    Ok(())
}

/// Gets next chunk from buffer. If cap_at_file_chunk_size is set to true,
/// it will get FILE_CHUNK_SIZE as data size, else it will use the remainder
/// of the buffer (used for last chunk when data is not a multiple of FILE_CHUNK_SIZE).
fn get_next_chunk(
    buffer: &mut Vec<u8>,
    cap_at_file_chunk_size: bool,
) -> anyhow::Result<(ChunkId, Vec<u8>)> {
    const CHUNK_ID_SIZE: usize = 8;
    let min_data_size: usize = if cap_at_file_chunk_size {
        FILE_CHUNK_SIZE as usize
    } else {
        1
    };
    let min_buffer_size = CHUNK_ID_SIZE + min_data_size;
    if buffer.len() < min_buffer_size {
        return Err(anyhow::anyhow!(
            "Not enough bytes to extract chunk from buffer. Required at least {} bytes got {}.",
            min_buffer_size,
            buffer.len()
        ));
    }

    let chunk_id_bytes: [u8; CHUNK_ID_SIZE] = buffer
        .drain(..CHUNK_ID_SIZE)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| anyhow::anyhow!("Failed to parse chunk ID"))?;
    let chunk_id_value = u64::from_le_bytes(chunk_id_bytes);
    let chunk_id = ChunkId::new(chunk_id_value);

    let chunk_data = if cap_at_file_chunk_size {
        let size = FILE_CHUNK_SIZE as usize;
        buffer.drain(..size).collect()
    } else {
        std::mem::take(buffer)
    };
    Ok((chunk_id, chunk_data))
}

async fn write_chunk<FL, FSH, Runtime>(
    context: &Context<FL, FSH, Runtime>,
    file_key: &sp_core::H256,
    chunk_id: &ChunkId,
    chunk_data: &Chunk,
) -> anyhow::Result<FileStorageWriteOutcome>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    let mut file_storage = context.file_storage.write().await;
    file_storage
        .write_chunk(file_key, chunk_id, chunk_data)
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to write chunk {} to storage: {}",
                chunk_id.as_u64(),
                e
            )
        })
}

/// Handle file completion: unregister from file transfer service and queue blockchain transaction
async fn handle_file_complete<FL, FSH, Runtime>(
    context: &Context<FL, FSH, Runtime>,
    file_key: &sp_core::H256,
) -> anyhow::Result<()>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    info!(
        target: LOG_TARGET,
        file_key = %file_key,
        "File upload complete"
    );

    // Unregister the file from the file transfer service
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

    // Queue a request to confirm the storing of the file
    context
        .blockchain
        .queue_msp_respond_storage_request(RespondStorageRequest::new(
            *file_key,
            MspRespondStorageRequest::Accept,
        ))
        .await?;
    Ok(())
}
