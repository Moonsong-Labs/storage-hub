use log::info;
use sp_runtime::BoundedVec;
use storage_hub_infra::event_bus::EventHandler;

use crate::services::{
    blockchain::{events::NewStorageRequest, handler::BlockchainServiceCommand},
    StorageHubHandler,
};

const LOG_TARGET: &str = "bsp-volunteer-mock-task";

#[derive(Clone)]
pub struct BspVolunteerMockTask {
    storage_hub_handler: StorageHubHandler,
}

impl BspVolunteerMockTask {
    pub fn new(storage_hub_handler: StorageHubHandler) -> Self {
        Self {
            storage_hub_handler: storage_hub_handler,
        }
    }
}

impl EventHandler<NewStorageRequest> for BspVolunteerMockTask {
    async fn handle_event(&self, event: NewStorageRequest) -> anyhow::Result<()> {
        info!(target: LOG_TARGET, "Received event: {:?}", event);

        // TODO: Here we would send the actual multiaddresses of this BSP.
        let multiaddresses = BoundedVec::default();

        // Build extrinsic.
        let call =
            storage_hub_runtime::RuntimeCall::FileSystem(pallet_file_system::Call::bsp_volunteer {
                location: event.location,
                fingerprint: event.fingerprint,
                multiaddresses,
            });

        // Build command to send to blockchain service.
        let message = BlockchainServiceCommand::SendExtrinsic { call, caller };

        let blockchain_service = self.storage_hub_handler.blockchain.clone();
        blockchain_service.send(message).await;

        Ok(())
    }
}
