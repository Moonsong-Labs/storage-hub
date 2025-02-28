use std::time::Duration;

use sc_tracing::tracing::*;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceInterface, events::SlashableProvider, types::SendExtrinsicOptions,
};

use crate::services::{handler::StorageHubHandler, types::ShNodeType};

const LOG_TARGET: &str = "slash-provider-task";

/// Slash provider task.
///
/// This task is responsible for slashing a provider. It listens for the [`SlashableProvider`] event and sends an extrinsic
/// to StorageHub runtime to slash the provider.
pub struct SlashProviderTask<NT>
where
    NT: ShNodeType,
{
    storage_hub_handler: StorageHubHandler<NT>,
}

impl<NT> Clone for SlashProviderTask<NT>
where
    NT: ShNodeType,
{
    fn clone(&self) -> SlashProviderTask<NT> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT> SlashProviderTask<NT>
where
    NT: ShNodeType,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

/// Handles the [`SlashableProvider`] event.
///
/// This event is triggered by the runtime when a provider is marked as slashable.
impl<NT> EventHandler<SlashableProvider> for SlashProviderTask<NT>
where
    NT: ShNodeType + 'static,
{
    async fn handle_event(&mut self, event: SlashableProvider) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Slashing provider {:?}",
            event.provider,
        );

        self.handle_slashable_provider_event(event).await
    }
}

impl<NT> SlashProviderTask<NT>
where
    NT: ShNodeType,
{
    async fn handle_slashable_provider_event(
        &mut self,
        event: SlashableProvider,
    ) -> anyhow::Result<()> {
        // Build extrinsic.
        let call =
            storage_hub_runtime::RuntimeCall::Providers(pallet_storage_providers::Call::slash {
                provider_id: event.provider,
            });

        // Send extrinsic and wait for it to be included in the block.
        self.storage_hub_handler
            .blockchain
            .send_extrinsic(
                call,
                SendExtrinsicOptions::new(Duration::from_secs(
                    self.storage_hub_handler
                        .provider_config
                        .blockchain_service
                        .extrinsic_retry_timeout,
                )),
            )
            .await?
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await?;

        Ok(())
    }
}
