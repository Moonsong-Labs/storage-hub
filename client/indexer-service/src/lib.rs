pub mod handler;

use std::sync::Arc;

use shc_actors_framework::actor::{ActorHandle, ActorSpawner, TaskSpawner};
use shc_common::types::OpaqueBlock;
use shc_common::types::ParachainClient;
use shc_indexer_db::DbPool;
use sp_api::ProvideRuntimeApi;

pub use self::handler::IndexerService;

pub async fn spawn_indexer_service<
    RuntimeApi: ProvideRuntimeApi<OpaqueBlock> + Clone + Send + Sync + 'static,
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
