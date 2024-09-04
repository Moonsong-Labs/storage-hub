use async_channel::Receiver;
use sc_network::{config::IncomingRequest, ProtocolName};
use sc_service::RpcHandlers;
use shc_common::types::StorageProofsMerkleTrieLayout;
use sp_keystore::KeystorePtr;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::RwLock;

use shc_actors_framework::actor::{ActorHandle, TaskSpawner};
use shc_blockchain_service::{spawn_blockchain_service, BlockchainService};
use shc_common::types::{ParachainClient, ParachainNetworkService};
use shc_file_manager::{
    in_memory::InMemoryFileStorage, rocksdb::RocksDbFileStorage, traits::FileStorage,
};
use shc_file_transfer_service::{spawn_file_transfer_service, FileTransferService};
use shc_forest_manager::{
    in_memory::InMemoryForestStorage, rocksdb::RocksDBForestStorage, traits::ForestStorage,
};
use shc_rpc::StorageHubClientRpcConfig;

use super::handler::StorageHubHandler;

/// Builds the [`StorageHubHandler`] by adding each component separately.
/// Provides setters and getters for each component.
pub struct StorageHubBuilder<FL, FS> {
    task_spawner: Option<TaskSpawner>,
    file_transfer: Option<ActorHandle<FileTransferService>>,
    blockchain: Option<ActorHandle<BlockchainService>>,
    file_storage: Option<Arc<RwLock<FL>>>,
    forest_storage: Option<Arc<RwLock<FS>>>,
    provider_pub_key: Option<[u8; 32]>,
    storage_path: Option<String>,
}

impl<FL, FS> StorageHubBuilder<FL, FS>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync + 'static,
{
    pub fn new(task_spawner: TaskSpawner) -> Self {
        Self {
            task_spawner: Some(task_spawner),
            file_transfer: None,
            blockchain: None,
            file_storage: None,
            forest_storage: None,
            provider_pub_key: None,
            storage_path: None,
        }
    }

    /// Add a new [`FileTransferService`] to the builder and spawn it.
    ///
    /// This is the service that handles the transfer of data between peers.
    /// It plugs into Substrate's p2p network and handles the transfer of a file
    /// between a user and a Storage Provider, for example.
    pub async fn with_file_transfer(
        &mut self,
        file_transfer_request_receiver: Receiver<IncomingRequest>,
        file_transfer_request_protocol_name: ProtocolName,
        network: Arc<ParachainNetworkService>,
    ) -> &mut Self {
        let file_transfer_service_handle = spawn_file_transfer_service(
            self.task_spawner
                .as_ref()
                .expect("Task spawner is not set."),
            file_transfer_request_receiver,
            file_transfer_request_protocol_name,
            network,
        )
        .await;

        self.file_transfer = Some(file_transfer_service_handle);
        self
    }

    /// Returns the storage path that the StorageHub will use based on the available configuration.
    pub fn get_storage_path(&self) -> String {
        if let Some(path) = &self.storage_path {
            path.clone()
        } else {
            let provider_pub_key = self
                .provider_pub_key
                .expect("Could not get storage path: Provider public key not set.");
            hex::encode(provider_pub_key)
        }
    }

    /// Set the storage path that the StorageHub will use.
    pub fn with_storage_path(&mut self, storage_path: Option<String>) -> &mut Self {
        self.storage_path = storage_path;
        self
    }

    /// Add a new [`BlockchainService`] to the builder and spawn it.
    ///
    /// This is the service that handles the interaction with the blockchain.
    /// It listens to on-chain events and bubbles them up to other tasks listening,
    /// and also offers blockchain related functionality like sending extrinsics.
    pub async fn with_blockchain(
        &mut self,
        client: Arc<ParachainClient>,
        rpc_handlers: Arc<RpcHandlers>,
        keystore: KeystorePtr,
        rocksdb_root_path: impl Into<PathBuf>,
    ) -> &mut Self {
        let blockchain_service_handle = spawn_blockchain_service(
            &self
                .task_spawner
                .as_ref()
                .expect("Task spawner is not set."),
            client.clone(),
            rpc_handlers.clone(),
            keystore.clone(),
            rocksdb_root_path,
        )
        .await;

        self.blockchain = Some(blockchain_service_handle);
        self
    }

    /// Add a new [`FileStorage`] to the builder.
    ///
    /// This is the set of tools that allows a StorageHub node to store files as Merkle Patricia
    /// Tries, in the way that the StorageHub protocol specifies.
    pub fn with_file_storage(&mut self, file_storage: Arc<RwLock<FL>>) -> &mut Self {
        self.file_storage = Some(file_storage);
        self
    }

    /// Add a new [`ForestStorage`] to the builder.
    ///
    /// This is the set of tools that allows a StorageHub node to manage the files it is storing
    /// as a Merkle Patricia Forest (a trie of Merkle Patricia Tries). It follows the specification
    /// of the StorageHub protocol.
    pub fn with_forest_storage(&mut self, forest_storage: Arc<RwLock<FS>>) -> &mut Self {
        self.forest_storage = Some(forest_storage);
        self
    }

    /// Set the public key that a StorageProvider will use to, for example, sign transactions.
    pub fn with_provider_pub_key(&mut self, provider_pub_key: [u8; 32]) -> &mut Self {
        self.provider_pub_key = Some(provider_pub_key);
        self
    }

    /// Creates a new [`StorageHubClientRpcConfig`] to be used when setting up the RPCs.
    pub fn rpc_config(&self, keystore: KeystorePtr) -> StorageHubClientRpcConfig<FL, FS> {
        StorageHubClientRpcConfig::new(
            self.file_storage
                .clone()
                .expect("File Storage not initialized"),
            self.forest_storage
                .clone()
                .expect("Forest Storage not initialized"),
            keystore,
        )
    }

    /// Build the [`StorageHubHandler`] with the configuration set in the builder.
    pub fn build(self) -> StorageHubHandler<FL, FS> {
        StorageHubHandler::<FL, FS>::new(
            self.task_spawner.expect("Task Spawner not set"),
            self.file_transfer.expect("File Transfer not set."),
            self.blockchain.expect("Blockchain Service not set."),
            self.file_storage.expect("File Storage not set."),
            self.forest_storage.expect("Forest Storage not set."),
        )
    }
}

/// Provides an interface for defining the concrete types of
/// each `StorageLayer` kind, so that their specific requirements can be fulfilled.
pub trait StorageLayerBuilder {
    fn setup_storage_layer(&mut self) -> &mut Self;
}

impl StorageLayerBuilder
    for StorageHubBuilder<
        InMemoryFileStorage<StorageProofsMerkleTrieLayout>,
        InMemoryForestStorage<StorageProofsMerkleTrieLayout>,
    >
{
    fn setup_storage_layer(&mut self) -> &mut Self {
        self.with_file_storage(Arc::new(RwLock::new(InMemoryFileStorage::new())))
            .with_forest_storage(Arc::new(RwLock::new(InMemoryForestStorage::new())))
    }
}

impl StorageLayerBuilder
    for StorageHubBuilder<
        RocksDbFileStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>,
        RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>,
    >
{
    fn setup_storage_layer(&mut self) -> &mut Self {
        let storage_path = self.get_storage_path();
        let forest_storage = RocksDBForestStorage::<_, kvdb_rocksdb::Database>::rocksdb_storage(
            storage_path.clone(),
        )
        .expect("Failed to create RocksDB");
        let file_storage =
            RocksDbFileStorage::<_, kvdb_rocksdb::Database>::rocksdb_storage(storage_path)
                .expect("Failed to create RocksDB");

        self.with_file_storage(Arc::new(RwLock::new(RocksDbFileStorage::<
            _,
            kvdb_rocksdb::Database,
        >::new(file_storage))))
            .with_forest_storage(Arc::new(RwLock::new(
                RocksDBForestStorage::<_, kvdb_rocksdb::Database>::new(forest_storage)
                    .expect("Failed to create RocksDB"),
            )))
    }
}
