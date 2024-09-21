pub mod handler;

use anyhow::Result;
use std::sync::Arc;

use shc_actors_framework::actor::{ActorHandle, ActorSpawner, TaskSpawner};
use shc_common::types::ParachainClient;
use shc_indexer_db::DbPool;

pub use self::handler::IndexerService;

pub async fn spawn_indexer_service(
    task_spawner: &TaskSpawner,
    client: Arc<ParachainClient>,
    db_pool: DbPool,
) -> Result<ActorHandle<IndexerService>> {
    let task_spawner = task_spawner
        .with_name("indexer-service")
        .with_group("network");

    let indexer_service = IndexerService::new(client, db_pool);

    Ok(task_spawner.spawn_actor(indexer_service))
}
