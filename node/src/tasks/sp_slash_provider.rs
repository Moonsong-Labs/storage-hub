use std::time::Duration;

use sc_tracing::tracing::*;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{commands::BlockchainServiceInterface, events::SlashableProvider};
use shc_common::types::StorageProofsMerkleTrieLayout;
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::ForestStorage;

use crate::services::handler::StorageHubHandler;

const LOG_TARGET: &str = "slash-provider-task";

/// Slash provider task.
///
/// This task is responsible for slashing a provider. It listens for the `SlashableProvider` event and sends an extrinsic
/// to StorageHub runtime to slash the provider.
pub struct SlashProviderTask<FL, FS>
where
    FL: Send + Sync + FileStorage<StorageProofsMerkleTrieLayout>,
    FS: Send + Sync + ForestStorage<StorageProofsMerkleTrieLayout>,
{
    storage_hub_handler: StorageHubHandler<FL, FS>,
}

impl<FL, FS> Clone for SlashProviderTask<FL, FS>
where
    FL: Send + Sync + FileStorage<StorageProofsMerkleTrieLayout>,
    FS: Send + Sync + ForestStorage<StorageProofsMerkleTrieLayout>,
{
    fn clone(&self) -> SlashProviderTask<FL, FS> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<FL, FS> SlashProviderTask<FL, FS>
where
    FL: Send + Sync + FileStorage<StorageProofsMerkleTrieLayout>,
    FS: Send + Sync + ForestStorage<StorageProofsMerkleTrieLayout>,
{
    pub fn new(storage_hub_handler: StorageHubHandler<FL, FS>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

/// Handles the `SlashaProvider` event.
///
/// This event is triggered by the runtime when a provider is marked as slashable.
impl<FL, FS> EventHandler<SlashableProvider> for SlashProviderTask<FL, FS>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync + 'static,
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

impl<FL, FS> SlashProviderTask<FL, FS>
where
    FL: Send + Sync + FileStorage<StorageProofsMerkleTrieLayout>,
    FS: Send + Sync + ForestStorage<StorageProofsMerkleTrieLayout>,
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
