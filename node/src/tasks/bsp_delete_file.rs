use anyhow::anyhow;
use sc_tracing::tracing::*;
use shc_blockchain_service::events::{BspConfirmStoppedStoring, FinalisedBspConfirmStoppedStoring};
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

        info!(
            target: LOG_TARGET,
            "File {:?} successfuly removed from forest",
            event.file_key,
        );

        Ok(())
    }
}

impl<FL, FSH> EventHandler<FinalisedBspConfirmStoppedStoring> for BspDeleteFileTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
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
        let read_fs = self
            .storage_hub_handler
            .forest_storage_handler
            .get(&NoKey)
            .await
            .ok_or_else(|| anyhow!("Failed to get forest storage."))?;
        if read_fs
            .read()
            .await
            .contains_file_key(&event.file_key.into())?
        {
            warn!(
                target: LOG_TARGET,
                "FinalisedBspConfirmStoppedStoring applied and finalised for file key {:?}, but file key is still in Forest. This can only happen if the same file key was added again after deleted by the user.",
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