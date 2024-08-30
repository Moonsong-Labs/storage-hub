use std::sync::Arc;
use tokio::sync::RwLock;

use shc_actors_framework::{
    actor::{ActorHandle, TaskSpawner},
    event_bus::{EventBusListener, EventHandler},
};
use shc_blockchain_service::events::{ProcessSubmitProofRequest, SlashableProvider};
use shc_blockchain_service::{
    events::{NewChallengeSeed, NewStorageRequest, ProcessConfirmStoringRequest},
    BlockchainService,
};
use shc_file_manager::traits::FileStorage;
use shc_file_transfer_service::{
    events::{RemoteDownloadRequest, RemoteUploadRequest},
    FileTransferService,
};
use shc_forest_manager::traits::ForestStorageHandler;
use storage_hub_runtime::StorageProofsMerkleTrieLayout;

use crate::tasks::sp_slash_provider::SlashProviderTask;
use crate::tasks::{
    bsp_download_file::BspDownloadFileTask, bsp_submit_proof::BspSubmitProofTask,
    bsp_upload_file::BspUploadFileTask, user_sends_file::UserSendsFileTask,
};

use super::forest_storage::NoKey;

/// Represents the handler for the Storage Hub service.
pub struct StorageHubHandler<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Clone + Send + Sync,
{
    /// The task spawner for spawning asynchronous tasks.
    pub task_spawner: TaskSpawner,
    /// The actor handle for the file transfer service.
    pub file_transfer: ActorHandle<FileTransferService>,
    /// The actor handle for the blockchain service.
    pub blockchain: ActorHandle<BlockchainService>,
    /// The file storage layer which stores all files in chunks.
    pub file_storage: Arc<RwLock<FL>>,
    /// The forest storage layer which tracks all complete files stored in the file storage layer.
    pub forest_storage_handler: FSH,
}

impl<FL, FSH> Clone for StorageHubHandler<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Clone + Send + Sync,
{
    fn clone(&self) -> StorageHubHandler<FL, FSH> {
        Self {
            task_spawner: self.task_spawner.clone(),
            file_transfer: self.file_transfer.clone(),
            blockchain: self.blockchain.clone(),
            file_storage: self.file_storage.clone(),
            forest_storage_handler: self.forest_storage_handler.clone(),
        }
    }
}

impl<FL, FSH> StorageHubHandler<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    pub fn new(
        task_spawner: TaskSpawner,
        file_transfer: ActorHandle<FileTransferService>,
        blockchain: ActorHandle<BlockchainService>,
        file_storage: Arc<RwLock<FL>>,
        forest_storage_handler: FSH,
    ) -> Self {
        Self {
            task_spawner,
            file_transfer,
            blockchain,
            file_storage,
            forest_storage_handler,
        }
    }

    pub fn start_user_tasks(&self) {
        log::info!("Starting User tasks.");

        UserSendsFileTask::new(self.clone())
            .subscribe_to(&self.task_spawner, &self.blockchain)
            .start();
    }
}

impl<FL, FSH> StorageHubHandler<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler<Key = Vec<u8>> + Clone + Send + Sync + 'static,
{
    pub fn start_msp_tasks(&self) {
        log::info!("Starting MSP tasks");

        // TODO: Implement MSP tasks
    }
}

impl<FL, FSH> StorageHubHandler<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler<Key = NoKey> + Clone + Send + Sync + 'static,
{
    pub fn start_bsp_tasks(&self) {
        log::info!("Starting BSP tasks");

        // BspUploadFileTask is triggered by a NewStorageRequest event, to which it responds by
        // volunteering to store the file. Then it waits for RemoteUploadRequest events, which
        // happens when the user, now aware of the BSP volunteering, submits chunks of the file,
        // along with a proof of storage.
        let bsp_upload_file_task = BspUploadFileTask::new(self.clone());
        // Subscribing to NewStorageRequest event from the BlockchainService.
        let new_storage_request_event_bus_listener: EventBusListener<NewStorageRequest, _> =
            bsp_upload_file_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain);
        new_storage_request_event_bus_listener.start();
        // Subscribing to RemoteUploadRequest event from the FileTransferService.
        let remote_upload_request_event_bus_listener: EventBusListener<RemoteUploadRequest, _> =
            bsp_upload_file_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.file_transfer);
        remote_upload_request_event_bus_listener.start();
        // Subscribing to ProcessConfirmStoringRequest event from the BlockchainService.
        let process_confirm_storing_request_event_bus_listener: EventBusListener<
            ProcessConfirmStoringRequest,
            _,
        > = bsp_upload_file_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain);
        process_confirm_storing_request_event_bus_listener.start();

        // The BspDownloadFileTask
        let bsp_download_file_task = BspDownloadFileTask::new(self.clone());
        // Subscribing to RemoteDownloadRequest event from the FileTransferService.
        let remote_download_request_event_bus_listener: EventBusListener<RemoteDownloadRequest, _> =
            bsp_download_file_task.subscribe_to(&self.task_spawner, &self.file_transfer);
        remote_download_request_event_bus_listener.start();

        // BspSubmitProofTask is triggered by a NewChallengeSeed event emitted by the BlockchainService.
        // It responds by computing challenges derived from the seed, taking also into account
        // the custom challenges in checkpoint challenge rounds and enqueuing them in BlockchainService.
        // BspSubmitProofTask also listens to ProcessSubmitProofRequest events, which are emitted by the
        // BlockchainService when it is time to actually submit the proof of storage.
        // Additionally, it handles file deletions as a consequence of inclusion proofs in custom challenges.
        let bsp_submit_proof_task = BspSubmitProofTask::new(self.clone());
        // Subscribing to NewChallengeSeed event from the BlockchainService.
        let new_challenge_seed_event_bus_listener: EventBusListener<NewChallengeSeed, _> =
            bsp_submit_proof_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain);
        new_challenge_seed_event_bus_listener.start();
        // Subscribing to ProcessSubmitProofRequest event from the BlockchainService.
        let process_submit_proof_request_event_bus_listener: EventBusListener<
            ProcessSubmitProofRequest,
            _,
        > = bsp_submit_proof_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain);
        process_submit_proof_request_event_bus_listener.start();

        // Slash your own kin or potentially commit seppuku on your own stake.
        // Running this is as a BSP is very honourable and shows a great sense of justice.
        let bsp_slash_provider_task = SlashProviderTask::new(self.clone());
        // Subscribing to SlashableProvider event from the BlockchainService.
        let slashable_provider_event_bus_listener: EventBusListener<SlashableProvider, _> =
            bsp_slash_provider_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain);
        slashable_provider_event_bus_listener.start();
    }
}
