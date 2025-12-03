use anyhow::anyhow;
use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::events::{
    FinalisedBucketMutationsApplied, FinalisedStorageRequestRejected,
};
use shc_common::{
    traits::StorageEnableRuntime,
    types::{FileKey, TrieMutation, TrieRemoveMutation},
};
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};

use crate::{
    handler::StorageHubHandler,
    inc_counter,
    metrics::{STATUS_FAILURE, STATUS_SUCCESS},
    types::{MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "msp-delete_file-task";

/// MSP Remove Finalised Files Task: Handles the removal of files from the MSP's File Storage after
/// mutations have been applied and finalised on-chain for one of this MSP's buckets.
///
/// This task reacts to the events:
/// - **[`FinalisedBucketMutationsApplied`] Event:**
///   - Triggered when mutations applied to a Bucket's Forest that's managed by this MSP have been finalised,
///     signalling that certain keys (representing files) should be removed from the File Storage if they are
///     not present in the Bucket's Forest Storage. If the key is still present in the Forest Storage, it sends out
///     a warning, since it could indicate that the key has been re-added after being deleted.
///
/// This task performs the following actions:
/// - Iterates over each removed file key from the mutations.
/// - Checks if the file key is present in the Bucket's Forest Storage.
///   - If the key is still present, it logs a warning,
///     since this could indicate that the key has been re-added after being deleted.
///   - If the key is not present in the Forest Storage, it safely removes the key from the File Storage.
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
        // Remove the file from the File Storage.
        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;
        write_file_storage.delete_file(file_key).map_err(|e| {
					error!(target: LOG_TARGET, "Failed to remove file from File Storage after it was removed from the Bucket's Forest. \\nError: {:?}", e);
					anyhow!(
							"Failed to delete file from File Storage after it was removed from the Bucket's Forest: {:?}",
							e
					)
			})?;

        Ok(())
    }
}

/// Handles the [`FinalisedBucketMutationsApplied`] event.
impl<NT, Runtime> EventHandler<FinalisedBucketMutationsApplied<Runtime>>
    for MspDeleteFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: FinalisedBucketMutationsApplied<Runtime>,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing finalised bucket mutations applied for bucket [{:?}]",
            event.bucket_id
        );

        debug!(target: LOG_TARGET, "Mutations to apply: {:?}", event.mutations);

        for mutation in event.mutations {
            // Get the file key from the mutation.
            let file_key = FileKey::from(mutation.0);

            // Only process remove mutations in this task..
            if mutation.1 != TrieMutation::Remove(TrieRemoveMutation::new()) {
                debug!(target: LOG_TARGET, "Skipping non-remove mutation for file key {:?}", file_key);
                continue;
            }

            // Check that the file key is not in the Bucket's Forest.
            let bucket_forest_key = event.bucket_id.as_ref().to_vec();
            let read_fs = self
                .storage_hub_handler
                .forest_storage_handler
                .get(&bucket_forest_key.into())
                .await
                .ok_or_else(|| {
                    anyhow!(
                        "CRITICAL❗️❗️ Failed to get forest storage for bucket [{:?}].",
                        event.bucket_id
                    )
                })?;
            if read_fs.read().await.contains_file_key(&file_key.into())? {
                warn!(
                    target: LOG_TARGET,
                    "BucketMutationsApplied and finalised for file key {:?} in bucket {:?}, but file key is still in Forest. This can only happen if the same file key was added again after deleted by the user.\\n Mutation: {:?}",
                    file_key,
                    event.bucket_id,
                    mutation
                );
            } else {
                // If file key is not in Forest, we can now safely remove it from the File Storage.
                match self
                    .remove_file_from_file_storage(&file_key.into())
                    .await
                {
                    Ok(_) => inc_counter!(
                        self.storage_hub_handler,
                        msp_files_deleted_total,
                        STATUS_SUCCESS
                    ),
                    Err(e) => {
                        inc_counter!(
                            self.storage_hub_handler,
                            msp_files_deleted_total,
                            STATUS_FAILURE
                        );
                        return Err(e);
                    }
                }
            }
        }

        Ok(())
    }
}

/// Handles the [`FinalisedStorageRequestRejected`] event.
impl<NT, Runtime> EventHandler<FinalisedStorageRequestRejected<Runtime>>
    for MspDeleteFileTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: FinalisedStorageRequestRejected<Runtime>,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing finalised storage request expired for file key {:?} in bucket {:?}",
            event.file_key,
            event.bucket_id
        );

        // Ensure the file key is not present in the bucket's Forest.
        let bucket_forest_key = event.bucket_id.as_ref().to_vec();
        let read_fs = self
            .storage_hub_handler
            .forest_storage_handler
            .get(&bucket_forest_key.into())
            .await
            .ok_or_else(|| {
                anyhow!(
                    "CRITICAL❗️❗️ Failed to get forest storage for bucket [{:?}].",
                    event.bucket_id
                )
            })?;

        if read_fs
            .read()
            .await
            .contains_file_key(&event.file_key.into())?
        {
            warn!(
                target: LOG_TARGET,
                "StorageRequestExpired and finalised for file key {:?} in bucket {:?}, but file key is still in Forest. This can only happen if the same file key was added again after deleted by the user.",
                event.file_key,
                event.bucket_id
            );
        }

        // Check that the file is present in the File Storage.
        let is_in_file_storage = {
            let read_file_storage = self.storage_hub_handler.file_storage.read().await;
            read_file_storage
                .get_metadata(&event.file_key.into())
                .map_err(|e| {
                    error!(target: LOG_TARGET, "Failed to get file metadata from File Storage: {:?}", e);
                    anyhow!("Failed to get file metadata from File Storage: {:?}", e)
                })?
                .is_some()
        };

        if is_in_file_storage {
            // If file is present in File Storage and not in Forest, remove it from File Storage.
            match self
                .remove_file_from_file_storage(&event.file_key.into())
                .await
            {
                Ok(_) => inc_counter!(
                    self.storage_hub_handler,
                    msp_files_deleted_total,
                    STATUS_SUCCESS
                ),
                Err(e) => {
                    inc_counter!(
                        self.storage_hub_handler,
                        msp_files_deleted_total,
                        STATUS_FAILURE
                    );
                    return Err(e);
                }
            }
        } else {
            debug!(
                target: LOG_TARGET,
                "File key {:?} not present in File Storage; skipping removal.",
                event.file_key
            );
        }

        Ok(())
    }
}
