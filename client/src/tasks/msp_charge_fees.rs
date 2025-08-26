use anyhow::anyhow;
use sc_tracing::tracing::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{commands::BlockchainServiceCommandInterface, events::NotifyPeriod};
use shc_common::traits::StorageEnableRuntime;
use shc_common::types::{MaxUsersToCharge, StorageProviderId};
use shc_common::task_context::TaskContext;
use shc_common::telemetry_error::TelemetryErrorCategory;
use shc_telemetry_service::{
    create_base_event, BaseTelemetryEvent, TelemetryEvent, TelemetryServiceCommandInterfaceExt,
};
use serde::{Deserialize, Serialize};
use sp_core::Get;

// Local MSP telemetry event definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MspFeeCalculationStartedEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    task_name: String,
    msp_id: String,
    min_debt_threshold: u64,
}

impl TelemetryEvent for MspFeeCalculationStartedEvent {
    fn event_type(&self) -> &str {
        "msp_fee_calculation_started"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MspFeesChargedEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    msp_id: String,
    users_charged: u32,
    total_chunks_processed: u32,
    duration_ms: u64,
    transaction_hashes: Vec<String>,
}

impl TelemetryEvent for MspFeesChargedEvent {
    fn event_type(&self) -> &str {
        "msp_fees_charged"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MspFeeCollectionFailedEvent {
    #[serde(flatten)]
    base: BaseTelemetryEvent,
    task_id: String,
    msp_id: String,
    error_type: String,
    error_message: String,
    duration_ms: Option<u64>,
    users_attempted: u32,
    chunks_processed: u32,
}

impl TelemetryEvent for MspFeeCollectionFailedEvent {
    fn event_type(&self) -> &str {
        "msp_fee_collection_failed"
    }
}

use crate::{
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

pub struct MspChargeFeesTask<NT, Runtime>
where
    NT: ShNodeType,
    NT::FSH: MspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
    /// Configuration for this task
    config: MspChargeFeesConfig,
}

impl<NT, Runtime> Clone for MspChargeFeesTask<NT, Runtime>
where
    NT: ShNodeType,
    NT::FSH: MspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> MspChargeFeesTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            config: self.config.clone(),
        }
    }
}

impl<NT, Runtime> MspChargeFeesTask<NT, Runtime>
where
    NT: ShNodeType,
    NT::FSH: MspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
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
impl<NT, Runtime> EventHandler<NotifyPeriod> for MspChargeFeesTask<NT, Runtime>
where
    NT: ShNodeType + 'static,
    NT::FSH: MspForestStorageHandlerT,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, _event: NotifyPeriod) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Charging users",
        );

        // Create task context for tracking
        let ctx = TaskContext::new("msp_charge_fees");

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
                    let error = anyhow!(err_msg);
                    
                    // Send failure telemetry
                    if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
                        let failed_event = MspFeeCollectionFailedEvent {
                            base: create_base_event("msp_fee_collection_failed", "storage-hub-msp".to_string(), None),
                            task_id: ctx.task_id.clone(),
                            msp_id: "unknown".to_string(),
                            error_type: error.telemetry_category().to_string(),
                            error_message: error.to_string(),
                            duration_ms: Some(ctx.elapsed_ms()),
                            users_attempted: 0,
                            chunks_processed: 0,
                        };
                        telemetry_service.queue_typed_event(failed_event).await.ok();
                    }
                    
                    return Err(error);
                }
            },
            None => {
                warn!(target: LOG_TARGET, "Provider not registred yet. We can't charge users.");
                return Ok(());
            }
        };

        // Send task started telemetry event
        if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
            let start_event = MspFeeCalculationStartedEvent {
                base: create_base_event("msp_fee_calculation_started", "storage-hub-msp".to_string(), None),
                task_id: ctx.task_id.clone(),
                task_name: ctx.task_name.clone(),
                msp_id: format!("{:?}", own_msp_id),
                min_debt_threshold: self.config.min_debt,
            };
            telemetry_service.queue_typed_event(start_event).await.ok();
        }

        let users_with_debt = match self
            .storage_hub_handler
            .blockchain
            .query_users_with_debt(own_msp_id, self.config.min_debt as u128)
            .await
        {
            Ok(users) => users,
            Err(e) => {
                let error = anyhow!(
                    "Failed to retrieve users with debt from the runtime: {:?}",
                    e
                );
                
                // Send failure telemetry
                if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
                    let failed_event = MspFeeCollectionFailedEvent {
                        base: create_base_event("msp_fee_collection_failed", "storage-hub-msp".to_string(), None),
                        task_id: ctx.task_id.clone(),
                        msp_id: format!("{:?}", own_msp_id),
                        error_type: error.telemetry_category().to_string(),
                        error_message: error.to_string(),
                        duration_ms: Some(ctx.elapsed_ms()),
                        users_attempted: 0,
                        chunks_processed: 0,
                    };
                    telemetry_service.queue_typed_event(failed_event).await.ok();
                }
                
                return Err(error);
            }
        };

        // Divides the users to charge in chunks of MaxUsersToCharge to avoid exceeding the block limit.
        // Calls the `charge_multiple_users_payment_streams` extrinsic for each chunk in the list to be charged.
        // Logs an error in case of failure and continues.
        let user_chunk_size: u32 = MaxUsersToCharge::get();
        let total_users = users_with_debt.len() as u32;
        let total_chunks = users_with_debt.chunks(user_chunk_size as usize).len() as u32;
        let mut transaction_hashes = Vec::new();
        let mut successful_chunks = 0u32;
        let mut failed_chunks = 0u32;

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
                .send_extrinsic(call.into(), Default::default())
                .await;

            match charging_result {
                Ok(submitted_transaction) => {
                    debug!(target: LOG_TARGET, "Submitted extrinsic to charge users with debt: {}", submitted_transaction.hash());
                    transaction_hashes.push(format!("{:?}", submitted_transaction.hash()));
                    successful_chunks += 1;
                }
                Err(e) => {
                    error!(target: LOG_TARGET, "Failed to send extrinsic to charge users with debt: {}", e);
                    failed_chunks += 1;
                }
            }
        }

        // Send completion or failure telemetry
        if let Some(telemetry_service) = &self.storage_hub_handler.telemetry {
            if failed_chunks == 0 {
                // All chunks processed successfully
                let success_event = MspFeesChargedEvent {
                    base: create_base_event("msp_fees_charged", "storage-hub-msp".to_string(), None),
                    task_id: ctx.task_id.clone(),
                    msp_id: format!("{:?}", own_msp_id),
                    users_charged: total_users,
                    total_chunks_processed: total_chunks,
                    duration_ms: ctx.elapsed_ms(),
                    transaction_hashes,
                };
                telemetry_service.queue_typed_event(success_event).await.ok();
            } else {
                // Some or all chunks failed
                let failed_event = MspFeeCollectionFailedEvent {
                    base: create_base_event("msp_fee_collection_failed", "storage-hub-msp".to_string(), None),
                    task_id: ctx.task_id.clone(),
                    msp_id: format!("{:?}", own_msp_id),
                    error_type: "partial_failure".to_string(),
                    error_message: format!("{} out of {} chunks failed to process", failed_chunks, total_chunks),
                    duration_ms: Some(ctx.elapsed_ms()),
                    users_attempted: total_users,
                    chunks_processed: successful_chunks,
                };
                telemetry_service.queue_typed_event(failed_event).await.ok();
            }
        }

        Ok(())
    }
}
