use codec::Decode;
use rand::seq::SliceRandom;
use std::time::Duration;

use pallet_file_system::types::BucketMoveRequestResponse;
use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::types::Tip;
use shc_blockchain_service::{
    commands::BlockchainServiceInterface, events::MoveBucketRequestedForNewMsp,
};
use shc_common::types::{BucketId, FileKeyProof, HashT, StorageProofsMerkleTrieLayout};
use shc_file_transfer_service::commands::FileTransferServiceInterface;
use shc_forest_manager::traits::ForestStorage;
use shp_file_metadata::ChunkId;

use crate::services::handler::StorageHubHandler;
use crate::tasks::{FileStorageT, MspForestStorageHandlerT};

const LOG_TARGET: &str = "msp-move-bucket-task";
const DOWNLOAD_REQUEST_RETRY_COUNT: usize = 30;

pub struct MspMoveBucketTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: MspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<FL, FSH>,
}

impl<FL, FSH> Clone for MspMoveBucketTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: MspForestStorageHandlerT,
{
    fn clone(&self) -> MspMoveBucketTask<FL, FSH> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<FL, FSH> MspMoveBucketTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: MspForestStorageHandlerT,
{
    pub fn new(storage_hub_handler: StorageHubHandler<FL, FSH>) -> Self {
        Self {
            storage_hub_handler,
        }
    }

    async fn reject_bucket_move(&self, bucket_id: BucketId) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "MSP: rejecting move bucket request for bucket {:?}",
            bucket_id.as_ref(),
        );

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

        info!(
            target: LOG_TARGET,
            "MSP: accepting move bucket request for bucket {:?}",
            bucket_id.as_ref(),
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
        bsp_peer_ids.shuffle(&mut rand::thread_rng());

        let mut bsp_peer_ids_iter = bsp_peer_ids.iter().cycle();

        for chunk in 0..chunks_count {
            let mut downloaded = false;
            for _ in 0..DOWNLOAD_REQUEST_RETRY_COUNT {
                let peer_id = bsp_peer_ids_iter.next().unwrap();

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
}

impl<FL, FSH> EventHandler<MoveBucketRequestedForNewMsp> for MspMoveBucketTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: MspForestStorageHandlerT,
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

        let mut indexer_connection = indexer_db_pool.get().await?;
        let bucket = event.bucket_id.as_ref().to_vec();

        let forest_storage = self
            .storage_hub_handler
            .forest_storage_handler
            .get_or_create(&bucket)
            .await;

        let files = shc_indexer_db::models::File::get_by_onchain_bucket_id(
            &mut indexer_connection,
            bucket.clone(),
        )
        .await?;

        // Try to insert all files before accepting the request
        for file in &files {
            let file_metadata = file.to_file_metadata(bucket.clone());
            let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();

            if let Err(error) = self
                .storage_hub_handler
                .file_storage
                .write()
                .await
                .insert_file(file_key.clone(), file_metadata.clone())
            {
                error!(
                    target: LOG_TARGET,
                    "Failed to insert file {:?} into file storage: {:?}",
                    file_key, error
                );
                return self.reject_bucket_move(event.bucket_id).await;
            }

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
