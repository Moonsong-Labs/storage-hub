use anyhow::anyhow;
use sp_runtime::AccountId32;
use std::time::Duration;

use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceInterface,
    events::{
        FinalisedMspStopStoringBucketInsolventUser, MspStopStoringBucketInsolventUser,
        UserWithoutFunds,
    },
    types::SendExtrinsicOptions,
};
use shc_common::types::{ProviderId, StorageProviderId};
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::ForestStorageHandler;

use crate::services::{
    handler::StorageHubHandler,
    types::{MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "msp-stop-storing-bucket-insolvent-user-task";

/// MSP Stop Storing Bucket for Insolvent User Task: Handles stopping storing all buckets that belong to an insolvent user.
///
/// The task has three handlers:
/// - [`UserWithoutFunds`] and [`MspStopStoringBucketInsolventUser`]: React to the events emitted by the runtime when a user has no funds to pay
/// for their payment streams or when this provider has correctly deleted a bucket from a user without funds.
/// - [`FinalisedMspStopStoringBucketInsolventUser`]: Reacts to the event emitted by the state when the on-chain event `MspStopStoringBucketInsolventUser`
/// gets finalised.
///
/// The flow of each handler is as follows:
/// - Reacting to [`UserWithoutFunds`] and [`MspStopStoringBucketInsolventUser`] event from the runtime:
/// 	- Sends extrinsics to stop storing each bucket for the insolvent user.
///
/// - Reacting to [`FinalisedMspStopStoringBucketInsolventUser`] event from the BlockchainService:
/// 	- Deletes the bucket from the MSP's storage.
/// 	- Deletes all the files in the bucket.
pub struct MspStopStoringInsolventUserTask<NT>
where
    NT: ShNodeType,
    NT::FSH: MspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<NT>,
}

impl<NT> Clone for MspStopStoringInsolventUserTask<NT>
where
    NT: ShNodeType,
    NT::FSH: MspForestStorageHandlerT,
{
    fn clone(&self) -> MspStopStoringInsolventUserTask<NT> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT> MspStopStoringInsolventUserTask<NT>
where
    NT: ShNodeType,
    NT::FSH: MspForestStorageHandlerT,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<NT> EventHandler<UserWithoutFunds> for MspStopStoringInsolventUserTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: UserWithoutFunds) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing UserWithoutFunds for user {:?}",
            event.who
        );

        // Get this MSP's on-chain ID
        let own_provider_id = self
            .storage_hub_handler
            .blockchain
            .query_storage_provider_id(None)
            .await?;

        let msp_on_chain_id =
            match own_provider_id.ok_or_else(|| anyhow!("Failed to get own provider ID"))? {
                StorageProviderId::MainStorageProvider(msp_id) => msp_id,
                _ => return Err(anyhow!("Invalid MSP ID")),
            };

        self.handle_insolvent_user_buckets(event.who, msp_on_chain_id)
            .await
    }
}

impl<NT> EventHandler<MspStopStoringBucketInsolventUser> for MspStopStoringInsolventUserTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
{
    async fn handle_event(
        &mut self,
        event: MspStopStoringBucketInsolventUser,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing MspStopStoringBucketInsolventUser for user {:?}",
            event.owner
        );

        self.handle_insolvent_user_buckets(event.owner, event.msp_id)
            .await
    }
}

/// Handles the `FinalisedMspStopStoringBucketInsolventUser` event.
///
/// This event is triggered when the on-chain event `MspStopStoringBucketInsolventUser` gets finalised,
/// which means the block in which it was emitted has been anchored by a finalised relay chain block.
///
/// This task will:
/// - Delete the bucket from the MSP's storage.
/// - Delete all the files in the bucket.
impl<NT> EventHandler<FinalisedMspStopStoringBucketInsolventUser>
    for MspStopStoringInsolventUserTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
{
    async fn handle_event(
        &mut self,
        event: FinalisedMspStopStoringBucketInsolventUser,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Deleting bucket {:?} for MSP {:?} from file storage since its stop storing event reached finality.",
            event.bucket_id,
            event.msp_id
        );

        // Get the file storage.
        let file_storage = self.storage_hub_handler.file_storage.clone();

        // Get a write-lock on the file storage since we are going to be modifying it by removing all files from a bucket.
        let mut file_storage_write = file_storage.write().await;

        // Delete all files in the bucket from the file storage.
        file_storage_write
            .delete_files_with_prefix(
                &event
                    .bucket_id
                    .as_ref()
                    .try_into()
                    .map_err(|_| anyhow!("Invalid bucket id"))?,
            )
            .map_err(|e| anyhow!("Failed to delete files with prefix: {:?}", e))?;

        // Release the write-lock on the file storage.
        drop(file_storage_write);

        // Delete the bucket from the forest storage.
        self.storage_hub_handler
            .forest_storage_handler
            .remove_forest_storage(&event.bucket_id.as_bytes().to_vec())
            .await;

        Ok(())
    }
}

impl<NT> MspStopStoringInsolventUserTask<NT>
where
    NT: ShNodeType,
    NT::FSH: MspForestStorageHandlerT,
{
    /// Common function to handle querying and sending extrinsics for each bucket of an insolvent user
    async fn handle_insolvent_user_buckets(
        &self,
        insolvent_user: AccountId32,
        msp_id: ProviderId,
    ) -> anyhow::Result<()> {
        // Get all buckets stored by this MSP for the insolvent user directly from the runtime
        let stored_buckets = self
            .storage_hub_handler
            .blockchain
            .query_buckets_for_insolvent_user(msp_id, insolvent_user.clone())
            .await
            .map_err(|e| anyhow!("Failed to query buckets: {:?}", e))?;

        if !stored_buckets.is_empty() {
            info!(
                target: LOG_TARGET,
                "Found {} buckets for insolvent user {:?}, sending stop storing extrinsics",
                stored_buckets.len(),
                insolvent_user
            );

            for bucket_id in stored_buckets {
                // Build the extrinsic to stop storing the bucket of the insolvent user
                let stop_storing_bucket_for_insolvent_user_call =
                    storage_hub_runtime::RuntimeCall::FileSystem(
                        pallet_file_system::Call::msp_stop_storing_bucket_for_insolvent_user {
                            bucket_id: bucket_id.clone(),
                        },
                    );

                // Send the transaction and wait for it to be included in the block
                if let Err(e) = self
                    .storage_hub_handler
                    .blockchain
                    .send_extrinsic(
                        stop_storing_bucket_for_insolvent_user_call,
                        SendExtrinsicOptions::default(),
                    )
                    .await?
                    .with_timeout(Duration::from_secs(
                        self.storage_hub_handler
                            .provider_config
                            .extrinsic_retry_timeout,
                    ))
                    .watch_for_success(&self.storage_hub_handler.blockchain)
                    .await
                {
                    // TODO: Export a list of failed buckets to a file for manual intervention by the operator.
                    error!(
                        target: LOG_TARGET,
                        "Failed to stop storing bucket {:?}. Continuing with next bucket. Error: {:?}",
                        bucket_id,
                        e
                    );
                    continue;
                }

                trace!(target: LOG_TARGET, "Stop storing bucket {:?} for insolvent user submitted successfully", bucket_id);
            }
        }

        Ok(())
    }
}
