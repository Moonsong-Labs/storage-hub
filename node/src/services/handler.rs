use std::sync::Arc;

use shc_actors_framework::{
    actor::{ActorHandle, TaskSpawner},
    event_bus::{EventBusListener, EventHandler},
};
use shc_blockchain_service::{
    events::{
        AcceptedBspVolunteer, BspConfirmStoppedStoring, FinalisedBspConfirmStoppedStoring,
        FinalisedMspStoppedStoringBucket, LastChargeableInfoUpdated, MultipleNewChallengeSeeds,
        NewStorageRequest, ProcessConfirmStoringRequest, ProcessMspRespondStoringRequest,
        ProcessStopStoringForInsolventUserRequest, ProcessSubmitProofRequest, SlashableProvider,
        SpStopStoringInsolventUser, UserWithoutFunds, NotifyPeriod,
    },
    BlockchainService,
};
use shc_file_transfer_service::{
    events::{RemoteDownloadRequest, RemoteUploadRequest},
    FileTransferService,
};
use shc_forest_manager::traits::ForestStorageHandler;
use storage_hub_runtime::StorageDataUnit;
use tokio::sync::RwLock;

use crate::tasks::{
    bsp_charge_fees::BspChargeFeesTask, bsp_download_file::BspDownloadFileTask,
    bsp_submit_proof::BspSubmitProofTask, bsp_upload_file::BspUploadFileTask,
    msp_charge_fees::MspChargeFeesTask, msp_delete_bucket::MspStoppedStoringTask, bsp_delete_file::BspDeleteFileTask,
    msp_upload_file::MspUploadFileTask, sp_slash_provider::SlashProviderTask,
    user_sends_file::UserSendsFileTask, BspForestStorageHandlerT, FileStorageT,
    MspForestStorageHandlerT,
};

/// Configuration paramaters for Storage Providers.
#[derive(Clone)]
pub struct ProviderConfig {
    /// Maximum storage capacity of the provider (bytes).
    ///
    /// The Storage Provider will not request to increase its storage capacity beyond this value.
    pub max_storage_capacity: StorageDataUnit,
    /// Jump capacity (bytes).
    ///
    /// Storage capacity increases in jumps of this size.
    pub jump_capacity: StorageDataUnit,
    /// The time in seconds to wait before retrying an extrinsic.
    pub extrinsic_retry_timeout: u64,
}

/// Represents the handler for the Storage Hub service.
pub struct StorageHubHandler<FL, FSH>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
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
    /// The configuration parameters for the provider.
    pub provider_config: ProviderConfig,
}

impl<FL, FSH> Clone for StorageHubHandler<FL, FSH>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    fn clone(&self) -> StorageHubHandler<FL, FSH> {
        Self {
            task_spawner: self.task_spawner.clone(),
            file_transfer: self.file_transfer.clone(),
            blockchain: self.blockchain.clone(),
            file_storage: self.file_storage.clone(),
            forest_storage_handler: self.forest_storage_handler.clone(),
            provider_config: self.provider_config.clone(),
        }
    }
}

impl<FL, FSH> StorageHubHandler<FL, FSH>
where
    FL: FileStorageT,
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    pub fn new(
        task_spawner: TaskSpawner,
        file_transfer: ActorHandle<FileTransferService>,
        blockchain: ActorHandle<BlockchainService>,
        file_storage: Arc<RwLock<FL>>,
        forest_storage_handler: FSH,
        provider_config: ProviderConfig,
    ) -> Self {
        Self {
            task_spawner,
            file_transfer,
            blockchain,
            file_storage,
            forest_storage_handler,
            provider_config,
        }
    }

    pub fn start_user_tasks(&self) {
        log::info!("Starting User tasks.");

        let user_sends_file_task = UserSendsFileTask::new(self.clone());

        // Subscribing to NewStorageRequest event from the BlockchainService.
        let new_storage_request_event_bus_listener: EventBusListener<NewStorageRequest, _> =
            user_sends_file_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain);
        new_storage_request_event_bus_listener.start();

        let accepted_bsp_volunteer_event_bus_listener: EventBusListener<AcceptedBspVolunteer, _> =
            user_sends_file_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain);
        accepted_bsp_volunteer_event_bus_listener.start();
    }
}

impl<FL, FSH> StorageHubHandler<FL, FSH>
where
    FL: FileStorageT,
    FSH: MspForestStorageHandlerT,
{
    pub fn start_msp_tasks(&self) {
        log::info!("Starting MSP tasks");

        // MspUploadFileTask is triggered by a NewStorageRequest event which registers the user's peer address for
        // an upcoming RemoteUploadRequest events, which happens when the user connects to the MSP and submits chunks of the file,
        // along with a proof of storage, which is then queued to batch accept many storage requests at once.
        // Finally once the ProcessMspRespondStoringRequest event is emitted, the MSP will respond to the user with a confirmation.
        let msp_upload_file_task = MspUploadFileTask::new(self.clone());
        // Subscribing to NewStorageRequest event from the BlockchainService.
        let new_storage_request_event_bus_listener: EventBusListener<NewStorageRequest, _> =
            msp_upload_file_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain);
        new_storage_request_event_bus_listener.start();
        // Subscribing to RemoteUploadRequest event from the FileTransferService.
        let remote_upload_request_event_bus_listener: EventBusListener<RemoteUploadRequest, _> =
            msp_upload_file_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.file_transfer);
        remote_upload_request_event_bus_listener.start();
        // Subscribing to ProcessMspRespondStoringRequest event from the BlockchainService.
        let process_confirm_storing_request_event_bus_listener: EventBusListener<
            ProcessMspRespondStoringRequest,
            _,
        > = msp_upload_file_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain);
        process_confirm_storing_request_event_bus_listener.start();

        // MspStoppedStoringTask handles events for handling data deletion.
        let msp_stopped_storing_task = MspStoppedStoringTask::new(self.clone());
        // Subscribing to FinalisedMspStoppedStoringBucket event from the BlockchainService.
        let finalised_msp_stopped_storing_bucket_event_bus_listener: EventBusListener<
            FinalisedMspStoppedStoringBucket,
            _,
        > = msp_stopped_storing_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain);
        finalised_msp_stopped_storing_bucket_event_bus_listener.start();

        let msp_charge_fees_task = MspChargeFeesTask::new(self.clone());
        // Subscribing to NewStorageRequest event from the BlockchainService.
        let notify_period_event_bus_listener: EventBusListener<NotifyPeriod, _> =
            msp_charge_fees_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain);
        notify_period_event_bus_listener.start();
    }
}

impl<FL, FSH> StorageHubHandler<FL, FSH>
where
    FL: FileStorageT,
    FSH: BspForestStorageHandlerT,
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

        // BspSubmitProofTask is triggered by a MultipleNewChallengeSeeds event emitted by the BlockchainService.
        // It responds by computing challenges derived from the seeds, taking also into account
        // the custom challenges in checkpoint challenge rounds and enqueuing them in BlockchainService.
        // BspSubmitProofTask also listens to ProcessSubmitProofRequest events, which are emitted by the
        // BlockchainService when it is time to actually submit the proof of storage.
        // Additionally, it handles file deletions as a consequence of inclusion proofs in custom challenges.
        let bsp_submit_proof_task = BspSubmitProofTask::new(self.clone());
        // Subscribing to MultipleNewChallengeSeeds event from the BlockchainService.
        let multiple_new_challenge_seeds_event_bus_listener: EventBusListener<
            MultipleNewChallengeSeeds,
            _,
        > = bsp_submit_proof_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain);
        multiple_new_challenge_seeds_event_bus_listener.start();
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

        // Subscribing to ProcessStopStoringForInsolventUserRequest event from the BlockchainService.
        let process_stop_storing_for_insolvent_user_request_event_bus_listener: EventBusListener<
            ProcessStopStoringForInsolventUserRequest,
            _,
        > = bsp_charge_fees_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain);
        process_stop_storing_for_insolvent_user_request_event_bus_listener.start();

        // Start deletion process for stored files owned by a user that has been declared as without funds and charge
        // its payment stream afterwards, getting the owed tokens and deleting it.
        let user_without_funds_event_bus_listener: EventBusListener<UserWithoutFunds, _> =
            bsp_charge_fees_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain);
        user_without_funds_event_bus_listener.start();

        // Continue deletion process for stored files owned by a user that has been declared as without funds.
        // Once the last file has been deleted, get the owed tokens and delete the payment stream.
        let sp_stop_storing_insolvent_user_event_bus_listener: EventBusListener<
            SpStopStoringInsolventUser,
            _,
        > = bsp_charge_fees_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain);
        sp_stop_storing_insolvent_user_event_bus_listener.start();

        // Delete file and update the root.
        let bsp_delete_file_task = BspDeleteFileTask::new(self.clone());
        let bsp_confirm_stopped_storing_event_bus_listener: EventBusListener<
            BspConfirmStoppedStoring,
            _,
        > = bsp_delete_file_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain);
        bsp_confirm_stopped_storing_event_bus_listener.start();
        let finalised_bsp_confirm_stopped_storing_event_bus_listener: EventBusListener<
            FinalisedBspConfirmStoppedStoring,
            _,
        > = bsp_delete_file_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain);
        finalised_bsp_confirm_stopped_storing_event_bus_listener.start();
    }
}
