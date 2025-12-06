use anyhow::anyhow;
use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::events::{
    FinalisedBspConfirmStoppedStoring, FinalisedTrieRemoveMutationsAppliedForBsp,
};
use shc_common::{
    consts::CURRENT_FOREST_KEY,
    traits::StorageEnableRuntime,
    types::{FileKey, TrieMutation, TrieRemoveMutation},
};
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use sp_core::H256;

use crate::{
    handler::StorageHubHandler,
    inc_counter,
    metrics::{STATUS_FAILURE, STATUS_SUCCESS},
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

/// BSP Delete File Task: Handles the deletion of files from the File Storage.
///
/// The flow includes the following steps:
/// - **[`FinalisedBspConfirmStoppedStoring`] Event:**
///   - Triggered when a BSP confirms that it has stopped storing a file.
///   - Checks if the file key is still present in the Forest Storage:
///     - If the key is still present, logs a warning, as this may indicate that the key was re-added after deletion.
///     - If the key is absent from the Forest Storage, safely removes the corresponding file from the File Storage.
///   - Ensures that no residual file keys remain in the File Storage when they should have been deleted.
///
/// - **[`FinalisedTrieRemoveMutationsAppliedForBsp`] Event:**
///   - Triggered when mutations applied to the Merkle Trie have been finalized, indicating that certain keys should be removed.
///   - Iterates over each file key that was part of the finalised mutations.
///   - Checks if the file key is still present in the Forest Storage:
///     - If the key is still present, logs a warning, as this may indicate that the key was re-added after deletion.
///     - If the key is absent from the Forest Storage, safely removes the corresponding file from the File Storage.
///   - Ensures that no residual file keys remain in the File Storage when they should have been deleted.
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
        let result = write_file_storage.delete_file(file_key).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to remove file from File Storage after it was removed from the Forest. \nError: {:?}", e);
            anyhow!(
                "Failed to delete file from File Storage after it was removed from the Forest: {:?}",
                e
            )
        });

        // Record metric based on result.
        match &result {
            Ok(_) => {
                inc_counter!(
                    handler: self.storage_hub_handler,
                    bsp_files_deleted_total,
                    STATUS_SUCCESS
                );
            }
            Err(_) => {
                inc_counter!(
                    handler: self.storage_hub_handler,
                    bsp_files_deleted_total,
                    STATUS_FAILURE
                );
            }
        }

        result
    }
}

/// Handles the [`FinalisedBspConfirmStoppedStoring`] event.
///
/// This event is triggered when a BSP confirms that it has stopped storing a file,
/// signalling that the file should be removed from the File Storage if it is not present in the Forest Storage.
/// If the key is still present in the Forest Storage, it sends out a warning, since it could indicate that the
/// key has been re-added after being deleted.
///
/// This task performs the following actions:
///   - If the key is still present, it logs a warning, since it could indicate that the key has been re-added after being deleted.
///   - If the key is not present in the Forest Storage, it safely removes the key from the File Storage.
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
    ) -> anyhow::Result<String> {
        info!(
            target: LOG_TARGET,
            "Processing finalised BSP confirm stopped storing file key [{:x}] for BSP [{:x}]",
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
            .ok_or_else(|| {
                inc_counter!(
                    handler: self.storage_hub_handler,
                    bsp_files_deleted_total,
                    STATUS_FAILURE
                );
                anyhow!("CRITICAL❗️❗️ Failed to get forest storage.")
            })?;
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
            // Metrics are recorded inside remove_file_from_file_storage.
            self.remove_file_from_file_storage(&event.file_key.into())
                .await?;
        }

        Ok(format!(
            "Handled FinalisedBspConfirmStoppedStoring for file key [{:x}]",
            event.file_key
        ))
    }
}

/// Handles the [`FinalisedTrieRemoveMutationsAppliedForBsp`] event.
///
/// This event is triggered when mutations applied to the Forest of this BSP have been finalised,
/// signalling that certain keys (representing files) should be removed from the File Storage if they are
/// not present in the Forest Storage. If the key is still present in the Forest Storage, it sends out
/// a warning, since it could indicate that the key has been re-added after being deleted.
///
/// This task performs the following actions:
/// - Iterates over each removed file key.
/// - Checks if the file key is present in the Forest Storage.
///   - If the key is still present, it logs a warning,
///     since this could indicate that the key has been re-added after being deleted.
///   - If the key is not present in the Forest Storage, it safely removes the key from the File Storage.
impl<NT, Runtime> EventHandler<FinalisedTrieRemoveMutationsAppliedForBsp<Runtime>>
    for BspDeleteFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: FinalisedTrieRemoveMutationsAppliedForBsp<Runtime>,
    ) -> anyhow::Result<String> {
        info!(
            target: LOG_TARGET,
            "Processing finalised mutations applied for provider [{:x}]",
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
                .ok_or_else(|| {
                    inc_counter!(
                        handler: self.storage_hub_handler,
                        bsp_files_deleted_total,
                        STATUS_FAILURE
                    );
                    anyhow!("CRITICAL❗️❗️ Failed to get forest storage.")
                })?;
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

        Ok(format!(
            "Handled FinalisedTrieRemoveMutationsAppliedForBsp for provider [{:x}]",
            event.provider_id
        ))
    }
}
