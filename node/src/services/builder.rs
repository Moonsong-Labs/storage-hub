use async_channel::Receiver;
use log::info;
use sc_network::{config::IncomingRequest, service::traits::NetworkService, ProtocolName};
use sc_service::RpcHandlers;
use serde::Deserialize;
use shc_indexer_db::DbPool;
use sp_keystore::KeystorePtr;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::RwLock;

use shc_actors_framework::actor::{ActorHandle, TaskSpawner};
use shc_blockchain_service::{
    capacity_manager::CapacityConfig, handler::BlockchainServiceConfig, spawn_blockchain_service,
    BlockchainService,
};
use shc_common::types::ParachainClient;
use shc_file_manager::{in_memory::InMemoryFileStorage, rocksdb::RocksDbFileStorage};
use shc_file_transfer_service::{spawn_file_transfer_service, FileTransferService};
use shc_forest_manager::traits::ForestStorageHandler;
use shc_rpc::StorageHubClientRpcConfig;

const LOG_TARGET: &str = "storage_hub_builder";

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
    capacity_config: Option<CapacityConfig>,
    indexer_db_pool: Option<DbPool>,
    notify_period: Option<u32>,
    // Configuration options for tasks and services
    msp_delete_file_config: Option<MspDeleteFileConfig>,
    msp_charge_fees_config: Option<MspChargeFeesConfig>,
    msp_move_bucket_config: Option<MspMoveBucketConfig>,
    bsp_upload_file_config: Option<BspUploadFileConfig>,
    bsp_move_bucket_config: Option<BspMoveBucketConfig>,
    bsp_charge_fees_config: Option<BspChargeFeesConfig>,
    bsp_submit_proof_config: Option<BspSubmitProofConfig>,
    blockchain_service_config: Option<BlockchainServiceConfig>,
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
            indexer_db_pool: None,
            notify_period: None,
            msp_delete_file_config: None,
            msp_charge_fees_config: None,
            msp_move_bucket_config: None,
            bsp_upload_file_config: None,
            bsp_move_bucket_config: None,
            bsp_charge_fees_config: None,
            bsp_submit_proof_config: None,
            blockchain_service_config: None,
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

        let capacity_config = self.capacity_config.clone();

        let blockchain_service_config = self.blockchain_service_config.clone().unwrap_or_default();

        let blockchain_service_handle = spawn_blockchain_service::<<(R, S) as ShNodeType>::FSH>(
            self.task_spawner
                .as_ref()
                .expect("Task spawner is not set."),
            blockchain_service_config,
            client.clone(),
            keystore.clone(),
            rpc_handlers.clone(),
            forest_storage_handler,
            rocksdb_root_path,
            self.notify_period,
            capacity_config,
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
    pub fn with_msp_delete_file_config(&mut self, config: MspDeleteFileOptions) -> &mut Self {
        let mut msp_delete_file_config = MspDeleteFileConfig::default();

        if let Some(max_tip) = config.max_tip {
            msp_delete_file_config.max_tip = max_tip;
        }

        if let Some(max_try_count) = config.max_try_count {
            msp_delete_file_config.max_try_count = max_try_count;
        }

        self.msp_delete_file_config = Some(msp_delete_file_config);
        self
    }

    /// Set configuration options for the MSP charge fees task.
    pub fn with_msp_charge_fees_config(&mut self, config: MspChargeFeesOptions) -> &mut Self {
        let mut msp_charge_fees_config = MspChargeFeesConfig::default();

        if let Some(min_debt) = config.min_debt {
            msp_charge_fees_config.min_debt = min_debt;
        }

        self.msp_charge_fees_config = Some(msp_charge_fees_config);
        self
    }

    /// Set configuration options for the MSP move bucket task.
    pub fn with_msp_move_bucket_config(&mut self, config: MspMoveBucketOptions) -> &mut Self {
        let mut msp_move_bucket_config = MspMoveBucketConfig::default();

        if let Some(max_try_count) = config.max_try_count {
            msp_move_bucket_config.max_try_count = max_try_count;
        }

        if let Some(max_tip) = config.max_tip {
            msp_move_bucket_config.max_tip = max_tip;
        }

        if let Some(processing_interval) = config.processing_interval {
            msp_move_bucket_config.processing_interval = processing_interval;
        }

        if let Some(max_concurrent_file_downloads) = config.max_concurrent_file_downloads {
            msp_move_bucket_config.max_concurrent_file_downloads = max_concurrent_file_downloads;
        }

        if let Some(max_concurrent_chunks_per_file) = config.max_concurrent_chunks_per_file {
            msp_move_bucket_config.max_concurrent_chunks_per_file = max_concurrent_chunks_per_file;
        }

        if let Some(max_chunks_per_request) = config.max_chunks_per_request {
            msp_move_bucket_config.max_chunks_per_request = max_chunks_per_request;
        }

        if let Some(chunk_request_peer_retry_attempts) = config.chunk_request_peer_retry_attempts {
            msp_move_bucket_config.chunk_request_peer_retry_attempts =
                chunk_request_peer_retry_attempts;
        }

        if let Some(download_retry_attempts) = config.download_retry_attempts {
            msp_move_bucket_config.download_retry_attempts = download_retry_attempts;
        }

        self.msp_move_bucket_config = Some(msp_move_bucket_config);
        self
    }

    /// Set configuration options for the BSP upload file task.
    pub fn with_bsp_upload_file_config(&mut self, config: BspUploadFileOptions) -> &mut Self {
        let mut bsp_upload_file_config = BspUploadFileConfig::default();

        if let Some(max_try_count) = config.max_try_count {
            bsp_upload_file_config.max_try_count = max_try_count;
        }

        self.bsp_upload_file_config = Some(bsp_upload_file_config);
        self
    }

    /// Set configuration options for the BSP move bucket task.
    pub fn with_bsp_move_bucket_config(&mut self, config: BspMoveBucketOptions) -> &mut Self {
        let mut bsp_move_bucket_config = BspMoveBucketConfig::default();

        if let Some(move_bucket_accepted_grace_period) = config.move_bucket_accepted_grace_period {
            bsp_move_bucket_config.move_bucket_accepted_grace_period =
                move_bucket_accepted_grace_period;
        }

        self.bsp_move_bucket_config = Some(bsp_move_bucket_config);
        self
    }

    /// Set configuration options for the BSP charge fees task.
    pub fn with_bsp_charge_fees_config(&mut self, config: BspChargeFeesOptions) -> &mut Self {
        let mut bsp_charge_fees_config = BspChargeFeesConfig::default();

        if let Some(min_debt) = config.min_debt {
            bsp_charge_fees_config.min_debt = min_debt;
        }

        self.bsp_charge_fees_config = Some(bsp_charge_fees_config);
        self
    }

    /// Set configuration options for the BSP submit proof task.
    pub fn with_bsp_submit_proof_config(&mut self, config: BspSubmitProofOptions) -> &mut Self {
        let mut bsp_submit_proof_config = BspSubmitProofConfig::default();

        if let Some(max_submission_attempts) = config.max_submission_attempts {
            bsp_submit_proof_config.max_submission_attempts = max_submission_attempts;
        }

        self.bsp_submit_proof_config = Some(bsp_submit_proof_config);
        self
    }

    /// Set configuration options for the blockchain service.
    pub fn with_blockchain_service_config(
        &mut self,
        config: BlockchainServiceOptions,
    ) -> &mut Self {
        let mut blockchain_service_config = BlockchainServiceConfig::default();

        if let Some(extrinsic_retry_timeout) = config.extrinsic_retry_timeout {
            blockchain_service_config.extrinsic_retry_timeout = extrinsic_retry_timeout;
        }

        self.blockchain_service_config = Some(blockchain_service_config);
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
        let handler = StorageHubHandler::new(
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
                msp_delete_file: self.msp_delete_file_config.unwrap_or_default(),
                msp_charge_fees: self.msp_charge_fees_config.unwrap_or_default(),
                msp_move_bucket: self.msp_move_bucket_config.unwrap_or_default(),
                bsp_upload_file: self.bsp_upload_file_config.unwrap_or_default(),
                bsp_move_bucket: self.bsp_move_bucket_config.unwrap_or_default(),
                bsp_charge_fees: self.bsp_charge_fees_config.unwrap_or_default(),
                bsp_submit_proof: self.bsp_submit_proof_config.unwrap_or_default(),
                blockchain_service: self.blockchain_service_config.unwrap_or_default(),
            },
            self.indexer_db_pool.clone(),
        );

        info!(target: LOG_TARGET, "StorageHubHandler configurations: {:?}", handler);

        handler
    }
}

impl<S: ShStorageLayer> Buildable<(MspProvider, S)> for StorageHubBuilder<MspProvider, S>
where
    (MspProvider, S): ShNodeType,
    <(MspProvider, S) as ShNodeType>::FSH: MspForestStorageHandlerT,
{
    fn build(self) -> StorageHubHandler<(MspProvider, S)> {
        let handler = StorageHubHandler::new(
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
                msp_delete_file: self.msp_delete_file_config.unwrap_or_default(),
                msp_charge_fees: self.msp_charge_fees_config.unwrap_or_default(),
                msp_move_bucket: self.msp_move_bucket_config.unwrap_or_default(),
                bsp_upload_file: self.bsp_upload_file_config.unwrap_or_default(),
                bsp_move_bucket: self.bsp_move_bucket_config.unwrap_or_default(),
                bsp_charge_fees: self.bsp_charge_fees_config.unwrap_or_default(),
                bsp_submit_proof: self.bsp_submit_proof_config.unwrap_or_default(),
                blockchain_service: self.blockchain_service_config.unwrap_or_default(),
            },
            self.indexer_db_pool.clone(),
        );

        info!(target: LOG_TARGET, "StorageHubHandler configurations: {:?}", handler);

        handler
    }
}

impl Buildable<(UserRole, NoStorageLayer)> for StorageHubBuilder<UserRole, NoStorageLayer>
where
    (UserRole, NoStorageLayer): ShNodeType,
    <(UserRole, NoStorageLayer) as ShNodeType>::FSH:
        ForestStorageHandler + Clone + Send + Sync + 'static,
{
    fn build(self) -> StorageHubHandler<(UserRole, NoStorageLayer)> {
        let handler = StorageHubHandler::new(
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
                msp_delete_file: self.msp_delete_file_config.unwrap_or_default(),
                msp_charge_fees: self.msp_charge_fees_config.unwrap_or_default(),
                msp_move_bucket: self.msp_move_bucket_config.unwrap_or_default(),
                bsp_upload_file: self.bsp_upload_file_config.unwrap_or_default(),
                bsp_move_bucket: self.bsp_move_bucket_config.unwrap_or_default(),
                bsp_charge_fees: self.bsp_charge_fees_config.unwrap_or_default(),
                bsp_submit_proof: self.bsp_submit_proof_config.unwrap_or_default(),
                blockchain_service: self.blockchain_service_config.unwrap_or_default(),
            },
            self.indexer_db_pool.clone(),
        );

        info!(target: LOG_TARGET, "StorageHubHandler configurations: {:?}", handler);

        handler
    }
}

/// Configuration options for the MSP Delete File task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MspDeleteFileOptions {
    /// Maximum number of times to retry a file deletion request.
    pub max_try_count: Option<u32>,
    /// Maximum tip amount to use when submitting a file deletion request extrinsic.
    pub max_tip: Option<f64>,
}

/// Configuration options for the MSP Charge Fees task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MspChargeFeesOptions {
    /// Minimum debt threshold for charging users.
    pub min_debt: Option<u64>,
}

/// Configuration options for the MSP Move Bucket task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MspMoveBucketOptions {
    /// Maximum number of times to retry a move bucket request.
    pub max_try_count: Option<u32>,
    /// Maximum tip amount to use when submitting a move bucket request extrinsic.
    pub max_tip: Option<f64>,
    /// Processing interval between batches of move bucket requests.
    pub processing_interval: Option<u64>,
    /// Maximum number of files to download in parallel.
    pub max_concurrent_file_downloads: Option<usize>,
    /// Maximum number of chunks requests to do in parallel per file.
    pub max_concurrent_chunks_per_file: Option<usize>,
    /// Maximum number of chunks to request in a single network request.
    pub max_chunks_per_request: Option<usize>,
    /// Number of peers to select for each chunk download attempt (2 best + x random).
    pub chunk_request_peer_retry_attempts: Option<usize>,
    /// Number of retries per peer for a single chunk request.
    pub download_retry_attempts: Option<usize>,
}

/// Configuration options for the BSP Upload File task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BspUploadFileOptions {
    /// Maximum number of times to retry an upload file request.
    pub max_try_count: Option<u32>,
    /// Maximum tip amount to use when submitting an upload file request extrinsic.
    pub max_tip: Option<f64>,
}

/// Configuration options for the BSP Move Bucket task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BspMoveBucketOptions {
    /// Grace period in seconds to accept download requests after a bucket move is accepted.
    pub move_bucket_accepted_grace_period: Option<u64>,
}

/// Configuration options for the BSP Charge Fees task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BspChargeFeesOptions {
    /// Minimum debt threshold for charging users.
    pub min_debt: Option<u64>,
}

/// Configuration options for the BSP Submit Proof task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BspSubmitProofOptions {
    /// Maximum number of attempts to submit a proof.
    pub max_submission_attempts: Option<u32>,
}
/// Configuration options for the Blockchain Service.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BlockchainServiceOptions {
    /// Extrinsic retry timeout in seconds.
    pub extrinsic_retry_timeout: Option<u64>,
}

/// Configuration for the indexer.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct IndexerOptions {
    /// Whether to enable the indexer.
    pub indexer: bool,
    /// Postgres database URL.
    pub database_url: Option<String>,
}
