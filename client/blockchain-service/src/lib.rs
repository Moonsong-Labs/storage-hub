pub mod commands;
pub mod events;
pub mod handler;
pub mod state;
pub mod transaction;
pub mod typed_store;
pub mod types;
pub mod utils;

use std::{path::PathBuf, sync::Arc};

use sc_service::RpcHandlers;
use sp_keystore::KeystorePtr;

use shc_actors_framework::actor::{ActorHandle, ActorSpawner, TaskSpawner};
use shc_common::types::ParachainClient;

pub use self::handler::BlockchainService;

pub async fn spawn_blockchain_service(
    task_spawner: &TaskSpawner,
    client: Arc<ParachainClient>,
    rpc_handlers: Arc<RpcHandlers>,
    keystore: KeystorePtr,
    rocksdb_root_path: impl Into<PathBuf>,
    notify_period: Option<u32>,
) -> ActorHandle<BlockchainService> {
    let task_spawner = task_spawner
        .with_name("blockchain-service")
        .with_group("network");

    let blockchain_service = BlockchainService::new(
        client,
        rpc_handlers,
        keystore,
        rocksdb_root_path,
        notify_period,
    );

    task_spawner.spawn_actor(blockchain_service)
}
