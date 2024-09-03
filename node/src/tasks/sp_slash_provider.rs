use std::time::Duration;

use sc_tracing::tracing::*;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{commands::BlockchainServiceInterface, events::SlashableProvider};
use shc_forest_manager::traits::ForestStorageHandler;

use crate::services::handler::StorageHubHandler;
use crate::tasks::FileStorageT;

const LOG_TARGET: &str = "slash-provider-task";

/// Slash provider task.
///
/// This task is responsible for slashing a provider. It listens for the `SlashableProvider` event and sends an extrinsic
/// to StorageHub runtime to slash the provider.
pub struct SlashProviderTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    storage_hub_handler: StorageHubHandler<FL, FSH>,
}

impl<FL, FSH> Clone for SlashProviderTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    fn clone(&self) -> SlashProviderTask<FL, FSH> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<FL, FSH> SlashProviderTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    pub fn new(storage_hub_handler: StorageHubHandler<FL, FSH>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

/// Handles the `SlashaProvider` event.
///
/// This event is triggered by the runtime when a provider is marked as slashable.
impl<FL, FSH> EventHandler<SlashableProvider> for SlashProviderTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
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

impl<FL, FSH> SlashProviderTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
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
            .send_extrinsic(call)
            .await?
            .with_timeout(Duration::from_secs(60))
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await?;

        Ok(())
    }
}
