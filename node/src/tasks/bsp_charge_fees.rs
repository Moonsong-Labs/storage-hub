use anyhow::anyhow;
use std::time::Duration;

use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceInterface,
    events::{
        LastChargeableInfoUpdated, ProcessStopStoringForInsolventUserRequest,
        SpStopStoringInsolventUser, UserWithoutFunds,
    },
    types::{StopStoringForInsolventUserRequest, Tip},
};
use shc_common::{consts::CURRENT_FOREST_KEY, types::MaxUsersToCharge};
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use sp_core::{Get, H256};
use storage_hub_runtime::Balance;

use crate::services::{
    handler::StorageHubHandler,
    types::{BspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "bsp-charge-fees-task";
const MIN_DEBT: Balance = 0;

/// BSP Charge Fees Task: Handles the debt collection from users served by a BSP.
///
/// The task has four handlers:
/// - [`LastChargeableInfoUpdated`]: Reacts to the event emitted by the runtime when a proof is accepted
///  for a provider and users' fees are going to be charged.
/// - [`UserWithoutFunds`] and [`SpStopStoringInsolventUser`]: Reacts to the event emitted by the runtime when a user has no funds to pay
/// for their payment streams or when this provider has correctly deleted a file from a user without funds.
/// - [`ProcessStopStoringForInsolventUserRequest`]: Reacts to the event emitted by the state when a write-lock can be acquired to process a
/// `StopStoringForInsolventUserRequest`.
///
/// The flow of each handler is as follows:
/// - Reacting to [`LastChargeableInfoUpdated`] event from the runtime:
///     - Calls a Runtime API to retrieve a list of users with debt over a certain custom threshold.
///     - For each user, submits an extrinsic to [`pallet_payment_streams`] to charge them.
///
/// - Reacting to [`UserWithoutFunds`] and [`SpStopStoringInsolventUser`] event from the runtime:
/// 	- Queues a request to stop storing a file for the insolvent user.
///
/// - Reacting to [`ProcessStopStoringForInsolventUserRequest`] event from the BlockchainService:
/// 	- Calls `stop_storing_for_insolvent_user` extrinsic from [`pallet_file_system`] for a file
/// 	  that the user is storing with this BSP to be able to stop storing those files without
/// 	  paying a penalty.
///  	- If the file was the last one, calls `charge_payment_streams` extrinsic from [`pallet_payment_streams`]
/// 	  to charge the user for the debt they have.
///
/// This flow works because the result of correctly deleting a file in the handler of the [`ProcessStopStoringForInsolventUserRequest`]
/// is the runtime event [`SpStopStoringInsolventUser`], which triggers the handler of the [`SpStopStoringInsolventUser`] event and continues
/// the file deletion flow until no more files from that user are stored.
pub struct BspChargeFeesTask<NT>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<NT>,
}

impl<NT> Clone for BspChargeFeesTask<NT>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
{
    fn clone(&self) -> BspChargeFeesTask<NT> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT> EventHandler<LastChargeableInfoUpdated> for BspChargeFeesTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: LastChargeableInfoUpdated) -> anyhow::Result<()> {
        info!(target: LOG_TARGET, "A proof was accepted for provider {:?} and users' fees are going to be charged.", event.provider_id);

        // TODO: Allow for customizable threshold, for example using YAML files.
        // Retrieves users with debt over the `min_debt` threshold
        // using a Runtime API.
        let users_with_debt = self
            .storage_hub_handler
            .blockchain
            .query_users_with_debt(event.provider_id, MIN_DEBT)
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to retrieve users with debt from the runtime: {:?}",
                    e
                )
            })?;

        // Divides the users to charge in chunks of MaxUsersToCharge to avoid exceeding the block limit.
        // Calls the `charge_multiple_users_payment_streams` extrinsic for each chunk in the list to be charged.
        // Logs an error in case of failure and continues.
        let user_chunk_size = <MaxUsersToCharge as Get<u32>>::get();
        for users_chunk in users_with_debt.chunks(user_chunk_size as usize) {
            let call = storage_hub_runtime::RuntimeCall::PaymentStreams(
                pallet_payment_streams::Call::charge_multiple_users_payment_streams {
                    user_accounts: users_chunk.to_vec().try_into().expect("Chunk size is the same as MaxUsersToCharge, it has to fit in the BoundedVec"),
                },
            );

            let charging_result = self
                .storage_hub_handler
                .blockchain
                .send_extrinsic(call, Tip::from(0))
                .await;

            match charging_result {
                Ok(submitted_transaction) => {
                    info!(target: LOG_TARGET, "Submitted extrinsic to charge users with debt: {}", submitted_transaction.hash());
                }
                Err(e) => {
                    error!(target: LOG_TARGET, "Failed to send extrinsic to charge users with debt: {}", e);
                }
            }
        }

        Ok(())
    }
}

impl<NT> EventHandler<UserWithoutFunds> for BspChargeFeesTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: UserWithoutFunds) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing UserWithoutFunds for user {:?}",
            event.who
        );

        // Get the insolvent user from the event.
        let insolvent_user = event.who;

        // Get the current Forest key of the Provider running this node.
        let current_forest_key = CURRENT_FOREST_KEY.to_vec();

        // Check if we are storing any file for this user.
        let fs = self
            .storage_hub_handler
            .forest_storage_handler
            .get(&current_forest_key)
            .await
            .ok_or_else(|| anyhow!("Failed to get forest storage."))?;

        let user_files = fs
            .read()
            .await
            .get_files_by_user(&insolvent_user)
            .map_err(|e| anyhow!("Failed to get metadata from Forest: {:?}", e))?;

        // If we are, queue up a file deletion request for that user.
        if !user_files.is_empty() {
            info!(target: LOG_TARGET, "Files found for user {:?}, queueing up file deletion", insolvent_user);
            // Queue a request to stop storing a file from the insolvent user.
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

impl<NT> EventHandler<SpStopStoringInsolventUser> for BspChargeFeesTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: SpStopStoringInsolventUser) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing SpStopStoringForInsolventUser for user {:?}",
            event.owner
        );

        // Get the insolvent user from the event.
        let insolvent_user = event.owner;

        // Get the current Forest key of the Provider running this node.
        let current_forest_key = CURRENT_FOREST_KEY.to_vec();

        // Check if we are storing any file for this user.
        let fs = self
            .storage_hub_handler
            .forest_storage_handler
            .get(&current_forest_key)
            .await
            .ok_or_else(|| anyhow!("Failed to get forest storage."))?;

        let user_files = fs
            .read()
            .await
            .get_files_by_user(&insolvent_user)
            .map_err(|e| anyhow!("Failed to get metadata from Forest: {:?}", e))?;

        // If we are, queue up a file deletion request for that user.
        if !user_files.is_empty() {
            info!(target: LOG_TARGET, "Files found for user {:?}, queueing up file deletion", insolvent_user);
            // Queue a request to stop storing a file from the insolvent user.
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
/// after receiving either a `UserWithoutFunds` or `SpStopStoringInsolventUser` event.
impl<NT> EventHandler<ProcessStopStoringForInsolventUserRequest> for BspChargeFeesTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
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

        // Get a write-lock on the forest root since we are going to be modifying it by removing a user's file.
        let forest_root_write_tx = match event.forest_root_write_tx.lock().await.take() {
            Some(tx) => tx,
            None => {
                error!(target: LOG_TARGET, "CRITICAL❗️❗️ This is a bug! Forest root write tx already taken. This is a critical bug. Please report it to the StorageHub team.");
                return Err(anyhow!(
                    "CRITICAL❗️❗️ This is a bug! Forest root write tx already taken!"
                ));
            }
        };

        // Get the current Forest key of the Provider running this node.
        let current_forest_key = CURRENT_FOREST_KEY.to_vec();

        // Get the forest storage.
        let fs = self
            .storage_hub_handler
            .forest_storage_handler
            .get(&current_forest_key)
            .await
            .ok_or_else(|| anyhow!("Failed to get forest storage."))?;

        // Get all the files that belong to the insolvent user, delete the first one.
        let user_files = fs
            .read()
            .await
            .get_files_by_user(&insolvent_user)
            .map_err(|e| anyhow!("Failed to get metadata from Forest: {:?}", e))?;

        if !user_files.is_empty() {
            let (file_key, metadata) = user_files.first().expect("User files is not empty");
            let bucket_id = H256::from_slice(metadata.bucket_id.as_ref());
            let location = sp_runtime::BoundedVec::truncate_from(metadata.location.clone());
            let owner = insolvent_user.clone();
            let fingerprint = H256(metadata.fingerprint.into());
            let size = metadata.file_size;
            let inclusion_forest_proof = fs
                .read()
                .await
                .generate_proof(vec![*file_key])
                .map_err(|e| anyhow!("Failed to generate proof from Forest: {:?}", e))?
                .proof;

            // Build the extrinsic to stop storing for an insolvent user.
            let stop_storing_for_insolvent_user_call = storage_hub_runtime::RuntimeCall::FileSystem(
                pallet_file_system::Call::stop_storing_for_insolvent_user {
                    file_key: *file_key,
                    bucket_id,
                    location,
                    owner,
                    fingerprint,
                    size,
                    inclusion_forest_proof,
                },
            );

            // Send the confirmation transaction and wait for it to be included in the block and
            // continue only if it is successful.
            self.storage_hub_handler
                .blockchain
                .send_extrinsic(stop_storing_for_insolvent_user_call, Tip::from(0))
                .await?
                .with_timeout(Duration::from_secs(
                    self.storage_hub_handler
                        .provider_config
                        .extrinsic_retry_timeout,
                ))
                .watch_for_success(&self.storage_hub_handler.blockchain)
                .await?;

            trace!(target: LOG_TARGET, "Stop storing submitted successfully");

            // If that was the last file of the user then charge the user for the debt they have.
            if user_files.len() == 1 {
                let call = storage_hub_runtime::RuntimeCall::PaymentStreams(
                    pallet_payment_streams::Call::charge_payment_streams {
                        user_account: insolvent_user,
                    },
                );

                let charging_result = self
                    .storage_hub_handler
                    .blockchain
                    .send_extrinsic(call, Tip::from(0))
                    .await;

                match charging_result {
                    Ok(submitted_transaction) => {
                        info!(target: LOG_TARGET, "Submitted extrinsic to charge users with debt: {}", submitted_transaction.hash());
                    }
                    Err(e) => {
                        error!(target: LOG_TARGET, "Failed to send extrinsic to charge users with debt: {}", e);
                    }
                }
            }
        }

        // Release the forest root write "lock" and finish the task.
        self.storage_hub_handler
            .blockchain
            .release_forest_root_write_lock(forest_root_write_tx)
            .await
    }
}

impl<NT> BspChargeFeesTask<NT>
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
