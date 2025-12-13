use std::{
    fmt::{self, Debug},
    sync::Arc,
};
use tokio::sync::RwLock;

use shc_actors_derive::{subscribe_actor_event, subscribe_actor_event_map};
use shc_actors_framework::{
    actor::{ActorHandle, TaskSpawner},
    event_bus::EventHandler,
};
use shc_blockchain_service::{
    capacity_manager::CapacityConfig,
    events::{
        AcceptedBspVolunteer, DistributeFileToBsp, FinalisedBspConfirmStoppedStoring,
        FinalisedBucketMovedAway, FinalisedBucketMutationsApplied,
        FinalisedMspStopStoringBucketInsolventUser, FinalisedMspStoppedStoringBucket,
        FinalisedStorageRequestRejected, FinalisedTrieRemoveMutationsAppliedForBsp,
        LastChargeableInfoUpdated, MoveBucketAccepted, MoveBucketExpired, MoveBucketRejected,
        MoveBucketRequested, MoveBucketRequestedForMsp, MultipleNewChallengeSeeds,
        NewStorageRequest, NotifyPeriod, ProcessConfirmStoringRequest,
        ProcessMspRespondStoringRequest, ProcessStopStoringForInsolventUserRequest,
        ProcessSubmitProofRequest, SlashableProvider, SpStopStoringInsolventUser,
        StartMovedBucketDownload, UserWithoutFunds,
    },
    handler::BlockchainServiceConfig,
    BlockchainService,
};
use shc_common::{consts::CURRENT_FOREST_KEY, traits::StorageEnableRuntime};
use shc_file_transfer_service::{
    events::{RemoteDownloadRequest, RemoteUploadRequest, RetryBucketMoveDownload},
    FileTransferService,
};
use shc_fisherman_service::events::BatchFileDeletions;
use shc_forest_manager::traits::ForestStorageHandler;
use shc_indexer_db::DbPool;

use crate::{
    bsp_peer_manager::BspPeerManager,
    file_download_manager::FileDownloadManager,
    tasks::{
        bsp_charge_fees::{BspChargeFeesConfig, BspChargeFeesTask},
        bsp_delete_file::BspDeleteFileTask,
        bsp_download_file::BspDownloadFileTask,
        bsp_move_bucket::{BspMoveBucketConfig, BspMoveBucketTask},
        bsp_submit_proof::{BspSubmitProofConfig, BspSubmitProofTask},
        bsp_upload_file::{BspUploadFileConfig, BspUploadFileTask},
        fisherman_process_batch_deletions::FishermanTask,
        msp_charge_fees::{MspChargeFeesConfig, MspChargeFeesTask},
        msp_delete_bucket::MspDeleteBucketTask,
        msp_delete_file::MspDeleteFileTask,
        msp_distribute_file::MspDistributeFileTask,
        msp_move_bucket::{MspMoveBucketConfig, MspRespondMoveBucketTask},
        msp_retry_bucket_move::MspRetryBucketMoveTask,
        msp_stop_storing_insolvent_user::MspStopStoringInsolventUserTask,
        msp_upload_file::MspUploadFileTask,
        sp_slash_provider::SlashProviderTask,
        user_sends_file::UserSendsFileTask,
    },
    types::{
        BspForestStorageHandlerT, BspProvider, FishermanForestStorageHandlerT, FishermanRole,
        ForestStorageKey, MspForestStorageHandlerT, MspProvider, NoStorageLayer, ShNodeType,
        ShStorageLayer, UserRole,
    },
};

/// Configuration parameters for Storage Providers.
#[derive(Clone, Debug)]
pub struct ProviderConfig<Runtime>
where
    Runtime: StorageEnableRuntime,
{
    /// Configuration for MSP charge fees task.
    pub msp_charge_fees: MspChargeFeesConfig,
    /// Configuration for MSP move bucket task.
    pub msp_move_bucket: MspMoveBucketConfig,
    /// Configuration for BSP upload file task.
    pub bsp_upload_file: BspUploadFileConfig,
    /// Configuration for BSP move bucket task.
    pub bsp_move_bucket: BspMoveBucketConfig,
    /// Configuration for BSP charge fees task.
    pub bsp_charge_fees: BspChargeFeesConfig,
    /// Configuration for BSP submit proof task.
    pub bsp_submit_proof: BspSubmitProofConfig,
    /// Configuration for blockchain service.
    pub blockchain_service: BlockchainServiceConfig<Runtime>,
    /// This is only required if running as a storage provider node.
    pub capacity_config: CapacityConfig<Runtime>,
}

/// Represents the handler for the Storage Hub service.
pub struct StorageHubHandler<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
    /// The task spawner for spawning asynchronous tasks.
    pub task_spawner: TaskSpawner,
    /// The actor handle for the file transfer service.
    pub file_transfer: ActorHandle<FileTransferService<Runtime>>,
    /// The actor handle for the blockchain service.
    pub blockchain: ActorHandle<BlockchainService<NT::FSH, Runtime>>,
    /// The file storage layer which stores all files in chunks.
    pub file_storage: Arc<RwLock<NT::FL>>,
    /// The forest storage layer which tracks all complete files stored in the file storage layer.
    pub forest_storage_handler: NT::FSH,
    /// The configuration parameters for the provider.
    pub provider_config: ProviderConfig<Runtime>,
    /// The indexer database pool.
    pub indexer_db_pool: Option<DbPool>,
    /// The BSP peer manager for tracking peer performance.
    pub peer_manager: Arc<BspPeerManager>,
    /// The file download manager for rate-limiting downloads.
    pub file_download_manager: Arc<FileDownloadManager<Runtime>>,
    /// The fisherman service handle (only used for FishermanRole)
    pub fisherman: Option<ActorHandle<shc_fisherman_service::FishermanService<Runtime>>>,
}

impl<NT, Runtime> Debug for StorageHubHandler<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StorageHubHandler")
            .field("provider_config", &self.provider_config)
            .finish()
    }
}

impl<NT, Runtime> Clone for StorageHubHandler<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> StorageHubHandler<NT, Runtime> {
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
            fisherman: self.fisherman.clone(),
        }
    }
}

impl<NT, Runtime> StorageHubHandler<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(
        task_spawner: TaskSpawner,
        file_transfer: ActorHandle<FileTransferService<Runtime>>,
        blockchain: ActorHandle<BlockchainService<NT::FSH, Runtime>>,
        file_storage: Arc<RwLock<NT::FL>>,
        forest_storage_handler: NT::FSH,
        provider_config: ProviderConfig<Runtime>,
        indexer_db_pool: Option<DbPool>,
        peer_manager: Arc<BspPeerManager>,
        fisherman: Option<ActorHandle<shc_fisherman_service::FishermanService<Runtime>>>,
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
            fisherman,
        }
    }
}

/// Abstraction trait to run the [`StorageHubHandler`] tasks, according to the set configuration and role.
///
/// This trait is implemented by the different [`StorageHubHandler`] variants,
/// and runs the tasks required to work as a specific [`ShRole`](super::types::ShRole).
pub trait RunnableTasks {
    fn run_tasks(&mut self) -> impl std::future::Future<Output = ()> + Send;
}

impl<S: ShStorageLayer, Runtime> RunnableTasks for StorageHubHandler<(BspProvider, S), Runtime>
where
    (BspProvider, S): ShNodeType<Runtime> + 'static,
    <(BspProvider, S) as ShNodeType<Runtime>>::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn run_tasks(&mut self) {
        self.initialise_bsp().await;
        self.start_bsp_tasks();
    }
}

impl<S: ShStorageLayer, Runtime> RunnableTasks for StorageHubHandler<(MspProvider, S), Runtime>
where
    (MspProvider, S): ShNodeType<Runtime> + 'static,
    <(MspProvider, S) as ShNodeType<Runtime>>::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn run_tasks(&mut self) {
        self.start_msp_tasks();
    }
}

impl<S: ShStorageLayer, Runtime> RunnableTasks for StorageHubHandler<(UserRole, S), Runtime>
where
    (UserRole, S): ShNodeType<Runtime> + 'static,
    Runtime: StorageEnableRuntime,
{
    async fn run_tasks(&mut self) {
        self.start_user_tasks();
    }
}

impl<Runtime> RunnableTasks for StorageHubHandler<(FishermanRole, NoStorageLayer), Runtime>
where
    (FishermanRole, NoStorageLayer): ShNodeType<Runtime> + 'static,
    <(FishermanRole, NoStorageLayer) as ShNodeType<Runtime>>::FSH:
        FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn run_tasks(&mut self) {
        self.start_fisherman_tasks();
    }
}

impl<S, Runtime> StorageHubHandler<(UserRole, S), Runtime>
where
    (UserRole, S): ShNodeType<Runtime> + 'static,
    Runtime: StorageEnableRuntime,
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
                NewStorageRequest<Runtime> => { task: UserSendsFileTask, critical: false },
                AcceptedBspVolunteer<Runtime> => UserSendsFileTask,
            ]
        );
    }
}

impl<S, Runtime> StorageHubHandler<(MspProvider, S), Runtime>
where
    (MspProvider, S): ShNodeType<Runtime> + 'static,
    <(MspProvider, S) as ShNodeType<Runtime>>::FSH: MspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn start_msp_tasks(&self) {
        log::info!("Starting MSP tasks");

        // MspUploadFileTask is triggered by a NewStorageRequest event which registers the user's peer address for
        // an upcoming RemoteUploadRequest events, which happens when the user connects to the MSP and submits chunks of the file,
        // along with a proof of storage, which is then queued to batch accept many storage requests at once.
        // Finally once the ProcessMspRespondStoringRequest event is emitted, the MSP will respond to the user with a confirmation.

        // RemoteUploadRequest comes from FileTransferService and requires a separate service parameter
        subscribe_actor_event_map!(
            service: &self.file_transfer,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: false,
            [
                RemoteUploadRequest<Runtime> => MspUploadFileTask,
                RetryBucketMoveDownload => MspRetryBucketMoveTask,
            ]
        );

        subscribe_actor_event_map!(
            service: &self.blockchain,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
            [
                FinalisedBucketMovedAway<Runtime> => MspDeleteBucketTask,
                FinalisedMspStoppedStoringBucket<Runtime> => MspDeleteBucketTask,
                NewStorageRequest<Runtime> => MspUploadFileTask,
                ProcessMspRespondStoringRequest<Runtime> => MspUploadFileTask,
                MoveBucketRequestedForMsp<Runtime> => MspRespondMoveBucketTask,
                StartMovedBucketDownload<Runtime> => MspRespondMoveBucketTask,
                // MspStopStoringInsolventUserTask handles events for deleting buckets owned by users that have become insolvent.
                UserWithoutFunds<Runtime> => MspStopStoringInsolventUserTask,
                FinalisedMspStopStoringBucketInsolventUser<Runtime> =>
                    MspStopStoringInsolventUserTask,
                NotifyPeriod => MspChargeFeesTask,
                DistributeFileToBsp<Runtime> => MspDistributeFileTask,
                // MspRemoveFinalisedFilesTask handles events for removing files from file storage after mutations are finalised.
                FinalisedBucketMutationsApplied<Runtime> => MspDeleteFileTask,
                FinalisedStorageRequestRejected<Runtime> => MspDeleteFileTask,
            ]
        );
    }
}

impl<S, Runtime> StorageHubHandler<(BspProvider, S), Runtime>
where
    (BspProvider, S): ShNodeType<Runtime> + 'static,
    <(BspProvider, S) as ShNodeType<Runtime>>::FSH: BspForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn initialise_bsp(&mut self) {
        // Create an empty Forest Storage instance.
        // A BSP is expected to always have at least one empty Forest Storage instance.
        let current_forest_key = ForestStorageKey::from(CURRENT_FOREST_KEY.to_vec());
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
                NewStorageRequest<Runtime> => BspUploadFileTask,
                ProcessConfirmStoringRequest<Runtime> => BspUploadFileTask,
                // BspSubmitProofTask is triggered by a MultipleNewChallengeSeeds event emitted by the BlockchainService.
                // It responds by computing challenges derived from the seeds, taking also into account
                // the custom challenges in checkpoint challenge rounds and enqueuing them in BlockchainService.
                // BspSubmitProofTask also listens to ProcessSubmitProofRequest events, which are emitted by the
                // BlockchainService when it is time to actually submit the proof of storage.
                // Additionally, it handles file deletions as a consequence of inclusion proofs in custom challenges.
                MultipleNewChallengeSeeds<Runtime> => BspSubmitProofTask,
                ProcessSubmitProofRequest<Runtime> => BspSubmitProofTask,
                // Slash your own kin or potentially commit seppuku on your own stake.
                // Running this is as a BSP is very honourable and shows a great sense of justice.
                SlashableProvider<Runtime> => SlashProviderTask,
                // Collect debt from users after a BSP proof is accepted.
                LastChargeableInfoUpdated<Runtime> => BspChargeFeesTask,
                ProcessStopStoringForInsolventUserRequest<Runtime> => BspChargeFeesTask,
                // Start deletion process for stored files owned by a user that has been declared as without funds and charge
                // its payment stream afterwards, getting the owed tokens and deleting it.
                UserWithoutFunds<Runtime> => BspChargeFeesTask,
                // Continue deletion process for stored files owned by a user that has been declared as without funds.
                // Once the last file has been deleted, get the owed tokens and delete the payment stream.
                SpStopStoringInsolventUser<Runtime> => BspChargeFeesTask,
                // BspMoveBucketTask handles events for moving buckets to a new MSP.
                MoveBucketRequested<Runtime> => BspMoveBucketTask,
                MoveBucketAccepted<Runtime> => BspMoveBucketTask,
                MoveBucketRejected<Runtime> => BspMoveBucketTask,
                MoveBucketExpired<Runtime> => BspMoveBucketTask,
                FinalisedBspConfirmStoppedStoring<Runtime> => BspDeleteFileTask,
                FinalisedTrieRemoveMutationsAppliedForBsp<Runtime> => BspDeleteFileTask,
            ]
        );

        // Subscribing tasks to events from the FileTransferService.
        subscribe_actor_event_map!(
            service: &self.file_transfer,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: false,
            [
                RemoteDownloadRequest<Runtime> => BspDownloadFileTask,
                RemoteUploadRequest<Runtime> => BspUploadFileTask,
            ]
        );
    }
}

impl<Runtime: StorageEnableRuntime> StorageHubHandler<(FishermanRole, NoStorageLayer), Runtime>
where
    (FishermanRole, NoStorageLayer): ShNodeType<Runtime> + 'static,
    <(FishermanRole, NoStorageLayer) as ShNodeType<Runtime>>::FSH:
        FishermanForestStorageHandlerT<Runtime>,
{
    fn start_fisherman_tasks(&self) {
        log::info!("🎣 Starting Fisherman tasks");

        let fisherman = self
            .fisherman
            .as_ref()
            .expect("Fisherman service not set for FishermanRole");

        // Register FishermanTask to handle BatchFileDeletions events
        subscribe_actor_event_map!(
            service: fisherman,
            spawner: &self.task_spawner,
            context: self.clone(),
            critical: true,
            [
                // This task processes batched file deletions (for user deletion requests and incomplete storage requests) after every configured interval (`fisherman_batch_interval_seconds`) of time.
                BatchFileDeletions => FishermanTask,
            ]
        );

        log::info!("🎣 Fisherman batch processing task registered");
    }
}
