use std::time::Duration;

use log::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{commands::BlockchainServiceInterface, events::NewChallengeSeed};
use shc_common::types::StorageProofsMerkleTrieLayout;
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::ForestStorageHandler;

use crate::services::handler::StorageHubHandler;

const LOG_TARGET: &str = "sp-react-to-event-mock-task";

pub type EventToReactTo = NewChallengeSeed;

/// SpReactToEventMockTask is a mocked task used specifically for testing events emitted by the
/// BlockchainService, which this tasks reacts to by sending a remark with event transaction.
///
/// This can be used for debugging purposes.
/// The event to react to can be configured by setting the [`EventToReactTo`] type.
pub struct SpReactToEventMockTask<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    storage_hub_handler: StorageHubHandler<FL, FSH>,
}

impl<FL, FSH> Clone for SpReactToEventMockTask<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    fn clone(&self) -> SpReactToEventMockTask<FL, FSH> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<FL, FSH> SpReactToEventMockTask<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    pub fn new(storage_hub_handler: StorageHubHandler<FL, FSH>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<FL, FSH> EventHandler<EventToReactTo> for SpReactToEventMockTask<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    async fn handle_event(&mut self, event: EventToReactTo) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Initiating task for event: {:?}",
            event
        );

        // Build extrinsic.
        let call =
            storage_hub_runtime::RuntimeCall::System(frame_system::Call::remark_with_event {
                remark: "Remark as a mock for testing events emitted by the BlockchainService."
                    .as_bytes()
                    .to_vec(),
            });

        self.storage_hub_handler
            .blockchain
            .send_extrinsic(call)
            .await?
            .with_timeout(Duration::from_secs(60))
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await?;

        Ok(())
    }
}
