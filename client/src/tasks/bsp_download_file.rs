use sc_tracing::tracing::{error, trace};
use serde::{Deserialize, Serialize};
use shc_actors_framework::event_bus::EventHandler;
use shc_common::task_context::TaskContext;
use shc_common::telemetry_error::TelemetryErrorCategory;
use shc_common::traits::StorageEnableRuntime;
use shc_file_manager::traits::FileStorage;
use shc_file_transfer_service::{
    commands::FileTransferServiceCommandInterface, events::RemoteDownloadRequest,
};
use shc_telemetry_service::{
    create_base_event, BaseTelemetryEvent, TelemetryEvent, TelemetryServiceCommandInterfaceExt,
};

// Local BSP download telemetry event definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BspDownloadRequestedEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    task_name: String,
    file_key: String,
    chunk_ids: String,
    request_id: String,
    bucket_id: Option<String>,
}

impl TelemetryEvent for BspDownloadRequestedEvent {
    fn event_type(&self) -> &str {
        "bsp_download_requested"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BspDownloadChunkSentEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    file_key: String,
    chunk_ids: String,
    request_id: String,
    chunk_count: u32,
    total_size_bytes: u64,
}

impl TelemetryEvent for BspDownloadChunkSentEvent {
    fn event_type(&self) -> &str {
        "bsp_download_chunk_sent"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BspDownloadCompletedEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    file_key: String,
    request_id: String,
    duration_ms: u64,
    chunk_count: u32,
    total_size_bytes: u64,
}

impl TelemetryEvent for BspDownloadCompletedEvent {
    fn event_type(&self) -> &str {
        "bsp_download_completed"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BspDownloadFailedEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    file_key: String,
    error_type: String,
    error_message: String,
    request_id: String,
    duration_ms: Option<u64>,
}

impl TelemetryEvent for BspDownloadFailedEvent {
    fn event_type(&self) -> &str {
        "bsp_download_failed"
    }
}

use crate::{
    handler::StorageHubHandler,
    types::{BspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "bsp-download-file-task";

pub struct BspDownloadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for BspDownloadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> BspDownloadFileTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, Runtime> BspDownloadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

/// Handles a remote download request.
///
/// This will generate a proof for the chunk and send it back to the requester.
/// If there is a bucket ID provided, this will also check that it matches the local file's bucket.
impl<NT, Runtime> EventHandler<RemoteDownloadRequest<Runtime>> for BspDownloadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: RemoteDownloadRequest<Runtime>) -> anyhow::Result<()> {
        trace!(target: LOG_TARGET, "Received remote download request with id {:?} for file {:?}", event.request_id, event.file_key);

        // Create task context for tracking
        let ctx = TaskContext::new("bsp_download_file");

        // Send task started telemetry event
        if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
            let start_event = BspDownloadRequestedEvent {
                base: create_base_event(
                    "bsp_download_requested",
                    "storage-hub-bsp".to_string(),
                    None,
                ),
                task_id: ctx.task_id.clone(),
                task_name: ctx.task_name.clone(),
                file_key: format!("{:?}", event.file_key),
                chunk_ids: format!("{:?}", event.chunk_ids),
                request_id: format!("{:?}", event.request_id),
                bucket_id: event.bucket_id.as_ref().map(|b| format!("{:?}", b)),
            };
            telemetry_service.queue_typed_event(start_event).await.ok();
        }

        let result = self.handle_download_request_internal(event.clone()).await;

        // Send completion or failure telemetry
        if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
            match &result {
                Ok((chunk_count, total_size_bytes)) => {
                    let completed_event = BspDownloadCompletedEvent {
                        base: create_base_event(
                            "bsp_download_completed",
                            "storage-hub-bsp".to_string(),
                            None,
                        ),
                        task_id: ctx.task_id.clone(),
                        file_key: format!("{:?}", event.file_key),
                        request_id: format!("{:?}", event.request_id),
                        duration_ms: ctx.elapsed_ms(),
                        chunk_count: *chunk_count,
                        total_size_bytes: *total_size_bytes,
                    };
                    telemetry_service
                        .queue_typed_event(completed_event)
                        .await
                        .ok();
                }
                Err(e) => {
                    let failed_event = BspDownloadFailedEvent {
                        base: create_base_event(
                            "bsp_download_failed",
                            "storage-hub-bsp".to_string(),
                            None,
                        ),
                        task_id: ctx.task_id.clone(),
                        file_key: format!("{:?}", event.file_key),
                        error_type: e.telemetry_category().to_string(),
                        error_message: e.to_string(),
                        request_id: format!("{:?}", event.request_id),
                        duration_ms: Some(ctx.elapsed_ms()),
                    };
                    telemetry_service.queue_typed_event(failed_event).await.ok();
                }
            }
        }

        result.map(|_| ())
    }
}

impl<NT, Runtime> BspDownloadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_download_request_internal(
        &mut self,
        event: RemoteDownloadRequest<Runtime>,
    ) -> anyhow::Result<(u32, u64)> {
        let RemoteDownloadRequest {
            chunk_ids,
            request_id,
            bucket_id,
            ..
        } = event;

        // Get the file metadata from the file storage.
        let file_storage_read_lock = self.storage_hub_handler.file_storage.read().await;
        let file_metadata = file_storage_read_lock
            .get_metadata(&event.file_key.into())
            .map_err(|_| anyhow::anyhow!("Failed to get file metadata"))?;

        // If the file metadata is not found, return an error.
        let file_metadata = if let Some(file_metadata) = file_metadata {
            file_metadata
        } else {
            error!(target: LOG_TARGET, "File not found in file storage");
            return Err(anyhow::anyhow!("File not found in file storage"));
        };

        // If we have a bucket ID in the request, check if the file bucket matches the bucket ID in
        // the request.
        if let Some(bucket_id) = bucket_id {
            if file_metadata.bucket_id() != bucket_id.as_ref() {
                error!(
                    target: LOG_TARGET,
                    "File bucket mismatch for file {:?}: expected {:?}, got {:?}",
                    event.file_key, file_metadata.bucket_id(), bucket_id
                );
                return Err(anyhow::anyhow!("File bucket mismatch"));
            }
        }

        // Generate the proof for the chunk (which also contains the chunk data itself).
        let generate_proof_result =
            file_storage_read_lock.generate_proof(&event.file_key.into(), &chunk_ids);

        let (chunk_count, total_size_bytes) = match generate_proof_result {
            Ok(file_key_proof) => {
                // Calculate metrics for telemetry
                let chunk_count = chunk_ids.len() as u32;
                let total_size_bytes = file_key_proof
                    .proven::<shc_common::types::StorageProofsMerkleTrieLayout>()
                    .map(|proven| proven.iter().map(|chunk| chunk.data.len() as u64).sum())
                    .unwrap_or(0);

                // Send chunk sent telemetry event
                if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
                    let ctx = TaskContext::new("bsp_download_chunk");
                    let chunk_sent_event = BspDownloadChunkSentEvent {
                        base: create_base_event(
                            "bsp_download_chunk_sent",
                            "storage-hub-bsp".to_string(),
                            None,
                        ),
                        task_id: ctx.task_id.clone(),
                        file_key: format!("{:?}", event.file_key),
                        chunk_ids: format!("{:?}", chunk_ids),
                        request_id: format!("{:?}", request_id),
                        chunk_count,
                        total_size_bytes,
                    };
                    telemetry_service
                        .queue_typed_event(chunk_sent_event)
                        .await
                        .ok();
                }

                // Send the chunk data and proof back to the requester.
                self.storage_hub_handler
                    .file_transfer
                    .download_response(request_id, file_key_proof)
                    .await?;

                (chunk_count, total_size_bytes)
            }
            Err(error) => {
                error!(target: LOG_TARGET, "Failed to generate proof for chunk id {:?} of file {:?}", chunk_ids, event.file_key);
                return Err(anyhow::anyhow!("{:?}", error));
            }
        };

        Ok((chunk_count, total_size_bytes))
    }
}
