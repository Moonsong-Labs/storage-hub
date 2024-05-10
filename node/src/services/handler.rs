use sp_trie::TrieLayout;
use std::sync::Arc;
use tokio::sync::RwLock;

use file_manager::{in_memory::InMemoryFileStorage, traits::FileStorage};
use forest_manager::{
    in_memory::InMemoryForestStorage, rocksdb::RocksDBForestStorage, traits::ForestStorage,
};
use storage_hub_infra::{
    actor::{ActorHandle, TaskSpawner},
    event_bus::EventHandler,
};

use crate::tasks::{bsp_volunteer_mock::BspVolunteerMockTask, user_sends_file::UserSendsFileTask};

use super::{blockchain::handler::BlockchainService, file_transfer::FileTransferService};

pub trait StorageHubHandlerConfig: StorageHubHandlerInitializer + Send + 'static {
    type FileStorage: FileStorage + Send + Sync;
    type ForestStorage: ForestStorage + Send + Sync;
}

pub struct InMemoryStorageHubConfig<T: TrieLayout>(std::marker::PhantomData<T>);

impl<T: TrieLayout + Send + 'static> StorageHubHandlerConfig for InMemoryStorageHubConfig<T> {
    type FileStorage = InMemoryFileStorage<T>;
    type ForestStorage = InMemoryForestStorage<T>;
}

impl<T: TrieLayout + Send + 'static> StorageHubHandlerInitializer for InMemoryStorageHubConfig<T> {
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

    pub fn start_user_tasks(&self) {
        log::info!("Starting User tasks.");

        UserSendsFileTask::new(self.clone())
            .subscribe_to(&self.task_spawner, &self.blockchain)
            .start();
    }

    pub fn start_bsp_tasks(&self) {
        log::info!("Starting BSP tasks");

        // TODO: Start the actual BSP tasks here and remove mock task.
        BspVolunteerMockTask::new(self.clone())
            .subscribe_to(&self.task_spawner, &self.blockchain)
            .start();
    }
}
