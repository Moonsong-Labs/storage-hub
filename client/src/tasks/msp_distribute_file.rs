use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface, events::DistributeFileToBsp,
};
use shc_common::{
    traits::StorageEnableRuntime,
    types::{BackupStorageProviderId, FileKey},
};
use shc_file_manager::traits::FileStorage;
use shc_file_transfer_service::commands::FileTransferServiceCommandInterfaceExt;

use crate::{
    handler::StorageHubHandler,
    tasks::shared::chunk_uploader::ChunkUploaderExt,
    types::{MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "msp-distribute-file-task";

pub struct MspDistributeFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for MspDistributeFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> MspDistributeFileTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, Runtime> MspDistributeFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler: storage_hub_handler.clone(),
        }
    }
}

/// Handles the [`DistributeFileToBsp`] event.
///
/// This event is emitted when this node is an MSP and it was selected by the
/// user to distribute the file from a storage request to the BSPs who volunteer.
/// The MSP has to support this feature via configuration.
///
/// This handler will:
/// 1. Register the BSP as "distributing" for the given `file_key` to prevent
///    duplicate concurrent distributions.
/// 2. Load the file metadata from the local file storage.
/// 3. Query the BSP's multiaddresses from the runtime and resolve their peer IDs,
///    registering them as known addresses in the File Transfer Service.
/// 4. Upload the file chunks to the BSP using the shared chunk uploader.
///
/// If any step fails, the BSP is unregistered from the "distributing" set so
/// the operation can be retried later.
impl<NT, Runtime> EventHandler<DistributeFileToBsp<Runtime>> for MspDistributeFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: DistributeFileToBsp<Runtime>) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Distributing file to BSP",
        );

        let file_key = event.file_key;
        let bsp_id = event.bsp_id;

        // Register that this task is distributing the file to the BSP.
        // This avoids a second instance of this task from being spawned.
        // This can fail if the BSP is already registered as distributing file.
        self.storage_hub_handler
            .blockchain
            .register_bsp_distributing(file_key, bsp_id)
            .await?;

        // This function handles the whole process of distributing the file to the BSP.
        // If anything fails, we unregister the BSP as distributing file, thus allowing
        // for a retry.
        if let Err(e) = self.handle_distribute_file_to_bsp(file_key, bsp_id).await {
            error!(target: LOG_TARGET, "Failed to distribute file to BSP: {:?}", e);

            // Unregister BSP as distributing file.
            // This in itself can fail. If it does, we have no other choice but to
            // just log the error and return, and this BSP will not be able to get
            // the file from this MSP at least.
            if let Err(e) = self
                .storage_hub_handler
                .blockchain
                .unregister_bsp_distributing(file_key, bsp_id)
                .await
            {
                error!(target: LOG_TARGET, "CRITICAL❗️❗️ Failed to unregister BSP as distributing file. This means that this BSP will not be able to get the file from this MSP at least. {:?}", e);
            }

            return Err(e);
        }

        Ok(())
    }
}

impl<NT, Runtime> MspDistributeFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_distribute_file_to_bsp(
        &mut self,
        file_key: FileKey,
        bsp_id: BackupStorageProviderId<Runtime>,
    ) -> anyhow::Result<()> {
        // Get file metadata from local file storage.
        let file_metadata = self
            .storage_hub_handler
            .file_storage
            .read()
            .await
            .get_metadata(&file_key.into())
            .map_err(|e| anyhow::anyhow!("Failed to get metadata from file storage: {:?}", e))?;
        let file_metadata = file_metadata.ok_or(anyhow::anyhow!("File metadata not found"))?;

        // Get MSP multiaddresses from BSP from runtime.
        let msp_multiaddresses = self
            .storage_hub_handler
            .blockchain
            .query_provider_multiaddresses(bsp_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get MSP multiaddresses from BSP: {:?}", e))?;

        // Get peer ids from multiaddresses and register them as known addresses.
        let peer_ids = self
            .storage_hub_handler
            .file_transfer
            .extract_peer_ids_and_register_known_addresses(msp_multiaddresses)
            .await;

        // Send chunks to provider using shared uploader.
        self.storage_hub_handler
            .upload_file_to_peer_ids(peer_ids, &file_metadata)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send chunks to provider: {:?}", e))?;

        info!(target: LOG_TARGET, "Successfully distributed file {:?} to BSP {:?}", file_key, bsp_id);

        // BSP will be moved from the "distributing" set to the "confirmed" set
        // when the BSP confirms to store the file. This is done by processing
        // the `BspConfirmedStoring` event in `handler_msp.rs`.
        Ok(())
    }
}
