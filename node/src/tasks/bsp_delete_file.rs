use anyhow::anyhow;
use sc_tracing::tracing::*;
use shc_blockchain_service::events::BspConfirmStoppedStoring;
use shc_forest_manager::traits::ForestStorage;
use sp_core::H256;

use crate::services::handler::StorageHubHandler;
use crate::tasks::{BspForestStorageHandlerT, FileStorageT, NoKey};
use shc_actors_framework::event_bus::EventHandler;

const LOG_TARGET: &str = "bsp-delete-file-task";

pub struct BspDeleteFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<FL, FSH>,
}

impl<FL, FSH> Clone for BspDeleteFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    fn clone(&self) -> BspDeleteFileTask<FL, FSH> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<FL, FSH> BspDeleteFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    pub fn new(storage_hub_handler: StorageHubHandler<FL, FSH>) -> Self {
        Self {
            storage_hub_handler,
        }
    }

    async fn remove_file_from_forest(&self, file_key: &H256) -> anyhow::Result<()> {
        // Remove the file key from the Forest.
        // Check that the new Forest root matches the one on-chain.
        {
            let fs = self
                .storage_hub_handler
                .forest_storage_handler
                .get(&NoKey)
                .await
                .ok_or_else(|| anyhow!("Failed to get forest storage."))?;

            fs.write().await.delete_file_key(file_key).map_err(|e| {
                error!(target: LOG_TARGET, "CRITICAL❗️❗️ Failed to apply mutation to Forest storage. This may result in a mismatch between the Forest root on-chain and in this node. \nThis is a critical bug. Please report it to the StorageHub team. \nError: {:?}", e);
                anyhow!(
                    "Failed to remove file key from Forest storage: {:?}",
                    e
                )
            })?;
        };

        Ok(())
    }
}

impl<FL, FSH> EventHandler<BspConfirmStoppedStoring> for BspDeleteFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: BspConfirmStoppedStoring) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Deleting file {:?} for BSP {:?}",
            event.file_key,
            event.bsp_id
        );

        // Remove the file from the forest.
        self.remove_file_from_forest(&event.file_key.into()).await?;

        // TODO: add log about file being deleted

        Ok(())
    }
}
