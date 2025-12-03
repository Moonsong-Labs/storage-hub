use std::time::Duration;

use sc_tracing::tracing::*;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface, events::SlashableProvider,
    types::SendExtrinsicOptions,
};
use shc_common::traits::StorageEnableRuntime;

use crate::{handler::StorageHubHandler, types::ShNodeType};

const LOG_TARGET: &str = "slash-provider-task";

/// Slash provider task.
///
/// This task is responsible for slashing a provider. It listens for the [`SlashableProvider`] event and sends an extrinsic
/// to StorageHub runtime to slash the provider.
pub struct SlashProviderTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for SlashProviderTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> SlashProviderTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, Runtime> SlashProviderTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

/// Handles the [`SlashableProvider`] event.
///
/// This event is triggered by the runtime when a provider is marked as slashable.
impl<NT, Runtime> EventHandler<SlashableProvider<Runtime>> for SlashProviderTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: SlashableProvider<Runtime>) -> anyhow::Result<String> {
        let provider = event.provider.clone();
        info!(
            target: LOG_TARGET,
            "Slashing provider {:x}",
            provider,
        );

        self.handle_slashable_provider_event(event).await?;

        Ok(format!(
            "Handled SlashableProvider event for provider {:x}",
            provider
        ))
    }
}

impl<NT, Runtime> SlashProviderTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_slashable_provider_event(
        &mut self,
        event: SlashableProvider<Runtime>,
    ) -> anyhow::Result<()> {
        // Build extrinsic.
        let call: Runtime::Call = pallet_storage_providers::Call::<Runtime>::slash {
            provider_id: event.provider,
        }
        .into();

        // Send extrinsic and wait for it to be included in the block.
        self.storage_hub_handler
            .blockchain
            .send_extrinsic(
                call,
                SendExtrinsicOptions::new(
                    Duration::from_secs(
                        self.storage_hub_handler
                            .provider_config
                            .blockchain_service
                            .extrinsic_retry_timeout,
                    ),
                    Some("storageProviders".to_string()),
                    Some("slash".to_string()),
                ),
            )
            .await?
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await?;

        Ok(())
    }
}
