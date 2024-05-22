use shc_common::types::HasherOutT;
use sp_trie::TrieLayout;
use std::sync::Arc;
use tokio::sync::RwLock;

use file_manager::{in_memory::InMemoryFileStorage, traits::FileStorage};
use forest_manager::{
    in_memory::InMemoryForestStorage, rocksdb::RocksDBForestStorage, traits::ForestStorage,
};
use storage_hub_infra::{
    actor::{ActorHandle, TaskSpawner},
    event_bus::{EventBusListener, EventHandler},
};

use crate::{
    services::{blockchain::events::NewStorageRequest, file_transfer::events::RemoteUploadRequest},
    tasks::{bsp_upload_file::BspUploadFileTask, user_sends_file::UserSendsFileTask},
};

use super::{blockchain::handler::BlockchainService, file_transfer::FileTransferService};

/// Wrapper trait encompassing all the necessary implementations which [`StorageHubHandler`] requires.
pub trait StorageHubHandlerConfig: StorageHubHandlerInitializer + Send + 'static {
    /// Type which implements [`TrieLayout`].
    ///
    /// This is primarily used for constructing
    type TrieLayout: TrieLayout;
    /// Type which implements [`FileStorage`].
    ///
    /// This layer stores all files (chunked).
    type FileStorage: FileStorage<Self::TrieLayout> + Send + Sync;
    /// Type which implements [`ForestStorage`].
    ///
    /// This layer tracks all of the files stored in [`FileStorage`](StorageHubHandlerConfig::FileStorage).
    type ForestStorage: ForestStorage<Self::TrieLayout> + Send + Sync;
}

pub struct InMemoryStorageHubConfig<T: TrieLayout>(std::marker::PhantomData<T>);

impl<T: TrieLayout + Send + 'static> StorageHubHandlerConfig for InMemoryStorageHubConfig<T>
where
    <<T as TrieLayout>::Hash as sp_core::Hasher>::Out: TryFrom<[u8; 32]>,
{
    type TrieLayout = T;
    type FileStorage = InMemoryFileStorage<T>;
    type ForestStorage = InMemoryForestStorage<T>;
}

impl<T: TrieLayout + Send + 'static> StorageHubHandlerInitializer for InMemoryStorageHubConfig<T>
where
    <<T as TrieLayout>::Hash as sp_core::Hasher>::Out: TryFrom<[u8; 32]>,
{
    fn initialize(
        _provider_pub_key: [u8; 32],
        task_spawner: TaskSpawner,
        file_transfer: ActorHandle<FileTransferService>,
        blockchain: ActorHandle<BlockchainService>,
    ) -> StorageHubHandler<Self> {
        StorageHubHandler::new(
            task_spawner,
            file_transfer,
            blockchain,
            Arc::new(RwLock::new(InMemoryFileStorage::<T>::new())),
            Arc::new(RwLock::new(InMemoryForestStorage::<T>::new())),
        )
    }
}

pub struct RocksDBStorageHubConfig<T: TrieLayout>(std::marker::PhantomData<T>);

impl<T: TrieLayout + Send + Sync + 'static> StorageHubHandlerConfig for RocksDBStorageHubConfig<T>
where
    <<T as TrieLayout>::Hash as sp_core::Hasher>::Out: TryFrom<[u8; 32]>,
{
    type TrieLayout = T;
    type FileStorage = InMemoryFileStorage<T>;
    type ForestStorage = RocksDBForestStorage<T>;
}

impl<T: TrieLayout + Send + Sync + 'static> StorageHubHandlerInitializer
    for RocksDBStorageHubConfig<T>
where
    <<T as TrieLayout>::Hash as sp_core::Hasher>::Out: TryFrom<[u8; 32]>,
{
    fn initialize(
        provider_pub_key: [u8; 32],
        task_spawner: TaskSpawner,
        file_transfer: ActorHandle<FileTransferService>,
        blockchain: ActorHandle<BlockchainService>,
    ) -> StorageHubHandler<Self> {
        let storage_path = hex::encode(provider_pub_key);
        let storage = RocksDBForestStorage::<T>::rocksdb_storage(storage_path)
            .expect("Failed to create RocksDB");

        StorageHubHandler::new(
            task_spawner,
            file_transfer,
            blockchain,
            Arc::new(RwLock::new(InMemoryFileStorage::<T>::new())),
            Arc::new(RwLock::new(
                RocksDBForestStorage::<T>::new(Box::new(storage))
                    .expect("Failed to create RocksDB"),
            )),
        )
    }
}

/// A trait which initializes the [`StorageHubHandler`] with the necessary components.
pub trait StorageHubHandlerInitializer {
    fn initialize(
        provider_pub_key: [u8; 32],
        task_spawner: TaskSpawner,
        file_transfer: ActorHandle<FileTransferService>,
        blockchain: ActorHandle<BlockchainService>,
    ) -> StorageHubHandler<Self>
    where
        Self: Sized + StorageHubHandlerConfig;
}

/// Represents the handler for the Storage Hub service.
pub struct StorageHubHandler<S: StorageHubHandlerConfig> {
    /// The task spawner for spawning asynchronous tasks.
    pub task_spawner: TaskSpawner,
    /// The actor handle for the file transfer service.
    pub file_transfer: ActorHandle<FileTransferService>,
    /// The actor handle for the blockchain service.
    pub blockchain: ActorHandle<BlockchainService>,
    /// The file storage layer which stores all files in chunks.
    pub file_storage: Arc<RwLock<S::FileStorage>>,
    /// The forest storage layer which tracks all complete files stored in the file storage layer.
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

impl<SHC: StorageHubHandlerConfig> StorageHubHandler<SHC>
where
    HasherOutT<SHC::TrieLayout>: TryFrom<[u8; 32]>,
{
    pub fn new(
        task_spawner: TaskSpawner,
        file_transfer: ActorHandle<FileTransferService>,
        blockchain: ActorHandle<BlockchainService>,
        file_storage: Arc<RwLock<SHC::FileStorage>>,
        forest_storage: Arc<RwLock<SHC::ForestStorage>>,
    ) -> Self {
        Self {
            task_spawner,
            file_transfer,
            blockchain,
            file_storage,
            forest_storage,
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
