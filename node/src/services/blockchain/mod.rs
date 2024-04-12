pub mod events;
pub mod handler;

use std::sync::Arc;

use sc_service::RpcHandlers;
use storage_hub_infra::actor::{ActorHandle, ActorSpawner, TaskSpawner};

use crate::service::ParachainClient;

use self::handler::BlockchainService;

pub async fn spawn_blockchain_service(
    task_spawner: &TaskSpawner,
    client: Arc<ParachainClient>,
    rpc_handlers: Arc<RpcHandlers>,
) -> ActorHandle<BlockchainService> {
    let task_spawner = task_spawner
        .with_name("blockchain-service")
        .with_group("network");

    let blockchain_service = BlockchainService::new(client, rpc_handlers);

    task_spawner.spawn_actor(blockchain_service)
}
