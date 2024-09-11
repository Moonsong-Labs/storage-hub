use std::time::Duration;

use log::*;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::types::Tip;
use shc_blockchain_service::{commands::BlockchainServiceInterface, events::NewStorageRequest};
use sp_core::H256;

use crate::services::handler::StorageHubHandler;
use crate::tasks::{BspForestStorageHandlerT, FileStorageT};

const LOG_TARGET: &str = "bsp-volunteer-mock-task";

pub struct BspVolunteerMockTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    storage_hub_handler: StorageHubHandler<FL, FSH>,
}

impl<FL, FSH> Clone for BspVolunteerMockTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    fn clone(&self) -> BspVolunteerMockTask<FL, FSH> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<FL, FSH> BspVolunteerMockTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    pub fn new(storage_hub_handler: StorageHubHandler<FL, FSH>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

impl<FL, FSH> EventHandler<NewStorageRequest> for BspVolunteerMockTask<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
{
    async fn handle_event(&mut self, event: NewStorageRequest) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Initiating BSP volunteer mock for file key: {:?}",
            event.file_key
        );

        // Build extrinsic.
        let call =
            storage_hub_runtime::RuntimeCall::FileSystem(pallet_file_system::Call::bsp_volunteer {
                file_key: H256(event.file_key.into()),
            });

        self.storage_hub_handler
            .blockchain
            .send_extrinsic(call, Tip::from(0))
            .await?
            .with_timeout(Duration::from_secs(60))
            .watch_for_success(&self.storage_hub_handler.blockchain)
            .await?;

        Ok(())
    }
}
