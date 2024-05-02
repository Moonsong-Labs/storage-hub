pub mod blockchain;
pub mod file_transfer;

use sp_core::H256;
use std::sync::Arc;
use tokio::sync::RwLock;

use file_manager::traits::FileStorage;
use forest_manager::traits::ForestStorage;
use storage_hub_infra::{
    actor::{ActorHandle, TaskSpawner},
    event_bus::EventHandler,
    types::Metadata,
};

use crate::tasks::bsp_volunteer_mock::BspVolunteerMockTask;

use self::{blockchain::handler::BlockchainService, file_transfer::FileTransferService};

pub trait StorageHubHandlerConfig: Send + 'static {
    type FileStorage: FileStorage + Send + Sync;
    type ForestStorage: ForestStorage + Send + Sync;
}

pub struct StorageHubHandler<S: StorageHubHandlerConfig> {
    pub task_spawner: TaskSpawner,
    pub file_transfer: ActorHandle<FileTransferService>,
    pub blockchain: ActorHandle<BlockchainService>,
    pub file_storage: Arc<RwLock<S::FileStorage>>,
    pub forest_storage: Arc<RwLock<S::ForestStorage>>,
}

impl<SHC: StorageHubHandlerConfig> Clone for StorageHubHandler<SHC> {
    fn clone(&self) -> StorageHubHandler<SHC> {
        Self {
            task_spawner: self.task_spawner.clone(),
            file_transfer: self.file_transfer.clone(),
            blockchain: self.blockchain.clone(),
            file_storage: self.file_storage.clone(),
            forest_storage: self.forest_storage.clone(),
        }
    }
}

impl<S: StorageHubHandlerConfig> StorageHubHandler<S> {
    pub fn new(
        task_spawner: TaskSpawner,
        file_transfer: ActorHandle<FileTransferService>,
        blockchain: ActorHandle<BlockchainService>,
        file_storage: Arc<RwLock<S::FileStorage>>,
        forest_storage: Arc<RwLock<S::ForestStorage>>,
    ) -> Self {
        Self {
            task_spawner,
            file_transfer,
            blockchain,
            file_storage,
            forest_storage,
        }
    }

    /// Add file to the forest storage.
    pub async fn _add_file_to_forest<T: StorageHubHandlerConfig>(
        &self,
        who: String,
        location: Vec<u8>,
        size: u64,
        fingerprint: H256,
    ) -> anyhow::Result<Vec<u8>> {
        let mut forest_storage = self.forest_storage.write().await;

        let metadata = Metadata::new(who, location, size, fingerprint);

        let metadata_serialized = bincode::serialize(&metadata)?;

        let file_key = forest_storage
            .insert_file_key(
                &metadata_serialized.clone().into(),
                &metadata_serialized.into(),
            )
            .map_err(|e| anyhow::anyhow!("Failed to insert file key: {:?}", e))?;

        Ok(file_key.as_ref().to_vec())
    }

    pub fn start_bsp_tasks(&self) {
        log::info!("Starting BSP tasks");

        // TODO: Start the actual BSP tasks here and remove mock task.
        BspVolunteerMockTask::new(self.clone())
            .subscribe_to(&self.task_spawner, &self.blockchain)
            .start();
    }
}
