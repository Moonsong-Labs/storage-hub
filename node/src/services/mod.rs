pub mod file_transfer;

use storage_hub_infra::{
    actor::{ActorHandle, TaskSpawner},
    event_bus::EventHandler,
};

use crate::tasks::ResolveRemoteUploadRequest;

use self::file_transfer::FileTransferService;

#[derive(Debug, Clone)]
pub struct StorageHubHandler {
    pub task_spawner: TaskSpawner,
    pub file_transfer: ActorHandle<FileTransferService>,
}

impl StorageHubHandler {
    pub fn new(task_spawner: TaskSpawner, file_transfer: ActorHandle<FileTransferService>) -> Self {
        Self {
            task_spawner,
            file_transfer,
        }
    }

    pub fn start_bsp_tasks(&self) {
        ResolveRemoteUploadRequest::new(self.clone())
            .subscribe_to(&self.task_spawner, &self.file_transfer);
    }
}
