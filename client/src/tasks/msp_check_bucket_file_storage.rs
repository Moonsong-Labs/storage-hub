// Standard library imports
use std::collections::HashSet;

// External crate imports
use anyhow::anyhow;
use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::events::CheckBucketFileStorage;
use shc_common::{
    traits::StorageEnableRuntime,
    types::{BucketId, FileMetadata},
};
use shc_file_manager::traits::{FileStorage, FileStorageError};
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use shc_indexer_db::{models::File, DbPool};
use sp_core::H256;
use tokio::task::JoinSet;

// Project imports
use crate::{
    handler::StorageHubHandler,
    types::{ForestStorageKey, MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "msp-check-bucket-file-storage-task";

/// MSP task that handles [`CheckBucketFileStorage`] events.
///
/// This task verifies that every file referenced by a bucket forest is present and complete in
/// local file storage. When files are missing or incomplete, it attempts recovery by discovering
/// serving BSP peers via the indexer and then delegating downloads to the file download manager.
///
/// Processing flow:
/// - Load all `(file_key, metadata)` entries from the bucket forest
/// - Check each file in local file storage for presence and completion
/// - For missing/incomplete files, query indexer records and collect BSP peer IDs
/// - Register candidate peers in the peer manager and trigger file recovery
/// - Report a final per-bucket recovery summary (recovered/failed/panicked)
///
/// If the forest is empty, or recovery dependencies are unavailable (for example, indexer disabled
/// or unreachable), the task exits gracefully with an informative result string.
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

        // Spawn background tasks to recover missing/incomplete files from BSPs, and await all of them.
        let mut join_set = JoinSet::new();
        let total_recoveries = missing_or_incomplete.len();

        for (file_key, file_metadata) in missing_or_incomplete {
            let indexer_db_pool = indexer_db_pool.clone();
            let bucket_id = bucket_id;
            let task = self.clone();

            join_set.spawn(async move {
                let result = task
                    .recover_file_from_bsps(indexer_db_pool, bucket_id, file_key, file_metadata)
                    .await;
                (file_key, result)
            });
        }

        let mut recovered = 0usize;
        let mut failed = 0usize;
        let mut panicked = 0usize;

        while let Some(join_result) = join_set.join_next().await {
            match join_result {
                Ok((file_key, Ok(true))) => {
                    recovered += 1;
                    debug!(
                        target: LOG_TARGET,
                        "Recovered file [{:x}] for bucket [0x{:x}]",
                        file_key,
                        bucket_id
                    );
                }
                Ok((file_key, Ok(false))) => {
                    failed += 1;
                    error!(
                        target: LOG_TARGET,
                        "Finished recovering file [{:x}] for bucket [0x{:x}] with no errors, but it is still incomplete",
                        file_key,
                        bucket_id
                    );
                }
                Ok((file_key, Err(e))) => {
                    failed += 1;
                    error!(
                        target: LOG_TARGET,
                        "Error recovering file [{:x}] for bucket [0x{:x}]: {:?}",
                        file_key,
                        bucket_id,
                        e
                    );
                }
                Err(e) => {
                    panicked += 1;
                    error!(
                        target: LOG_TARGET,
                        "Recovery task panicked for bucket [0x{:x}]: {:?}",
                        bucket_id,
                        e
                    );
                }
            }
        }

        Ok(format!(
            "Bucket [0x{:x}] recovery finished: recovered={}, failed={}, panicked={}, total={}",
            bucket_id, recovered, failed, panicked, total_recoveries
        ))
    }
}

impl<NT, Runtime> MspCheckBucketFileStorageTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn recover_file_from_bsps(
        &self,
        indexer_db_pool: DbPool,
        bucket_id: BucketId<Runtime>,
        file_key: H256,
        file_metadata: FileMetadata,
    ) -> anyhow::Result<bool> {
        // Ensure the file metadata exists in file storage. If it's missing, insert it so that
        // chunk writes during download have a known file to attach to.
        let is_missing_metadata = {
            let file_storage = self.storage_hub_handler.file_storage.read().await;
            file_storage
                .get_metadata(&file_key)
                .map_err(|e| {
                    anyhow!(
                        "Failed to read file metadata from file storage for file [{:x}]: {:?}",
                        file_key,
                        e
                    )
                })?
                .is_none()
        };

        if is_missing_metadata {
            let mut file_storage = self.storage_hub_handler.file_storage.write().await;
            match file_storage.insert_file(file_key, file_metadata.clone()) {
                Ok(()) => {
                    info!(
                        target: LOG_TARGET,
                        "Inserted missing file metadata for file [{:x}] (bucket [0x{:x}])",
                        file_key,
                        bucket_id
                    );
                }
                Err(FileStorageError::FileAlreadyExists) => {
                    // Another task raced us; that's fine.
                }
                Err(e) => {
                    return Err(anyhow!(
                        "Failed to insert file metadata for file [{:x}] into file storage: {:?}",
                        file_key,
                        e
                    ));
                }
            }
        }

        let mut indexer_connection = indexer_db_pool.get().await.map_err(|e| {
            anyhow!(
                "Failed to get indexer connection to recover file [{:x}] (bucket [0x{:x}]): {:?}",
                file_key,
                bucket_id,
                e
            )
        })?;

        // Query indexer for this specific file key (may return multiple rows due to repeated
        // storage requests for the same file).
        let file_records = File::get_by_file_key(&mut indexer_connection, file_key.as_ref())
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to query indexer by file key [{:x}] (bucket [0x{:x}]): {:?}",
                    file_key,
                    bucket_id,
                    e
                )
            })?;

        if file_records.is_empty() {
            return Err(anyhow!(
                "Indexer returned no records for file [{:x}] (bucket [0x{:x}])",
                file_key,
                bucket_id
            ));
        }

        // For each record, query the BSP peer IDs that can serve it, deduplicating across rows.
        let mut bsp_peer_ids = HashSet::new();
        for record in &file_records {
            match record.get_bsp_peer_ids(&mut indexer_connection).await {
                Ok(peer_ids) => {
                    bsp_peer_ids.extend(peer_ids);
                }
                Err(e) => {
                    warn!(
                        target: LOG_TARGET,
                        "Failed to get BSP peer IDs from indexer for file [{:x}] (bucket [0x{:x}]). There can be BSPs in other file records for the same file key: {:?}",
                        file_key,
                        bucket_id,
                        e
                    );
                }
            }
        }

        if bsp_peer_ids.is_empty() {
            return Err(anyhow!(
                "No BSP peer IDs found in indexer for file [{:x}] (bucket [0x{:x}])",
                file_key,
                bucket_id
            ));
        }

        // Register peers for peer selection in the FileDownloadManager.
        for peer_id in &bsp_peer_ids {
            self.storage_hub_handler
                .peer_manager
                .add_peer(*peer_id, file_key)
                .await;
        }

        // Download missing chunks from BSPs.
        //
        // NOTE: This uses the existing FileDownloadManager logic (peer selection, retries,
        // fingerprint checks, chunk-size validation, persistence, etc.).
        if let Err(e) = self
            .storage_hub_handler
            .file_download_manager
            .download_file(
                file_metadata.clone(),
                bucket_id,
                self.storage_hub_handler.file_transfer.clone(),
                self.storage_hub_handler.file_storage.clone(),
            )
            .await
        {
            return Err(anyhow!(
                "Download attempt failed for file [{:x}] (bucket [0x{:x}]): {:?}",
                file_key,
                bucket_id,
                e
            ));
        }

        // Post-check: if still incomplete, we report failure.
        let is_complete = self
            .storage_hub_handler
            .file_storage
            .read()
            .await
            .is_file_complete(&file_key)
            .map_err(|e| {
                anyhow!(
                    "Failed to check completion status for file [{:x}] after recovery (bucket [0x{:x}]): {:?}",
                    file_key,
                    bucket_id,
                    e
                )
            })?;

        Ok(is_complete)
    }
}
