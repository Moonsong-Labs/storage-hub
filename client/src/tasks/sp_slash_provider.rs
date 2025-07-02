use std::time::Duration;

use sc_tracing::tracing::*;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface, events::SlashableProvider,
    types::SendExtrinsicOptions,
};
use shc_common::traits::{StorageEnableApiCollection, StorageEnableRuntimeApi};

use crate::{handler::StorageHubHandler, types::ShNodeType};

const LOG_TARGET: &str = "slash-provider-task";

/// Slash provider task.
///
/// This task is responsible for slashing a provider. It listens for the [`SlashableProvider`] event and sends an extrinsic
/// to StorageHub runtime to slash the provider.
pub struct SlashProviderTask<NT, RuntimeApi>
where
    NT: ShNodeType,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    storage_hub_handler: StorageHubHandler<NT, RuntimeApi>,
}

impl<NT, RuntimeApi> Clone for SlashProviderTask<NT, RuntimeApi>
where
    NT: ShNodeType,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    fn clone(&self) -> SlashProviderTask<NT, RuntimeApi> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, RuntimeApi> SlashProviderTask<NT, RuntimeApi>
where
    NT: ShNodeType,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, RuntimeApi>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

/// Handles the [`SlashableProvider`] event.
///
/// This event is triggered by the runtime when a provider is marked as slashable.
impl<NT, RuntimeApi> EventHandler<SlashableProvider> for SlashProviderTask<NT, RuntimeApi>
where
    NT: ShNodeType + 'static,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
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

impl<NT, RuntimeApi> SlashProviderTask<NT, RuntimeApi>
where
    NT: ShNodeType,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    async fn handle_slashable_provider_event(
        &mut self,
        event: SlashableProvider,
    ) -> anyhow::Result<()> {
        // Build extrinsic.
        let call = pallet_storage_providers::Call::slash {
            provider_id: event.provider,
        };

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
