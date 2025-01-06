use anyhow::anyhow;
use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::events::{BspConfirmStoppedStoring, FinalisedBspConfirmStoppedStoring};
use shc_common::consts::CURRENT_FOREST_KEY;
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use sp_core::H256;

use crate::services::{
    handler::StorageHubHandler,
    types::{BspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "bsp-delete-file-task";

pub struct BspDeleteFileTask<NT>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<NT>,
}

impl<NT> Clone for BspDeleteFileTask<NT>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
{
    fn clone(&self) -> BspDeleteFileTask<NT> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT> BspDeleteFileTask<NT>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT>) -> Self {
        Self {
            storage_hub_handler,
        }
    }

    async fn remove_file_from_forest(&self, file_key: &H256) -> anyhow::Result<()> {
        // Remove the file key from the Forest.
        let current_forest_key = CURRENT_FOREST_KEY.to_vec();
        {
            let fs = self
                .storage_hub_handler
                .forest_storage_handler
                .get(&current_forest_key)
                .await
                .ok_or_else(|| anyhow!("Failed to get forest storage."))?;

            fs.write().await.delete_file_key(file_key).map_err(|e| {
                warn!(target: LOG_TARGET, "Failed to apply mutation to Forest storage. This may result in a mismatch between the Forest root on-chain and in this node. \nError: {:?}", e);
                anyhow!(
                    "Failed to remove file key from Forest storage: {:?}",
                    e
                )
            })?;
        };

        Ok(())
    }

    async fn remove_file_from_file_storage(&self, file_key: &H256) -> anyhow::Result<()> {
        // Remove the file from the File Storage.
        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;
        write_file_storage.delete_file(file_key).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to remove file from File Storage after it was removed from the Forest. \nError: {:?}", e);
            anyhow!(
                "Failed to delete file from File Storage after it was removed from the Forest: {:?}",
                e
            )
        })?;

        // Release the file storage write lock.
        drop(write_file_storage);

        Ok(())
    }
}

impl<NT> EventHandler<BspConfirmStoppedStoring> for BspDeleteFileTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
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

        info!(
            target: LOG_TARGET,
            "File {:?} successfully removed from forest",
            event.file_key,
        );

        Ok(())
    }
}

impl<NT> EventHandler<FinalisedBspConfirmStoppedStoring> for BspDeleteFileTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
{
    async fn handle_event(
        &mut self,
        event: FinalisedBspConfirmStoppedStoring,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Deleting file {:?} for BSP {:?}",
            event.file_key,
            event.bsp_id
        );

        // Check that the file_key is not in the Forest.
        let current_forest_key = CURRENT_FOREST_KEY.to_vec();
        let read_fs = self
            .storage_hub_handler
            .forest_storage_handler
            .get(&current_forest_key)
            .await
            .ok_or_else(|| anyhow!("Failed to get forest storage."))?;
        if read_fs
            .read()
            .await
            .contains_file_key(&event.file_key.into())?
        {
            warn!(
                target: LOG_TARGET,
                "FinalisedBspConfirmStoppedStoring applied and finalised for file key {:?}, but file key is still in Forest. This can only happen if the same file key was added again after deleted by this BSP.",
                event.file_key,
            );
        } else {
            // If file key is not in Forest, we can now safely remove it from the File Storage.
            self.remove_file_from_file_storage(&event.file_key.into())
                .await?;
        }
        Ok(())
    }
}
