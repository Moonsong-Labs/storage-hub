use anyhow::anyhow;
use log::error;
use log::info;
use log::trace;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceInterface, events::LastChargeableInfoUpdated,
};
use storage_hub_runtime::Balance;

use crate::services::handler::StorageHubHandler;
use crate::tasks::{BspForestStorageHandlerT, FileStorageT};

const LOG_TARGET: &str = "bsp-charge-fees-task";
const MIN_DEBT: Balance = 0;

/// BSP Charge Fees Task: Handles the debt collection from users served by a BSP.
///
/// The flow includes the following steps:
/// - Reacting to [`LastChargeableInfoUpdated`] event from the runtime:
///     - Calls a Runtime API to retrieve a list of users with debt over a certain custom threshold.
///     - For each user, submits an extrinsic to [`pallet_payment_streams`] to charge them.
pub struct BspChargeFeesTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<FL, FSH>,
}

impl<FL, FSH> Clone for BspChargeFeesTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    fn clone(&self) -> BspChargeFeesTask<FL, FSH> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<FL, FSH> BspChargeFeesTask<FL, FSH>
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

impl<FL, FSH> EventHandler<LastChargeableInfoUpdated> for BspChargeFeesTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
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

        // Calls the `charge_payment_streams` extrinsic for each user in the list to be charged.
        // Logs an error in case of failure and continues.
        for user in users_with_debt {
            trace!(target: LOG_TARGET, "Charging user {:?}", user);

            let call = storage_hub_runtime::RuntimeCall::PaymentStreams(
                pallet_payment_streams::Call::charge_payment_streams { user_account: user },
            );

            let charging_result = self
                .storage_hub_handler
                .blockchain
                .send_extrinsic(call)
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
