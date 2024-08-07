use std::time::Duration;

use sc_tracing::tracing::*;
use sp_trie::TrieLayout;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{commands::BlockchainServiceInterface, events::SlashableProvider};
use shc_common::types::HasherOutT;
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::ForestStorage;

use crate::services::handler::StorageHubHandler;

const LOG_TARGET: &str = "slash-provider-task";

/// Slash provider task.
///
/// This task is responsible for slashing a provider. It listens for the `SlashableProvider` event and sends an extrinsic
/// to StorageHub runtime to slash the provider.
pub struct SlashProviderTask<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    storage_hub_handler: StorageHubHandler<T, FL, FS>,
}

impl<T, FL, FS> Clone for SlashProviderTask<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    fn clone(&self) -> SlashProviderTask<T, FL, FS> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<T, FL, FS> SlashProviderTask<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    pub fn new(storage_hub_handler: StorageHubHandler<T, FL, FS>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

/// Handles the `SlashaProvider` event.
///
/// This event is triggered by the runtime when a provider is marked as slashable.
impl<T, FL, FS> EventHandler<SlashableProvider> for SlashProviderTask<T, FL, FS>
where
    T: TrieLayout + Send + Sync + 'static,
    FL: FileStorage<T> + Send + Sync,
    FS: ForestStorage<T> + Send + Sync + 'static,
    HasherOutT<T>: TryFrom<[u8; 32]>,
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

impl<T, FL, FS> SlashProviderTask<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    async fn handle_slashable_provider_event(
        &mut self,
        event: SlashableProvider,
    ) -> anyhow::Result<()>
    where
        HasherOutT<T>: TryFrom<[u8; 32]>,
    {
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
