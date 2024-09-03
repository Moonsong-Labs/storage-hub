use std::sync::Arc;
use tokio::sync::RwLock;

use shc_actors_framework::{
    actor::{ActorHandle, TaskSpawner},
    event_bus::{EventBusListener, EventHandler},
};
use shc_blockchain_service::{
    events::{
        LastChargeableInfoUpdated, NewChallengeSeed, NewStorageRequest,
        ProcessConfirmStoringRequest, ProcessSubmitProofRequest, SlashableProvider,
    },
    BlockchainService,
};
use shc_file_manager::traits::FileStorage;
use shc_file_transfer_service::{
    events::{RemoteDownloadRequest, RemoteUploadRequest},
    FileTransferService,
};
use shc_forest_manager::traits::ForestStorage;
use storage_hub_runtime::StorageProofsMerkleTrieLayout;

use crate::tasks::{
    bsp_charge_fees::BspChargeFeesTask, bsp_download_file::BspDownloadFileTask,
    bsp_submit_proof::BspSubmitProofTask, bsp_upload_file::BspUploadFileTask,
    sp_slash_provider::SlashProviderTask, user_sends_file::UserSendsFileTask,
};

/// Represents the handler for the Storage Hub service.
pub struct StorageHubHandler<FL, FS>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
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
    pub forest_storage: Arc<RwLock<FS>>,
}

impl<FL, FS> Clone for StorageHubHandler<FL, FS>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
{
    fn clone(&self) -> StorageHubHandler<FL, FS> {
        Self {
            task_spawner: self.task_spawner.clone(),
            file_transfer: self.file_transfer.clone(),
            blockchain: self.blockchain.clone(),
            file_storage: self.file_storage.clone(),
            forest_storage: self.forest_storage.clone(),
        }
    }
}

impl<FL, FS> StorageHubHandler<FL, FS>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync + 'static,
{
    pub fn new(
        task_spawner: TaskSpawner,
        file_transfer: ActorHandle<FileTransferService>,
        blockchain: ActorHandle<BlockchainService>,
        file_storage: Arc<RwLock<FL>>,
        forest_storage: Arc<RwLock<FS>>,
    ) -> Self {
        Self {
            task_spawner,
            file_transfer,
            blockchain,
            file_storage,
            forest_storage,
        }
    }

    pub fn start_user_tasks(&self) {
        log::info!("Starting User tasks.");

        UserSendsFileTask::new(self.clone())
            .subscribe_to(&self.task_spawner, &self.blockchain)
            .start();
    }

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

        // Collect debt from users after a BSP proof is accepted.
        let bsp_charge_fees_task = BspChargeFeesTask::new(self.clone());
        let last_chargeable_info_updated_event_bus_listener: EventBusListener<
            LastChargeableInfoUpdated,
            _,
        > = bsp_charge_fees_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain);
        last_chargeable_info_updated_event_bus_listener.start();
    }

    pub fn start_msp_tasks(&self) {
        log::info!("Starting MSP tasks");

        // TODO: Implement MSP tasks
    }
}
