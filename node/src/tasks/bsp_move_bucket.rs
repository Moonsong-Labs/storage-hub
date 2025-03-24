use anyhow::anyhow;
use sc_tracing::tracing::*;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceInterface,
    events::{MoveBucketAccepted, MoveBucketExpired, MoveBucketRejected, MoveBucketRequested},
};
use shc_file_transfer_service::commands::{
    FileTransferServiceCommandInterface, FileTransferServiceCommandInterfaceExt,
};

use crate::services::{
    handler::StorageHubHandler,
    types::{BspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "bsp-move-bucket-task";

const MOVE_BUCKET_ACCEPTED_GRACE_PERIOD_SECONDS: u64 = 4 * 60 * 60; // 4 hours

/// Task that handles the [`MoveBucketRequested`], [`MoveBucketAccepted`], [`MoveBucketRejected`]
/// and [`MoveBucketExpired`] events from the BSP point of view.
pub struct BspMoveBucketTask<NT>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<NT>,
}

impl<NT> Clone for BspMoveBucketTask<NT>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
{
    fn clone(&self) -> BspMoveBucketTask<NT> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT> BspMoveBucketTask<NT>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

/// Handles the [`MoveBucketRequested`] event.
///
/// This event is triggered when an user requests to move a bucket to a new MSP.
/// As a BSP, we need to allow the new MSP to download the files we have from the bucket.
impl<NT> EventHandler<MoveBucketRequested> for BspMoveBucketTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
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
/// This event is triggered when the new MSP accepts the move bucket request.
/// This does not mean that the move bucket request is complete, but that the new MSP has committed.
/// For this to be complete, we need to wait for the new MSP to download all the files from the
/// bucket.
impl<NT> EventHandler<MoveBucketAccepted> for BspMoveBucketTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: MoveBucketAccepted) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "MoveBucketAccepted: New MSP {:?} accepted move bucket request for bucket {:?} from old MSP {:?}. Will keep accepting download requests for a window of time.",
            event.new_msp_id,
            event.bucket_id,
            event.old_msp_id
        );

        self.storage_hub_handler
            .file_transfer
            .schedule_unregister_bucket(
                event.bucket_id,
                Some(MOVE_BUCKET_ACCEPTED_GRACE_PERIOD_SECONDS),
            )
            .await
            .map_err(|e| anyhow!("Failed to unregister bucket: {:?}", e))?;

        Ok(())
    }
}

/// Handles the [`MoveBucketRejected`] event.
///
/// This event is triggered when the new MSP rejects the move bucket request.
/// In this case, we need to stop accepting download requests for the bucket.
impl<NT> EventHandler<MoveBucketRejected> for BspMoveBucketTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: MoveBucketRejected) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "MoveBucketRejected: BSP will no longer accept download requests for files in bucket {:?} from MSP {:?}",
            event.bucket_id,
            event.new_msp_id
        );

        self.storage_hub_handler
            .file_transfer
            .schedule_unregister_bucket(event.bucket_id, None)
            .await
            .map_err(|e| anyhow!("Failed to unregister bucket: {:?}", e))?;

        Ok(())
    }
}

/// Handles the [`MoveBucketExpired`] event.
///
/// This event is triggered when the move bucket request expires.
/// In this case, we need to stop accepting download requests for the bucket.
impl<NT> EventHandler<MoveBucketExpired> for BspMoveBucketTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: MoveBucketExpired) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "MoveBucketExpired: BSP will no longer accept download requests for files in bucket {:?}",
            event.bucket_id,
        );

        self.storage_hub_handler
            .file_transfer
            .schedule_unregister_bucket(event.bucket_id, None)
            .await
            .map_err(|e| anyhow!("Failed to unregister bucket: {:?}", e))?;

        Ok(())
    }
}
