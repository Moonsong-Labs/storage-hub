pub mod capacity_manager;
pub mod commands;
pub mod events;
pub mod handler;
pub mod handler_bsp;
pub mod handler_msp;
pub mod state;
pub mod transaction;
pub mod typed_store;
pub mod types;
pub mod utils;

use std::{path::PathBuf, sync::Arc};

use capacity_manager::{CapacityConfig, CapacityRequestQueue};
use sc_service::RpcHandlers;
use sp_keystore::KeystorePtr;

use shc_actors_framework::actor::{ActorHandle, ActorSpawner, TaskSpawner};
use shc_common::types::ParachainClient;

pub use self::handler::BlockchainService;

pub async fn spawn_blockchain_service<FSH>(
    task_spawner: &TaskSpawner,
    client: Arc<ParachainClient>,
    keystore: KeystorePtr,
    rpc_handlers: Arc<RpcHandlers>,
    forest_storage_handler: FSH,
    rocksdb_root_path: impl Into<PathBuf>,
    notify_period: Option<u32>,
    capacity_config: Option<CapacityConfig>,
    maintenance_mode: bool,
) -> ActorHandle<BlockchainService<FSH>>
where
    FSH: shc_forest_manager::traits::ForestStorageHandler + Clone + Send + Sync + 'static,
{
    let task_spawner = task_spawner
        .with_name("blockchain-service")
        .with_group("network");

    let blockchain_service = BlockchainService::<FSH>::new(
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
