pub mod handler;

use std::sync::Arc;

use shc_actors_framework::actor::{ActorHandle, ActorSpawner, TaskSpawner};
use shc_common::traits::{StorageEnableApiCollection, StorageEnableRuntimeApi};
use shc_common::types::ParachainClient;
use shc_indexer_db::DbPool;

pub use self::handler::IndexerService;

pub async fn spawn_indexer_service<
    RuntimeApi: StorageEnableRuntimeApi<RuntimeApi: StorageEnableApiCollection>,
>(
    task_spawner: &TaskSpawner,
    client: Arc<ParachainClient<RuntimeApi>>,
    db_pool: DbPool,
) -> ActorHandle<IndexerService<RuntimeApi>> {
    let task_spawner = task_spawner
        .with_name("indexer-service")
        .with_group("network");

    let indexer_service = IndexerService::new(client, db_pool);

    task_spawner.spawn_actor(indexer_service)
}
