#![allow(dead_code)]

use std::time::Duration;

use log::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface, events::MultipleNewChallengeSeeds,
    types::SendExtrinsicOptions,
};
use shc_common::traits::{
    StorageEnableApiCollection, StorageEnableRuntime, StorageEnableRuntimeApi,
};

use crate::{handler::StorageHubHandler, types::ShNodeType};

const LOG_TARGET: &str = "sp-react-to-event-mock-task";

pub type EventToReactTo = MultipleNewChallengeSeeds;

/// [`SpReactToEventMockTask`] is a mocked task used specifically for testing events emitted by the
/// BlockchainService, which this tasks reacts to by sending a remark with event transaction.
///
/// This can be used for debugging purposes.
/// The event to react to can be configured by setting the [`EventToReactTo`] type.
pub struct SpReactToEventMockTask<NT, RuntimeApi, Runtime>
where
    NT: ShNodeType,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, RuntimeApi, Runtime>,
}

impl<NT, RuntimeApi, Runtime> Clone for SpReactToEventMockTask<NT, RuntimeApi, Runtime>
where
    NT: ShNodeType,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> SpReactToEventMockTask<NT, RuntimeApi, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, RuntimeApi, Runtime> SpReactToEventMockTask<NT, RuntimeApi, Runtime>
where
    NT: ShNodeType,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, RuntimeApi, Runtime>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<NT, RuntimeApi, Runtime> EventHandler<EventToReactTo>
    for SpReactToEventMockTask<NT, RuntimeApi, Runtime>
where
    NT: ShNodeType + 'static,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
    Runtime: StorageEnableRuntime,
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
                call.into(),
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
