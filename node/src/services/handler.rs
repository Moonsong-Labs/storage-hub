use std::sync::Arc;
use tokio::sync::RwLock;

use shc_actors_framework::{
    actor::{ActorHandle, TaskSpawner},
    event_bus::{EventBusListener, EventHandler},
};
use shc_blockchain_service::{
    capacity_manager::CapacityConfig,
    events::{
        AcceptedBspVolunteer, FileDeletionRequest, FinalisedBspConfirmStoppedStoring,
        FinalisedBucketMovedAway, FinalisedMspStopStoringBucketInsolventUser,
        FinalisedMspStoppedStoringBucket, FinalisedProofSubmittedForPendingFileDeletionRequest,
        LastChargeableInfoUpdated, MoveBucketAccepted, MoveBucketExpired, MoveBucketRejected,
        MoveBucketRequested, MoveBucketRequestedForMsp, MultipleNewChallengeSeeds,
        NewStorageRequest, NotifyPeriod, ProcessConfirmStoringRequest, ProcessFileDeletionRequest,
        ProcessMspRespondStoringRequest, ProcessStopStoringForInsolventUserRequest,
        ProcessSubmitProofRequest, SlashableProvider, SpStopStoringInsolventUser,
        StartMovedBucketDownload, UserWithoutFunds,
    },
    BlockchainService,
};
use shc_common::consts::CURRENT_FOREST_KEY;
use shc_file_transfer_service::{
    events::{RemoteDownloadRequest, RemoteUploadRequest, RetryBucketMoveDownload},
    FileTransferService,
};
use shc_forest_manager::traits::ForestStorageHandler;
use shc_indexer_db::DbPool;

use crate::{
    services::{
        bsp_peer_manager::BspPeerManager,
        file_download_manager::FileDownloadManager,
        types::{
            BspForestStorageHandlerT, BspProvider, MspForestStorageHandlerT, MspProvider,
            ShNodeType, ShStorageLayer, UserRole,
        },
    },
    tasks::{
        bsp_charge_fees::BspChargeFeesTask, bsp_delete_file::BspDeleteFileTask,
        bsp_download_file::BspDownloadFileTask, bsp_move_bucket::BspMoveBucketTask,
        bsp_submit_proof::BspSubmitProofTask, bsp_upload_file::BspUploadFileTask,
        msp_charge_fees::MspChargeFeesTask, msp_delete_bucket::MspDeleteBucketTask,
        msp_delete_file::MspDeleteFileTask, msp_move_bucket::MspRespondMoveBucketTask,
        msp_stop_storing_insolvent_user::MspStopStoringInsolventUserTask,
        msp_upload_file::MspUploadFileTask, retry_bucket_move::RetryBucketMoveTask,
        sp_slash_provider::SlashProviderTask, user_sends_file::UserSendsFileTask,
    },
};

/// Configuration parameters for Storage Providers.
#[derive(Clone)]
pub struct ProviderConfig {
    /// Configuration parameters necessary to run the capacity manager.
    ///
    /// This is only required if running as a storage provider node.
    pub capacity_config: CapacityConfig,
    /// The time in seconds to wait before retrying an extrinsic.
    pub extrinsic_retry_timeout: u64,
}

/// Represents the handler for the Storage Hub service.
pub struct StorageHubHandler<NT>
where
    NT: ShNodeType,
{
    /// The task spawner for spawning asynchronous tasks.
    pub task_spawner: TaskSpawner,
    /// The actor handle for the file transfer service.
    pub file_transfer: ActorHandle<FileTransferService>,
    /// The actor handle for the blockchain service.
    pub blockchain: ActorHandle<BlockchainService<NT::FSH>>,
    /// The file storage layer which stores all files in chunks.
    pub file_storage: Arc<RwLock<NT::FL>>,
    /// The forest storage layer which tracks all complete files stored in the file storage layer.
    pub forest_storage_handler: NT::FSH,
    /// The configuration parameters for the provider.
    pub provider_config: ProviderConfig,
    /// The indexer database pool.
    pub indexer_db_pool: Option<DbPool>,
    /// The BSP peer manager for tracking peer performance.
    pub peer_manager: Arc<BspPeerManager>,
    /// The file download manager for rate-limiting downloads.
    pub file_download_manager: Arc<FileDownloadManager>,
}

impl<NT> Clone for StorageHubHandler<NT>
where
    NT: ShNodeType,
{
    fn clone(&self) -> StorageHubHandler<NT> {
        Self {
            task_spawner: self.task_spawner.clone(),
            file_transfer: self.file_transfer.clone(),
            blockchain: self.blockchain.clone(),
            file_storage: self.file_storage.clone(),
            forest_storage_handler: self.forest_storage_handler.clone(),
            provider_config: self.provider_config.clone(),
            indexer_db_pool: self.indexer_db_pool.clone(),
            peer_manager: self.peer_manager.clone(),
            file_download_manager: self.file_download_manager.clone(),
        }
    }
}

impl<NT> StorageHubHandler<NT>
where
    NT: ShNodeType,
{
    pub fn new(
        task_spawner: TaskSpawner,
        file_transfer: ActorHandle<FileTransferService>,
        blockchain: ActorHandle<BlockchainService<NT::FSH>>,
        file_storage: Arc<RwLock<NT::FL>>,
        forest_storage_handler: NT::FSH,
        provider_config: ProviderConfig,
        indexer_db_pool: Option<DbPool>,
        peer_manager: Arc<BspPeerManager>,
    ) -> Self {
        // Get the data directory path from the peer manager's directory
        // This assumes the peer manager stores data in a similar location to where we want our download state
        let data_dir = std::env::temp_dir().join("storagehub");

        // Create a FileDownloadManager with the peer manager already initialized
        let file_download_manager = Arc::new(
            FileDownloadManager::new(Arc::clone(&peer_manager), data_dir)
                .expect("Failed to initialize FileDownloadManager"),
        );

        Self {
            task_spawner,
            file_transfer,
            blockchain,
            file_storage,
            forest_storage_handler,
            provider_config,
            indexer_db_pool,
            peer_manager,
            file_download_manager,
        }
    }
}

/// Abstraction trait to run the [`StorageHubHandler`] tasks, according to the set configuration and role.
///
/// This trait is implemented by the different [`StorageHubHandler`] variants,
/// and runs the tasks required to work as a specific [`ShRole`](super::types::ShRole).
pub trait RunnableTasks {
    async fn run_tasks(&mut self);
}

impl<S: ShStorageLayer> RunnableTasks for StorageHubHandler<(BspProvider, S)>
where
    (BspProvider, S): ShNodeType + 'static,
    <(BspProvider, S) as ShNodeType>::FSH: BspForestStorageHandlerT,
{
    async fn run_tasks(&mut self) {
        self.initialise_bsp().await;
        self.start_bsp_tasks();
    }
}

impl<S: ShStorageLayer> RunnableTasks for StorageHubHandler<(MspProvider, S)>
where
    (MspProvider, S): ShNodeType + 'static,
    <(MspProvider, S) as ShNodeType>::FSH: MspForestStorageHandlerT,
{
    async fn run_tasks(&mut self) {
        self.start_msp_tasks();
    }
}

impl<S: ShStorageLayer> RunnableTasks for StorageHubHandler<(UserRole, S)>
where
    (UserRole, S): ShNodeType + 'static,
{
    async fn run_tasks(&mut self) {
        self.start_user_tasks();
    }
}

impl<S> StorageHubHandler<(UserRole, S)>
where
    (UserRole, S): ShNodeType + 'static,
{
    fn start_user_tasks(&self) {
        log::info!("Starting User tasks.");

        let user_sends_file_task = UserSendsFileTask::new(self.clone());

        // Subscribing to NewStorageRequest event from the BlockchainService.
        let new_storage_request_event_bus_listener: EventBusListener<NewStorageRequest, _> =
            user_sends_file_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain, true);
        new_storage_request_event_bus_listener.start();

        let accepted_bsp_volunteer_event_bus_listener: EventBusListener<AcceptedBspVolunteer, _> =
            user_sends_file_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain, true);
        accepted_bsp_volunteer_event_bus_listener.start();
    }
}

impl<S> StorageHubHandler<(MspProvider, S)>
where
    (MspProvider, S): ShNodeType + 'static,
    <(MspProvider, S) as ShNodeType>::FSH: MspForestStorageHandlerT,
{
    fn start_msp_tasks(&self) {
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
                .subscribe_to(&self.task_spawner, &self.blockchain, true);
        new_storage_request_event_bus_listener.start();
        // Subscribing to RemoteUploadRequest event from the FileTransferService.
        let remote_upload_request_event_bus_listener: EventBusListener<RemoteUploadRequest, _> =
            msp_upload_file_task.clone().subscribe_to(
                &self.task_spawner,
                &self.file_transfer,
                false,
            );
        remote_upload_request_event_bus_listener.start();
        // Subscribing to ProcessMspRespondStoringRequest event from the BlockchainService.
        let process_confirm_storing_request_event_bus_listener: EventBusListener<
            ProcessMspRespondStoringRequest,
            _,
        > = msp_upload_file_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain, true);
        process_confirm_storing_request_event_bus_listener.start();

        // Task that handles bucket deletion (both move and stop storing)
        let msp_delete_bucket_task = MspDeleteBucketTask::new(self.clone());
        // Subscribing to FinalisedMspStoppedStoringBucket event
        let finalised_msp_stopped_storing_bucket_event_bus_listener: EventBusListener<
            FinalisedMspStoppedStoringBucket,
            _,
        > = msp_delete_bucket_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain, true);
        finalised_msp_stopped_storing_bucket_event_bus_listener.start();

        // Subscribing to FinalisedBucketMovedAway event
        let finalised_bucket_moved_away_event_bus_listener: EventBusListener<
            FinalisedBucketMovedAway,
            _,
        > = msp_delete_bucket_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain, true);
        finalised_bucket_moved_away_event_bus_listener.start();

        // MspDeleteFileTask handles events for deleting individual files from an MSP.
        let msp_delete_file_task = MspDeleteFileTask::new(self.clone());
        // Subscribing to FileDeletionRequest event from the BlockchainService.
        let file_deletion_request_event_bus_listener: EventBusListener<FileDeletionRequest, _> =
            msp_delete_file_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain, true);
        file_deletion_request_event_bus_listener.start();
        // Subscribing to ProcessFileDeletionRequest event from the BlockchainService.
        let process_file_deletion_request_event_bus_listener: EventBusListener<
            ProcessFileDeletionRequest,
            _,
        > = msp_delete_file_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain, true);
        process_file_deletion_request_event_bus_listener.start();
        // Subscribing to FinalisedProofSubmittedForPendingFileDeletionRequest event from the BlockchainService.
        let finalised_file_deletion_request_event_bus_listener: EventBusListener<
            FinalisedProofSubmittedForPendingFileDeletionRequest,
            _,
        > = msp_delete_file_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain, true);
        finalised_file_deletion_request_event_bus_listener.start();

        // MspMoveBucketTask handles events for moving buckets to a new MSP.
        let msp_move_bucket_task = MspRespondMoveBucketTask::new(self.clone());
        // Subscribing to MoveBucketRequestedForNewMsp event from the FileTransferService.
        let move_bucket_requested_for_new_msp_event_bus_listener: EventBusListener<
            MoveBucketRequestedForMsp,
            _,
        > = msp_move_bucket_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain, true);
        move_bucket_requested_for_new_msp_event_bus_listener.start();

        // MspDownloadMovedBucketTask handles downloading files after a bucket move is confirmed.
        let msp_download_moved_bucket_task = MspRespondMoveBucketTask::new(self.clone());
        // Subscribing to StartMovedBucketDownload event from the BlockchainService.
        let start_moved_bucket_download_event_bus_listener: EventBusListener<
            StartMovedBucketDownload,
            _,
        > = msp_download_moved_bucket_task.clone().subscribe_to(
            &self.task_spawner,
            &self.blockchain,
            true,
        );
        start_moved_bucket_download_event_bus_listener.start();

        let msp_charge_fees_task = MspChargeFeesTask::new(self.clone());

        // MspStopStoringInsolventUserTask handles events for deleting buckets owned by users that have become insolvent.
        let msp_stop_storing_insolvent_user = MspStopStoringInsolventUserTask::new(self.clone());

        // Subscribing to UserInsolvent event from the BlockchainService to delete all stored buckets owned by a
        // user that has been declared as without funds.
        let user_without_funds_event_bus_listener: EventBusListener<UserWithoutFunds, _> =
            msp_stop_storing_insolvent_user.clone().subscribe_to(
                &self.task_spawner,
                &self.blockchain,
                true,
            );
        user_without_funds_event_bus_listener.start();

        // Subscribing to FinalisedMspStopStoringBucketInsolventUser event from the BlockchainService.
        let finalised_msp_stop_storing_bucket_insolvent_user_event_bus_listener: EventBusListener<
            FinalisedMspStopStoringBucketInsolventUser,
            _,
        > = msp_stop_storing_insolvent_user.clone().subscribe_to(
            &self.task_spawner,
            &self.blockchain,
            true,
        );
        finalised_msp_stop_storing_bucket_insolvent_user_event_bus_listener.start();

        // Subscribing to NotifyPeriod event from the BlockchainService.
        let notify_period_event_bus_listener: EventBusListener<NotifyPeriod, _> =
            msp_charge_fees_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain, true);
        notify_period_event_bus_listener.start();

        // Create the RetryBucketMoveTask and subscribe to events
        let retry_bucket_move_download_task = RetryBucketMoveTask::new(self.clone());

        // Subscribing to RetryBucketMoveDownload event from the FileTransferService.
        let retry_bucket_move_download_event_bus_listener: EventBusListener<
            RetryBucketMoveDownload,
            _,
        > = retry_bucket_move_download_task.clone().subscribe_to(
            &self.task_spawner,
            &self.file_transfer,
            false,
        );
        retry_bucket_move_download_event_bus_listener.start();
    }
}

impl<S> StorageHubHandler<(BspProvider, S)>
where
    (BspProvider, S): ShNodeType + 'static,
    <(BspProvider, S) as ShNodeType>::FSH: BspForestStorageHandlerT,
{
    async fn initialise_bsp(&mut self) {
        // Create an empty Forest Storage instance.
        // A BSP is expected to always have at least one empty Forest Storage instance.
        let current_forest_key = CURRENT_FOREST_KEY.to_vec();
        self.forest_storage_handler
            .create(&current_forest_key)
            .await;
    }

    fn start_bsp_tasks(&self) {
        log::info!("Starting BSP tasks");

        // TODO: When `pallet-cr-randomness` is integrated to the runtime we should also spawn the task that
        // manages the randomness commit-reveal cycle for BSPs here.
        // The task that manages this should be added to the `tasks` folder (name suggestion: `bsp_cr_randomness`).

        // BspUploadFileTask is triggered by a NewStorageRequest event, to which it responds by
        // volunteering to store the file. Then it waits for RemoteUploadRequest events, which
        // happens when the user, now aware of the BSP volunteering, submits chunks of the file,
        // along with a proof of storage.
        let bsp_upload_file_task = BspUploadFileTask::new(self.clone());
        // Subscribing to NewStorageRequest event from the BlockchainService.
        let new_storage_request_event_bus_listener: EventBusListener<NewStorageRequest, _> =
            bsp_upload_file_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain, true);
        new_storage_request_event_bus_listener.start();
        // Subscribing to RemoteUploadRequest event from the FileTransferService.
        let remote_upload_request_event_bus_listener: EventBusListener<RemoteUploadRequest, _> =
            bsp_upload_file_task.clone().subscribe_to(
                &self.task_spawner,
                &self.file_transfer,
                false,
            );
        remote_upload_request_event_bus_listener.start();
        // Subscribing to ProcessConfirmStoringRequest event from the BlockchainService.
        let process_confirm_storing_request_event_bus_listener: EventBusListener<
            ProcessConfirmStoringRequest,
            _,
        > = bsp_upload_file_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain, true);
        process_confirm_storing_request_event_bus_listener.start();

        // The BspDownloadFileTask
        let bsp_download_file_task = BspDownloadFileTask::new(self.clone());
        // Subscribing to RemoteDownloadRequest event from the FileTransferService.
        let remote_download_request_event_bus_listener: EventBusListener<RemoteDownloadRequest, _> =
            bsp_download_file_task.subscribe_to(&self.task_spawner, &self.file_transfer, false);
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
            .subscribe_to(&self.task_spawner, &self.blockchain, true);
        multiple_new_challenge_seeds_event_bus_listener.start();
        // Subscribing to ProcessSubmitProofRequest event from the BlockchainService.
        let process_submit_proof_request_event_bus_listener: EventBusListener<
            ProcessSubmitProofRequest,
            _,
        > = bsp_submit_proof_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain, true);
        process_submit_proof_request_event_bus_listener.start();

        // Slash your own kin or potentially commit seppuku on your own stake.
        // Running this is as a BSP is very honourable and shows a great sense of justice.
        let bsp_slash_provider_task = SlashProviderTask::new(self.clone());
        // Subscribing to SlashableProvider event from the BlockchainService.
        let slashable_provider_event_bus_listener: EventBusListener<SlashableProvider, _> =
            bsp_slash_provider_task.clone().subscribe_to(
                &self.task_spawner,
                &self.blockchain,
                true,
            );
        slashable_provider_event_bus_listener.start();

        // Collect debt from users after a BSP proof is accepted.
        let bsp_charge_fees_task = BspChargeFeesTask::new(self.clone());
        let last_chargeable_info_updated_event_bus_listener: EventBusListener<
            LastChargeableInfoUpdated,
            _,
        > = bsp_charge_fees_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain, true);
        last_chargeable_info_updated_event_bus_listener.start();

        // Subscribing to ProcessStopStoringForInsolventUserRequest event from the BlockchainService.
        let process_stop_storing_for_insolvent_user_request_event_bus_listener: EventBusListener<
            ProcessStopStoringForInsolventUserRequest,
            _,
        > = bsp_charge_fees_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain, true);
        process_stop_storing_for_insolvent_user_request_event_bus_listener.start();

        // Start deletion process for stored files owned by a user that has been declared as without funds and charge
        // its payment stream afterwards, getting the owed tokens and deleting it.
        let user_without_funds_event_bus_listener: EventBusListener<UserWithoutFunds, _> =
            bsp_charge_fees_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain, true);
        user_without_funds_event_bus_listener.start();

        // Continue deletion process for stored files owned by a user that has been declared as without funds.
        // Once the last file has been deleted, get the owed tokens and delete the payment stream.
        let sp_stop_storing_insolvent_user_event_bus_listener: EventBusListener<
            SpStopStoringInsolventUser,
            _,
        > = bsp_charge_fees_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain, true);
        sp_stop_storing_insolvent_user_event_bus_listener.start();

        // BspMoveBucketTask handles events for moving buckets to a new MSP.
        let bsp_move_bucket_task = BspMoveBucketTask::new(self.clone());
        // Subscribing to MoveBucketRequested event from the BlockchainService.
        let move_bucket_requested_event_bus_listener: EventBusListener<MoveBucketRequested, _> =
            bsp_move_bucket_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain, true);
        move_bucket_requested_event_bus_listener.start();

        // Subscribing to MoveBucketAccepted event from the BlockchainService.
        let move_bucket_accepted_event_bus_listener: EventBusListener<MoveBucketAccepted, _> =
            bsp_move_bucket_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain, true);
        move_bucket_accepted_event_bus_listener.start();

        // Subscribing to MoveBucketRejected event from the BlockchainService.
        let move_bucket_rejected_event_bus_listener: EventBusListener<MoveBucketRejected, _> =
            bsp_move_bucket_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain, true);
        move_bucket_rejected_event_bus_listener.start();

        // Subscribing to MoveBucketExpired event from the BlockchainService.
        let move_bucket_expired_event_bus_listener: EventBusListener<MoveBucketExpired, _> =
            bsp_move_bucket_task
                .clone()
                .subscribe_to(&self.task_spawner, &self.blockchain, true);
        move_bucket_expired_event_bus_listener.start();

        // Task that listen for `FinalisedBspConfirmStoppedStoring` to delete file
        let bsp_delete_file_task = BspDeleteFileTask::new(self.clone());
        let finalised_bsp_confirm_stopped_storing_event_bus_listener: EventBusListener<
            FinalisedBspConfirmStoppedStoring,
            _,
        > = bsp_delete_file_task
            .clone()
            .subscribe_to(&self.task_spawner, &self.blockchain, true);
        finalised_bsp_confirm_stopped_storing_event_bus_listener.start();
    }
}
