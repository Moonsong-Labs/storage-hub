#![allow(dead_code)]

use std::time::Duration;

use log::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface, events::MultipleNewChallengeSeeds,
    types::SendExtrinsicOptions,
};
use shc_common::traits::StorageEnableRuntime;

use crate::{handler::StorageHubHandler, types::ShNodeType};

const LOG_TARGET: &str = "sp-react-to-event-mock-task";

pub type EventToReactTo<Runtime> = MultipleNewChallengeSeeds<Runtime>;

/// [`SpReactToEventMockTask`] is a mocked task used specifically for testing events emitted by the
/// BlockchainService, which this tasks reacts to by sending a remark with event transaction.
///
/// This can be used for debugging purposes.
/// The event to react to can be configured by setting the [`EventToReactTo`] type.
pub struct SpReactToEventMockTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for SpReactToEventMockTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> SpReactToEventMockTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, Runtime> SpReactToEventMockTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<NT, Runtime> EventHandler<EventToReactTo<Runtime>> for SpReactToEventMockTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: EventToReactTo<Runtime>) -> anyhow::Result<String> {
        info!(
            target: LOG_TARGET,
            "Initiating task for event: {:?}",
            event
        );

        // Build extrinsic.
        let call: Runtime::Call = frame_system::Call::<Runtime>::remark_with_event {
            remark: "Remark as a mock for testing events emitted by the BlockchainService."
                .as_bytes()
                .to_vec(),
        }
        .into();

        self.storage_hub_handler
            .blockchain
            .send_extrinsic(
                call,
                SendExtrinsicOptions::new(
                    Duration::from_secs(
                        self.storage_hub_handler
                            .provider_config
                            .blockchain_service
                            .extrinsic_retry_timeout,
                    ),
                    Some("system".to_string()),
                    Some("remarkWithEvent".to_string()),
                ),
            )
            .await?
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await?;

        Ok(format!("Handled EventToReactTo mock event: {:?}", event))
    }
}
