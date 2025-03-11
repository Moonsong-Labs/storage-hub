use std::sync::Arc;
use tokio::sync::RwLock;

use shc_actors_derive::subscribe_actor_event;
use shc_actors_framework::{
    actor::{ActorHandle, TaskSpawner},
    event_bus::EventHandler,
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
    events::{RemoteDownloadRequest, RemoteUploadRequest},
    FileTransferService,
};
use shc_forest_manager::traits::ForestStorageHandler;
use shc_indexer_db::DbPool;

use crate::{
    services::types::{
        BspForestStorageHandlerT, BspProvider, MspForestStorageHandlerT, MspProvider, ShNodeType,
        ShStorageLayer, UserRole,
    },
    tasks::{
        bsp_charge_fees::BspChargeFeesTask, bsp_delete_file::BspDeleteFileTask,
        bsp_download_file::BspDownloadFileTask, bsp_move_bucket::BspMoveBucketTask,
        bsp_submit_proof::BspSubmitProofTask, bsp_upload_file::BspUploadFileTask,
        msp_charge_fees::MspChargeFeesTask, msp_delete_bucket::MspDeleteBucketTask,
        msp_delete_file::MspDeleteFileTask, msp_move_bucket::MspRespondMoveBucketTask,
        msp_stop_storing_insolvent_user::MspStopStoringInsolventUserTask,
        msp_upload_file::MspUploadFileTask, sp_slash_provider::SlashProviderTask,
        user_sends_file::UserSendsFileTask,
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
    ) -> Self {
        Self {
            task_spawner,
            file_transfer,
            blockchain,
            file_storage,
            forest_storage_handler,
            provider_config,
            indexer_db_pool,
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

        // Subscribing to NewStorageRequest event from the BlockchainService.
        subscribe_actor_event!(
            event: NewStorageRequest,
            task: UserSendsFileTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );

        // Subscribing to AcceptedBspVolunteer event from the BlockchainService.
        subscribe_actor_event!(
            event: AcceptedBspVolunteer,
            task: UserSendsFileTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );
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
        // Subscribing to NewStorageRequest event from the BlockchainService.
        subscribe_actor_event!(
            event: NewStorageRequest,
            task: MspUploadFileTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );
        // Subscribing to RemoteUploadRequest event from the FileTransferService.
        subscribe_actor_event!(
            event: RemoteUploadRequest,
            task: MspUploadFileTask,
            service: &self.file_transfer,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: false,
        );
        // Subscribing to ProcessMspRespondStoringRequest event from the BlockchainService.
        subscribe_actor_event!(
            event: ProcessMspRespondStoringRequest,
            task: MspUploadFileTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );

        // Task that handles bucket deletion (both move and stop storing)
        // Subscribing to FinalisedMspStoppedStoringBucket event
        subscribe_actor_event!(
            event: FinalisedMspStoppedStoringBucket,
            task: MspDeleteBucketTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );
        // Subscribing to FinalisedBucketMovedAway event
        subscribe_actor_event!(
            event: FinalisedBucketMovedAway,
            task: MspDeleteBucketTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );

        // MspDeleteFileTask handles events for deleting individual files from an MSP.
        // Subscribing to FileDeletionRequest event from the BlockchainService.
        subscribe_actor_event!(
            event: FileDeletionRequest,
            task: MspDeleteFileTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );
        // Subscribing to ProcessFileDeletionRequest event from the BlockchainService.
        subscribe_actor_event!(
            event: ProcessFileDeletionRequest,
            task: MspDeleteFileTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );
        // Subscribing to FinalisedProofSubmittedForPendingFileDeletionRequest event from the BlockchainService.
        subscribe_actor_event!(
            event: FinalisedProofSubmittedForPendingFileDeletionRequest,
            task: MspDeleteFileTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );

        // MspMoveBucketTask handles events for moving buckets to a new MSP.
        // Subscribing to MoveBucketRequestedForNewMsp event from the FileTransferService.
        subscribe_actor_event!(
            event: MoveBucketRequestedForMsp,
            task: MspRespondMoveBucketTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );

        // MspDownloadMovedBucketTask handles downloading files after a bucket move is confirmed.
        // Subscribing to StartMovedBucketDownload event from the BlockchainService.
        subscribe_actor_event!(
            event: StartMovedBucketDownload,
            task: MspRespondMoveBucketTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );

        // MspStopStoringInsolventUserTask handles events for deleting buckets owned by users that have become insolvent.
        // Subscribing to UserInsolvent event from the BlockchainService to delete all stored buckets owned by a
        // user that has been declared as without funds.
        subscribe_actor_event!(
            event: UserWithoutFunds,
            task: MspStopStoringInsolventUserTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );
        // Subscribing to FinalisedMspStopStoringBucketInsolventUser event from the BlockchainService.
        subscribe_actor_event!(
            event: FinalisedMspStopStoringBucketInsolventUser,
            task: MspStopStoringInsolventUserTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );

        // Subscribing to NotifyPeriod event from the BlockchainService.
        subscribe_actor_event!(
            event: NotifyPeriod,
            task: MspChargeFeesTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );
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

        // Subscribing to NewStorageRequest event from the BlockchainService.
        subscribe_actor_event!(
            event: NewStorageRequest,
            task: BspUploadFileTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );
        // Subscribing to RemoteUploadRequest event from the FileTransferService.
        subscribe_actor_event!(
            event: RemoteUploadRequest,
            task: BspUploadFileTask,
            service: &self.file_transfer,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: false,
        );
        // Subscribing to ProcessConfirmStoringRequest event from the BlockchainService.
        subscribe_actor_event!(
            event: ProcessConfirmStoringRequest,
            task: BspUploadFileTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );

        // Subscribing to RemoteDownloadRequest event from the FileTransferService.
        subscribe_actor_event!(
            event: RemoteDownloadRequest,
            task: BspDownloadFileTask,
            service: &self.file_transfer,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: false,
        );

        // BspSubmitProofTask is triggered by a MultipleNewChallengeSeeds event emitted by the BlockchainService.
        // It responds by computing challenges derived from the seeds, taking also into account
        // the custom challenges in checkpoint challenge rounds and enqueuing them in BlockchainService.
        // BspSubmitProofTask also listens to ProcessSubmitProofRequest events, which are emitted by the
        // BlockchainService when it is time to actually submit the proof of storage.
        // Additionally, it handles file deletions as a consequence of inclusion proofs in custom challenges.

        // Subscribing to MultipleNewChallengeSeeds event from the BlockchainService.
        subscribe_actor_event!(
            event: MultipleNewChallengeSeeds,
            task: BspSubmitProofTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );
        // Subscribing to ProcessSubmitProofRequest event from the BlockchainService.
        subscribe_actor_event!(
            event: ProcessSubmitProofRequest,
            task: BspSubmitProofTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );

        // Slash your own kin or potentially commit seppuku on your own stake.
        // Running this is as a BSP is very honourable and shows a great sense of justice.

        // Subscribing to SlashableProvider event from the BlockchainService.
        subscribe_actor_event!(
            event: SlashableProvider,
            task: SlashProviderTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );

        // Collect debt from users after a BSP proof is accepted.
        // Subscribing to LastChargeableInfoUpdated event from the BlockchainService.
        subscribe_actor_event!(
            event: LastChargeableInfoUpdated,
            task: BspChargeFeesTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );
        // Subscribing to ProcessStopStoringForInsolventUserRequest event from the BlockchainService.
        subscribe_actor_event!(
            event: ProcessStopStoringForInsolventUserRequest,
            task: BspChargeFeesTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );
        // Start deletion process for stored files owned by a user that has been declared as without funds and charge
        // its payment stream afterwards, getting the owed tokens and deleting it.
        subscribe_actor_event!(
            event: UserWithoutFunds,
            task: BspChargeFeesTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );
        // Continue deletion process for stored files owned by a user that has been declared as without funds.
        // Once the last file has been deleted, get the owed tokens and delete the payment stream.
        subscribe_actor_event!(
            event: SpStopStoringInsolventUser,
            task: BspChargeFeesTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );

        // BspMoveBucketTask handles events for moving buckets to a new MSP.
        // Subscribing to MoveBucketRequested event from the BlockchainService.
        subscribe_actor_event!(
            event: MoveBucketRequested,
            task: BspMoveBucketTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );
        // Subscribing to MoveBucketAccepted event from the BlockchainService.
        subscribe_actor_event!(
            event: MoveBucketAccepted,
            task: BspMoveBucketTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );
        // Subscribing to MoveBucketRejected event from the BlockchainService.
        subscribe_actor_event!(
            event: MoveBucketRejected,
            task: BspMoveBucketTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );
        // Subscribing to MoveBucketExpired event from the BlockchainService.
        subscribe_actor_event!(
            event: MoveBucketExpired,
            task: BspMoveBucketTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );

        // Task that listen for `FinalisedBspConfirmStoppedStoring` to delete file
        subscribe_actor_event!(
            event: FinalisedBspConfirmStoppedStoring,
            task: BspDeleteFileTask,
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
        );
    }
}
