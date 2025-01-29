use codec::Decode;
use rand::seq::SliceRandom;
use sp_core::H256;
use std::time::Duration;

use pallet_file_system::types::BucketMoveRequestResponse;
use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceInterface, events::MoveBucketRequestedForNewMsp, types::Tip,
};
use shc_common::types::{
    BucketId, FileKeyProof, HashT, ProviderId, StorageProofsMerkleTrieLayout, StorageProviderId,
};
use shc_file_manager::traits::FileStorage;
use shc_file_transfer_service::commands::FileTransferServiceInterface;
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use shp_file_metadata::ChunkId;
use std::cmp::max;
use storage_hub_runtime::StorageDataUnit;

use crate::services::{
    handler::StorageHubHandler,
    types::{MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "msp-move-bucket-task";
const DOWNLOAD_REQUEST_RETRY_COUNT: usize = 30;

/// MSP Move Bucket Task
///
/// This task handles the `MoveBucketRequestedForNewMsp` event which is triggered when a bucket needs to be
/// moved from another MSP to *this* MSP. The task follows this lifecycle:
///
/// 1. Validates prerequisites:
///    - Checks if indexer is enabled and available
///    - Verifies sufficient storage capacity
///    - Ensures bucket data integrity
///
/// 2. Downloads bucket data:
///    - Retrieves chunks from BSPs in a round-robin fashion
///    - Retries failed downloads up to DOWNLOAD_REQUEST_RETRY_COUNT times
///
/// 3. Handles failures:
///    - Rejects move request if prerequisites aren't met
///    - Cleans up partial downloads on failure
///    - Reports errors through appropriate channels
///
/// [`MspMoveBucketTask`]: Handles the [`MoveBucketRequestedForNewMsp`] event.
pub struct MspMoveBucketTask<NT>
where
    NT: ShNodeType,
    NT::FSH: MspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<NT>,
    file_storage_inserted_file_keys: Vec<H256>,
    pending_bucket_id: Option<BucketId>,
}

impl<NT> Clone for MspMoveBucketTask<NT>
where
    NT: ShNodeType,
    NT::FSH: MspForestStorageHandlerT,
{
    fn clone(&self) -> MspMoveBucketTask<NT> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            file_storage_inserted_file_keys: self.file_storage_inserted_file_keys.clone(),
            pending_bucket_id: self.pending_bucket_id.clone(),
        }
    }
}

impl<NT> MspMoveBucketTask<NT>
where
    NT: ShNodeType,
    NT::FSH: MspForestStorageHandlerT,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT>) -> Self {
        Self {
            storage_hub_handler,
            file_storage_inserted_file_keys: Vec::new(),
            pending_bucket_id: None,
        }
    }

    async fn reject_bucket_move(&mut self, bucket_id: BucketId) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "MSP: rejecting move bucket request for bucket {:?}",
            bucket_id.as_ref(),
        );

        for file_key in self.file_storage_inserted_file_keys.iter() {
            if let Err(error) = self
                .storage_hub_handler
                .file_storage
                .write()
                .await
                .delete_file(file_key)
            {
                error!(
                    target: LOG_TARGET,
                    "Failed to delete (move bucket rollback) file {:?} from file storage: {:?}",
                    file_key, error
                );
            }
        }

        if let Some(bucket_id) = self.pending_bucket_id {
            self.storage_hub_handler
                .forest_storage_handler
                .remove_forest_storage(&bucket_id.as_ref().to_vec())
                .await;
        }

        let call = storage_hub_runtime::RuntimeCall::FileSystem(
            pallet_file_system::Call::msp_respond_move_bucket_request {
                bucket_id,
                response: BucketMoveRequestResponse::Rejected,
            },
        );

        self.storage_hub_handler
            .blockchain
            .send_extrinsic(call, Tip::from(0))
            .await?
            .with_timeout(Duration::from_secs(
                self.storage_hub_handler
                    .provider_config
                    .extrinsic_retry_timeout,
            ))
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await?;

        Ok(())
    }

    async fn accept_bucket_move(&self, bucket_id: BucketId) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "MSP: accepting move bucket request for bucket {:?}",
            bucket_id.as_ref(),
        );

        let call = storage_hub_runtime::RuntimeCall::FileSystem(
            pallet_file_system::Call::msp_respond_move_bucket_request {
                bucket_id,
                response: BucketMoveRequestResponse::Accepted,
            },
        );

        self.storage_hub_handler
            .blockchain
            .send_extrinsic(call, Tip::from(0))
            .await?
            .with_timeout(Duration::from_secs(
                self.storage_hub_handler
                    .provider_config
                    .extrinsic_retry_timeout,
            ))
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await?;

        Ok(())
    }

    async fn download_file(
        &self,
        file: &shc_indexer_db::models::File,
        bucket: &BucketId,
    ) -> anyhow::Result<()> {
        let file_metadata = file.to_file_metadata(bucket.as_ref().to_vec());
        let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();

        info!(
            target: LOG_TARGET,
            "MSP: downloading file {:?}",
            file_key,
        );

        let chunks_count = file_metadata.chunks_count();
        let mut bsp_peer_ids = file
            .get_bsp_peer_ids(
                &mut self
                    .storage_hub_handler
                    .indexer_db_pool
                    .as_ref()
                    .unwrap()
                    .get()
                    .await?,
            )
            .await?;

        // Shuffle BSP peer IDs to distribute load evenly across BSPs and prevent always hitting
        // the same BSPs first, which could lead to hotspots
        bsp_peer_ids.shuffle(&mut rand::thread_rng());

        let mut bsp_peer_ids_iter = bsp_peer_ids.iter().cycle();

        for chunk in 0..chunks_count {
            let mut downloaded = false;
            for _ in 0..DOWNLOAD_REQUEST_RETRY_COUNT {
                let peer_id = bsp_peer_ids_iter
                    .next()
                    .expect("peer_id will always be available due to .cycle() iterator");

                let download_request = match self
                    .storage_hub_handler
                    .file_transfer
                    .download_request(
                        *peer_id,
                        file_key.into(),
                        ChunkId::new(chunk),
                        Some(*bucket),
                    )
                    .await
                {
                    Ok(request) => request,
                    Err(error) => {
                        error!(
                            target: LOG_TARGET,
                            "Failed to download chunk {:?} from peer {:?}: {:?}",
                            chunk, peer_id, error
                        );
                        continue;
                    }
                };

                let file_key_proof =
                    match FileKeyProof::decode(&mut download_request.file_key_proof.as_ref()) {
                        Ok(proof) => proof,
                        Err(error) => {
                            error!(
                                target: LOG_TARGET,
                                "Failed to decode file key proof: {:?}",
                                error
                            );
                            continue;
                        }
                    };

                let proven = match file_key_proof.proven::<StorageProofsMerkleTrieLayout>() {
                    Ok(data) => data,
                    Err(error) => {
                        error!(
                            target: LOG_TARGET,
                            "Failed to get proven data: {:?}",
                            error
                        );
                        continue;
                    }
                };

                if proven.len() != 1 {
                    error!(
                        target: LOG_TARGET,
                        "Expected exactly one proven chunk but got {}",
                        proven.len()
                    );
                    continue;
                }

                let chunk_data = proven[0].data.clone();
                let chunk_id = ChunkId::new(chunk);

                if chunk_id != proven[0].key {
                    error!(
                        target: LOG_TARGET,
                        "Expected chunk id {:?} but got {:?}",
                        chunk, proven[0].key
                    );
                    continue;
                }

                if let Err(error) = self
                    .storage_hub_handler
                    .file_storage
                    .write()
                    .await
                    .write_chunk(&file_key, &chunk_id, &chunk_data)
                {
                    error!(
                        target: LOG_TARGET,
                        "Failed to write chunk: {:?}",
                        error
                    );
                } else {
                    downloaded = true;
                    break;
                }
            }

            if !downloaded {
                error!(
                    target: LOG_TARGET,
                    "Failed to download chunk {:?} after {} retries",
                    chunk,
                    DOWNLOAD_REQUEST_RETRY_COUNT
                );
            }
        }

        Ok(())
    }

    async fn check_and_increase_capacity(
        &self,
        required_size: u64,
        own_msp_id: ProviderId,
    ) -> anyhow::Result<()> {
        let available_capacity = self
            .storage_hub_handler
            .blockchain
            .query_available_storage_capacity(own_msp_id)
            .await
            .map_err(|e| {
                let err_msg = format!("Failed to query available storage capacity: {:?}", e);
                error!(target: LOG_TARGET, err_msg);
                anyhow::anyhow!(err_msg)
            })?;

        // Increase storage capacity if the available capacity is less than the required size
        if available_capacity < required_size {
            warn!(
                target: LOG_TARGET,
                "Insufficient storage capacity to accept bucket move. Available: {}, Required: {}",
                available_capacity,
                required_size
            );

            let current_capacity = self
                .storage_hub_handler
                .blockchain
                .query_storage_provider_capacity(own_msp_id)
                .await
                .map_err(|e| {
                    let err_msg = format!("Failed to query storage provider capacity: {:?}", e);
                    error!(target: LOG_TARGET, err_msg);
                    anyhow::anyhow!(err_msg)
                })?;

            let max_storage_capacity = self
                .storage_hub_handler
                .provider_config
                .max_storage_capacity;

            if max_storage_capacity == current_capacity {
                let err_msg =
                    "Reached maximum storage capacity limit. Unable to add more storage capacity.";
                warn!(target: LOG_TARGET, err_msg);
                return Err(anyhow::anyhow!(err_msg));
            }

            let new_capacity = self.calculate_capacity(required_size, current_capacity)?;

            let call = storage_hub_runtime::RuntimeCall::Providers(
                pallet_storage_providers::Call::change_capacity { new_capacity },
            );

            let earliest_change_capacity_block = self
                .storage_hub_handler
                .blockchain
                .query_earliest_change_capacity_block(own_msp_id)
                .await
                .map_err(|e| {
                    error!(
                        target: LOG_TARGET,
                        "Failed to query earliest change capacity block: {:?}", e
                    );
                    anyhow::anyhow!("Failed to query earliest change capacity block: {:?}", e)
                })?;

            // Wait for the earliest block where the capacity can be changed
            self.storage_hub_handler
                .blockchain
                .wait_for_block(earliest_change_capacity_block)
                .await?;

            self.storage_hub_handler
                .blockchain
                .send_extrinsic(call, Tip::from(0))
                .await?
                .with_timeout(Duration::from_secs(60))
                .watch_for_success(&self.storage_hub_handler.blockchain)
                .await?;

            info!(
                target: LOG_TARGET,
                "Increased storage capacity to {:?} bytes",
                new_capacity
            );

            let available_capacity = self
                .storage_hub_handler
                .blockchain
                .query_available_storage_capacity(own_msp_id)
                .await
                .map_err(|e| {
                    error!(
                        target: LOG_TARGET,
                        "Failed to query available storage capacity: {:?}", e
                    );
                    anyhow::anyhow!("Failed to query available storage capacity: {:?}", e)
                })?;

            // Reject bucket move if the new available capacity is still less than required
            if available_capacity < required_size {
                let err_msg =
                    "Increased storage capacity is still insufficient to accept bucket move.";
                warn!(target: LOG_TARGET, "{}", err_msg);
                return Err(anyhow::anyhow!(err_msg));
            }
        }

        Ok(())
    }

    fn calculate_capacity(
        &self,
        required_size: u64,
        current_capacity: StorageDataUnit,
    ) -> Result<StorageDataUnit, anyhow::Error> {
        let jump_capacity = self.storage_hub_handler.provider_config.jump_capacity;
        let jumps_needed = (required_size + jump_capacity - 1) / jump_capacity;
        let jumps = max(jumps_needed, 1);
        let bytes_to_add = jumps * jump_capacity;
        let required_capacity = current_capacity.checked_add(bytes_to_add).ok_or_else(|| {
            anyhow::anyhow!("Reached maximum storage capacity limit. Cannot accept bucket move.")
        })?;

        let max_storage_capacity = self
            .storage_hub_handler
            .provider_config
            .max_storage_capacity;

        let new_capacity = std::cmp::min(required_capacity, max_storage_capacity);

        Ok(new_capacity)
    }
}

impl<NT> EventHandler<MoveBucketRequestedForNewMsp> for MspMoveBucketTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: MoveBucketRequestedForNewMsp) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "MSP: user requested to move bucket {:?} to us",
            event.bucket_id,
        );

        let indexer_db_pool = if let Some(indexer_db_pool) =
            self.storage_hub_handler.indexer_db_pool.clone()
        {
            indexer_db_pool
        } else {
            error!(
                target: LOG_TARGET,
                "Indexer is disabled but a move bucket event was received. Please provide a database URL (and enable indexer) for it to use this feature."
            );
            return self.reject_bucket_move(event.bucket_id).await;
        };

        let mut indexer_connection = match indexer_db_pool.get().await {
            Ok(connection) => connection,
            Err(error) => {
                error!(target: LOG_TARGET, "Failed to get indexer connection after timeout: {:?}", error);
                return self.reject_bucket_move(event.bucket_id).await;
            }
        };
        let bucket = event.bucket_id.as_ref().to_vec();

        let forest_storage = self
            .storage_hub_handler
            .forest_storage_handler
            .get_or_create(&bucket)
            .await;

        self.pending_bucket_id = Some(event.bucket_id);

        let files = shc_indexer_db::models::File::get_by_onchain_bucket_id(
            &mut indexer_connection,
            bucket.clone(),
        )
        .await?;

        let total_size: u64 = files.iter().map(|file| file.size as u64).sum();

        let own_provider_id = self
            .storage_hub_handler
            .blockchain
            .query_storage_provider_id(None)
            .await?;

        let own_msp_id = match own_provider_id {
            Some(StorageProviderId::MainStorageProvider(id)) => id,
            Some(StorageProviderId::BackupStorageProvider(_)) => {
                error!(
                    target: LOG_TARGET,
                    "Current node account is a Backup Storage Provider. Expected a Main Storage Provider ID."
                );
                return self.reject_bucket_move(event.bucket_id).await;
            }
            None => {
                error!(target: LOG_TARGET, "Failed to get own MSP ID.");
                return self.reject_bucket_move(event.bucket_id).await;
            }
        };

        // Check and increase capacity if needed
        if let Err(e) = self
            .check_and_increase_capacity(total_size, own_msp_id)
            .await
        {
            error!(
                target: LOG_TARGET,
                "Failed to ensure sufficient capacity: {:?}", e
            );
            return self.reject_bucket_move(event.bucket_id).await;
        }

        // Try to insert all files before accepting the request
        for file in &files {
            let file_metadata = file.to_file_metadata(bucket.clone());
            let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();

            let result = self
                .storage_hub_handler
                .file_storage
                .write()
                .await
                .insert_file(file_key.clone(), file_metadata.clone());
            if let Err(error) = result {
                error!(
                    target: LOG_TARGET,
                    "Failed to insert file {:?} into file storage: {:?}",
                    file_key, error
                );
                return self.reject_bucket_move(event.bucket_id).await;
            }
            self.file_storage_inserted_file_keys.push(file_key);

            if let Err(error) = forest_storage
                .write()
                .await
                .insert_files_metadata(&[file_metadata.clone()])
            {
                error!(
                    target: LOG_TARGET,
                    "Failed to insert file {:?} into forest storage: {:?}",
                    file_key, error
                );
                return self.reject_bucket_move(event.bucket_id).await;
            }

            let bsp_peer_ids = file.get_bsp_peer_ids(&mut indexer_connection).await?;
            if bsp_peer_ids.is_empty() {
                error!(
                    target: LOG_TARGET,
                    "No BSP peer IDs found for file {:?}",
                    file_key,
                );
                return self.reject_bucket_move(event.bucket_id).await;
            }
        }

        // Accept the request since we've verified we can handle all files
        self.accept_bucket_move(event.bucket_id).await?;

        // Now download all the files
        for file in files {
            if let Err(error) = self.download_file(&file, &event.bucket_id).await {
                error!(
                    target: LOG_TARGET,
                    "Failed to download file: {:?}",
                    error
                );
                // Continue with other files even if one fails
                continue;
            }
        }

        Ok(())
    }
}
