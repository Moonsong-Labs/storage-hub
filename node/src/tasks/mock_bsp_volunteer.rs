#![allow(dead_code)]

use std::time::Duration;

use log::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceInterface, events::NewStorageRequest, types::SendExtrinsicOptions,
};
use sp_core::H256;

use crate::services::{
    handler::StorageHubHandler,
    types::{BspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "bsp-volunteer-mock-task";

pub struct BspVolunteerMockTask<NT>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<NT>,
}

impl<NT> Clone for BspVolunteerMockTask<NT>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
{
    fn clone(&self) -> BspVolunteerMockTask<NT> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT> BspVolunteerMockTask<NT>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<NT> EventHandler<NewStorageRequest> for BspVolunteerMockTask<NT>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: NewStorageRequest) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Initiating BSP volunteer mock for file key: {:x}",
            event.file_key
        );

        // Build extrinsic.
        let call =
            storage_hub_runtime::RuntimeCall::FileSystem(pallet_file_system::Call::bsp_volunteer {
                file_key: H256(event.file_key.into()),
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
