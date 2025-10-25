//! # Fisherman Service
//!
//! The Fisherman Service is responsible for handling file deletion requests in the StorageHub network.
//! It monitors the blockchain for file deletion request events and constructs the necessary proofs for
//! storage providers to remove files from their storage.
//!
//! ## Key Features
//!
//! - Monitors blockchain events for file deletion requests
//! - Constructs proofs of inclusion from MSP or BSP forests
//! - Submits constructed proofs to the blockchain

pub mod commands;
pub mod events;
pub mod handler;

use std::sync::Arc;

use shc_actors_framework::actor::{ActorHandle, ActorSpawner, TaskSpawner};
use shc_common::traits::StorageEnableRuntime;
use shc_common::types::ParachainClient;

pub use self::commands::{FishermanServiceCommand, FishermanServiceError};
pub use self::handler::{FileKeyChange, FileKeyOperation, FishermanService};
pub use events::{
    FileDeletionTarget, FishermanServiceEventBusProvider, ProcessFileDeletionRequest,
    ProcessIncompleteStorageRequest,
};

/// Spawn the fisherman service as an actor
///
/// This function creates and spawns a new FishermanService actor that will monitor
/// the StorageHub network for file deletion requests and construct proofs of inclusion to delete file keys from Bucket and BSP forests.
pub async fn spawn_fisherman_service<Runtime: StorageEnableRuntime>(
    task_spawner: &TaskSpawner,
    client: Arc<ParachainClient<Runtime::RuntimeApi>>,
    incomplete_sync_max: u32,
    incomplete_sync_page_size: u32,
    sync_mode_min_blocks_behind: u32,
) -> ActorHandle<FishermanService<Runtime>> {
    // Create a named task spawner for the fisherman service
    let task_spawner = task_spawner
        .with_name("fisherman-service")
        .with_group("monitoring");

    // Create the fisherman service instance
    let fisherman_service = FishermanService::new(
        client,
        incomplete_sync_max,
        incomplete_sync_page_size,
        sync_mode_min_blocks_behind,
    );

    // Spawn the actor and return the handle
    task_spawner.spawn_actor(fisherman_service)
}
