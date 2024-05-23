use file_manager::traits::FileStorage;
use forest_manager::traits::ForestStorage;
use log::*;
use sp_trie::TrieLayout;
use storage_hub_infra::event_bus::EventHandler;

use crate::services::{
    blockchain::{commands::BlockchainServiceInterface, events::NewStorageRequest},
    handler::StorageHubHandler,
};

const LOG_TARGET: &str = "bsp-volunteer-mock-task";

pub struct BspVolunteerMockTask<T, FL, FS>
where
    T: Send + Sync + TrieLayout + 'static,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T> + 'static,
{
    storage_hub_handler: StorageHubHandler<T, FL, FS>,
}

impl<T, FL, FS> Clone for BspVolunteerMockTask<T, FL, FS>
where
    T: Send + Sync + TrieLayout + 'static,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T> + 'static,
{
    fn clone(&self) -> BspVolunteerMockTask<T, FL, FS> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<T, FL, FS> BspVolunteerMockTask<T, FL, FS>
where
    T: Send + Sync + TrieLayout + 'static,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T> + 'static,
{
    pub fn new(storage_hub_handler: StorageHubHandler<T, FL, FS>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<T, FL, FS> EventHandler<NewStorageRequest> for BspVolunteerMockTask<T, FL, FS>
where
    T: Send + Sync + TrieLayout + 'static,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T> + 'static,
{
    async fn handle_event(&mut self, event: NewStorageRequest) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Initiating BSP volunteer mock for location: {:?}, fingerprint: {:?}",
            event.location,
            event.fingerprint
        );

        let fingerprint: [u8; 32] = event
            .fingerprint
            .as_ref()
            .try_into()
            .expect("Fingerprint should be 32 bytes; qed");

        // Build extrinsic.
        let call =
            storage_hub_runtime::RuntimeCall::FileSystem(pallet_file_system::Call::bsp_volunteer {
                location: event.location.clone(),
                fingerprint: fingerprint.into(),
            });

        // Send extrinsic.
        let mut submitted_transaction = self
            .storage_hub_handler
            .blockchain
            .send_extrinsic(call)
            .await?;

        // Track transaction until success.
        submitted_transaction
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await?;

        Ok(())
    }
}
