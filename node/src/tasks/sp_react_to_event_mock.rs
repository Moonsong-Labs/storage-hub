use std::time::Duration;

use log::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::ForestStorage;
use sp_trie::TrieLayout;

use crate::services::{
    blockchain::{commands::BlockchainServiceInterface, events::NewChallengeSeed},
    handler::StorageHubHandler,
};

const LOG_TARGET: &str = "sp-react-to-event-mock-task";

pub type EventToReactTo = NewChallengeSeed;

/// SpReactToEventMockTask is a mocked task used specifically for testing events emitted by the
/// BlockchainService, which this tasks reacts to by sending a remark with event transaction.
///
/// This can be used for debugging purposes.
/// The event to react to can be configured by setting the [`EventToReactTo`] type.
pub struct SpReactToEventMockTask<T, FL, FS>
where
    T: Send + Sync + TrieLayout + 'static,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T> + 'static,
{
    storage_hub_handler: StorageHubHandler<T, FL, FS>,
}

impl<T, FL, FS> Clone for SpReactToEventMockTask<T, FL, FS>
where
    T: Send + Sync + TrieLayout + 'static,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T> + 'static,
{
    fn clone(&self) -> SpReactToEventMockTask<T, FL, FS> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<T, FL, FS> SpReactToEventMockTask<T, FL, FS>
where
    T: Send + Sync + TrieLayout + 'static,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T> + 'static,
{
    pub fn new(storage_hub_handler: StorageHubHandler<T, FL, FS>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<T, FL, FS> EventHandler<EventToReactTo> for SpReactToEventMockTask<T, FL, FS>
where
    T: Send + Sync + TrieLayout + 'static,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T> + 'static,
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
