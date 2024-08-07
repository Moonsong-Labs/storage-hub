pub mod commands;
pub mod events;
pub mod handler;
pub mod transaction;
pub mod types;

use std::sync::Arc;

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
) -> ActorHandle<BlockchainService> {
    let task_spawner = task_spawner
        .with_name("blockchain-service")
        .with_group("network");

    let blockchain_service = BlockchainService::new(client, rpc_handlers, keystore);

    task_spawner.spawn_actor(blockchain_service)
}
