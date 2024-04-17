pub mod blockchain;
pub mod file_transfer;

use std::sync::Arc;

use storage_hub_infra::{
    actor::{ActorHandle, TaskSpawner},
    event_bus::EventHandler,
    storage::{FileStorage, ForestStorage},
};
use tokio::sync::RwLock;

use crate::tasks::{
    AcceptedBspVolunteerHandler, NewStorageRequestHandler, ResolveRemoteUploadRequest,
};

use self::{blockchain::handler::BlockchainService, file_transfer::FileTransferService};

pub trait StorageHubHandlerConfig: Send + 'static {
    type FileStorage: FileStorage + Send + Sync;
    type ForestStorage: ForestStorage + Send + Sync;
}

#[derive(Debug)]
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

    pub fn start_bsp_tasks(&self) {
        ResolveRemoteUploadRequest::new(self.clone())
            .subscribe_to(&self.task_spawner, &self.file_transfer)
            .start();
        NewStorageRequestHandler::new(self.clone())
            .subscribe_to(&self.task_spawner, &self.blockchain)
            .start();
        AcceptedBspVolunteerHandler::new(self.clone())
            .subscribe_to(&self.task_spawner, &self.blockchain)
            .start();
    }
}
