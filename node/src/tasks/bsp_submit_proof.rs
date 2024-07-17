use sc_tracing::tracing::*;
use sp_trie::TrieLayout;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::events::NewChallengeSeed;
use shc_common::types::HasherOutT;
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::ForestStorage;

use crate::services::handler::StorageHubHandler;

const LOG_TARGET: &str = "bsp-submit-proof-task";

/// TODO: Document this task.
pub struct BspSubmitProofTask<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    storage_hub_handler: StorageHubHandler<T, FL, FS>,
}

impl<T, FL, FS> Clone for BspSubmitProofTask<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    fn clone(&self) -> BspSubmitProofTask<T, FL, FS> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<T, FL, FS> BspSubmitProofTask<T, FL, FS>
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

/// Handles the `NewStorageRequest` event.
///
/// This event is triggered by an on-chain event of a user submitting a storage request to StorageHub.
/// It responds by sending a volunteer transaction and registering the interest of this BSP in
/// receiving the file. This task optimistically assumes the transaction will succeed, and registers
/// the user and file key in the registry of the File Transfer Service, which handles incoming p2p
/// upload requests.
impl<T, FL, FS> EventHandler<NewChallengeSeed> for BspSubmitProofTask<T, FL, FS>
where
    T: TrieLayout + Send + Sync + 'static,
    FL: FileStorage<T> + Send + Sync,
    FS: ForestStorage<T> + Send + Sync + 'static,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    async fn handle_event(&mut self, event: NewChallengeSeed) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Initiating BSP proof submission for BSP ID: {:?}, at tick: {:?}, with seed: {:?}",
            event.provider_id,
            event.tick,
            event.seed
        );

        Ok(())
    }
}
