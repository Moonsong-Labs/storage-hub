//! MSP Internal File Transfer Server
//!
//! HTTP server for MSP to receive streamed file chunks from backend.
//! This server accepts POST requests with streamed chunks in the format:
//! [ChunkId: 8 bytes (u64, little-endian)][Chunk length: 4 bytes (u32, little-endian)][Chunk data: variable]

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
use shc_common::{traits::StorageEnableRuntime, types::ChunkId};
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
pub struct Context<FL: FileStorageT> {
    file_storage: Arc<RwLock<FL>>,
}

impl<FL: FileStorageT> Clone for Context<FL> {
    fn clone(&self) -> Self {
        Self {
            file_storage: Arc::clone(&self.file_storage),
        }
    }
}

impl<FL: FileStorageT> Context<FL> {
    pub fn new(file_storage: Arc<RwLock<FL>>) -> Self {
        Self { file_storage }
    }
}

/// Spawn the MSP internal file transfer HTTP server
pub async fn spawn_server<FL: FileStorageT>(
    config: Config,
    file_storage: Arc<RwLock<FL>>,
) -> anyhow::Result<()> {
    let context = Context::new(file_storage);

    let app = Router::new()
        .route("/upload/:file_key", post(upload_file))
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
async fn upload_file<FL: FileStorageT>(
    State(context): State<Context<FL>>,
    Path(file_key): Path<String>,
    body: Body,
) -> impl IntoResponse {
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
        Ok(chunk_count) => {
            debug!(
                target: LOG_TARGET,
                file_key = %file_key,
                chunks_processed = chunk_count,
                "Successfully processed chunk stream"
            );
            (StatusCode::OK, format!("Processed {} chunks", chunk_count)).into_response()
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
async fn process_chunk_stream<FL: FileStorageT>(
    context: &Context<FL>,
    file_key: &sp_core::H256,
    body: Body,
) -> anyhow::Result<u64> {
    let mut stream = body.into_data_stream();
    let mut buffer = Vec::new();
    let mut chunk_count = 0u64;

    // Constants for parsing the binary format
    const CHUNK_ID_SIZE: usize = 8; // u64
    const CHUNK_LENGTH_SIZE: usize = 4; // u32
    const HEADER_SIZE: usize = CHUNK_ID_SIZE + CHUNK_LENGTH_SIZE;

    loop {
        // Read data from the stream
        match stream.next().await {
            Some(Ok(bytes)) => {
                buffer.extend_from_slice(&bytes);
            }
            Some(Err(e)) => {
                return Err(anyhow::anyhow!("Error reading from stream: {}", e));
            }
            None => {
                // Stream ended
                if !buffer.is_empty() {
                    return Err(anyhow::anyhow!("Stream ended with incomplete chunk data"));
                }
                break;
            }
        }

        // Process complete chunks from the buffer
        while buffer.len() >= HEADER_SIZE {
            // Parse chunk ID (u64, little-endian)
            let chunk_id_bytes: [u8; CHUNK_ID_SIZE] = buffer[0..CHUNK_ID_SIZE]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Failed to parse chunk ID"))?;
            let chunk_id = u64::from_le_bytes(chunk_id_bytes);
            let chunk_id = ChunkId::new(chunk_id);

            // Parse chunk length (u32, little-endian)
            let length_bytes: [u8; CHUNK_LENGTH_SIZE] = buffer[CHUNK_ID_SIZE..HEADER_SIZE]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Failed to parse chunk length"))?;
            let chunk_length = u32::from_le_bytes(length_bytes) as usize;

            // Check if we have enough data for the chunk
            let total_chunk_size = HEADER_SIZE + chunk_length;
            if buffer.len() < total_chunk_size {
                // Not enough data yet, wait for more
                break;
            }

            // Extract the chunk data
            let chunk_data = buffer[HEADER_SIZE..total_chunk_size].to_vec();

            // Process the chunk immediately by storing it
            debug!(
                target: LOG_TARGET,
                file_key = %file_key,
                chunk_id = chunk_id.as_u64(),
                chunk_size = chunk_data.len(),
                "Processing chunk"
            );

            // Store chunk in file storage
            let mut storage = context.file_storage.write().await;
            storage
                .write_chunk(file_key, &chunk_id, &chunk_data)
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to write chunk {} to storage: {}",
                        chunk_id.as_u64(),
                        e
                    )
                })?;

            // Remove processed chunk from buffer
            buffer.drain(..total_chunk_size);
            chunk_count += 1;
        }
    }

    Ok(chunk_count)
}
