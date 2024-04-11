pub mod events;
pub mod handler;

use std::sync::Arc;

use storage_hub_infra::actor::{ActorHandle, ActorSpawner, TaskSpawner};

use crate::service::ParachainClient;

use self::handler::BlockchainService;

pub async fn spawn_blockchain_service(
    task_spawner: &TaskSpawner,
    client: Arc<ParachainClient>,
) -> ActorHandle<BlockchainService> {
    let task_spawner = task_spawner
        .with_name("blockchain-service")
        .with_group("network");

    let blockchain_service = BlockchainService::new(client);

    task_spawner.spawn_actor(blockchain_service)
}
