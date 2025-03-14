use async_channel::Receiver;
use sc_network::{config::IncomingRequest, service::traits::NetworkService, ProtocolName};
use sc_service::RpcHandlers;
use shc_indexer_db::DbPool;
use sp_keystore::KeystorePtr;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::RwLock;

use shc_actors_framework::actor::{ActorHandle, TaskSpawner};
use shc_blockchain_service::{
    capacity_manager::CapacityConfig, spawn_blockchain_service, BlockchainService,
};
use shc_common::types::ParachainClient;
use shc_file_manager::{in_memory::InMemoryFileStorage, rocksdb::RocksDbFileStorage};
use shc_file_transfer_service::{spawn_file_transfer_service, FileTransferService};
use shc_forest_manager::traits::ForestStorageHandler;
use shc_rpc::StorageHubClientRpcConfig;

const DEFAULT_EXTRINSIC_RETRY_TIMEOUT_SECONDS: u64 = 60;

use super::{
    handler::{ProviderConfig, StorageHubHandler},
    types::{
        BspForestStorageHandlerT, BspProvider, InMemoryStorageLayer, MspForestStorageHandlerT,
        MspProvider, NoStorageLayer, RocksDbStorageLayer, ShNodeType, ShRole, ShStorageLayer,
        UserRole,
    },
};

/// Builder for the [`StorageHubHandler`].
///
/// Abstracted over [`ShRole`] `R` and [`ShStorageLayer`] `S` to avoid any callers from having to know the internals of the
/// StorageHub system, such as the right storage layers to use for a given role.
pub struct StorageHubBuilder<R, S>
where
    R: ShRole,
    S: ShStorageLayer,
    (R, S): ShNodeType,
{
    task_spawner: Option<TaskSpawner>,
    file_transfer: Option<ActorHandle<FileTransferService>>,
    blockchain: Option<ActorHandle<BlockchainService<<(R, S) as ShNodeType>::FSH>>>,
    storage_path: Option<String>,
    file_storage: Option<Arc<RwLock<<(R, S) as ShNodeType>::FL>>>,
    forest_storage_handler: Option<<(R, S) as ShNodeType>::FSH>,
    capacity_config: Option<CapacityConfig>,
    extrinsic_retry_timeout: u64,
    indexer_db_pool: Option<DbPool>,
    notify_period: Option<u32>,
}

/// Common components to build for any given configuration of [`ShRole`] and [`ShStorageLayer`].
impl<R: ShRole, S: ShStorageLayer> StorageHubBuilder<R, S>
where
    (R, S): ShNodeType,
{
    pub fn new(task_spawner: TaskSpawner) -> Self {
        Self {
            task_spawner: Some(task_spawner),
            file_transfer: None,
            blockchain: None,
            storage_path: None,
            file_storage: None,
            forest_storage_handler: None,
            capacity_config: None,
            extrinsic_retry_timeout: DEFAULT_EXTRINSIC_RETRY_TIMEOUT_SECONDS,
            indexer_db_pool: None,
            notify_period: None,
        }
    }

    /// Spawn the File Transfer Service.
    pub async fn with_file_transfer(
        &mut self,
        file_transfer_request_receiver: Receiver<IncomingRequest>,
        file_transfer_request_protocol_name: ProtocolName,
        network: Arc<dyn NetworkService>,
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

    /// Set the maximum storage capacity.
    ///
    /// The node will not increase its on-chain capacity above this value.
    /// This is meant to reflect the actual physical storage capacity of the node.
    pub fn with_capacity_config(&mut self, capacity_config: Option<CapacityConfig>) -> &mut Self {
        self.capacity_config = capacity_config;
        self
    }

    /// Set the timeout for retrying extrinsics.
    ///
    /// The default value is `60` seconds.
    pub fn with_retry_timeout(&mut self, extrinsic_retry_timeout: u64) -> &mut Self {
        self.extrinsic_retry_timeout = extrinsic_retry_timeout;
        self
    }

    /// Add an alert notification for every X blocks to the Blockchain Service.
    ///
    /// Cannot be added if the Blockchain Service has already been spawned.
    pub fn with_notify_period(&mut self, notify_period: Option<u32>) -> &mut Self {
        if self.blockchain.is_some() {
            panic!("`with_notify_period` should be called before starting the Blockchain Service. Use `with_blockchain` after calling `with_notify_period`.");
        }
        self.notify_period = notify_period;
        self
    }

    /// Spawn the Blockchain Service.
    ///
    /// Cannot be called before setting the Forest Storage Handler.
    /// Call [`setup_storage_layer`](StorageHubBuilder::setup_storage_layer) before calling this method.
    pub async fn with_blockchain(
        &mut self,
        client: Arc<ParachainClient>,
        keystore: KeystorePtr,
        rpc_handlers: Arc<RpcHandlers>,
        rocksdb_root_path: impl Into<PathBuf>,
        maintenance_mode: bool,
    ) -> &mut Self {
        if self.forest_storage_handler.is_none() {
            panic!(
                "`with_blockchain` should be called after setting up the Forest Storage Handler. Use `setup_storage_layer` first."
            );
        }

        let forest_storage_handler = self
            .forest_storage_handler
            .clone()
            .expect("Just checked that this is not None; qed");

        let capacity_config = self.capacity_config.clone();

        let blockchain_service_handle = spawn_blockchain_service::<<(R, S) as ShNodeType>::FSH>(
            self.task_spawner
                .as_ref()
                .expect("Task spawner is not set."),
            client.clone(),
            keystore.clone(),
            rpc_handlers.clone(),
            forest_storage_handler,
            rocksdb_root_path,
            self.notify_period,
            capacity_config,
            maintenance_mode,
        )
        .await;

        self.blockchain = Some(blockchain_service_handle);
        self
    }

    /// Set the database pool for the Indexer Service.
    ///
    /// The Indexer Service is used by MSP nodes to retrieve information about files
    /// they are not storing, like which are the BSPs storing them.
    pub fn with_indexer_db_pool(&mut self, indexer_db_pool: Option<DbPool>) -> &mut Self {
        self.indexer_db_pool = indexer_db_pool;
        self
    }

    /// Create the RPC configuration needed to initialise the RPC methods of the StorageHub client.
    ///
    /// This method is meant to be called after the Storage Layer has been set up.
    /// Call [`setup_storage_layer`](StorageHubBuilder::setup_storage_layer) before calling this method.
    pub fn create_rpc_config(
        &self,
        keystore: KeystorePtr,
    ) -> StorageHubClientRpcConfig<<(R, S) as ShNodeType>::FL, <(R, S) as ShNodeType>::FSH> {
        StorageHubClientRpcConfig::new(
            self.file_storage
                .clone()
                .expect("File Storage not initialized. Use `setup_storage_layer` before calling `create_rpc_config`."),
            self.forest_storage_handler
                .clone()
                .expect("Forest Storage Handler not initialized. Use `setup_storage_layer` before calling `create_rpc_config`."),
            keystore,
        )
    }
}

/// Abstraction trait to build the Storage Layer of a [`ShNodeType`].
///
/// Each [`ShNodeType`] depends on a specific combination of [`ShRole`] and [`ShStorageLayer`],
/// and each of this combinations has a different way of building their Storage Layer.
///
/// This trait is implemented for `StorageHubBuilder<R, S>` where `R` is a [`ShRole`] and `S` is a [`ShStorageLayer`].
pub trait StorageLayerBuilder {
    fn setup_storage_layer(&mut self, storage_path: Option<String>) -> &mut Self;
}

impl StorageLayerBuilder for StorageHubBuilder<BspProvider, InMemoryStorageLayer> {
    fn setup_storage_layer(&mut self, _storage_path: Option<String>) -> &mut Self {
        self.file_storage = Some(Arc::new(RwLock::new(InMemoryFileStorage::new())));
        self.forest_storage_handler =
            Some(<(BspProvider, InMemoryStorageLayer) as ShNodeType>::FSH::new());

        self
    }
}

impl StorageLayerBuilder for StorageHubBuilder<BspProvider, RocksDbStorageLayer> {
    fn setup_storage_layer(&mut self, storage_path: Option<String>) -> &mut Self {
        self.storage_path = storage_path.clone();

        let storage_path = storage_path.expect("Storage path not set");

        let file_storage =
            RocksDbFileStorage::<_, kvdb_rocksdb::Database>::rocksdb_storage(storage_path.clone())
                .expect("Failed to create RocksDB");
        self.file_storage = Some(Arc::new(RwLock::new(RocksDbFileStorage::new(file_storage))));

        self.forest_storage_handler =
            Some(<(BspProvider, RocksDbStorageLayer) as ShNodeType>::FSH::new(storage_path));

        self
    }
}

impl StorageLayerBuilder for StorageHubBuilder<MspProvider, InMemoryStorageLayer> {
    fn setup_storage_layer(&mut self, _storage_path: Option<String>) -> &mut Self {
        self.file_storage = Some(Arc::new(RwLock::new(InMemoryFileStorage::new())));
        self.forest_storage_handler =
            Some(<(MspProvider, InMemoryStorageLayer) as ShNodeType>::FSH::new());

        self
    }
}

impl StorageLayerBuilder for StorageHubBuilder<MspProvider, RocksDbStorageLayer> {
    fn setup_storage_layer(&mut self, storage_path: Option<String>) -> &mut Self {
        let storage_path = storage_path.expect("Storage path not set");
        self.storage_path = Some(storage_path.clone());

        let file_storage =
            RocksDbFileStorage::<_, kvdb_rocksdb::Database>::rocksdb_storage(storage_path.clone())
                .expect("Failed to create RocksDB");
        self.file_storage = Some(Arc::new(RwLock::new(RocksDbFileStorage::new(file_storage))));

        self.forest_storage_handler =
            Some(<(MspProvider, RocksDbStorageLayer) as ShNodeType>::FSH::new(storage_path));

        self
    }
}

impl StorageLayerBuilder for StorageHubBuilder<UserRole, NoStorageLayer> {
    fn setup_storage_layer(&mut self, _storage_path: Option<String>) -> &mut Self {
        self.file_storage = Some(Arc::new(RwLock::new(InMemoryFileStorage::new())));
        self.forest_storage_handler = Some(<(UserRole, NoStorageLayer) as ShNodeType>::FSH::new());

        self
    }
}

/// Abstraction trait to build the [`StorageHubHandler`].
///
/// This trait is implemented by the different [`StorageHubBuilder`] variants,
/// and build a [`StorageHubHandler`] with the required configuration for the
/// corresponding [`ShRole`].
pub trait Buildable<NT: ShNodeType> {
    fn build(self) -> StorageHubHandler<NT>;
}

impl<S: ShStorageLayer> Buildable<(BspProvider, S)> for StorageHubBuilder<BspProvider, S>
where
    (BspProvider, S): ShNodeType,
    <(BspProvider, S) as ShNodeType>::FSH: BspForestStorageHandlerT,
{
    fn build(self) -> StorageHubHandler<(BspProvider, S)> {
        StorageHubHandler::new(
            self.task_spawner
                .as_ref()
                .expect("Task Spawner not set")
                .clone(),
            self.file_transfer
                .as_ref()
                .expect("File Transfer not set.")
                .clone(),
            self.blockchain
                .as_ref()
                .expect("Blockchain Service not set.")
                .clone(),
            self.file_storage
                .as_ref()
                .expect("File Storage not set.")
                .clone(),
            self.forest_storage_handler
                .as_ref()
                .expect("Forest Storage Handler not set.")
                .clone(),
            ProviderConfig {
                capacity_config: self.capacity_config.expect("Capacity Config not set"),
                extrinsic_retry_timeout: self.extrinsic_retry_timeout,
            },
            self.indexer_db_pool.clone(),
        )
    }
}

impl<S: ShStorageLayer> Buildable<(MspProvider, S)> for StorageHubBuilder<MspProvider, S>
where
    (MspProvider, S): ShNodeType,
    <(MspProvider, S) as ShNodeType>::FSH: MspForestStorageHandlerT,
{
    fn build(self) -> StorageHubHandler<(MspProvider, S)> {
        StorageHubHandler::new(
            self.task_spawner
                .as_ref()
                .expect("Task Spawner not set")
                .clone(),
            self.file_transfer
                .as_ref()
                .expect("File Transfer not set.")
                .clone(),
            self.blockchain
                .as_ref()
                .expect("Blockchain Service not set.")
                .clone(),
            self.file_storage
                .as_ref()
                .expect("File Storage not set.")
                .clone(),
            self.forest_storage_handler
                .as_ref()
                .expect("Forest Storage Handler not set.")
                .clone(),
            ProviderConfig {
                capacity_config: self.capacity_config.expect("Capacity Config not set"),
                extrinsic_retry_timeout: self.extrinsic_retry_timeout,
            },
            self.indexer_db_pool.clone(),
        )
    }
}

impl Buildable<(UserRole, NoStorageLayer)> for StorageHubBuilder<UserRole, NoStorageLayer>
where
    (UserRole, NoStorageLayer): ShNodeType,
    <(UserRole, NoStorageLayer) as ShNodeType>::FSH:
        ForestStorageHandler + Clone + Send + Sync + 'static,
{
    fn build(self) -> StorageHubHandler<(UserRole, NoStorageLayer)> {
        StorageHubHandler::new(
            self.task_spawner
                .as_ref()
                .expect("Task Spawner not set")
                .clone(),
            self.file_transfer
                .as_ref()
                .expect("File Transfer not set.")
                .clone(),
            self.blockchain
                .as_ref()
                .expect("Blockchain Service not set.")
                .clone(),
            self.file_storage
                .as_ref()
                .expect("File Storage not set.")
                .clone(),
            // Not used by the user role
            <(UserRole, NoStorageLayer) as ShNodeType>::FSH::new(),
            // Not used by the user role
            ProviderConfig {
                capacity_config: CapacityConfig::new(0, 0),
                extrinsic_retry_timeout: self.extrinsic_retry_timeout,
            },
            self.indexer_db_pool.clone(),
        )
    }
}
