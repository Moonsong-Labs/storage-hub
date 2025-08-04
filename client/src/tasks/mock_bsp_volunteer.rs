#![allow(dead_code)]
use log::*;
use std::time::Duration;

use sp_core::H256;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{
    commands::BlockchainServiceCommandInterface, events::NewStorageRequest,
    types::SendExtrinsicOptions,
};
use shc_common::traits::{
    StorageEnableApiCollection, StorageEnableRuntime, StorageEnableRuntimeApi,
};

use crate::{
    handler::StorageHubHandler,
    types::{BspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "bsp-volunteer-mock-task";

pub struct BspVolunteerMockTask<NT, RuntimeApi, Runtime>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, RuntimeApi, Runtime>,
}

impl<NT, RuntimeApi, Runtime> Clone for BspVolunteerMockTask<NT, RuntimeApi, Runtime>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> BspVolunteerMockTask<NT, RuntimeApi, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<NT, RuntimeApi, Runtime> BspVolunteerMockTask<NT, RuntimeApi, Runtime>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
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

impl<NT, RuntimeApi, Runtime> EventHandler<NewStorageRequest>
    for BspVolunteerMockTask<NT, RuntimeApi, Runtime>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
    Runtime: StorageEnableRuntime,
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
