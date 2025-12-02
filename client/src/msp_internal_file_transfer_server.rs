//! MSP Internal File Transfer Server
//!
//! HTTP server for MSP to receive streamed file chunks from backend.
//! This server accepts POST requests with streamed chunks in the format:
//! [Total Chunks: 8 bytes (u64, little-endian)]
//! [ChunkId: 8 bytes (u64, little-endian)][Chunk data: FILE_CHUNK_SIZE bytes]...
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
use sc_tracing::tracing::{debug, error, info, warn};
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

const LOG_TARGET: &str = "msp-internal-file-transfer-server";

/// Configuration for the MSP internal file transfer HTTP server
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

/// Global context for the MSP internal file transfer server
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

/// Spawn the MSP internal file transfer HTTP server
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
            "Failed to bind MSP internal file transfer server to {}: {}",
            addr,
            e
        )
    })?;

    info!(
        target: LOG_TARGET,
        host = %config.host,
        port = config.port,
        "MSP internal file transfer HTTP server listening"
    );

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            warn!(
                target: LOG_TARGET,
                error = %e,
                "MSP internal file transfer HTTP server error"
            );
        }
    });

    Ok(())
}

/// HTTP endpoint handler for receiving a file as chunks
///
/// The stream format is:
/// [ChunkId: 8 bytes (u64, little-endian)][Chunk length: 4 bytes (u32, little-endian)][Chunk data: variable]
///
/// This handler processes chunks as they arrive without loading the entire stream into memory.
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
    debug!(
        target: LOG_TARGET,
        file_key = %file_key,
        "Received upload file request"
    );

    // Validate file_key is a hex string
    let key = file_key.trim_start_matches("0x");
    let file_key_bytes = match hex::decode(key) {
        Ok(bytes) => bytes,
        Err(e) => {
            warn!(
                target: LOG_TARGET,
                file_key = %file_key,
                error = %e,
                "Invalid file key hex encoding"
            );
            return (StatusCode::BAD_REQUEST, "Invalid file key hex encoding").into_response();
        }
    };

    if file_key_bytes.len() != 32 {
        warn!(
            target: LOG_TARGET,
            file_key = %file_key,
            length = file_key_bytes.len(),
            "Invalid file key length"
        );
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
        Ok(_) => {
            debug!(
                target: LOG_TARGET,
                file_key = %file_key,
                "Successfully processed chunk stream"
            );
            (StatusCode::OK, ()).into_response()
        }
        Err(e) => {
            error!(
                target: LOG_TARGET,
                file_key = %file_key,
                error = %e,
                "Error processing chunk stream"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error processing chunks: {}", e),
            )
                .into_response()
        }
    }
}

/// Process a stream of chunks from the backend
///
/// Binary format: [Total Chunks: 8 bytes][ChunkId: 8 bytes][Data: FILE_CHUNK_SIZE]...
/// Note: All chunks are FILE_CHUNK_SIZE except the last one which may be smaller
async fn process_chunk_stream<FL, FSH, Runtime>(
    context: &Context<FL, FSH, Runtime>,
    file_key: &sp_core::H256,
    body: Body,
) -> anyhow::Result<()>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    let mut stream = body.into_data_stream();
    let mut buffer = Vec::new();
    let mut chunks_processed = 0u64;
    let mut maybe_total_chunks: Option<u64> = None;

    // Constants for parsing the binary format
    const TOTAL_CHUNKS_SIZE: usize = 8; // u64
    const CHUNK_ID_SIZE: usize = 8; // u64

    let file_chunk_size = FILE_CHUNK_SIZE as usize;

    while let Some(try_bytes) = stream.next().await {
        let bytes = try_bytes?;
        buffer.extend_from_slice(&bytes);

        // First, read the total chunks count if we haven't yet
        if maybe_total_chunks.is_none() {
            if buffer.len() >= TOTAL_CHUNKS_SIZE {
                let total_bytes: [u8; TOTAL_CHUNKS_SIZE] = buffer[0..TOTAL_CHUNKS_SIZE]
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Failed to parse total chunks count"))?;
                let total_chunks = u64::from_le_bytes(total_bytes);
                maybe_total_chunks = Some(total_chunks);
                buffer.drain(..TOTAL_CHUNKS_SIZE);

                debug!(
                    target: LOG_TARGET,
                    file_key = %file_key,
                    total_chunks = total_chunks,
                    "Received total chunks count"
                );
            } else {
                continue;
            }
        }

        // Process chunks from the buffer (all except last chunk)
        if let Some(total_chunks) = maybe_total_chunks {
            while chunks_processed < total_chunks - 1
                && buffer.len() >= CHUNK_ID_SIZE + file_chunk_size
            {
                let (chunk_id, chunk_data) = get_next_chunk(&mut buffer, Some(file_chunk_size))?;

                // For non-last chunks, just verify write succeeded (don't check FileComplete)
                write_chunk(context, file_key, &chunk_id, &chunk_data).await?;
                chunks_processed += 1;
            }
        }
    }

    // After stream ends, process the last chunk
    let total_chunks =
        maybe_total_chunks.ok_or(anyhow::anyhow!("Could not parse file total from stream"))?;
    if chunks_processed != total_chunks - 1 {
        return Err(anyhow::anyhow!(
            "Wrong processing of chunks. Expected {}, processed {}",
            total_chunks - 1,
            chunks_processed
        ));
    }
    let (chunk_id, chunk_data) = get_next_chunk(&mut buffer, None)?;
    let write_last_chunk_outcome = write_chunk(context, file_key, &chunk_id, &chunk_data).await?;

    if matches!(
        write_last_chunk_outcome,
        FileStorageWriteOutcome::FileComplete
    ) {
        handle_file_complete(context, file_key).await?;
    } else {
        return Err(anyhow::anyhow!(
            "File incomplete at the end of last chunk write"
        ));
    }
    Ok(())
}

/// Extract chunk from a buffer and write to storage
/// If no chunk data size is given it will consume the
/// whole buffer as its data
fn get_next_chunk(
    buffer: &mut Vec<u8>,
    maybe_file_chunk_size: Option<usize>,
) -> anyhow::Result<(ChunkId, Vec<u8>)> {
    const CHUNK_ID_SIZE: usize = 8;

    let chunk_id_bytes: [u8; CHUNK_ID_SIZE] = buffer
        .drain(..CHUNK_ID_SIZE)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| anyhow::anyhow!("Failed to parse chunk ID"))?;
    let chunk_id_value = u64::from_le_bytes(chunk_id_bytes);
    let chunk_id = ChunkId::new(chunk_id_value);

    let chunk_data = match maybe_file_chunk_size {
        Some(size) => buffer.drain(..size).collect(),
        None => std::mem::take(buffer),
    };
    Ok((chunk_id, chunk_data))
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

/// Write a chunk to file storage
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
