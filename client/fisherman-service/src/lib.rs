//! # Fisherman Service
//!
//! The Fisherman Service is responsible for monitoring the StorageHub network and validating
//! storage provider behavior. It runs as a concurrent actor alongside other services and
//! depends on the indexer service for blockchain data access.
//!
//! ## Key Features
//!
//! - Monitors blockchain events for storage provider activities
//! - Validates storage proofs and challenges
//! - Detects potential misbehavior patterns
//! - Can submit challenges when necessary
//! - Provides manual validation triggers via commands
//!
//! ## Dependencies
//!
//! - Requires the indexer service to be running
//! - Needs access to the blockchain client
//! - Requires database access for indexed data

pub mod handler;

use std::sync::Arc;

use shc_actors_framework::actor::{ActorHandle, ActorSpawner, TaskSpawner};
use shc_common::traits::{StorageEnableApiCollection, StorageEnableRuntimeApi};
use shc_common::types::ParachainClient;
use shc_indexer_db::DbPool;

pub use self::handler::{FishermanService, FishermanServiceCommand, FishermanServiceError};

/// Spawn the fisherman service as an actor
///
/// This function creates and spawns a new FishermanService actor that will monitor
/// the StorageHub network for storage provider behavior and validate activities.
///
/// # Arguments
///
/// * `task_spawner` - The task spawner for creating the actor
/// * `client` - Arc-wrapped parachain client for blockchain interaction
/// * `db_pool` - Database pool for accessing indexed data
///
/// # Returns
///
/// Returns an ActorHandle for the spawned FishermanService
///
/// # Example
///
/// ```rust,ignore
/// let task_spawner = TaskSpawner::new(task_manager.spawn_handle(), "fisherman");
/// let fisherman_handle = spawn_fisherman_service(
///     &task_spawner,
///     client.clone(),
///     db_pool.clone(),
/// ).await;
/// ```
pub async fn spawn_fisherman_service<
    RuntimeApi: StorageEnableRuntimeApi<RuntimeApi: StorageEnableApiCollection> + Send + 'static,
>(
    task_spawner: &TaskSpawner,
    client: Arc<ParachainClient<RuntimeApi>>,
    db_pool: DbPool,
) -> ActorHandle<FishermanService<RuntimeApi>>
where
    RuntimeApi::RuntimeApi: Send,
{
    // Create a named task spawner for the fisherman service
    let task_spawner = task_spawner
        .with_name("fisherman-service")
        .with_group("monitoring");

    // Create the fisherman service instance
    let fisherman_service = FishermanService::new(client, db_pool);

    // Spawn the actor and return the handle
    task_spawner.spawn_actor(fisherman_service)
}

#[cfg(test)]
mod tests {
    // Note: These are placeholder tests. Full implementation would require
    // mock clients and database setup.

    #[tokio::test]
    async fn test_fisherman_service_creation() {
        // This test would verify that the fisherman service can be created
        // with mock dependencies
        // TODO: Implement with proper mocks
    }

    #[tokio::test]
    async fn test_command_handling() {
        // This test would verify that commands are handled correctly
        // TODO: Implement with proper test harness
    }
}
