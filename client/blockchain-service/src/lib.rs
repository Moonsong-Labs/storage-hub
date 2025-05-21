pub mod capacity_manager;
pub mod commands;
pub mod events;
pub mod handler;
pub mod handler_bsp;
pub mod handler_msp;
pub mod state;
pub mod transaction;
pub mod types;
pub mod utils;

use std::{path::PathBuf, sync::Arc};

use handler::BlockchainServiceConfig;
use sc_service::RpcHandlers;
use sp_keystore::KeystorePtr;

use capacity_manager::{CapacityConfig, CapacityRequestQueue};
use shc_actors_framework::actor::{ActorHandle, ActorSpawner, TaskSpawner};
use shc_common::types::OpaqueBlock;
use shc_common::types::ParachainClient;
use sp_api::ProvideRuntimeApi;

pub use self::handler::BlockchainService;

pub async fn spawn_blockchain_service<FSH, RuntimeApi>(
    task_spawner: &TaskSpawner,
    config: BlockchainServiceConfig,
    client: Arc<ParachainClient<RuntimeApi>>,
    keystore: KeystorePtr,
    rpc_handlers: Arc<RpcHandlers>,
    forest_storage_handler: FSH,
    rocksdb_root_path: impl Into<PathBuf>,
    notify_period: Option<u32>,
    capacity_config: Option<CapacityConfig>,
    maintenance_mode: bool,
) -> ActorHandle<BlockchainService<FSH, RuntimeApi>>
where
    FSH: shc_forest_manager::traits::ForestStorageHandler + Clone + Send + Sync + 'static,
    RuntimeApi: ProvideRuntimeApi<OpaqueBlock> + Clone + Send + Sync + 'static,
{
    let task_spawner = task_spawner
        .with_name("blockchain-service")
        .with_group("network");

    let blockchain_service = BlockchainService::<FSH, RuntimeApi>::new(
        config,
        client,
        keystore,
        rpc_handlers,
        forest_storage_handler,
        rocksdb_root_path,
        notify_period,
        capacity_config.map(CapacityRequestQueue::new),
        maintenance_mode,
    );

    task_spawner.spawn_actor(blockchain_service)
}
