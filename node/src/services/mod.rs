pub mod blockchain;
pub mod file_transfer;

use storage_hub_infra::{
    actor::{ActorHandle, TaskSpawner},
    event_bus::EventHandler,
};

use crate::tasks::bsp_volunteer_mock::BspVolunteerMockTask;

use self::{blockchain::handler::BlockchainService, file_transfer::FileTransferService};

#[derive(Clone)]
pub struct StorageHubHandler {
    pub task_spawner: TaskSpawner,
    pub file_transfer: ActorHandle<FileTransferService>,
    pub blockchain: ActorHandle<BlockchainService>,
}

impl StorageHubHandler {
    pub fn new(
        task_spawner: TaskSpawner,
        file_transfer: ActorHandle<FileTransferService>,
        blockchain: ActorHandle<BlockchainService>,
    ) -> Self {
        Self {
            task_spawner,
            file_transfer,
            blockchain,
        }
    }

    pub fn start_bsp_tasks(&self) {
        log::info!("Starting BSP tasks");
        BspVolunteerMockTask::new(self.clone())
            .subscribe_to(&self.task_spawner, &self.blockchain)
            .start();
    }
}
