use anyhow::anyhow;
use futures::future::join_all;
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;

use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface,
    events::{FinalisedMspStopStoringBucketInsolventUser, UserWithoutFunds},
    types::SendExtrinsicOptions,
};
use shc_common::types::StorageProviderId;
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::ForestStorageHandler;
use sp_core::H256;

use crate::services::{
    handler::StorageHubHandler,
    types::{MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "msp-stop-storing-bucket-insolvent-user-task";

/// Maximum number of stop storing bucket extrinsics to send concurrently.
const MAX_CONCURRENT_STOP_STORING_EXTRINSICS: usize = 20;

/// MSP Stop Storing Bucket for Insolvent User Task: Handles stopping storing all buckets that belong to an insolvent user.
///
/// The task has two handlers:
/// - [`UserWithoutFunds`]: Reacts to the event emitted by the runtime when a user has no funds to pay
/// for their payment streams.
/// - [`FinalisedMspStopStoringBucketInsolventUser`]: Reacts to the event emitted by the state when the on-chain event `MspStopStoringBucketInsolventUser`
/// gets finalised.
///
/// The flow of each handler is as follows:
/// - Reacting to [`UserWithoutFunds`] event from the runtime:
///     - Gets the insolvent user from the event.
///     - Gets all buckets stored by this MSP for the insolvent user.
///     - If there are buckets stored by this MSP for the insolvent user:
/// 		- Creates a semaphore to allow sending parallel stop storing bucket extrinsics.
/// 		- Spawns a task per bucket to stop storing each one.
/// 		- Waits for all the stop storing bucket tasks to complete.
/// 		- If any of the stop storing bucket tasks fail, returns an error.
/// 		- If all the stop storing bucket tasks succeed, logs the success.
///     - If there are no buckets stored by this MSP for the insolvent user, logs that there is nothing to do.
///
/// - Reacting to [`FinalisedMspStopStoringBucketInsolventUser`] event from the BlockchainService:
/// 	- Deletes the bucket from the MSP's forest storage.
/// 	- Deletes all the files that were in the bucket from the MSP's file storage.
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
            "Processing UserWithoutFunds for user {:?}. Stopping storing all buckets for the insolvent user.",
            event.who
        );

        // Get the insolvent user from the event.
        let insolvent_user = event.who.clone();

        // Get this MSP's ID.
        let own_provider_id = self
            .storage_hub_handler
            .blockchain
            .query_storage_provider_id(None)
            .await?;

        let msp_id =
            match own_provider_id.ok_or_else(|| anyhow!("Failed to get own provider ID"))? {
                StorageProviderId::MainStorageProvider(msp_id) => msp_id,
                _ => return Err(anyhow!("Invalid MSP ID")),
            };

        // Get all buckets stored by this MSP for the insolvent user according to the runtime.
        let stored_buckets = self
            .storage_hub_handler
            .blockchain
            .query_buckets_of_user_stored_by_msp(msp_id, insolvent_user.clone())
            .await
            .map_err(|e| anyhow!("Failed to query buckets: {:?}", e))?;

        // Check if this MSP is currently storing any buckets that belong to the insolvent user.
        if !stored_buckets.is_empty() {
            let amount_of_buckets_to_stop_storing = stored_buckets.len();
            info!(
                target: LOG_TARGET,
                "Found {} buckets for insolvent user {:?}, sending stop storing extrinsics.",
                amount_of_buckets_to_stop_storing,
                insolvent_user
            );

            // Create a semaphore to allow sending parallel stop storing bucket extrinsics.
            let stop_storing_bucket_semaphore =
                Arc::new(Semaphore::new(MAX_CONCURRENT_STOP_STORING_EXTRINSICS));
            let stop_storing_bucket_tasks: Vec<_> = stored_buckets
                .into_iter()
                .map(|bucket_id| {
                    // Clone the semaphore and task for each bucket.
                    let semaphore = Arc::clone(&stop_storing_bucket_semaphore);
                    let task = self.clone();

                    // Spawn a task to stop storing the bucket.
                    tokio::spawn(async move {
                        // Try to acquire the semaphore. This is done to avoid having more concurrent tasks than the set limit.
                        let _permit = semaphore
                            .acquire()
                            .await
                            .map_err(|e| anyhow!("Failed to acquire file semaphore: {:?}", e))?;

                        // Stop storing the bucket using the existing stop_storing_bucket_for_insolvent_user method.
                        task.stop_storing_bucket_for_insolvent_user(&bucket_id)
                            .await
                    })
                })
                .collect();

            // Wait for all the stop storing bucket tasks to complete.
            let results = join_all(stop_storing_bucket_tasks).await;

            // Process results and count failures
            let mut failed_stop_storing_buckets = 0;
            for result in results {
                match result {
                    Ok(stop_storing_result) => {
                        if let Err(e) = stop_storing_result {
                            error!(
                                target: LOG_TARGET,
                                "Stop storing bucket for insolvent user task failed: {:?}", e
                            );
                            failed_stop_storing_buckets += 1;
                        }
                    }
                    Err(e) => {
                        error!(
                            target: LOG_TARGET,
                            "Stop storing bucket for insolvent user task panicked: {:?}", e
                        );
                        failed_stop_storing_buckets += 1;
                    }
                }
            }

            if failed_stop_storing_buckets > 0 {
                return Err(anyhow!(
                    "Failed to stop storing {} out of {} buckets for insolvent user {:?}",
                    failed_stop_storing_buckets,
                    amount_of_buckets_to_stop_storing,
                    insolvent_user
                ));
            } else {
                info!(
                    target: LOG_TARGET,
                    "Successfully completed the task of stop storing all buckets for the insolvent user {:?}",
                    insolvent_user
                );
            }
        } else {
            info!(
                target: LOG_TARGET,
                "No buckets found for insolvent user {:?}. Nothing to do.",
                insolvent_user
            );
        }

        Ok(())
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
    /// Common function to handle submitting an extrinsic to stop storing a bucket that belongs to an insolvent user.
    async fn stop_storing_bucket_for_insolvent_user(&self, bucket_id: &H256) -> anyhow::Result<()> {
        // Build the extrinsic to stop storing the bucket of the insolvent user
        let stop_storing_bucket_for_insolvent_user_call =
            storage_hub_runtime::RuntimeCall::FileSystem(
                pallet_file_system::Call::msp_stop_storing_bucket_for_insolvent_user {
                    bucket_id: *bucket_id,
                },
            );

        // Send the transaction and wait for it to be included in the block.
        if let Err(e) = self
            .storage_hub_handler
            .blockchain
            .send_extrinsic(
                stop_storing_bucket_for_insolvent_user_call,
                SendExtrinsicOptions::new(Duration::from_secs(
                    self.storage_hub_handler
                        .provider_config
                        .blockchain_service
                        .extrinsic_retry_timeout,
                )),
            )
            .await?
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await
        {
            Err(anyhow!(
				"Failed to submit extrinsic to stop storing bucket {:?} for insolvent user. Error: {:?}",
				bucket_id,
				e
			))
        } else {
            trace!(target: LOG_TARGET, "Stop storing bucket {:?} for insolvent user submitted successfully and included in block.", bucket_id);
            Ok(())
        }
    }
}
