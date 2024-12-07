use std::time::Duration;

use pallet_file_system::types::BucketMoveRequestResponse;
use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::types::Tip;
use shc_blockchain_service::{
    commands::BlockchainServiceInterface, events::MoveBucketRequestedForNewMsp,
};
use shc_common::types::{FileKeyProof, HashT, StorageProofsMerkleTrieLayout};
use shc_file_transfer_service::commands::FileTransferServiceInterface;
use shc_forest_manager::traits::ForestStorage;
use shp_file_metadata::ChunkId;

use crate::services::handler::StorageHubHandler;
use crate::tasks::{FileStorageT, MspForestStorageHandlerT};
use codec::Decode;
use rand::seq::SliceRandom;

const LOG_TARGET: &str = "msp-move-bucket-task";

const DOWNLOAD_REQUEST_RETRY_COUNT: usize = 30;

/// [`MspMoveBucketTask`]: Handles the [`MoveBucketRequestedForNewMsp`] event.
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
}

/// Handles the [`MoveBucketRequestedForNewMsp`] event.
///
/// This event is triggered when an user requests to move a bucket to a new MSP (us).
/// This means that we need to verify if we are able to download the files from the bucket
/// (i.e. have a track record of the files and their BSPs, enough size, etc.) and then accept
/// or reject the request.
///
/// If we accept the request, we need to start downloading the files from the bucket and insert
/// them into our forest storage.
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

        // Get the indexer database pool. If we don't have it, we can't continue.
        let indexer_db_pool = if let Some(indexer_db_pool) =
            self.storage_hub_handler.indexer_db_pool.clone()
        {
            indexer_db_pool
        } else {
            error!(
                target: LOG_TARGET,
                "Indexer is disabled but a move bucket event was received. Please enable indexer or provide a database URL for it to use this feature."
            );

            let call = storage_hub_runtime::RuntimeCall::FileSystem(
                pallet_file_system::Call::msp_respond_move_bucket_request {
                    bucket_id: event.bucket_id,
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

            return Ok(());
        };

        let call = storage_hub_runtime::RuntimeCall::FileSystem(
            pallet_file_system::Call::msp_respond_move_bucket_request {
                bucket_id: event.bucket_id,
                response: BucketMoveRequestResponse::Accepted,
            },
        );

        info!(
            target: LOG_TARGET,
            "MSP: accepting move bucket request for bucket {:?}",
            event.bucket_id,
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

        let bucket = event.bucket_id.as_ref().to_vec();

        let mut indexer_connection = indexer_db_pool.get().await?;

        let forest_storage = self
            .storage_hub_handler
            .forest_storage_handler
            .get_or_create(&event.bucket_id.as_ref().to_vec())
            .await;

        for file in shc_indexer_db::models::File::get_by_onchain_bucket_id(
            &mut indexer_connection,
            bucket.clone(),
        )
        .await?
        {
            info!(
                target: LOG_TARGET,
                "MSP: downloading file {:?} of bucket {:?}",
                file.file_key,
                event.bucket_id,
            );

            let file_metadata = file.to_file_metadata(bucket.clone());
            let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();

            self.storage_hub_handler
                .file_storage
                .write()
                .await
                .insert_file(file_key.clone(), file_metadata.clone())
                .unwrap();

            let chunks_count = file_metadata.chunks_count();

            let mut bsp_peer_ids = file.get_bsp_peer_ids(&mut indexer_connection).await?;

            // Shuffle in order to avoid consecutive requests to the same BSP node.
            bsp_peer_ids.shuffle(&mut rand::thread_rng());

            if bsp_peer_ids.is_empty() {
                error!(
                    target: LOG_TARGET,
                    "No BSP peer IDs found for file {:?} of bucket {:?}",
                    file_key, event.bucket_id,
                );
                continue;
            }

            // We will cycle through all the BSP peer IDs for each chunk until we successfully
            // download the file.
            let mut bsp_peer_ids_iter = bsp_peer_ids.iter().cycle();

            for chunk in 0..chunks_count {
                for _ in 0..DOWNLOAD_REQUEST_RETRY_COUNT {
                    let peer_id = bsp_peer_ids_iter.next().unwrap();

                    let download_request = self
                        .storage_hub_handler
                        .file_transfer
                        .download_request(
                            *peer_id,
                            file_key.into(),
                            ChunkId::new(chunk),
                            Some(event.bucket_id),
                        )
                        .await;

                    let download_request = match download_request {
                        Ok(download_request) => download_request,
                        Err(error) => {
                            error!(
                                target: LOG_TARGET,
                                "Failed to download chunk {:?} of file {:?} from peer {:?}: {:?}",
                                chunk, file_key, peer_id, error
                            );
                            continue;
                        }
                    };

                    let file_key_proof = match FileKeyProof::decode(
                        &mut download_request.file_key_proof.as_ref(),
                    ) {
                        Ok(file_key_proof) => file_key_proof,
                        Err(error) => {
                            error!(
                                target: LOG_TARGET,
                                "Failed to decode file key proof for chunk {:?} of file {:?}: {:?}",
                                chunk, file_key, error
                            );
                            continue;
                        }
                    };

                    let proven = match file_key_proof.proven::<StorageProofsMerkleTrieLayout>() {
                        Ok(chunk_data) => chunk_data,
                        Err(error) => {
                            error!(
                                target: LOG_TARGET,
                                "Failed to get proven data for file key proof: {:?}",
                                error
                            );
                            continue;
                        }
                    };

                    if proven.len() != 1 {
                        error!(
                            target: LOG_TARGET,
                            "Expected exactly one proven chunk but got {}.",
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
                        // TODO: Handle this error better.
                        error!(
                            target: LOG_TARGET,
                            "Failed to write chunk {:?} of file {:?} to storage: {:?}",
                            chunk, file_key, error
                        );
                    } else {
                        // We successfully downloaded the chunk, so we can break out of the retry loop.
                        break;
                    }
                }
            }

            info!(
                target: LOG_TARGET,
                "MSP: inserting downloaded file {:?} of bucket {:?} to forest storage",
                file_key,
                event.bucket_id,
            );

            if let Err(error) = forest_storage
                .write()
                .await
                .insert_files_metadata(&[file_metadata])
            {
                // TODO: Handle this error better.
                error!(
                    target: LOG_TARGET,
                    "Failed to insert file {:?} to forest storage: {:?}",
                    file_key, error
                );
            }
        }

        Ok(())
    }
}
