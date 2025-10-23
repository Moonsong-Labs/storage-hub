use anyhow::anyhow;
use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::events::FinalisedTrieRemoveMutationsAppliedForBucket;
use shc_common::{traits::StorageEnableRuntime, types::FileKey};
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};

use crate::{
    handler::StorageHubHandler,
    types::{ForestStorageKey, MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "msp-delete-file-task";

/// MSP Delete File Task: Handles the deletion of files from the File Storage.
///
/// The flow includes the following steps:
/// - [`FinalisedTrieRemoveMutationsAppliedForBucket`] Event:
///   - Triggered when mutations applied to a bucket's Merkle Trie have been finalized,
///     indicating that certain file keys should be removed.
///   - Iterates over each file key that was part of the finalized mutations.
///   - Checks if the file key is still present in the bucket's [`ForestStorage`]:
///     - If the key is still present, logs a warning, as this may indicate that the key was
///       re-added after deletion.
///     - If the key is absent from the Forest Storage, safely removes the corresponding file
///       from the File Storage.
///   - Ensures that no residual file keys remain in the File Storage when they should have
///     been deleted.
pub struct MspDeleteFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for MspDeleteFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> MspDeleteFileTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, Runtime> MspDeleteFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler,
        }
    }

    async fn remove_file_from_file_storage(&self, file_key: &sp_core::H256) -> anyhow::Result<()> {
        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;
        write_file_storage.delete_file(file_key).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to remove file from File Storage after it was removed from the Forest. \nError: {:?}", e);
            anyhow!(
                "Failed to delete file from File Storage after it was removed from the Forest: {:?}",
                e
            )
        })?;
        drop(write_file_storage);
        Ok(())
    }
}

/// Handles the [`FinalisedTrieRemoveMutationsAppliedForBucket`] event.
///
/// This event is triggered when mutations applied to the Forest of this bucket have been
/// finalised, signalling that certain keys (representing files) should be removed from the
/// File Storage if they are not present in the Forest Storage. If the key is still present in
/// the Forest Storage, it sends out a warning, since it could indicate that the key has been
/// re-added after being deleted.
///
/// This task performs the following actions:
/// - Iterates over each removed file key.
/// - Checks if the file key is present in the Forest Storage for the affected bucket.
///   - If the key is still present, it logs a warning, since this could indicate that the key
///     has been re-added after being deleted.
///   - If the key is not present in the Forest Storage, it safely removes the key from the
///     File Storage.
impl<NT, Runtime> EventHandler<FinalisedTrieRemoveMutationsAppliedForBucket<Runtime>>
    for MspDeleteFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: FinalisedTrieRemoveMutationsAppliedForBucket<Runtime>,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing finalised mutations applied for bucket {:?} with mutations: {:?}",
            event.bucket_id,
            event.mutations
        );

        // Load the forest storage for this bucket
        let bucket_forest_key = ForestStorageKey::from(event.bucket_id.as_ref().to_vec());
        let read_fs = self
            .storage_hub_handler
            .forest_storage_handler
            .get(&bucket_forest_key)
            .await
            .ok_or_else(|| anyhow!("CRITICAL❗️❗️ Failed to get forest storage for bucket."))?;

        // For each mutation, if the key is not present in the Forest, remove it from File Storage
        for mutation in event.mutations {
            let file_key = FileKey::from(mutation.0);

            if read_fs.read().await.contains_file_key(&file_key.into())? {
                warn!(
                    target: LOG_TARGET,
                    "TrieRemoveMutation applied and finalised for file key {:?} in bucket {:?}, but key is still in Forest. This can only happen if the same key was added again after deletion.\n Mutation: {:?}",
                    file_key,
                    event.bucket_id,
                    mutation
                );
            } else {
                self.remove_file_from_file_storage(&file_key.into()).await?;
            }
        }

        Ok(())
    }
}
