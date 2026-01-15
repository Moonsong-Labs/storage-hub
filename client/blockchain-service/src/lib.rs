pub mod capacity_manager;
pub mod commands;
pub mod events;
pub mod handler;
pub mod handler_bsp;
pub mod handler_msp;
pub mod state;
pub mod transaction_manager;
pub mod transaction_watchers;
pub mod types;
pub mod utils;

use std::{path::PathBuf, sync::Arc};

use handler::BlockchainServiceConfig;
use sc_service::RpcHandlers;
use shc_common::traits::StorageEnableRuntime;
use sp_keystore::KeystorePtr;

use capacity_manager::{CapacityConfig, CapacityRequestQueue};
use shc_actors_framework::actor::{ActorHandle, ActorSpawner, TaskSpawner};
use shc_common::types::StorageHubClient;
use shc_telemetry::MetricsLink;

// Re-export forest write lock types from actors-framework
pub use shc_actors_framework::forest_write_lock::{
    ForestRootWriteAccess, ForestRootWriteGate, ForestRootWriteGuard, ForestRootWriteGuardSlot,
};

pub use self::{
    handler::BlockchainService,
    types::{FileKeyStatus, FileKeyStatusUpdate},
};

pub async fn spawn_blockchain_service<FSH, Runtime>(
    task_spawner: &TaskSpawner,
    config: BlockchainServiceConfig<Runtime>,
    client: Arc<StorageHubClient<Runtime::RuntimeApi>>,
    keystore: KeystorePtr,
    rpc_handlers: Arc<RpcHandlers>,
    forest_storage_handler: FSH,
    rocksdb_root_path: impl Into<PathBuf>,
    notify_period: Option<u32>,
    capacity_config: Option<CapacityConfig<Runtime>>,
    maintenance_mode: bool,
    forest_lock_manager: Arc<ForestRootWriteGate>,
    metrics: MetricsLink,
) -> ActorHandle<BlockchainService<FSH, Runtime>>
where
    FSH: shc_forest_manager::traits::ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    let task_spawner = task_spawner
        .with_name("blockchain-service")
        .with_group("network");

    let blockchain_service = BlockchainService::<FSH, Runtime>::new(
        config,
        client,
        keystore,
        rpc_handlers,
        forest_storage_handler,
        rocksdb_root_path,
        notify_period,
        capacity_config.map(CapacityRequestQueue::new),
        maintenance_mode,
        forest_lock_manager,
        metrics,
    );

    task_spawner.spawn_actor(blockchain_service)
}
