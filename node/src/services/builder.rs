use async_channel::Receiver;
use sc_network::{config::IncomingRequest, service::traits::NetworkService, ProtocolName};
use sc_service::RpcHandlers;
use shc_common::types::StorageProofsMerkleTrieLayout;
use sp_keystore::KeystorePtr;
use std::{path::PathBuf, sync::Arc};
use storage_hub_runtime::StorageDataUnit;
use tokio::sync::RwLock;

use shc_actors_framework::actor::{ActorHandle, TaskSpawner};
use shc_blockchain_service::{spawn_blockchain_service, BlockchainService};
use shc_common::types::ParachainClient;
use shc_file_manager::{in_memory::InMemoryFileStorage, rocksdb::RocksDbFileStorage};
use shc_file_transfer_service::{spawn_file_transfer_service, FileTransferService};
use shc_forest_manager::{
    in_memory::InMemoryForestStorage, rocksdb::RocksDBForestStorage, traits::ForestStorageHandler,
};
use shc_rpc::StorageHubClientRpcConfig;

const DEFAULT_EXTRINSIC_RETRY_TIMEOUT_SECONDS: u64 = 60;

use super::{
    forest_storage::{ForestStorageCaching, ForestStorageSingle},
    handler::{ProviderConfig, StorageHubHandler},
};
use crate::tasks::{BspForestStorageHandlerT, FileStorageT, MspForestStorageHandlerT};

/// Abstraction over the supported roles used in the StorageHub system
pub trait RoleSupport {}

pub struct BspProvider;
impl RoleSupport for BspProvider {}

pub struct MspProvider;
impl RoleSupport for MspProvider {}

pub struct UserRole;
impl RoleSupport for UserRole {}

/// Abstraction over the supported storage layers used in the StorageHub system
pub trait StorageLayerSupport {}

pub struct NoStorageLayer;
impl StorageLayerSupport for NoStorageLayer {}

pub struct InMemoryStorageLayer;
impl StorageLayerSupport for InMemoryStorageLayer {}

pub struct RocksDbStorageLayer;
impl StorageLayerSupport for RocksDbStorageLayer {}

/// Abstraction over the [`FileStorage`](shc_file_manager::traits::FileStorage) and [`ForestStorageHandler`] used based on a specific configuration of [`RoleSupport`] and [`StorageLayerSupport`].
pub trait StorageTypes {
    type FL: FileStorageT;
    type FSH: ForestStorageHandler + Clone + Send + Sync + 'static;
}

impl StorageTypes for (BspProvider, InMemoryStorageLayer) {
    type FL = InMemoryFileStorage<StorageProofsMerkleTrieLayout>;
    type FSH = ForestStorageSingle<InMemoryForestStorage<StorageProofsMerkleTrieLayout>>;
}

impl StorageTypes for (BspProvider, RocksDbStorageLayer) {
    type FL = RocksDbFileStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>;
    type FSH = ForestStorageSingle<
        RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>,
    >;
}

impl StorageTypes for (MspProvider, InMemoryStorageLayer) {
    type FL = InMemoryFileStorage<StorageProofsMerkleTrieLayout>;
    type FSH = ForestStorageCaching<Vec<u8>, InMemoryForestStorage<StorageProofsMerkleTrieLayout>>;
}

impl StorageTypes for (MspProvider, RocksDbStorageLayer) {
    type FL = RocksDbFileStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>;
    type FSH = ForestStorageCaching<
        Vec<u8>,
        RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>,
    >;
}

// TODO: Implement default empty implementations for the forest storage handler since the user role only needs the file storage.
/// There is no default empty implementation for [`FileStorageT`] and [`ForestStorageHandler`] so
/// we use the in-memory storage layers which won't be used by the user role.
impl StorageTypes for (UserRole, NoStorageLayer) {
    type FL = InMemoryFileStorage<StorageProofsMerkleTrieLayout>;
    type FSH = ForestStorageSingle<InMemoryForestStorage<StorageProofsMerkleTrieLayout>>;
}

/// Builder for the [`StorageHubHandler`].
///
/// Abstracted over [`RoleSupport`] `R` and [`StorageLayerSupport`] `S` to avoid any callers from having to know the internals of the
/// StorageHub system, such as the right storage layers to use for a given role.
pub struct StorageHubBuilder<R, S>
where
    R: RoleSupport,
    S: StorageLayerSupport,
    (R, S): StorageTypes,
{
    task_spawner: Option<TaskSpawner>,
    file_transfer: Option<ActorHandle<FileTransferService>>,
    blockchain: Option<ActorHandle<BlockchainService>>,
    storage_path: Option<String>,
    file_storage: Option<Arc<RwLock<<(R, S) as StorageTypes>::FL>>>,
    forest_storage_handler: Option<<(R, S) as StorageTypes>::FSH>,
    max_storage_capacity: Option<StorageDataUnit>,
    jump_capacity: Option<StorageDataUnit>,
    extrinsic_retry_timeout: u64,
}

/// Common components to build for any given configuration of [`RoleSupport`] and [`StorageLayerSupport`].
impl<R: RoleSupport, S: StorageLayerSupport> StorageHubBuilder<R, S>
where
    (R, S): StorageTypes,
{
    pub fn new(task_spawner: TaskSpawner) -> Self {
        Self {
            task_spawner: Some(task_spawner),
            file_transfer: None,
            blockchain: None,
            storage_path: None,
            file_storage: None,
            forest_storage_handler: None,
            max_storage_capacity: None,
            jump_capacity: None,
            extrinsic_retry_timeout: DEFAULT_EXTRINSIC_RETRY_TIMEOUT_SECONDS,
        }
    }

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

    pub fn with_max_storage_capacity(
        &mut self,
        max_storage_capacity: Option<StorageDataUnit>,
    ) -> &mut Self {
        self.max_storage_capacity = max_storage_capacity;
        self
    }

    pub fn with_jump_capacity(&mut self, jump_capacity: Option<StorageDataUnit>) -> &mut Self {
        self.jump_capacity = jump_capacity;
        self
    }

    pub fn with_retry_timeout(&mut self, extrinsic_retry_timeout: u64) -> &mut Self {
        self.extrinsic_retry_timeout = extrinsic_retry_timeout;
        self
    }

    pub async fn with_blockchain(
        &mut self,
        client: Arc<ParachainClient>,
        rpc_handlers: Arc<RpcHandlers>,
        keystore: KeystorePtr,
        rocksdb_root_path: impl Into<PathBuf>,
    ) -> &mut Self {
        let blockchain_service_handle = spawn_blockchain_service(
            self.task_spawner
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
}

/// Abstraction over the [`StorageTypes`] used based on a specific configuration of [`RoleSupport`] and [`StorageLayerSupport`].
pub trait StorageLayerBuilder {
    fn setup_storage_layer(&mut self, storage_path: Option<String>);
}

impl StorageLayerBuilder for StorageHubBuilder<BspProvider, InMemoryStorageLayer> {
    fn setup_storage_layer(&mut self, _storage_path: Option<String>) {
        self.file_storage = Some(Arc::new(RwLock::new(InMemoryFileStorage::new())));
        self.forest_storage_handler = Some(ForestStorageSingle::new(InMemoryForestStorage::new()));
    }
}

impl StorageLayerBuilder for StorageHubBuilder<BspProvider, RocksDbStorageLayer> {
    fn setup_storage_layer(&mut self, storage_path: Option<String>) {
        self.storage_path = storage_path.clone();

        let storage_path = storage_path.expect("Storage path not set");

        let file_storage =
            RocksDbFileStorage::<_, kvdb_rocksdb::Database>::rocksdb_storage(storage_path.clone())
                .expect("Failed to create RocksDB");
        self.file_storage = Some(Arc::new(RwLock::new(RocksDbFileStorage::new(file_storage))));

        let forest_storage = RocksDBForestStorage::<
            StorageProofsMerkleTrieLayout,
            kvdb_rocksdb::Database,
        >::rocksdb_storage(storage_path)
        .expect("Failed to create RocksDB for BspProvider");
        let forest_storage =
            RocksDBForestStorage::new(forest_storage).expect("Failed to create Forest Storage");
        self.forest_storage_handler = Some(ForestStorageSingle::new(forest_storage));
    }
}

impl StorageLayerBuilder for StorageHubBuilder<MspProvider, InMemoryStorageLayer> {
    fn setup_storage_layer(&mut self, _storage_path: Option<String>) {
        self.file_storage = Some(Arc::new(RwLock::new(InMemoryFileStorage::new())));
        self.forest_storage_handler = Some(ForestStorageCaching::new());
    }
}

impl StorageLayerBuilder for StorageHubBuilder<MspProvider, RocksDbStorageLayer> {
    fn setup_storage_layer(&mut self, storage_path: Option<String>) {
        self.storage_path = storage_path.clone();

        let file_storage = RocksDbFileStorage::<_, kvdb_rocksdb::Database>::rocksdb_storage(
            storage_path.expect("Storage path not set"),
        )
        .expect("Failed to create RocksDB");
        self.file_storage = Some(Arc::new(RwLock::new(RocksDbFileStorage::new(file_storage))));

        self.forest_storage_handler = Some(ForestStorageCaching::new());
    }
}

impl StorageLayerBuilder for StorageHubBuilder<UserRole, NoStorageLayer> {
    fn setup_storage_layer(&mut self, _storage_path: Option<String>) {
        self.file_storage = Some(Arc::new(RwLock::new(InMemoryFileStorage::new())));
        self.forest_storage_handler = Some(ForestStorageSingle::new(InMemoryForestStorage::new()));
    }
}

pub trait RpcConfigBuilder<FL, FSH> {
    fn create_rpc_config(&self, keystore: KeystorePtr) -> StorageHubClientRpcConfig<FL, FSH>;
}

impl<R: RoleSupport, S: StorageLayerSupport>
    RpcConfigBuilder<<(R, S) as StorageTypes>::FL, <(R, S) as StorageTypes>::FSH>
    for StorageHubBuilder<R, S>
where
    (R, S): StorageTypes,
{
    fn create_rpc_config(
        &self,
        keystore: KeystorePtr,
    ) -> StorageHubClientRpcConfig<<(R, S) as StorageTypes>::FL, <(R, S) as StorageTypes>::FSH>
    {
        StorageHubClientRpcConfig::new(
            self.file_storage
                .clone()
                .expect("File Storage not initialized"),
            self.forest_storage_handler
                .clone()
                .expect("Forest Storage Handler not initialized"),
            keystore,
        )
    }
}

impl<S: StorageLayerSupport> StorageHubBuilder<BspProvider, S>
where
    (BspProvider, S): StorageTypes,
    <(BspProvider, S) as StorageTypes>::FSH: BspForestStorageHandlerT,
{
    fn build_handler(
        &self,
    ) -> StorageHubHandler<
        <(BspProvider, S) as StorageTypes>::FL,
        <(BspProvider, S) as StorageTypes>::FSH,
    > {
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
                max_storage_capacity: self
                    .max_storage_capacity
                    .expect("Max Storage Capacity not set"),
                jump_capacity: self.jump_capacity.expect("Jump Capacity not set"),
                extrinsic_retry_timeout: self.extrinsic_retry_timeout,
            },
        )
    }
}

impl<S: StorageLayerSupport> StorageHubBuilder<MspProvider, S>
where
    (MspProvider, S): StorageTypes,
    <(MspProvider, S) as StorageTypes>::FSH: MspForestStorageHandlerT,
{
    fn build_handler(
        &self,
    ) -> StorageHubHandler<
        <(MspProvider, S) as StorageTypes>::FL,
        <(MspProvider, S) as StorageTypes>::FSH,
    > {
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
                max_storage_capacity: self
                    .max_storage_capacity
                    .expect("Max Storage Capacity not set"),
                jump_capacity: self.jump_capacity.expect("Jump Capacity not set"),
                extrinsic_retry_timeout: self.extrinsic_retry_timeout,
            },
        )
    }
}

impl StorageHubBuilder<UserRole, NoStorageLayer>
where
    (UserRole, NoStorageLayer): StorageTypes,
    <(UserRole, NoStorageLayer) as StorageTypes>::FSH:
        ForestStorageHandler + Clone + Send + Sync + 'static,
{
    fn build_handler(
        &self,
    ) -> StorageHubHandler<
        <(UserRole, NoStorageLayer) as StorageTypes>::FL,
        <(UserRole, NoStorageLayer) as StorageTypes>::FSH,
    > {
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
            ForestStorageSingle::new(InMemoryForestStorage::<StorageProofsMerkleTrieLayout>::new()),
            // Not used by the user role
            ProviderConfig {
                max_storage_capacity: 0,
                jump_capacity: 0,
                extrinsic_retry_timeout: self.extrinsic_retry_timeout,
            },
        )
    }
}

pub trait RequiredStorageProviderSetup {
    fn setup(
        &mut self,
        storage_path: Option<String>,
        max_storage_capacity: Option<StorageDataUnit>,
        jump_capacity: Option<StorageDataUnit>,
        extrinsic_retry_timeout: u64,
    );
}

impl RequiredStorageProviderSetup for StorageHubBuilder<BspProvider, InMemoryStorageLayer>
where
    (BspProvider, InMemoryStorageLayer): StorageTypes,
    Self: StorageLayerBuilder,
{
    fn setup(
        &mut self,
        storage_path: Option<String>,
        max_storage_capacity: Option<StorageDataUnit>,
        jump_capacity: Option<StorageDataUnit>,
        extrinsic_retry_timeout: u64,
    ) {
        self.setup_storage_layer(storage_path);
        if max_storage_capacity.is_none() {
            panic!("Max storage capacity not set");
        }
        self.with_max_storage_capacity(max_storage_capacity);
        self.with_jump_capacity(jump_capacity);
        self.with_retry_timeout(extrinsic_retry_timeout);
    }
}

impl RequiredStorageProviderSetup for StorageHubBuilder<BspProvider, RocksDbStorageLayer>
where
    (BspProvider, RocksDbStorageLayer): StorageTypes,
    Self: StorageLayerBuilder,
{
    fn setup(
        &mut self,
        storage_path: Option<String>,
        max_storage_capacity: Option<StorageDataUnit>,
        jump_capacity: Option<StorageDataUnit>,
        extrinsic_retry_timeout: u64,
    ) {
        if storage_path.is_none() {
            panic!("Storage path not set");
        }
        self.setup_storage_layer(storage_path);
        if max_storage_capacity.is_none() {
            panic!("Max storage capacity not set");
        }
        self.with_max_storage_capacity(max_storage_capacity);
        self.with_jump_capacity(jump_capacity);
        self.with_retry_timeout(extrinsic_retry_timeout);
    }
}

impl RequiredStorageProviderSetup for StorageHubBuilder<MspProvider, InMemoryStorageLayer>
where
    (MspProvider, InMemoryStorageLayer): StorageTypes,
    Self: StorageLayerBuilder,
{
    fn setup(
        &mut self,
        storage_path: Option<String>,
        max_storage_capacity: Option<StorageDataUnit>,
        jump_capacity: Option<StorageDataUnit>,
        extrinsic_retry_timeout: u64,
    ) {
        self.setup_storage_layer(storage_path);
        if max_storage_capacity.is_none() {
            panic!("Max storage capacity not set");
        }
        self.with_max_storage_capacity(max_storage_capacity);
        self.with_jump_capacity(jump_capacity);
        self.with_retry_timeout(extrinsic_retry_timeout);
    }
}

impl RequiredStorageProviderSetup for StorageHubBuilder<MspProvider, RocksDbStorageLayer>
where
    (MspProvider, RocksDbStorageLayer): StorageTypes,
    Self: StorageLayerBuilder,
{
    fn setup(
        &mut self,
        storage_path: Option<String>,
        max_storage_capacity: Option<StorageDataUnit>,
        jump_capacity: Option<StorageDataUnit>,
        extrinsic_retry_timeout: u64,
    ) {
        if storage_path.is_none() {
            panic!("Storage path not set");
        }
        self.setup_storage_layer(storage_path);
        if max_storage_capacity.is_none() {
            panic!("Max storage capacity not set");
        }
        self.with_max_storage_capacity(max_storage_capacity);
        self.with_jump_capacity(jump_capacity);
        self.with_retry_timeout(extrinsic_retry_timeout);
    }
}

impl<S: StorageLayerSupport> RequiredStorageProviderSetup for StorageHubBuilder<UserRole, S>
where
    (UserRole, S): StorageTypes,
    Self: StorageLayerBuilder,
{
    fn setup(
        &mut self,
        _storage_path: Option<String>,
        _max_storage_capacity: Option<StorageDataUnit>,
        _jump_capacity: Option<StorageDataUnit>,
        extrinsic_retry_timeout: u64,
    ) {
        self.setup_storage_layer(None);
        self.with_retry_timeout(extrinsic_retry_timeout);
    }
}

/// Abstraction layer to run the [`StorageHubHandler`] built from a specific configuration of [`RoleSupport`] and [`StorageLayerSupport`].
pub trait Runnable {
    fn run(self);
}

impl<S: StorageLayerSupport> Runnable for StorageHubBuilder<BspProvider, S>
where
    (BspProvider, S): StorageTypes,
    <(BspProvider, S) as StorageTypes>::FSH: BspForestStorageHandlerT,
{
    fn run(self) {
        let handler = self.build_handler();
        handler.start_bsp_tasks();
    }
}

impl<S: StorageLayerSupport> Runnable for StorageHubBuilder<MspProvider, S>
where
    (MspProvider, S): StorageTypes,
    <(MspProvider, S) as StorageTypes>::FSH: MspForestStorageHandlerT,
{
    fn run(self) {
        let handler = self.build_handler();
        handler.start_msp_tasks();
    }
}

impl Runnable for StorageHubBuilder<UserRole, NoStorageLayer>
where
    (UserRole, NoStorageLayer): StorageTypes,
    <(UserRole, NoStorageLayer) as StorageTypes>::FSH:
        ForestStorageHandler + Clone + Send + Sync + 'static,
{
    fn run(self) {
        let handler = self.build_handler();
        handler.start_user_tasks();
    }
}
