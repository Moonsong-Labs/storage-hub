pub mod events;
pub mod handler;

use storage_hub_infra::actor::{ActorHandle, ActorSpawner, TaskSpawner};

use self::handler::BlockchainService;

pub async fn spawn_blockchain_service(
    task_spawner: &TaskSpawner,
) -> ActorHandle<BlockchainService> {
    let task_spawner = task_spawner
        .with_name("blockchain-service")
        .with_group("network");

    let blockchain_service = BlockchainService::new();

    task_spawner.spawn_actor(blockchain_service)
}
