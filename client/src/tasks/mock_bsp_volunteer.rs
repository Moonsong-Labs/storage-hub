#![allow(dead_code)]
use log::*;
use std::time::Duration;

use sp_core::H256;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface, events::NewStorageRequest,
    types::SendExtrinsicOptions,
};
use shc_common::traits::StorageEnableRuntime;

use crate::{
    handler::StorageHubHandler,
    types::{BspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "bsp-volunteer-mock-task";

pub struct BspVolunteerMockTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
}

impl<NT, Runtime> Clone for BspVolunteerMockTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> BspVolunteerMockTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, Runtime> BspVolunteerMockTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<NT, Runtime> EventHandler<NewStorageRequest<Runtime>> for BspVolunteerMockTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: NewStorageRequest<Runtime>) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Initiating BSP volunteer mock for file key: {:x}",
            event.file_key
        );

        // Build extrinsic.
        let call: Runtime::Call = pallet_file_system::Call::<Runtime>::bsp_volunteer {
            file_key: H256(event.file_key.into()),
        }
        .into();

        self.storage_hub_handler
            .blockchain
            .send_extrinsic(
                call.clone(),
                SendExtrinsicOptions::new(Duration::from_secs(
                    self.storage_hub_handler
                        .provider_config
                        .blockchain_service
                        .extrinsic_retry_timeout,
                )),
            )
            .await?
            .watch_for_success()
            .await?;

        Ok(())
    }
}
