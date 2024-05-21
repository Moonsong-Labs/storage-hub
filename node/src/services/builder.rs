use async_channel::Receiver;
use sc_network::{
    config::{FullNetworkConfiguration, IncomingRequest},
    ProtocolName,
};
use sc_service::RpcHandlers;
use shc_common::types::HasherOutT;
use sp_keystore::KeystorePtr;
use sp_trie::TrieLayout;
use std::{marker::PhantomData, sync::Arc};
use tokio::sync::RwLock;

use file_manager::{in_memory::InMemoryFileStorage, traits::FileStorage};
use forest_manager::{
    in_memory::InMemoryForestStorage, rocksdb::RocksDBForestStorage, traits::ForestStorage,
};
use storage_hub_infra::actor::{ActorHandle, TaskSpawner};

use crate::service::{ParachainClient, ParachainNetworkService};

use super::{
    blockchain::{spawn_blockchain_service, BlockchainService},
    file_transfer::{spawn_file_transfer_service, FileTransferService},
    handler::StorageHubHandler,
};

// TODO: Add documentation
pub struct StorageHubBuilder<'a, T, FL, FS> {
    task_spawner: Option<&'a TaskSpawner>,
    file_transfer: Option<ActorHandle<FileTransferService>>,
    blockchain: Option<ActorHandle<BlockchainService>>,
    file_storage: Option<Arc<RwLock<FL>>>,
    forest_storage: Option<Arc<RwLock<FS>>>,
    _marker: PhantomData<T>,
}

impl<T, FL, FS> StorageHubBuilder<'_, T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    pub fn new(task_spawner: &TaskSpawner) -> Self {
        Self {
            task_spawner: Some(task_spawner),
            file_transfer: None,
            blockchain: None,
            file_storage: None,
            forest_storage: None,
            _marker: Default::default(),
        }
    }

    pub fn with_task_spawner(mut self, task_spawner: &TaskSpawner) -> Self {
        self.task_spawner = Some(task_spawner);
        self
    }

    pub async fn with_file_transfer(
        mut self,
        file_transfer_request_receiver: Receiver<IncomingRequest>,
        file_transfer_request_protocol_name: ProtocolName,
        network: Arc<ParachainNetworkService>,
    ) -> Self {
        let file_transfer_service_handle = spawn_file_transfer_service(
            self.task_spawner.expect("Task spawner is not set."),
            file_transfer_request_receiver,
            file_transfer_request_protocol_name,
            network,
        )
        .await;

        self.file_transfer = Some(file_transfer_service_handle);
        self
    }

    pub async fn with_blockchain(
        mut self,
        client: Arc<ParachainClient>,
        rpc_handlers: Arc<RpcHandlers>,
        keystore: KeystorePtr,
    ) -> Self {
        let blockchain_service_handle = spawn_blockchain_service(
            self.task_spawner.expect("Task spawner is not set."),
            client.clone(),
            rpc_handlers.clone(),
            keystore.clone(),
        )
        .await;

        self.blockchain = Some(blockchain_service_handle);
        self
    }

    pub fn with_file_storage(mut self, file_storage: Arc<RwLock<FL>>) -> Self {
        self.file_storage = Some(file_storage);
        self
    }

    pub fn with_forest_storage(mut self, forest_storage: Arc<RwLock<FS>>) -> Self {
        self.forest_storage = Some(forest_storage);
        self
    }

    pub fn with_in_memory_storage(mut self) -> Self {
        self.with_file_storage(Arc::new(RwLock::new(InMemoryFileStorage::<T>::new())))
            .with_forest_storage(Arc::new(RwLock::new(InMemoryForestStorage::<T>::new())))
    }

    pub fn with_rocksdb_storage(mut self, provider_pub_key: [u8; 32]) -> Self {
        let storage_path = hex::encode(provider_pub_key);
        let storage = RocksDBForestStorage::<T>::rocksdb_storage(storage_path)
            .expect("Failed to create RocksDB");

        // TODO: Change this to RocksDB File Storage once it is implemented.
        self.with_file_storage(Arc::new(RwLock::new(InMemoryFileStorage::<T>::new())))
            .with_forest_storage(Arc::new(RwLock::new(
                RocksDBForestStorage::<T>::new(Box::new(storage))
                    .expect("Failed to create RocksDB"),
            )))
    }

    pub fn task_spawner(&self) -> Option<&TaskSpawner> {
        self.task_spawner
    }

    pub fn file_transfer(&self) -> Option<ActorHandle<FileTransferService>> {
        self.file_transfer
    }

    pub fn blockchain(&self) -> Option<ActorHandle<BlockchainService>> {
        self.blockchain
    }

    pub fn file_storage(&self) -> Option<Arc<RwLock<FL>>> {
        self.file_storage
    }

    pub fn forest_storage(&self) -> Option<Arc<RwLock<FS>>> {
        self.forest_storage
    }

    pub fn build(self) -> StorageHubHandler<T, FL, FS> {
        StorageHubHandler::<T, FL, FS>::new(
            self.task_spawner.expect("Task Spawner not set"),
            self.file_transfer.expect("File Transfer not set."),
            self.blockchain.expect("Blockchain Service not set."),
            self.file_storage.expect("File Storage not set."),
            self.forest_storage.expect("Forest Storage not set."),
        )
    }
}
