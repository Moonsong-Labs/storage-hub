use shc_common::types::HasherOutT;
use sp_trie::TrieLayout;
use std::{marker::PhantomData, sync::Arc};
use tokio::sync::RwLock;

use file_manager::traits::FileStorage;
use forest_manager::traits::ForestStorage;
use storage_hub_infra::{
    actor::{ActorHandle, TaskSpawner},
    event_bus::{EventBusListener, EventHandler},
};

use crate::{
    services::{blockchain::events::NewStorageRequest, file_transfer::events::RemoteUploadRequest},
    tasks::{bsp_upload_file::BspUploadFileTask, user_sends_file::UserSendsFileTask},
};

use super::{blockchain::handler::BlockchainService, file_transfer::FileTransferService};

/// Represents the handler for the Storage Hub service.
pub struct StorageHubHandler<T, FL, FS>
where
    T: TrieLayout,
    FL: FileStorage<T> + Send + Sync,
    FS: ForestStorage<T> + Send + Sync,
{
    /// The task spawner for spawning asynchronous tasks.
    pub task_spawner: TaskSpawner,
    /// The actor handle for the file transfer service.
    pub file_transfer: ActorHandle<FileTransferService>,
    /// The actor handle for the blockchain service.
    pub blockchain: ActorHandle<BlockchainService>,
    /// The file storage layer which stores all files in chunks.
    pub file_storage: Arc<RwLock<FL>>,
    /// The forest storage layer which tracks all complete files stored in the file storage layer.
    pub forest_storage: Arc<RwLock<FS>>,

    _marker: PhantomData<T>,
}

impl<T, FL, FS> Clone for StorageHubHandler<T, FL, FS>
where
    T: TrieLayout,
    FL: FileStorage<T> + Send + Sync,
    FS: ForestStorage<T> + Send + Sync,
{
    fn clone(&self) -> StorageHubHandler<T, FL, FS> {
        Self {
            task_spawner: self.task_spawner.clone(),
            file_transfer: self.file_transfer.clone(),
            blockchain: self.blockchain.clone(),
            file_storage: self.file_storage.clone(),
            forest_storage: self.forest_storage.clone(),
            _marker: self._marker,
        }
    }
}

impl<T, FL, FS> StorageHubHandler<T, FL, FS>
where
    T: TrieLayout + Send + Sync + 'static,
    FL: FileStorage<T> + Send + Sync,
    FS: ForestStorage<T> + Send + Sync + 'static,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    pub fn new(
        task_spawner: TaskSpawner,
        file_transfer: ActorHandle<FileTransferService>,
        blockchain: ActorHandle<BlockchainService>,
        file_storage: Arc<RwLock<FL>>,
        forest_storage: Arc<RwLock<FS>>,
    ) -> Self {
        Self {
            task_spawner,
            file_transfer,
            blockchain,
            file_storage,
            forest_storage,
            _marker: Default::default(),
        }
    }

    pub fn start_user_tasks(&self) {
        log::info!("Starting User tasks.");

        UserSendsFileTask::new(self.clone())
            .subscribe_to(&self.task_spawner, &self.blockchain)
            .start();
    }

    pub fn start_bsp_tasks(&self) {
        log::info!("Starting BSP tasks");

        // BspUploadFileTask is triggered by a NewStorageRequest event, to which it responds by
        // volunteering to store the file. Then it waits for RemoteUploadRequest events, which
        // happens when the user, now aware of the BSP volunteering, submits chunks of the file,
        // along with a proof of storage.
        let bsp_upload_file_task = BspUploadFileTask::new(self.clone());
        // Subscribing to events from the BlockchainService.
        let bs_event_bus_listener: EventBusListener<NewStorageRequest, _> = bsp_upload_file_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain);
        bs_event_bus_listener.start();
        // Subscribing to events from the FileTransferService.
        let fts_event_bus_listener: EventBusListener<RemoteUploadRequest, _> =
            bsp_upload_file_task.subscribe_to(&self.task_spawner, &self.file_transfer);
        fts_event_bus_listener.start();
    }
}
