use shc_common::types::HasherOutT;
use sp_trie::TrieLayout;
use std::{marker::PhantomData, sync::Arc};
use tokio::sync::RwLock;

use shc_actors_framework::{
    actor::{ActorHandle, TaskSpawner},
    event_bus::{EventBusListener, EventHandler},
};
use shc_blockchain_service::events::SlashableProvider;
use shc_blockchain_service::{
    events::{BspConfirmedStoring, NewStorageRequest},
    BlockchainService,
};
use shc_file_manager::traits::FileStorage;
use shc_file_transfer_service::{
    events::{RemoteDownloadRequest, RemoteUploadRequest},
    FileTransferService,
};
use shc_forest_manager::traits::ForestStorage;

use crate::tasks::slash_provider::SlashProviderTask;
use crate::tasks::{
    bsp_download_file::BspDownloadFileTask,
    bsp_upload_file::BspUploadFileTask,
    sp_react_to_event_mock::{EventToReactTo, SpReactToEventMockTask},
    user_sends_file::UserSendsFileTask,
};

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
        // Subscribing to NewStorageRequest event from the BlockchainService.
        let new_storage_request_event_bus_listener: EventBusListener<NewStorageRequest, _> =
            bsp_upload_file_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain);
        new_storage_request_event_bus_listener.start();
        // Subscribing to BspConfirmedStoring event from the BlockchainService.
        let bsp_confirmed_storing_event_bus_listener: EventBusListener<BspConfirmedStoring, _> =
            bsp_upload_file_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain);
        bsp_confirmed_storing_event_bus_listener.start();

        // The BspDownloadFileTask
        let bsp_download_file_task = BspDownloadFileTask::new(self.clone());
        // Subscribing to RemoteUploadRequest event from the FileTransferService.
        let remote_upload_request_event_bus_listener: EventBusListener<RemoteUploadRequest, _> =
            bsp_upload_file_task.subscribe_to(&self.task_spawner, &self.file_transfer);
        remote_upload_request_event_bus_listener.start();
        // Subscribing to RemoteDownloadRequest event from the FileTransferService.
        let remote_download_request_event_bus_listener: EventBusListener<RemoteDownloadRequest, _> =
            bsp_download_file_task.subscribe_to(&self.task_spawner, &self.file_transfer);
        remote_download_request_event_bus_listener.start();

        // Slash your own kin or potentially commit seppuku on your own stake.
        // Running this is as a BSP is very honourable and shows a great sense of justice.
        let bsp_slash_provider_task = SlashProviderTask::new(self.clone());
        // Subscribing to SlashableProvider event from the BlockchainService.
        let slashable_provider_event_bus_listener: EventBusListener<SlashableProvider, _> =
            bsp_slash_provider_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain);
        slashable_provider_event_bus_listener.start();

        // TODO: Remove this, this is just a mocked task for testing purposes.
        let sp_react_to_event_mock_task = SpReactToEventMockTask::new(self.clone());
        // Subscribing to events from the BlockchainService.
        let bs_event_bus_listener: EventBusListener<EventToReactTo, _> =
            sp_react_to_event_mock_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain);
        bs_event_bus_listener.start();
    }
}
