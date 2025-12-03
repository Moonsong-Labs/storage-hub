use sc_tracing::tracing::{error, trace};
use shc_actors_framework::event_bus::EventHandler;
use shc_common::traits::StorageEnableRuntime;
use shc_file_manager::traits::FileStorage;
use shc_file_transfer_service::{
    commands::FileTransferServiceCommandInterface, events::RemoteDownloadRequest,
};

use crate::{
    handler::StorageHubHandler,
    inc_counter,
    metrics::{STATUS_FAILURE, STATUS_SUCCESS},
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
    async fn handle_event(
        &mut self,
        event: RemoteDownloadRequest<Runtime>,
    ) -> anyhow::Result<String> {
        trace!(target: LOG_TARGET, "Received remote download request with id {:?} for file {:?}", event.request_id, event.file_key);

        // Process the download request and track success/failure with metrics.
        let result = self.handle_download_inner(event).await;

        // Record metric based on result.
        match &result {
            Ok(_) => {
                inc_counter!(
                    self.storage_hub_handler,
                    bsp_download_requests_total,
                    STATUS_SUCCESS
                );
            }
            Err(_) => {
                inc_counter!(
                    self.storage_hub_handler,
                    bsp_download_requests_total,
                    STATUS_FAILURE
                );
            }
        }

        result
    }
}

impl<NT, Runtime> BspDownloadFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    /// Inner implementation of download request handling.
    async fn handle_download_inner(
        &self,
        event: RemoteDownloadRequest<Runtime>,
    ) -> anyhow::Result<String> {
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

        match generate_proof_result {
            Ok(file_key_proof) => {
                // Send the chunk data and proof back to the requester.
                self.storage_hub_handler
                    .file_transfer
                    .download_response(request_id.clone(), file_key_proof)
                    .await?;
            }
            Err(error) => {
                error!(target: LOG_TARGET, "Failed to generate proof for chunk id {:?} of file {:?}", chunk_ids, event.file_key);
                return Err(anyhow::anyhow!("{:?}", error));
            }
        }

        Ok(format!(
            "Handled RemoteDownloadRequest [{:?}] for file [{:x}]",
            request_id, event.file_key
        ))
    }
}
