use anyhow::anyhow;
use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::events::CheckBucketFileStorage;
use shc_common::{traits::StorageEnableRuntime, types::FileMetadata};
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use sp_core::H256;

use crate::{
    handler::StorageHubHandler,
    types::{ForestStorageKey, MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "msp-check-bucket-file-storage-task";

/// MSP task that handles [`CheckBucketFileStorage`] events.
///
/// This is boilerplate wiring only. Behaviour will be implemented separately.
/// TODO: DOCUMENT THIS TASK
pub struct MspCheckBucketFileStorageTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for MspCheckBucketFileStorageTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> Self {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, Runtime> MspCheckBucketFileStorageTask<NT, Runtime>
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
}

impl<NT, Runtime> EventHandler<CheckBucketFileStorage<Runtime>>
    for MspCheckBucketFileStorageTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: CheckBucketFileStorage<Runtime>,
    ) -> anyhow::Result<String> {
        let bucket_id = event.bucket_id;

        info!(
            target: LOG_TARGET,
            "Checking all files are present in file storage for bucket [0x{:x}]",
            bucket_id
        );

        // Collect all file keys from the local forest for this bucket.
        let bucket_forest_key = ForestStorageKey::from(bucket_id.as_ref().to_vec());
        let bucket_forest_storage = self
            .storage_hub_handler
            .forest_storage_handler
            .get(&bucket_forest_key)
            .await
            .ok_or_else(|| {
                anyhow!(
                    "Failed to get forest storage for bucket [0x{:x}]",
                    bucket_id
                )
            })?;

        let forest_files = bucket_forest_storage
            .read()
            .await
            .get_all_files()
            .map_err(|e| {
                anyhow!(
                    "Failed to enumerate forest files for bucket [0x{:x}]: {:?}",
                    bucket_id,
                    e
                )
            })?;

        if forest_files.is_empty() {
            return Ok(format!(
                "Bucket [0x{:x}] forest is empty; nothing to check",
                bucket_id
            ));
        }

        // Iterate through all forest files and check if they are present and complete in file storage.
        let mut missing_or_incomplete: Vec<(H256, FileMetadata)> = Vec::new();
        {
            for (file_key, file_metadata) in &forest_files {
                // Getting read lock on file storage at every iteration to avoid holding the lock for too long.
                let file_storage = self.storage_hub_handler.file_storage.read().await;

                let stored_metadata = file_storage.get_metadata(file_key).map_err(|e| {
                    warn!(target: LOG_TARGET, "Failed to get file metadata from file storage, for file key [{:x}] in bucket [0x{:x}]. Treating as missing: {:?}", file_key, bucket_id, e)
                }).unwrap_or(None);

                match stored_metadata {
                    Some(_) => {
                        let is_complete = file_storage.is_file_complete(file_key).map_err(|e| {
                            warn!(target: LOG_TARGET, "Failed to check completion status for file [{:x}] in bucket [0x{:x}]: {:?}", file_key, bucket_id, e)
                        }).unwrap_or(false);

                        if !is_complete {
                            warn!(
                                target: LOG_TARGET,
                                "File [{:x}] is present but incomplete in file storage (bucket [0x{:x}])",
                                file_key,
                                bucket_id
                            );
                            missing_or_incomplete.push((*file_key, file_metadata.clone()));
                        }
                    }
                    None => {
                        warn!(
                            target: LOG_TARGET,
                            "File [{:x}] is missing from file storage (bucket [0x{:x}])",
                            file_key,
                            bucket_id
                        );
                        missing_or_incomplete.push((*file_key, file_metadata.clone()));
                    }
                }
            }
        }

        // Return early if all files are present and complete in file storage.
        if missing_or_incomplete.is_empty() {
            return Ok(format!(
                "Bucket [0x{:x}] OK: all {} forest files are present and complete in file storage",
                bucket_id,
                forest_files.len()
            ));
        }

        // We can only find get the BSPs that can serve the missing/incomplete files if the indexer is enabled.
        let indexer_db_pool = match self.storage_hub_handler.indexer_db_pool.clone() {
            Some(pool) => pool,
            None => {
                warn!(
                    target: LOG_TARGET,
                    "Bucket [0x{:x}] has {} missing/incomplete files but indexer is disabled; cannot schedule downloads",
                    bucket_id,
                    missing_or_incomplete.len()
                );
                return Ok(format!(
                    "Bucket [0x{:x}] has {} missing/incomplete files but indexer is disabled; cannot schedule downloads",
                    bucket_id,
                    missing_or_incomplete.len()
                ));
            }
        };

        // Acquire a connection to ensure the indexer is reachable before spawning background work.
        if let Err(e) = indexer_db_pool.get().await {
            warn!(
                target: LOG_TARGET,
                "Bucket [0x{:x}] has {} missing/incomplete files but failed to connect to indexer; cannot schedule downloads. Error: {:?}",
                bucket_id,
                missing_or_incomplete.len(),
                e
            );
            return Ok(format!(
                "Bucket [0x{:x}] has {} missing/incomplete files but failed to connect to indexer; cannot schedule downloads",
                bucket_id,
                missing_or_incomplete.len()
            ));
        }

        // Spawn background tasks to recover missing/incomplete files from BSPs.
        for (file_key, file_metadata) in missing_or_incomplete {
            let indexer_db_pool = indexer_db_pool.clone();
            let bucket_id = bucket_id.clone();

            tokio::spawn(async move {
                // TODO: Query indexer for BSP peer IDs that can serve this file.
                // TODO: Download file chunks from BSPs and persist into file storage.
                let _ = (indexer_db_pool, bucket_id, file_key, file_metadata);
            });
        }

        Ok(format!(
            "Bucket [0x{:x}] has missing/incomplete files; spawned background recovery tasks",
            bucket_id
        ))
    }
}
