use anyhow::anyhow;
use sc_tracing::tracing::*;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceInterface,
    events::{MoveBucketAccepted, MoveBucketExpired, MoveBucketRejected, MoveBucketRequested},
};
use shc_file_transfer_service::commands::FileTransferServiceInterface;

use crate::services::handler::StorageHubHandler;
use crate::tasks::{BspForestStorageHandlerT, FileStorageT};

const LOG_TARGET: &str = "bsp-move-bucket-task";

/// TODO DOCS
pub struct BspMoveBucketTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<FL, FSH>,
}

impl<FL, FSH> Clone for BspMoveBucketTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    fn clone(&self) -> BspMoveBucketTask<FL, FSH> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<FL, FSH> BspMoveBucketTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    pub fn new(storage_hub_handler: StorageHubHandler<FL, FSH>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

/// Handles the [`MoveBucketRequested`] event.
///
/// TODO DOCS
impl<FL, FSH> EventHandler<MoveBucketRequested> for BspMoveBucketTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: MoveBucketRequested) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "MoveBucketRequested: BSP will accept download requests for files in bucket {:?} from MSP {:?}",
            event.bucket_id,
            event.new_msp_id
        );

        let multiaddress_vec = self
            .storage_hub_handler
            .blockchain
            .query_provider_multiaddresses(event.new_msp_id)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to query MSP multiaddresses of MSP ID {:?}\n Error: {:?}",
                    event.new_msp_id,
                    e
                )
            })?;

        let peer_ids = self
            .storage_hub_handler
            .file_transfer
            .extract_peer_ids_and_register_known_addresses(multiaddress_vec)
            .await;

        for peer_id in peer_ids {
            self.storage_hub_handler
                .file_transfer
                .register_new_bucket_peer(peer_id, event.bucket_id)
                .await
                .map_err(|e| anyhow!("Failed to register new bucket peer: {:?}", e))?;
        }

        Ok(())
    }
}

/// Handles the [`MoveBucketAccepted`] event.
///
/// TODO DOCS
impl<FL, FSH> EventHandler<MoveBucketAccepted> for BspMoveBucketTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: MoveBucketAccepted) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "MoveBucketAccepted: New MSP {:?} accepted move bucket request for bucket {:?}. Will keep accepting download requests for a window of time.",
            event.msp_id,
            event.bucket_id
        );

        self.storage_hub_handler
            .file_transfer
            .unregister_bucket(event.bucket_id)
            .await
            .map_err(|e| anyhow!("Failed to unregister bucket: {:?}", e))?;

        Ok(())
    }
}

/// Handles the [`MoveBucketRejected`] event.
///
/// TODO DOCS
impl<FL, FSH> EventHandler<MoveBucketRejected> for BspMoveBucketTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: MoveBucketRejected) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "MoveBucketRejected: BSP will no longer accept download requests for files in bucket {:?} from MSP {:?}",
            event.bucket_id,
            event.msp_id
        );

        self.storage_hub_handler
            .file_transfer
            .unregister_bucket(event.bucket_id)
            .await
            .map_err(|e| anyhow!("Failed to unregister bucket: {:?}", e))?;

        Ok(())
    }
}

/// Handles the [`MoveBucketExpired`] event.
///
/// TODO DOCS
impl<FL, FSH> EventHandler<MoveBucketExpired> for BspMoveBucketTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: MoveBucketExpired) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "MoveBucketExpired: BSP will no longer accept download requests for files in bucket {:?} from MSP {:?}",
            event.bucket_id,
            event.msp_id
        );

        self.storage_hub_handler
            .file_transfer
            .unregister_bucket(event.bucket_id)
            .await
            .map_err(|e| anyhow!("Failed to unregister bucket: {:?}", e))?;

        Ok(())
    }
}
