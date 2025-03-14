use std::sync::Arc;
use tokio::sync::RwLock;

use shc_actors_derive::{subscribe_actor_event, subscribe_actor_event_map};
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
        msp_retry_bucket_move::MspRetryBucketMoveTask,
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

        // Subscribing to NewStorageRequest event from the BlockchainService.
        // NewStorageRequest event can be used by the user to spam, by spamming the network with new
        // storage requests. To prevent this from affecting a BSP node, we make this event NOT
        // critical. This means that if used to spam, some of those spam NewStorageRequest events
        // will be dropped.
        // Subscribing to AcceptedBspVolunteer event from the BlockchainService.
        subscribe_actor_event_map!(
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
            [
                // Override critical for NewStorageRequest to make it non-critical
                NewStorageRequest => { task: UserSendsFileTask, critical: false },
                AcceptedBspVolunteer => UserSendsFileTask,
            ]
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

        // RemoteUploadRequest comes from FileTransferService and requires a separate service parameter
        subscribe_actor_event!(
            event: RemoteUploadRequest,
            task: MspUploadFileTask,
            service: &self.file_transfer,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: false,
        );

        subscribe_actor_event_map!(
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
            [
                // MspDeleteFileTask handles events for deleting individual files from an MSP.
                FileDeletionRequest => MspDeleteFileTask,
                ProcessFileDeletionRequest => MspDeleteFileTask,
                FinalisedProofSubmittedForPendingFileDeletionRequest => MspDeleteFileTask,
                FinalisedBucketMovedAway => MspDeleteBucketTask,
                FinalisedMspStoppedStoringBucket => MspDeleteBucketTask,
                NewStorageRequest => MspUploadFileTask,
                ProcessMspRespondStoringRequest => MspUploadFileTask,
                MoveBucketRequestedForMsp => MspRespondMoveBucketTask,
                StartMovedBucketDownload => MspRespondMoveBucketTask,
                // MspStopStoringInsolventUserTask handles events for deleting buckets owned by users that have become insolvent.
                UserWithoutFunds => MspStopStoringInsolventUserTask,
                FinalisedMspStopStoringBucketInsolventUser => MspStopStoringInsolventUserTask,
                NotifyPeriod => MspChargeFeesTask,
            ]
        );

        subscribe_actor_event!(
            event: RetryBucketMoveDownload,
            task: MspRetryBucketMoveTask,
            service: &self.file_transfer,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: false,
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

        // Subscribing tasks to events from the BlockchainService.
        subscribe_actor_event_map!(
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
            [
                NewStorageRequest => BspUploadFileTask,
                ProcessConfirmStoringRequest => BspUploadFileTask,
                // BspSubmitProofTask is triggered by a MultipleNewChallengeSeeds event emitted by the BlockchainService.
                // It responds by computing challenges derived from the seeds, taking also into account
                // the custom challenges in checkpoint challenge rounds and enqueuing them in BlockchainService.
                // BspSubmitProofTask also listens to ProcessSubmitProofRequest events, which are emitted by the
                // BlockchainService when it is time to actually submit the proof of storage.
                // Additionally, it handles file deletions as a consequence of inclusion proofs in custom challenges.
                MultipleNewChallengeSeeds => BspSubmitProofTask,
                ProcessSubmitProofRequest => BspSubmitProofTask,
                // Slash your own kin or potentially commit seppuku on your own stake.
                // Running this is as a BSP is very honourable and shows a great sense of justice.
                SlashableProvider => SlashProviderTask,
                // Collect debt from users after a BSP proof is accepted.
                LastChargeableInfoUpdated => BspChargeFeesTask,
                ProcessStopStoringForInsolventUserRequest => BspChargeFeesTask,
                // Start deletion process for stored files owned by a user that has been declared as without funds and charge
                // its payment stream afterwards, getting the owed tokens and deleting it.
                UserWithoutFunds => BspChargeFeesTask,
                // Continue deletion process for stored files owned by a user that has been declared as without funds.
                // Once the last file has been deleted, get the owed tokens and delete the payment stream.
                SpStopStoringInsolventUser => BspChargeFeesTask,
                // BspMoveBucketTask handles events for moving buckets to a new MSP.
                MoveBucketRequested => BspMoveBucketTask,
                MoveBucketAccepted => BspMoveBucketTask,
                MoveBucketRejected => BspMoveBucketTask,
                MoveBucketExpired => BspMoveBucketTask,
                // Task that listen for `FinalisedBspConfirmStoppedStoring` to delete file
                FinalisedBspConfirmStoppedStoring => BspDeleteFileTask,
            ]
        );

        // Subscribing tasks to events from the FileTransferService.
        subscribe_actor_event_map!(
            service: &self.file_transfer,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: false,
            [
                RemoteDownloadRequest => BspDownloadFileTask,
                RemoteUploadRequest => BspUploadFileTask,
            ]
        );
    }
}
