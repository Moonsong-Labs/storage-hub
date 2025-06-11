#![allow(dead_code)]

use std::time::Duration;

use log::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface, events::MultipleNewChallengeSeeds,
    types::SendExtrinsicOptions,
};
use shc_common::traits::{StorageEnableApiCollection, StorageEnableRuntimeApi};

use crate::{handler::StorageHubHandler, types::ShNodeType};

const LOG_TARGET: &str = "sp-react-to-event-mock-task";

pub type EventToReactTo = MultipleNewChallengeSeeds;

/// [`SpReactToEventMockTask`] is a mocked task used specifically for testing events emitted by the
/// BlockchainService, which this tasks reacts to by sending a remark with event transaction.
///
/// This can be used for debugging purposes.
/// The event to react to can be configured by setting the [`EventToReactTo`] type.
pub struct SpReactToEventMockTask<NT, RuntimeApi>
where
    NT: ShNodeType,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    storage_hub_handler: StorageHubHandler<NT, RuntimeApi>,
}

impl<NT, RuntimeApi> Clone for SpReactToEventMockTask<NT, RuntimeApi>
where
    NT: ShNodeType,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    fn clone(&self) -> SpReactToEventMockTask<NT, RuntimeApi> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, RuntimeApi> SpReactToEventMockTask<NT, RuntimeApi>
where
    NT: ShNodeType,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, RuntimeApi>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<NT, RuntimeApi> EventHandler<EventToReactTo> for SpReactToEventMockTask<NT, RuntimeApi>
where
    NT: ShNodeType + 'static,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
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
            .send_extrinsic(
                call,
                SendExtrinsicOptions::new(Duration::from_secs(
                    self.storage_hub_handler
                        .provider_config
                        .blockchain_service
                        .extrinsic_retry_timeout,
                )),
            )
            .await?
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await?;

        Ok(())
    }
}
