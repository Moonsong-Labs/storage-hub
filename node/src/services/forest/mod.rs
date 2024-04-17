pub mod events;
pub mod handler;

use storage_hub_infra::actor::{ActorHandle, ActorSpawner, TaskSpawner};

use self::handler::ForestService;

pub async fn spawn_forest_service(task_spawner: &TaskSpawner) -> ActorHandle<ForestService> {
    let task_spawner = task_spawner
        .with_name("forest-service")
        .with_group("network");

    let forest_service = ForestService::new();

    task_spawner.spawn_actor(forest_service)
}
