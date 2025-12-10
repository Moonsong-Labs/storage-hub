//! HTTP server for receiving file chunks from trusted backends

use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
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
use shc_common::traits::StorageEnableRuntime;
use shc_file_transfer_service::{
    commands::FileTransferServiceCommandInterface, FileTransferService,
};
use shc_forest_manager::traits::ForestStorageHandler;
use tokio::{net::TcpListener, sync::RwLock};

use crate::{trusted_file_transfer::files::process_chunk_stream, types::FileStorageT};

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
        "👂 Trusted file transfer HTTP server listening"
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
    match process_chunk_stream(&context.file_storage, &file_key_hash, body).await {
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
        .await?;
    Ok(())
}
