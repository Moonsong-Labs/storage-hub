use async_channel::Receiver;
use sc_network::{config::IncomingRequest, service::traits::NetworkService, ProtocolName};
use sc_service::RpcHandlers;
use shc_indexer_db::DbPool;
use sp_keystore::KeystorePtr;
use std::{path::PathBuf, sync::Arc};
use storage_hub_runtime::StorageDataUnit;
use tokio::sync::RwLock;

use shc_actors_framework::actor::{ActorHandle, TaskSpawner};
use shc_blockchain_service::{
    handler::BlockchainServiceConfig, spawn_blockchain_service, BlockchainService,
};
use shc_common::types::ParachainClient;
use shc_file_manager::{in_memory::InMemoryFileStorage, rocksdb::RocksDbFileStorage};
use shc_file_transfer_service::{
    handler::FileTransferServiceConfig, spawn_file_transfer_service, FileTransferService,
};
use shc_forest_manager::traits::ForestStorageHandler;
use shc_rpc::StorageHubClientRpcConfig;

/// TODO: CONSTANTS
const DEFAULT_EXTRINSIC_RETRY_TIMEOUT_SECONDS: u64 = 60;

use crate::tasks::{
    bsp_charge_fees::BspChargeFeesConfig, bsp_move_bucket::BspMoveBucketConfig,
    bsp_submit_proof::BspSubmitProofConfig, bsp_upload_file::BspUploadFileConfig,
    msp_charge_fees::MspChargeFeesConfig, msp_delete_file::MspDeleteFileConfig,
    msp_move_bucket::MspMoveBucketConfig,
};

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
    max_storage_capacity: Option<StorageDataUnit>,
    jump_capacity: Option<StorageDataUnit>,
    extrinsic_retry_timeout: u64,
    indexer_db_pool: Option<DbPool>,
    notify_period: Option<u32>,
    // Configuration options for tasks and services
    msp_delete_file_options: Option<MspDeleteFileConfig>,
    msp_charge_fees_options: Option<MspChargeFeesConfig>,
    msp_move_bucket_options: Option<MspMoveBucketConfig>,
    bsp_upload_file_options: Option<BspUploadFileConfig>,
    bsp_move_bucket_options: Option<BspMoveBucketConfig>,
    bsp_charge_fees_options: Option<BspChargeFeesConfig>,
    bsp_submit_proof_options: Option<BspSubmitProofConfig>,
    blockchain_service_options: Option<BlockchainServiceConfig>,
    file_transfer_service_options: Option<FileTransferServiceConfig>,
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
            max_storage_capacity: None,
            jump_capacity: None,
            extrinsic_retry_timeout: DEFAULT_EXTRINSIC_RETRY_TIMEOUT_SECONDS,
            indexer_db_pool: None,
            notify_period: None,
            msp_delete_file_options: None,
            msp_charge_fees_options: None,
            msp_move_bucket_options: None,
            bsp_upload_file_options: None,
            bsp_move_bucket_options: None,
            bsp_charge_fees_options: None,
            bsp_submit_proof_options: None,
            blockchain_service_options: None,
            file_transfer_service_options: None,
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
    pub fn with_max_storage_capacity(
        &mut self,
        max_storage_capacity: Option<StorageDataUnit>,
    ) -> &mut Self {
        self.max_storage_capacity = max_storage_capacity;
        self
    }

    /// Set the jump capacity.
    ///
    /// The jump capacity is the amount of storage that the node will increase in its on-chain
    /// capacity by adding more stake. For example, if the jump capacity is set to 1k, and the
    /// node needs 100 units of storage more to store a file, the node will automatically increase
    /// its on-chain capacity by 1k units.
    pub fn with_jump_capacity(&mut self, jump_capacity: Option<StorageDataUnit>) -> &mut Self {
        self.jump_capacity = jump_capacity;
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

    /// Set configuration options for the MSP delete file task.
    pub fn with_msp_delete_file_options(
        &mut self,
        options: crate::command::MspDeleteFileOptions,
    ) -> &mut Self {
        let mut config = crate::tasks::msp_delete_file::MspDeleteFileConfig::default();

        // Apply any non-None values from options to the config
        if let Some(max_try_count) = options.max_try_count {
            config.max_try_count = max_try_count;
        }

        if let Some(max_tip) = options.max_tip {
            config.max_tip = max_tip;
        }

        self.msp_delete_file_options = Some(config);
        self
    }

    /// Set configuration options for the MSP charge fees task.
    pub fn with_msp_charge_fees_options(
        &mut self,
        options: crate::command::MspChargeFeesOptions,
    ) -> &mut Self {
        let mut config = crate::tasks::msp_charge_fees::MspChargeFeesConfig::default();

        // Apply any non-None values from options to the config
        if let Some(min_debt) = options.min_debt {
            config.min_debt = min_debt;
        }

        self.msp_charge_fees_options = Some(config);
        self
    }

    /// Set configuration options for the MSP move bucket task.
    pub fn with_msp_move_bucket_options(
        &mut self,
        options: crate::command::MspMoveBucketOptions,
    ) -> &mut Self {
        let mut config = crate::tasks::msp_move_bucket::MspMoveBucketConfig::default();

        // Apply any non-None values from options to the config
        if let Some(max_concurrent_file_downloads) = options.max_concurrent_file_downloads {
            config.max_concurrent_file_downloads = max_concurrent_file_downloads;
        }

        if let Some(max_concurrent_chunks_per_file) = options.max_concurrent_chunks_per_file {
            config.max_concurrent_chunks_per_file = max_concurrent_chunks_per_file;
        }

        if let Some(max_chunks_per_request) = options.max_chunks_per_request {
            config.max_chunks_per_request = max_chunks_per_request;
        }

        if let Some(chunk_request_peer_retry_attempts) = options.chunk_request_peer_retry_attempts {
            config.chunk_request_peer_retry_attempts = chunk_request_peer_retry_attempts;
        }

        if let Some(download_retry_attempts) = options.download_retry_attempts {
            config.download_retry_attempts = download_retry_attempts;
        }

        if let Some(max_try_count) = options.max_try_count {
            config.max_try_count = max_try_count;
        }

        if let Some(max_tip) = options.max_tip {
            config.max_tip = max_tip;
        }

        if let Some(processing_interval) = options.processing_interval {
            config.processing_interval = processing_interval;
        }

        if let Some(max_batch_size) = options.max_batch_size {
            config.max_batch_size = max_batch_size;
        }

        if let Some(max_parallel_tasks) = options.max_parallel_tasks {
            config.max_parallel_tasks = max_parallel_tasks;
        }

        self.msp_move_bucket_options = Some(config);
        self
    }

    /// Set configuration options for the BSP upload file task.
    pub fn with_bsp_upload_file_options(
        &mut self,
        options: crate::command::BspUploadFileOptions,
    ) -> &mut Self {
        let mut config = crate::tasks::bsp_upload_file::BspUploadFileConfig::default();

        // Apply any non-None values from options to the config
        if let Some(max_try_count) = options.max_try_count {
            config.max_try_count = max_try_count;
        }

        if let Some(max_tip) = options.max_tip {
            config.max_tip = max_tip;
        }

        self.bsp_upload_file_options = Some(config);
        self
    }

    /// Set configuration options for the BSP move bucket task.
    pub fn with_bsp_move_bucket_options(
        &mut self,
        options: crate::command::BspMoveBucketOptions,
    ) -> &mut Self {
        let mut config = crate::tasks::bsp_move_bucket::BspMoveBucketConfig::default();

        // Apply any non-None values from options to the config
        if let Some(move_bucket_accepted_grace_period) = options.move_bucket_accepted_grace_period {
            config.move_bucket_accepted_grace_period = move_bucket_accepted_grace_period;
        }

        self.bsp_move_bucket_options = Some(config);
        self
    }

    /// Set configuration options for the BSP charge fees task.
    pub fn with_bsp_charge_fees_options(
        &mut self,
        options: crate::command::BspChargeFeesOptions,
    ) -> &mut Self {
        let mut config = crate::tasks::bsp_charge_fees::BspChargeFeesConfig::default();

        // Apply any non-None values from options to the config
        if let Some(min_debt) = options.min_debt {
            config.min_debt = min_debt;
        }

        self.bsp_charge_fees_options = Some(config);
        self
    }

    /// Set configuration options for the BSP submit proof task.
    pub fn with_bsp_submit_proof_options(
        &mut self,
        options: crate::command::BspSubmitProofOptions,
    ) -> &mut Self {
        let mut config = crate::tasks::bsp_submit_proof::BspSubmitProofConfig::default();

        // Apply any non-None values from options to the config
        if let Some(max_submission_attempts) = options.max_submission_attempts {
            config.max_submission_attempts = max_submission_attempts;
        }

        self.bsp_submit_proof_options = Some(config);
        self
    }

    /// Set configuration options for the blockchain service.
    pub fn with_blockchain_service_options(
        &mut self,
        options: crate::services::blockchain_service_config::BlockchainServiceConfig,
    ) -> &mut Self {
        self.blockchain_service_options = Some(options);
        self
    }

    /// Set configuration options for the file transfer service.
    pub fn with_file_transfer_service_options(
        &mut self,
        options: FileTransferServiceConfig,
    ) -> &mut Self {
        self.file_transfer_service_options = Some(options);
        self
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
                max_storage_capacity: self
                    .max_storage_capacity
                    .expect("Max Storage Capacity not set"),
                jump_capacity: self.jump_capacity.expect("Jump Capacity not set"),
                extrinsic_retry_timeout: self.extrinsic_retry_timeout,
                msp_delete_file: self.msp_delete_file_options.unwrap_or_default(),
                msp_charge_fees: self.msp_charge_fees_options.unwrap_or_default(),
                msp_move_bucket: self.msp_move_bucket_options.unwrap_or_default(),
                bsp_upload_file: self.bsp_upload_file_options.unwrap_or_default(),
                bsp_move_bucket: self.bsp_move_bucket_options.unwrap_or_default(),
                bsp_charge_fees: self.bsp_charge_fees_options.unwrap_or_default(),
                bsp_submit_proof: self.bsp_submit_proof_options.unwrap_or_default(),
                blockchain_service: self.blockchain_service_options.unwrap_or_default(),
                file_transfer_service: self.file_transfer_service_options.unwrap_or_default(),
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
                max_storage_capacity: self
                    .max_storage_capacity
                    .expect("Max Storage Capacity not set"),
                jump_capacity: self.jump_capacity.expect("Jump Capacity not set"),
                extrinsic_retry_timeout: self.extrinsic_retry_timeout,
                msp_delete_file: self.msp_delete_file_options.unwrap_or_default(),
                msp_charge_fees: self.msp_charge_fees_options.unwrap_or_default(),
                msp_move_bucket: self.msp_move_bucket_options.unwrap_or_default(),
                bsp_upload_file: self.bsp_upload_file_options.unwrap_or_default(),
                bsp_move_bucket: self.bsp_move_bucket_options.unwrap_or_default(),
                bsp_charge_fees: self.bsp_charge_fees_options.unwrap_or_default(),
                bsp_submit_proof: self.bsp_submit_proof_options.unwrap_or_default(),
                blockchain_service: self.blockchain_service_options.unwrap_or_default(),
                file_transfer_service: self.file_transfer_service_options.unwrap_or_default(),
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
                max_storage_capacity: 0,
                jump_capacity: 0,
                extrinsic_retry_timeout: self.extrinsic_retry_timeout,
                msp_delete_file: self.msp_delete_file_options.unwrap_or_default(),
                msp_charge_fees: self.msp_charge_fees_options.unwrap_or_default(),
                msp_move_bucket: self.msp_move_bucket_options.unwrap_or_default(),
                bsp_upload_file: self.bsp_upload_file_options.unwrap_or_default(),
                bsp_move_bucket: self.bsp_move_bucket_options.unwrap_or_default(),
                bsp_charge_fees: self.bsp_charge_fees_options.unwrap_or_default(),
                bsp_submit_proof: self.bsp_submit_proof_options.unwrap_or_default(),
                blockchain_service: self.blockchain_service_options.unwrap_or_default(),
                file_transfer_service: self.file_transfer_service_options.unwrap_or_default(),
            },
            self.indexer_db_pool.clone(),
        )
    }
}
