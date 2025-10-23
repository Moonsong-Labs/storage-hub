use anyhow::anyhow;
use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::events::{
    FinalisedBspConfirmStoppedStoring, FinalisedTrieRemoveMutationsApplied,
};
use shc_common::consts::CURRENT_FOREST_KEY;
use shc_common::traits::StorageEnableRuntime;
use shc_common::types::{FileKey, TrieMutation, TrieRemoveMutation};
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use sp_core::H256;

use crate::{
    handler::StorageHubHandler,
    types::{BspForestStorageHandlerT, ForestStorageKey, ShNodeType},
};

const LOG_TARGET: &str = "bsp-delete-file-task";

/// BSP Delete File Task: Handles the removal of files from the BSP's File Storage after
/// file removal has been finalised on-chain.
///
/// This task reacts to the events:
/// - **[`FinalisedBspConfirmStoppedStoring`] Event:**
///   - Triggered when a specific file stop-storing confirmation has been finalised on-chain for this BSP.
///     The file is removed from File Storage if it is not present in the BSP's Forest Storage. If the key
///     is still present in the Forest Storage, it sends out a warning, since it could indicate that the
///     same file key was added again after being deleted by this BSP.
/// - **[`FinalisedTrieRemoveMutationsApplied`] Event:**
///   - Triggered when provider-wide Forest mutations have been finalised on-chain. This task only processes
///     remove mutations, checking each affected key and removing it from File Storage if it is not present
///     in the BSP's Forest Storage. If the key is still present, it logs a warning, since this could indicate
///     that the key has been re-added after being deleted.
///
/// This task performs the following actions:
/// - For each removed file key:
///   - Checks if the file key is present in the BSP's Forest Storage.
///     - If the key is still present, it logs a warning,
///       since this could indicate that the key has been re-added after being deleted.
///     - If the key is not present in the Forest Storage, it safely removes the key from the File Storage.
pub struct BspDeleteFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for BspDeleteFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> BspDeleteFileTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, Runtime> BspDeleteFileTask<NT, Runtime>
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

    async fn remove_file_from_file_storage(&self, file_key: &H256) -> anyhow::Result<()> {
        // Remove the file from the File Storage.
        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;
        write_file_storage.delete_file(file_key).map_err(|e| {
			error!(target: LOG_TARGET, "Failed to remove file from File Storage after it was removed from the Forest. \\nError: {:?}", e);
			anyhow!(
					"Failed to delete file from File Storage after it was removed from the Forest: {:?}",
					e
			)
		})?;

        Ok(())
    }
}

/// Handles the [`FinalisedBspConfirmStoppedStoring`] event.
impl<NT, Runtime> EventHandler<FinalisedBspConfirmStoppedStoring<Runtime>>
    for BspDeleteFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: FinalisedBspConfirmStoppedStoring<Runtime>,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing finalised BSP confirm stopped storing file key [{:x}] for BSP [{:?}]",
            event.file_key,
            event.bsp_id
        );

        // Check that the file key is not in the Forest.
        let current_forest_key = ForestStorageKey::from(CURRENT_FOREST_KEY.to_vec());
        let read_fs = self
            .storage_hub_handler
            .forest_storage_handler
            .get(&current_forest_key)
            .await
            .ok_or_else(|| anyhow!("CRITICAL❗️❗️ Failed to get forest storage."))?;
        if read_fs
            .read()
            .await
            .contains_file_key(&event.file_key.into())?
        {
            warn!(
                target: LOG_TARGET,
                "FinalisedBspConfirmStoppedStoring applied and finalised for file key {:x}, but file key is still in Forest. This can only happen if the same file key was added again after deleted by this BSP.",
                event.file_key
            );
        } else {
            // If file key is not in Forest, we can now safely remove it from the File Storage.
            self.remove_file_from_file_storage(&event.file_key.into())
                .await?;
        }

        Ok(())
    }
}

/// Handles the [`FinalisedTrieRemoveMutationsApplied`] event.
impl<NT, Runtime> EventHandler<FinalisedTrieRemoveMutationsApplied<Runtime>>
    for BspDeleteFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: FinalisedTrieRemoveMutationsApplied<Runtime>,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing finalised mutations applied for provider [{:?}]",
            event.provider_id
        );
        debug!(target: LOG_TARGET, "Mutations to apply: {:?}", event.mutations);

        for mutation in event.mutations {
            // Get the file key from the mutation.
            let file_key = FileKey::from(mutation.0);

            // Only process remove mutations in this task.
            if mutation.1 != TrieMutation::Remove(TrieRemoveMutation::new()) {
                debug!(target: LOG_TARGET, "Skipping non-remove mutation for file key {:?}", file_key);
                continue;
            }

            // Check that the file key is not in the Forest.
            let current_forest_key = ForestStorageKey::from(CURRENT_FOREST_KEY.to_vec());
            let read_fs = self
                .storage_hub_handler
                .forest_storage_handler
                .get(&current_forest_key)
                .await
                .ok_or_else(|| anyhow!("CRITICAL❗️❗️ Failed to get forest storage."))?;
            if read_fs.read().await.contains_file_key(&file_key.into())? {
                warn!(
                    target: LOG_TARGET,
                    "TrieRemoveMutation applied and finalised for file key {:?}, but file key is still in Forest. This can only happen if the same file key was added again after deleted by the user.\n Mutation: {:?}",
                    file_key,
                    mutation
                );
            } else {
                // If file key is not in Forest, we can now safely remove it from the File Storage.
                self.remove_file_from_file_storage(&file_key.into()).await?;
            }
        }

        Ok(())
    }
}
