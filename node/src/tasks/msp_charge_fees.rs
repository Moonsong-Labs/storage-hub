use anyhow::anyhow;
use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceInterface, events::NotifyPeriod, types::SendExtrinsicOptions,
};
use shc_common::types::{MaxUsersToCharge, StorageProviderId};
use sp_core::Get;

use crate::services::{
    handler::StorageHubHandler,
    types::{MspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "msp-charge-fees-task";

/// Configuration for the MspChargeFeesTask
#[derive(Debug, Clone)]
pub struct MspChargeFeesConfig {
    /// Minimum debt threshold for charging users
    pub min_debt: u64,
}

impl Default for MspChargeFeesConfig {
    fn default() -> Self {
        Self {
            min_debt: 0, // Default value that was in command.rs
        }
    }
}

pub struct MspChargeFeesTask<NT>
where
    NT: ShNodeType,
    NT::FSH: MspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<NT>,
    /// Configuration for this task
    config: MspChargeFeesConfig,
}

impl<NT> Clone for MspChargeFeesTask<NT>
where
    NT: ShNodeType,
    NT::FSH: MspForestStorageHandlerT,
{
    fn clone(&self) -> MspChargeFeesTask<NT> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            config: self.config.clone(),
        }
    }
}

impl<NT> MspChargeFeesTask<NT>
where
    NT: ShNodeType,
    NT::FSH: MspForestStorageHandlerT,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT>) -> Self {
        Self {
            storage_hub_handler: storage_hub_handler.clone(),
            config: storage_hub_handler.provider_config.msp_charge_fees.clone(),
        }
    }
}

/// Handles the [`NotifyPeriod`] event.
///
/// This event is triggered every X amount of blocks.
///
/// This task will:
/// - Charge users for the MSP when triggered
impl<NT> EventHandler<NotifyPeriod> for MspChargeFeesTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
{
    async fn handle_event(&mut self, _event: NotifyPeriod) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Charging users",
        );

        let own_provider_id = self
            .storage_hub_handler
            .blockchain
            .query_storage_provider_id(None)
            .await?;

        let own_msp_id = match own_provider_id {
            Some(id) => match id {
                StorageProviderId::MainStorageProvider(id) => id,
                StorageProviderId::BackupStorageProvider(_) => {
                    let err_msg = "Current node account is a Backup Storage Provider. Expected a Main Storage Provider ID.";
                    error!(target: LOG_TARGET, err_msg);
                    return Err(anyhow!(err_msg));
                }
            },
            None => {
                let err_msg = "Failed to get own MSP ID.";
                error!(target: LOG_TARGET, err_msg);
                return Err(anyhow!(err_msg));
            }
        };

        let users_with_debt = self
            .storage_hub_handler
            .blockchain
            .query_users_with_debt(own_msp_id, self.config.min_debt as u128)
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
        let user_chunk_size: u32 = MaxUsersToCharge::get();
        for users_chunk in users_with_debt.chunks(user_chunk_size as usize) {
            let call = storage_hub_runtime::RuntimeCall::PaymentStreams(
                pallet_payment_streams::Call::charge_multiple_users_payment_streams {
                    user_accounts: users_chunk.to_vec().try_into().expect("Chunk size is the same as MaxUsersToCharge, it has to fit in the BoundedVec"),
                },
            );

            // TODO: watch for success (we might want to do it for BSP too)
            let charging_result = self
                .storage_hub_handler
                .blockchain
                .send_extrinsic(call, SendExtrinsicOptions::default())
                .await;

            match charging_result {
                Ok(submitted_transaction) => {
                    debug!(target: LOG_TARGET, "Submitted extrinsic to charge users with debt: {}", submitted_transaction.hash());
                }
                Err(e) => {
                    error!(target: LOG_TARGET, "Failed to send extrinsic to charge users with debt: {}", e);
                }
            }
        }

        Ok(())
    }
}
