use anyhow::anyhow;
use std::time::Duration;

use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceInterface,
    events::{
        FinalisedMspStopStoringBucketInsolventUser, MspStopStoringBucketInsolventUser,
        ProcessStopStoringForInsolventUserRequest, UserWithoutFunds,
    },
    types::{StopStoringForInsolventUserRequest, Tip},
};
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::ForestStorageHandler;
use sp_core::H256;

use crate::services::{
    handler::StorageHubHandler,
    types::{MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "msp-stop-storing-bucket-insolvent-user-task";

/// MSP Stop Storing Bucket for Insolvent User Task: Handles stopping storing all buckets that belong to an insolvent user.
///
/// The task has four handlers:
/// - [`UserWithoutFunds`] and [`MspStopStoringBucketInsolventUser`]: React to the events emitted by the runtime when a user has no funds to pay
/// for their payment streams or when this provider has correctly deleted a bucket from a user without funds.
/// - [`ProcessStopStoringForInsolventUserRequest`]: Reacts to the event emitted by the state when a write-lock can be acquired to process a
/// `StopStoringForInsolventUserRequest`.
/// - [`FinalisedMspStopStoringBucketInsolventUser`]: Reacts to the event emitted by the state when the on-chain event `MspStopStoringBucketInsolventUser`
/// gets finalised.
///
/// The flow of each handler is as follows:
/// - Reacting to [`UserWithoutFunds`] and [`SpStopStoringInsolventUser`] event from the runtime:
/// 	- Queues a request to stop storing a bucket for the insolvent user.
///
/// - Reacting to [`ProcessStopStoringForInsolventUserRequest`] event from the BlockchainService:
/// 	- Calls `msp_stop_storing_bucket_for_insolvent_user` extrinsic from [`pallet_file_system`] for a bucket
/// 	  that the user is storing with this MSP to be able to stop storing it without paying a penalty.
///
/// - Reacting to [`FinalisedMspStopStoringBucketInsolventUser`] event from the BlockchainService:
/// 	- Deletes the bucket from the MSP's storage.
/// 	- Deletes all the files in the bucket.
///
/// This flow works because the result of correctly deleting a bucket in the handler of the [`ProcessStopStoringForInsolventUserRequest`]
/// is the runtime event [`MspStopStoringBucketInsolventUser`], which triggers the handler of the [`MspStopStoringBucketInsolventUser`] event
/// and continues the bucket deletion flow until no more buckets from that user are stored.
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

        // Get the insolvent user from the event.
        let insolvent_user = event.who;

        // Get the indexer database pool. If this MSP is not keeping an indexer, it won't be able to check if it has any buckets for
        // this user and as such wont be able to stop storing them.
        let indexer_db_pool = if let Some(indexer_db_pool) =
            self.storage_hub_handler.indexer_db_pool.clone()
        {
            indexer_db_pool
        } else {
            error!(
                target: LOG_TARGET,
                "Indexer is disabled but a insolvent user event was received. Please provide a database URL (and enable indexer) for it to use this feature."
            );

            return Err(anyhow!("Indexer is disabled but a insolvent user event was received. Please provide a database URL (and enable indexer) for it to use this feature."));
        };

        // Try to connect to the indexer. It's needed to check if there are any buckets for
        // this user that this MSP is storing.
        let mut indexer_connection = indexer_db_pool.get().await?;

        // Get all the buckets this MSP is currently storing for the user.
        let stored_buckets = shc_indexer_db::models::Bucket::get_by_owner(
            &mut indexer_connection,
            insolvent_user.to_string(),
        )
        .await?;

        // If we are, queue up a bucket deletion request for that user.
        if !stored_buckets.is_empty() {
            info!(target: LOG_TARGET, "Buckets found for user {:?}, queueing up bucket stop storing", insolvent_user);
            // Queue a request to stop storing a bucket from the insolvent user.
            self.storage_hub_handler
                .blockchain
                .queue_stop_storing_for_insolvent_user_request(
                    StopStoringForInsolventUserRequest::new(insolvent_user),
                )
                .await?;
        }

        Ok(())
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

        // Get the insolvent user from the event.
        let insolvent_user = event.owner;

        // Get the indexer database pool. If this MSP is not keeping an indexer, it won't be able to check if it has any buckets for
        // this user and as such wont be able to stop storing them.
        let indexer_db_pool = if let Some(indexer_db_pool) =
            self.storage_hub_handler.indexer_db_pool.clone()
        {
            indexer_db_pool
        } else {
            error!(
                target: LOG_TARGET,
                "Indexer is disabled but a insolvent user event was received. Please provide a database URL (and enable indexer) for it to use this feature."
            );

            return Err(anyhow!("Indexer is disabled but a insolvent user event was received. Please provide a database URL (and enable indexer) for it to use this feature."));
        };

        // Try to connect to the indexer. It's needed to check if there are any buckets for
        // this user that this MSP is storing.
        let mut indexer_connection = indexer_db_pool.get().await?;

        // Get all the buckets this MSP is currently storing for the user.
        let stored_buckets = shc_indexer_db::models::Bucket::get_by_owner(
            &mut indexer_connection,
            insolvent_user.to_string(),
        )
        .await?;

        // If we are, queue up a bucket deletion request for that user.
        if !stored_buckets.is_empty() {
            info!(target: LOG_TARGET, "Buckets found for user {:?}, queueing up bucket stop storing", insolvent_user);
            // Queue a request to stop storing a bucket from the insolvent user.
            self.storage_hub_handler
                .blockchain
                .queue_stop_storing_for_insolvent_user_request(
                    StopStoringForInsolventUserRequest::new(insolvent_user),
                )
                .await?;
        }

        Ok(())
    }
}

/// Handles the `ProcessStopStoringForInsolventUserRequest` event.
///
/// This event is triggered whenever a Forest write-lock can be acquired to process a `StopStoringForInsolventUserRequest`
/// after receiving either a `UserWithoutFunds` or `MspStopStoringBucketInsolventUser` event from the runtime.\
/// This task will:
/// - Stop storing the bucket for the insolvent user.
/// - Delete the bucket from the forest storage.
impl<NT> EventHandler<ProcessStopStoringForInsolventUserRequest>
    for MspStopStoringInsolventUserTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
{
    async fn handle_event(
        &mut self,
        event: ProcessStopStoringForInsolventUserRequest,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing StopStoringForInsolventUserRequest for user: {:?}",
            event.data.who,
        );

        // Get the insolvent user from the event.
        let insolvent_user = event.data.who;

        // Get a write-lock on the forest root since we are going to be modifying it by removing a user's bucket.
        let forest_root_write_tx = match event.forest_root_write_tx.lock().await.take() {
            Some(tx) => tx,
            None => {
                error!(target: LOG_TARGET, "CRITICAL❗️❗️ This is a bug! Forest root write tx already taken. This is a critical bug. Please report it to the StorageHub team.");
                return Err(anyhow!(
                    "CRITICAL❗️❗️ This is a bug! Forest root write tx already taken!"
                ));
            }
        };

        // Get the indexer database pool. If this MSP is not keeping an indexer, it won't be able to check if it has any buckets for
        // this user and as such wont be able to stop storing them.
        let indexer_db_pool = if let Some(indexer_db_pool) =
            self.storage_hub_handler.indexer_db_pool.clone()
        {
            indexer_db_pool
        } else {
            error!(
                target: LOG_TARGET,
                "Indexer is disabled but a insolvent user event was received. Please provide a database URL (and enable indexer) for it to use this feature."
            );

            return Err(anyhow!("Indexer is disabled but a insolvent user event was received. Please provide a database URL (and enable indexer) for it to use this feature."));
        };

        // Try to connect to the indexer. It's needed to check if there are any buckets for
        // this user that this MSP is storing.
        let mut indexer_connection = indexer_db_pool.get().await?;

        // Get all the buckets this MSP is currently storing for the user.
        let stored_buckets = shc_indexer_db::models::Bucket::get_by_owner(
            &mut indexer_connection,
            insolvent_user.to_string(),
        )
        .await?;

        // Try to get the forest storage for a bucket from the list.
        // Return the bucket ID of the first one that succeeds, or exit early if none are found. This is done because the indexed buckets
        // could have already been deleted from the forest storage but not from the indexer yet if finality has not been reached.
        let bucket_id = {
            let mut bucket_id_found = None;
            for bucket in stored_buckets {
                let bucket_id = bucket.onchain_bucket_id.clone();
                if let Some(_) = self
                    .storage_hub_handler
                    .forest_storage_handler
                    .get(&bucket_id)
                    .await
                {
                    bucket_id_found = Some(H256::from_slice(&bucket_id));
                    break;
                }
            }

            if let Some(bucket_id) = bucket_id_found {
                bucket_id
            } else {
                info!(target: LOG_TARGET, "No valid forest storage found for any indexed bucket. Exiting task.");
                return Ok(());
            }
        };

        // Build the extrinsic to stop storing the bucket of the insolvent user.
        let stop_storing_bucket_for_insolvent_user_call =
            storage_hub_runtime::RuntimeCall::FileSystem(
                pallet_file_system::Call::msp_stop_storing_bucket_for_insolvent_user { bucket_id },
            );

        // Send the transaction and wait for it to be included in the block, continue only if it is successful.
        self.storage_hub_handler
            .blockchain
            .send_extrinsic(stop_storing_bucket_for_insolvent_user_call, Tip::from(0))
            .await?
            .with_timeout(Duration::from_secs(
                self.storage_hub_handler
                    .provider_config
                    .extrinsic_retry_timeout,
            ))
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await?;

        trace!(target: LOG_TARGET, "Stop storing bucket for insolvent user submitted successfully");

        // Release the forest root write "lock" since the on-chain bucket root has been deleted, and finish the task.
        self.storage_hub_handler
            .blockchain
            .release_forest_root_write_lock(forest_root_write_tx)
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

        // Delete the bucket from the forest storage.
        self.storage_hub_handler
            .forest_storage_handler
            .remove_forest_storage(&event.bucket_id.as_bytes().to_vec())
            .await;

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

        Ok(())
    }
}
