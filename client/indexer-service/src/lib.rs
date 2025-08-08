pub mod handler;

use std::sync::Arc;

use shc_actors_framework::actor::{ActorHandle, ActorSpawner, TaskSpawner};
use shc_common::traits::{StorageEnableApiCollection, StorageEnableRuntimeApi};
use shc_common::types::ParachainClient;
use shc_indexer_db::DbPool;

pub use self::handler::IndexerService;

/// The mode in which the indexer runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexerMode {
    /// Full indexing mode - indexes all blockchain data
    #[serde(rename = "full")]
    Full,
    /// Lite indexing mode - indexes only essential data for storage operations
    #[serde(rename = "lite")]
    Lite,
    /// Fishing mode - indexes only events relevant to fisherman monitoring (file tracking)
    #[serde(rename = "fishing")]
    Fishing,
}

impl Default for IndexerMode {
    fn default() -> Self {
        Self::Full
    }
}

impl std::str::FromStr for IndexerMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "full" => Ok(Self::Full),
            "lite" => Ok(Self::Lite),
            "fishing" => Ok(Self::Fishing),
            _ => Err(format!(
                "Invalid indexer mode: '{}'. Expected 'full', 'lite', or 'fishing'",
                s
            )),
        }
    }
}

pub async fn spawn_indexer_service<
    RuntimeApi: StorageEnableRuntimeApi<RuntimeApi: StorageEnableApiCollection>,
>(
    task_spawner: &TaskSpawner,
    client: Arc<ParachainClient<RuntimeApi>>,
    db_pool: DbPool,
    indexer_mode: IndexerMode,
) -> ActorHandle<IndexerService<RuntimeApi>> {
    let task_spawner = task_spawner
        .with_name("indexer-service")
        .with_group("network");

    let indexer_service = IndexerService::new(client, db_pool, indexer_mode);

    task_spawner.spawn_actor(indexer_service)
}
