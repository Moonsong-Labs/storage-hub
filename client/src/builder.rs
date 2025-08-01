use async_channel::Receiver;
use log::*;
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
use shc_common::{
    traits::{StorageEnableApiCollection, StorageEnableRuntimeApi},
    types::ParachainClient,
};
use shc_file_manager::{in_memory::InMemoryFileStorage, rocksdb::RocksDbFileStorage};
use shc_file_transfer_service::{spawn_file_transfer_service, FileTransferService};
use shc_forest_manager::traits::ForestStorageHandler;
use shc_rpc::{remote_file::RemoteFileConfig, StorageHubClientRpcConfig};

use crate::tasks::{
    bsp_charge_fees::BspChargeFeesConfig, bsp_move_bucket::BspMoveBucketConfig,
    bsp_submit_proof::BspSubmitProofConfig, bsp_upload_file::BspUploadFileConfig,
    msp_charge_fees::MspChargeFeesConfig, msp_move_bucket::MspMoveBucketConfig,
};

use super::{
    bsp_peer_manager::BspPeerManager,
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
pub struct StorageHubBuilder<R, S, RuntimeApi>
where
    R: ShRole,
    S: ShStorageLayer,
    (R, S): ShNodeType,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    task_spawner: Option<TaskSpawner>,
    file_transfer: Option<ActorHandle<FileTransferService>>,
    blockchain: Option<ActorHandle<BlockchainService<<(R, S) as ShNodeType>::FSH, RuntimeApi>>>,
    storage_path: Option<String>,
    file_storage: Option<Arc<RwLock<<(R, S) as ShNodeType>::FL>>>,
    forest_storage_handler: Option<<(R, S) as ShNodeType>::FSH>,
    capacity_config: Option<CapacityConfig>,
    indexer_db_pool: Option<DbPool>,
    notify_period: Option<u32>,
    // Configuration options for tasks and services
    msp_charge_fees_config: Option<MspChargeFeesConfig>,
    msp_move_bucket_config: Option<MspMoveBucketConfig>,
    bsp_upload_file_config: Option<BspUploadFileConfig>,
    bsp_move_bucket_config: Option<BspMoveBucketConfig>,
    bsp_charge_fees_config: Option<BspChargeFeesConfig>,
    bsp_submit_proof_config: Option<BspSubmitProofConfig>,
    blockchain_service_config: Option<BlockchainServiceConfig>,
    peer_manager: Option<Arc<BspPeerManager>>,
}

/// Common components to build for any given configuration of [`ShRole`] and [`ShStorageLayer`].
impl<R: ShRole, S: ShStorageLayer, RuntimeApi> StorageHubBuilder<R, S, RuntimeApi>
where
    (R, S): ShNodeType,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
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
            msp_charge_fees_config: None,
            msp_move_bucket_config: None,
            bsp_upload_file_config: None,
            bsp_move_bucket_config: None,
            bsp_charge_fees_config: None,
            bsp_submit_proof_config: None,
            blockchain_service_config: None,
            peer_manager: None,
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
        client: Arc<ParachainClient<RuntimeApi>>,
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

        let blockchain_service_config = self.blockchain_service_config.clone().unwrap_or_default();

        let blockchain_service_handle =
            spawn_blockchain_service::<<(R, S) as ShNodeType>::FSH, RuntimeApi>(
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

    /// Initialize the BSP peer manager for tracking peer performance
    pub fn with_peer_manager(&mut self, rocks_db_path: PathBuf) -> &mut Self {
        let mut peer_db_path = rocks_db_path;
        peer_db_path.push("bsp_peer_manager");

        // Create directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(&peer_db_path) {
            warn!(
                "Failed to create directory for BSP peer manager at {:?}: {}. Will continue without peer manager.",
                peer_db_path, e
            );
            return self;
        }

        let manager = BspPeerManager::new(peer_db_path)
            .expect("Failed to initialize BSP peer manager. This is a critical component and the node cannot function without it.");

        info!("Successfully initialized BSP peer manager");
        self.peer_manager = Some(Arc::new(manager));
        self
    }

    /// Create the RPC configuration needed to initialise the RPC methods of the StorageHub client.
    ///
    /// This method is meant to be called after the Storage Layer has been set up.
    /// Call [`setup_storage_layer`](StorageHubBuilder::setup_storage_layer) before calling this method.
    pub fn create_rpc_config(
        &self,
        keystore: KeystorePtr,
        remote_file_config: RemoteFileConfig,
    ) -> StorageHubClientRpcConfig<<(R, S) as ShNodeType>::FL, <(R, S) as ShNodeType>::FSH> {
        StorageHubClientRpcConfig::new(
            self.file_storage
                .clone()
                .expect("File Storage not initialized. Use `setup_storage_layer` before calling `create_rpc_config`."),
            self.forest_storage_handler
                .clone()
                .expect("Forest Storage Handler not initialized. Use `setup_storage_layer` before calling `create_rpc_config`."),
            keystore,
            remote_file_config,
        )
    }

    /// Set configuration options for the MSP charge fees task.
    pub fn with_msp_charge_fees_config(
        &mut self,
        config: Option<MspChargeFeesOptions>,
    ) -> &mut Self {
        self.msp_charge_fees_config = config.map(Into::into);
        self
    }

    /// Set configuration options for the MSP move bucket task.
    pub fn with_msp_move_bucket_config(
        &mut self,
        config: Option<MspMoveBucketOptions>,
    ) -> &mut Self {
        self.msp_move_bucket_config = config.map(Into::into);
        self
    }

    /// Set configuration options for the BSP upload file task.
    pub fn with_bsp_upload_file_config(
        &mut self,
        config: Option<BspUploadFileOptions>,
    ) -> &mut Self {
        self.bsp_upload_file_config = config.map(Into::into);
        self
    }

    /// Set configuration options for the BSP move bucket task.
    pub fn with_bsp_move_bucket_config(
        &mut self,
        config: Option<BspMoveBucketOptions>,
    ) -> &mut Self {
        self.bsp_move_bucket_config = config.map(Into::into);
        self
    }

    /// Set configuration options for the BSP charge fees task.
    pub fn with_bsp_charge_fees_config(
        &mut self,
        config: Option<BspChargeFeesOptions>,
    ) -> &mut Self {
        self.bsp_charge_fees_config = config.map(Into::into);
        self
    }

    /// Set configuration options for the BSP submit proof task.
    pub fn with_bsp_submit_proof_config(
        &mut self,
        config: Option<BspSubmitProofOptions>,
    ) -> &mut Self {
        self.bsp_submit_proof_config = config.map(Into::into);
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

        if let Some(sync_mode_min_blocks_behind) = config.sync_mode_min_blocks_behind {
            blockchain_service_config.sync_mode_min_blocks_behind = sync_mode_min_blocks_behind;
        }

        if let Some(check_for_pending_proofs_period) = config.check_for_pending_proofs_period {
            blockchain_service_config.check_for_pending_proofs_period =
                check_for_pending_proofs_period;
        }

        if let Some(max_blocks_behind_to_catch_up_root_changes) =
            config.max_blocks_behind_to_catch_up_root_changes
        {
            blockchain_service_config.max_blocks_behind_to_catch_up_root_changes =
                max_blocks_behind_to_catch_up_root_changes;
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

impl<RuntimeApi> StorageLayerBuilder
    for StorageHubBuilder<BspProvider, InMemoryStorageLayer, RuntimeApi>
where
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    fn setup_storage_layer(&mut self, _storage_path: Option<String>) -> &mut Self {
        self.file_storage = Some(Arc::new(RwLock::new(InMemoryFileStorage::new())));
        self.forest_storage_handler =
            Some(<(BspProvider, InMemoryStorageLayer) as ShNodeType>::FSH::new());

        self
    }
}

impl<RuntimeApi> StorageLayerBuilder
    for StorageHubBuilder<BspProvider, RocksDbStorageLayer, RuntimeApi>
where
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
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

impl<RuntimeApi> StorageLayerBuilder
    for StorageHubBuilder<MspProvider, InMemoryStorageLayer, RuntimeApi>
where
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    fn setup_storage_layer(&mut self, _storage_path: Option<String>) -> &mut Self {
        self.file_storage = Some(Arc::new(RwLock::new(InMemoryFileStorage::new())));
        self.forest_storage_handler =
            Some(<(MspProvider, InMemoryStorageLayer) as ShNodeType>::FSH::new());

        self
    }
}

impl<RuntimeApi> StorageLayerBuilder
    for StorageHubBuilder<MspProvider, RocksDbStorageLayer, RuntimeApi>
where
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
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

impl<RuntimeApi> StorageLayerBuilder for StorageHubBuilder<UserRole, NoStorageLayer, RuntimeApi>
where
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
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
pub trait Buildable<
    NT: ShNodeType,
    RuntimeApi: StorageEnableRuntimeApi<RuntimeApi: StorageEnableApiCollection>,
>
{
    fn build(self) -> StorageHubHandler<NT, RuntimeApi>;
}

impl<S: ShStorageLayer, RuntimeApi> Buildable<(BspProvider, S), RuntimeApi>
    for StorageHubBuilder<BspProvider, S, RuntimeApi>
where
    (BspProvider, S): ShNodeType,
    <(BspProvider, S) as ShNodeType>::FSH: BspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    fn build(self) -> StorageHubHandler<(BspProvider, S), RuntimeApi> {
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
                msp_charge_fees: self.msp_charge_fees_config.unwrap_or_default(),
                msp_move_bucket: self.msp_move_bucket_config.unwrap_or_default(),
                bsp_upload_file: self.bsp_upload_file_config.unwrap_or_default(),
                bsp_move_bucket: self.bsp_move_bucket_config.unwrap_or_default(),
                bsp_charge_fees: self.bsp_charge_fees_config.unwrap_or_default(),
                bsp_submit_proof: self.bsp_submit_proof_config.unwrap_or_default(),
                blockchain_service: self.blockchain_service_config.unwrap_or_default(),
            },
            self.indexer_db_pool.clone(),
            self.peer_manager.expect("Peer Manager not set"),
        )
    }
}

impl<S: ShStorageLayer, RuntimeApi> Buildable<(MspProvider, S), RuntimeApi>
    for StorageHubBuilder<MspProvider, S, RuntimeApi>
where
    (MspProvider, S): ShNodeType,
    <(MspProvider, S) as ShNodeType>::FSH: MspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    fn build(self) -> StorageHubHandler<(MspProvider, S), RuntimeApi> {
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
                msp_charge_fees: self.msp_charge_fees_config.unwrap_or_default(),
                msp_move_bucket: self.msp_move_bucket_config.unwrap_or_default(),
                bsp_upload_file: self.bsp_upload_file_config.unwrap_or_default(),
                bsp_move_bucket: self.bsp_move_bucket_config.unwrap_or_default(),
                bsp_charge_fees: self.bsp_charge_fees_config.unwrap_or_default(),
                bsp_submit_proof: self.bsp_submit_proof_config.unwrap_or_default(),
                blockchain_service: self.blockchain_service_config.unwrap_or_default(),
            },
            self.indexer_db_pool.clone(),
            self.peer_manager.expect("Peer Manager not set"),
        )
    }
}

impl<RuntimeApi> Buildable<(UserRole, NoStorageLayer), RuntimeApi>
    for StorageHubBuilder<UserRole, NoStorageLayer, RuntimeApi>
where
    (UserRole, NoStorageLayer): ShNodeType,
    <(UserRole, NoStorageLayer) as ShNodeType>::FSH:
        ForestStorageHandler + Clone + Send + Sync + 'static,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    fn build(self) -> StorageHubHandler<(UserRole, NoStorageLayer), RuntimeApi> {
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
                msp_charge_fees: self.msp_charge_fees_config.unwrap_or_default(),
                msp_move_bucket: self.msp_move_bucket_config.unwrap_or_default(),
                bsp_upload_file: self.bsp_upload_file_config.unwrap_or_default(),
                bsp_move_bucket: self.bsp_move_bucket_config.unwrap_or_default(),
                bsp_charge_fees: self.bsp_charge_fees_config.unwrap_or_default(),
                bsp_submit_proof: self.bsp_submit_proof_config.unwrap_or_default(),
                blockchain_service: self.blockchain_service_config.unwrap_or_default(),
            },
            self.indexer_db_pool.clone(),
            self.peer_manager.expect("Peer Manager not set"),
        )
    }
}

/// Configuration options for the MSP Charge Fees task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MspChargeFeesOptions {
    /// Minimum debt threshold for charging users.
    pub min_debt: Option<u64>,
}

impl Into<MspChargeFeesConfig> for MspChargeFeesOptions {
    fn into(self) -> MspChargeFeesConfig {
        MspChargeFeesConfig {
            min_debt: self.min_debt.unwrap_or_default(),
        }
    }
}

/// Configuration options for the MSP Move Bucket task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MspMoveBucketOptions {
    /// Maximum number of times to retry a move bucket request.
    pub max_try_count: Option<u32>,
    /// Maximum tip amount to use when submitting a move bucket request extrinsic.
    pub max_tip: Option<f64>,
}

impl Into<MspMoveBucketConfig> for MspMoveBucketOptions {
    fn into(self) -> MspMoveBucketConfig {
        MspMoveBucketConfig {
            max_try_count: self.max_try_count.unwrap_or_default(),
            max_tip: self.max_tip.unwrap_or_default(),
        }
    }
}

/// Configuration options for the BSP Upload File task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BspUploadFileOptions {
    /// Maximum number of times to retry an upload file request.
    pub max_try_count: Option<u32>,
    /// Maximum tip amount to use when submitting an upload file request extrinsic.
    pub max_tip: Option<f64>,
}

impl Into<BspUploadFileConfig> for BspUploadFileOptions {
    fn into(self) -> BspUploadFileConfig {
        BspUploadFileConfig {
            max_try_count: self.max_try_count.unwrap_or_default(),
            max_tip: self.max_tip.unwrap_or_default(),
        }
    }
}

/// Configuration options for the BSP Move Bucket task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BspMoveBucketOptions {
    /// Grace period in seconds to accept download requests after a bucket move is accepted.
    pub move_bucket_accepted_grace_period: Option<u64>,
}

impl Into<BspMoveBucketConfig> for BspMoveBucketOptions {
    fn into(self) -> BspMoveBucketConfig {
        BspMoveBucketConfig {
            move_bucket_accepted_grace_period: self
                .move_bucket_accepted_grace_period
                .unwrap_or_default(),
        }
    }
}

/// Configuration options for the BSP Charge Fees task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BspChargeFeesOptions {
    /// Minimum debt threshold for charging users.
    pub min_debt: Option<u64>,
}

impl Into<BspChargeFeesConfig> for BspChargeFeesOptions {
    fn into(self) -> BspChargeFeesConfig {
        BspChargeFeesConfig {
            min_debt: self.min_debt.unwrap_or_default(),
        }
    }
}

/// Configuration options for the BSP Submit Proof task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BspSubmitProofOptions {
    /// Maximum number of attempts to submit a proof.
    pub max_submission_attempts: Option<u32>,
}

impl Into<BspSubmitProofConfig> for BspSubmitProofOptions {
    fn into(self) -> BspSubmitProofConfig {
        BspSubmitProofConfig {
            max_submission_attempts: self.max_submission_attempts.unwrap_or_default(),
        }
    }
}

/// Configuration options for the Blockchain Service.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BlockchainServiceOptions {
    /// Extrinsic retry timeout in seconds.
    pub extrinsic_retry_timeout: Option<u64>,
    /// The minimum number of blocks behind the current best block to consider the node out of sync.
    pub sync_mode_min_blocks_behind: Option<u32>,
    /// On blocks that are multiples of this number, the blockchain service will trigger the catch of proofs.
    pub check_for_pending_proofs_period: Option<u32>,
    /// The maximum number of blocks from the past that will be processed for catching up the root changes.
    pub max_blocks_behind_to_catch_up_root_changes: Option<u32>,
}

impl Into<BlockchainServiceConfig> for BlockchainServiceOptions {
    fn into(self) -> BlockchainServiceConfig {
        BlockchainServiceConfig {
            extrinsic_retry_timeout: self.extrinsic_retry_timeout.unwrap_or_default(),
            sync_mode_min_blocks_behind: self.sync_mode_min_blocks_behind.unwrap_or_default(),
            check_for_pending_proofs_period: self
                .check_for_pending_proofs_period
                .unwrap_or_default(),
            max_blocks_behind_to_catch_up_root_changes: self
                .max_blocks_behind_to_catch_up_root_changes
                .unwrap_or_default(),
        }
    }
}

/// Configuration for the indexer.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct IndexerOptions {
    /// Whether to enable the indexer.
    pub indexer: bool,
    /// Postgres database URL.
    pub database_url: Option<String>,
}

/// Remote file configuration options.
#[derive(Clone, Debug, serde::Serialize, Deserialize)]
pub struct RemoteFileOptions {
    /// Maximum file size in bytes (default: 10GB)
    #[serde(default)]
    pub max_file_size: u64,
    /// Connection timeout in seconds (default: 30)
    #[serde(default)]
    pub connection_timeout: u64,
    /// Read timeout in seconds (default: 300)
    #[serde(default)]
    pub read_timeout: u64,
    /// Whether to follow redirects (default: true)
    #[serde(default)]
    pub follow_redirects: bool,
    /// Maximum number of redirects (default: 10)
    #[serde(default)]
    pub max_redirects: u32,
    /// User agent string (default: "StorageHub-Client/1.0")
    #[serde(default)]
    pub user_agent: String,
    /// Chunk size in bytes (default: 8192)
    #[serde(default)]
    pub chunk_size: usize,
    /// Number of FILE_CHUNK_SIZE chunks to buffer (default: 512)
    #[serde(default)]
    pub chunks_buffer: usize,
}

impl Default for RemoteFileOptions {
    fn default() -> Self {
        let config = shc_rpc::remote_file::RemoteFileConfig::default();
        Self {
            max_file_size: config.max_file_size,
            connection_timeout: config.connection_timeout,
            read_timeout: config.read_timeout,
            follow_redirects: config.follow_redirects,
            max_redirects: config.max_redirects,
            user_agent: config.user_agent,
            chunk_size: config.chunk_size,
            chunks_buffer: config.chunks_buffer,
        }
    }
}

impl From<RemoteFileOptions> for shc_rpc::remote_file::RemoteFileConfig {
    fn from(options: RemoteFileOptions) -> Self {
        Self {
            max_file_size: options.max_file_size,
            connection_timeout: options.connection_timeout,
            read_timeout: options.read_timeout,
            follow_redirects: options.follow_redirects,
            max_redirects: options.max_redirects,
            user_agent: options.user_agent,
            chunk_size: options.chunk_size,
            chunks_buffer: options.chunks_buffer,
        }
    }
}

/// RPC configuration options.
#[derive(Clone, Debug, serde::Serialize, Deserialize)]
pub struct RpcOptions {
    /// Remote file configuration options
    #[serde(default)]
    pub remote_file: RemoteFileOptions,
}

impl Default for RpcOptions {
    fn default() -> Self {
        Self {
            remote_file: RemoteFileOptions::default(),
        }
    }
}
