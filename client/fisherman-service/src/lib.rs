//! # Fisherman Service
//!
//! The Fisherman Service is responsible for handling file deletion requests in the StorageHub network.
//! It monitors the blockchain for file deletion events and constructs the necessary proofs for
//! storage providers to remove files from their storage.
//!
//! ## Key Features
//!
//! - Monitors blockchain events for file deletion requests
//! - Constructs proofs of inclusion from MSP or BSP forests
//! - Submits constructed proofs to the blockchain

pub mod events;
pub mod handler;

use std::sync::Arc;

use shc_actors_framework::actor::{ActorHandle, ActorSpawner, TaskSpawner};
use shc_common::traits::{StorageEnableApiCollection, StorageEnableRuntimeApi};
use shc_common::types::ParachainClient;

pub use self::handler::{FishermanService, FishermanServiceCommand, FishermanServiceError};
pub use events::{
    FileDeletionTarget, FishermanServiceEventBusProvider, ProcessFileDeletionRequest,
};

/// Spawn the fisherman service as an actor
///
/// This function creates and spawns a new FishermanService actor that will monitor
/// the StorageHub network for file deletion requests and construct proofs of inclusion to delete file keys from Bucket and BSP forests.
pub async fn spawn_fisherman_service<
    RuntimeApi: StorageEnableRuntimeApi<RuntimeApi: StorageEnableApiCollection> + Send + 'static,
>(
    task_spawner: &TaskSpawner,
    client: Arc<ParachainClient<RuntimeApi>>,
) -> ActorHandle<FishermanService<RuntimeApi>>
where
    RuntimeApi::RuntimeApi: Send,
{
    // Create a named task spawner for the fisherman service
    let task_spawner = task_spawner
        .with_name("fisherman-service")
        .with_group("monitoring");

    // Create the fisherman service instance
    let fisherman_service = FishermanService::new(client);

    // Spawn the actor and return the handle
    task_spawner.spawn_actor(fisherman_service)
}
