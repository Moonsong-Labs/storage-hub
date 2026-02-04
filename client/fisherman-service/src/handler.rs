use codec::Decode;
use futures::stream::StreamExt;
use log::{debug, error, info, trace, warn};
use pallet_file_system_runtime_api::FileSystemApi;
use sc_client_api::HeaderBackend;
use sp_api::ProvideRuntimeApi;
use sp_core::H256;
use sp_runtime::traits::{One, SaturatedConversion, Saturating};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{mpsc, Semaphore};
use tokio::time::{self, interval, Instant as TokioInstant};

use shc_actors_framework::actor::{Actor, ActorEventLoop};
use shc_common::{
    blockchain_utils::get_events_at_block,
    traits::StorageEnableRuntime,
    types::{BlockNumber, StorageEnableEvents, StorageHubClient},
};
use shc_indexer_db::models::FileDeletionType;
use shc_telemetry::{observe_histogram, MetricsLink, STATUS_FAILURE, STATUS_SUCCESS};
use shp_types::Hash;

use crate::{
    commands::{FishermanServiceCommand, FishermanServiceError},
    events::{FileDeletionTarget, FishermanServiceEventBusProvider},
    types::{BatchDeletionPermitGuard, BatchDeletionPermitReleased},
};

pub(crate) const LOG_TARGET: &str = "fisherman-service";
pub(crate) const CONSECUTIVE_NO_WORK_BATCHES_THRESHOLD: u8 = 4;

/// The main FishermanService actor
///
/// This service monitors the StorageHub blockchain for file deletion requests,
/// constructs proofs of inclusion from Bucket/BSP forests, and submits these proofs
/// to the StorageHub protocol to permissionlessly mutate (delete the file key) the merkle forest on chain.
pub struct FishermanService<Runtime: StorageEnableRuntime> {
    /// Substrate client for blockchain interaction
    client: Arc<StorageHubClient<Runtime::RuntimeApi>>,
    /// Event bus provider for emitting fisherman events
    event_bus_provider: FishermanServiceEventBusProvider,
    /// Semaphore to prevent overlapping batch processing cycles (size 1)
    batch_processing_semaphore: Arc<Semaphore>,
    /// Channel for batch deletion permit release notifications.
    ///
    /// When a [`BatchDeletionPermitGuard`][crate::types::BatchDeletionPermitGuard] is dropped from a task,
    /// it sends a notification through this channel to the event loop.
    permit_release_sender: mpsc::UnboundedSender<BatchDeletionPermitReleased>,
    /// Track last deletion type processed (for alternating User/Incomplete)
    last_deletion_type: Option<FileDeletionType>,
    /// Cooldown enforced after a completed batch that attempted work.
    ///
    /// After a batch deletion cycle that attempted work, the scheduler will back off for this
    /// duration before starting the next batch deletion cycle. If the batch deletion cycle found no work
    /// in the last [`CONSECUTIVE_NO_WORK_BATCHES_THRESHOLD`] batch cycles, the scheduler will wait
    /// `idle_poll_interval_duration` before starting the next batch deletion cycle.
    batch_cooldown_duration: Duration,
    /// Idle poll interval enforced after a completed batch that found no work.
    ///
    /// After a batch deletion cycle that found no work for the last [`CONSECUTIVE_NO_WORK_BATCHES_THRESHOLD`]
    /// consecutive batch cycles, the scheduler will wait this duration before starting the next batch deletion
    /// cycle. If the batch deletion cycle attempted work, the scheduler will wait `batch_cooldown_duration`
    /// before starting the next batch deletion cycle.
    idle_poll_interval_duration: Duration,
    /// Number of consecutive completed batches that found no work (`did_work = false`).
    ///
    /// We only apply the slower `idle_poll_interval_duration` after we receive `did_work = false` for the last
    /// [`CONSECUTIVE_NO_WORK_BATCHES_THRESHOLD`] consecutive batch cycles.
    ///
    /// This avoids stalling `User` deletions when `Incomplete` is temporarily empty (or vice-versa) while still
    /// backing off when *both* kinds of work are absent regularly.
    consecutive_no_work_batches: u8,
    /// When the next batch attempt is scheduled to run.
    next_scheduled_run: TokioInstant,
    /// Maximum number of files to process per batch deletion cycle
    batch_deletion_limit: u64,
    /// Metrics link for recording command processing
    pub(crate) metrics: MetricsLink,
}

/// Represents a change to a file key between blocks
#[derive(Debug, Clone)]
pub struct FileKeyChange {
    /// The file key that changed
    pub file_key: Hash,
    /// The operation that was applied
    pub operation: FileKeyOperation,
}

/// Represents an operation that occurred on a file key
#[derive(Debug, Clone)]
pub enum FileKeyOperation {
    /// File key was added with complete metadata
    Add(shc_common::types::FileMetadata),
    /// File key was removed
    Remove,
}

impl<Runtime: StorageEnableRuntime> FishermanService<Runtime> {
    /// Create a new FishermanService instance
    pub fn new(
        client: Arc<StorageHubClient<Runtime::RuntimeApi>>,
        batch_interval_seconds: u64,
        batch_cooldown_seconds: u64,
        batch_deletion_limit: u64,
        metrics: MetricsLink,
    ) -> Self {
        // Placeholder sender; overwritten in `FishermanServiceEventLoop::new`.
        let (permit_release_sender, _permit_release_receiver) = mpsc::unbounded_channel();

        Self {
            client,
            event_bus_provider: FishermanServiceEventBusProvider::new(),
            batch_processing_semaphore: Arc::new(Semaphore::new(1)),
            permit_release_sender,
            last_deletion_type: None,
            batch_cooldown_duration: Duration::from_secs(batch_cooldown_seconds),
            idle_poll_interval_duration: Duration::from_secs(batch_interval_seconds),
            consecutive_no_work_batches: 0,
            next_scheduled_run: TokioInstant::now(),
            batch_deletion_limit,
            metrics,
        }
    }

    /// Handles a permit-drop notification from a completed batch deletion task.
    ///
    /// The task owns an `Arc<BatchDeletionPermitGuard>` for the lifetime of its handler. When the
    /// guard is dropped, it notifies this service through `permit_release_receiver`, carrying a
    /// `did_work` flag:
    /// - `did_work = true`: the batch attempted at least one deletion target, so we schedule the
    ///   next attempt after the configured cooldown.
    /// - `did_work = false`: the batch found no work. We keep a fast cadence using the cooldown on
    ///   the first consecutive `false`, and only back off to the idle poll interval after the last
    ///   [`CONSECUTIVE_NO_WORK_BATCHES_THRESHOLD`] consecutive `false` signals.
    ///
    /// This is the key mechanism that eliminates dead time: the scheduler is notified immediately
    /// after the batch completes. If there is no work for the last [`CONSECUTIVE_NO_WORK_BATCHES_THRESHOLD`]
    /// consecutive batch cycles, the scheduler backs off for `idle_poll_interval_duration` seconds before
    /// trying again.
    /// If there was work for at least one of the last [`CONSECUTIVE_NO_WORK_BATCHES_THRESHOLD`]
    /// consecutive batch cycles, the scheduler cools down for just `batch_cooldown_duration` seconds
    /// before trying again.
    fn handle_batch_deletion_permit_released(&mut self, msg: BatchDeletionPermitReleased) {
        let now = TokioInstant::now();
        let delay = if msg.did_work {
            self.consecutive_no_work_batches = 0;
            self.batch_cooldown_duration
        } else {
            self.consecutive_no_work_batches = self.consecutive_no_work_batches.saturating_add(1);
            if self.consecutive_no_work_batches >= CONSECUTIVE_NO_WORK_BATCHES_THRESHOLD {
                self.idle_poll_interval_duration
            } else {
                self.batch_cooldown_duration
            }
        };

        self.next_scheduled_run = now + delay;
        debug!(
            target: LOG_TARGET,
            "🎣 Batch deletion permit released (did_work: {}, no_work_streak: {}), next run scheduled in {:?}",
            msg.did_work,
            self.consecutive_no_work_batches,
            delay
        );
    }

    /// Attempts to start a new batch deletion cycle.
    ///
    /// This is triggered by the scheduler timer (see `FishermanServiceEventLoop::run`). It tries to
    /// acquire the batch semaphore non-blockingly:
    /// - If acquired, emits a `BatchFileDeletions` event holding an `Arc<BatchDeletionPermitGuard>`.
    ///   The guard keeps the permit alive for the handler lifetime and triggers a reschedule on
    ///   `Drop`.
    /// - If not acquired, a previous batch is still running; we keep a conservative idle-based
    ///   retry schedule and otherwise wait for the permit-drop notification.
    fn try_start_batch_deletion_cycle(&mut self) {
        let now = TokioInstant::now();

        match self.batch_processing_semaphore.clone().try_acquire_owned() {
            Ok(permit) => {
                // Determine next deletion type (alternate User ↔ Incomplete)
                let deletion_type = match self.last_deletion_type {
                    None => FileDeletionType::User,
                    Some(FileDeletionType::User) => FileDeletionType::Incomplete,
                    Some(FileDeletionType::Incomplete) => FileDeletionType::User,
                };

                debug!(
                    target: LOG_TARGET,
                    "🎣 Starting batch deletion cycle for {:?} deletions",
                    deletion_type
                );

                // Update state service state
                self.last_deletion_type = Some(deletion_type);

                // Safety net scheduling: if we never receive the permit-drop notification (e.g.
                // during shutdown), retry after the idle interval.
                self.next_scheduled_run = now + self.idle_poll_interval_duration;

                // Wrap permit in a guard that notifies the event loop on drop.
                // The guard is held by the event handler for its lifetime.
                let permit_wrapper = Arc::new(BatchDeletionPermitGuard::new(
                    permit,
                    self.permit_release_sender.clone(),
                ));

                // Emit event to trigger batch processing
                self.emit(crate::events::BatchFileDeletions {
                    deletion_type,
                    batch_deletion_limit: self.batch_deletion_limit,
                    permit: permit_wrapper,
                });
            }
            Err(_) => {
                // Permit is held by an ongoing batch; wait for permit-drop notification.
                self.next_scheduled_run = now + self.idle_poll_interval_duration;
                trace!(
                    target: LOG_TARGET,
                    "🎣 Batch attempt due but permit is held; will retry after idle interval or on permit release"
                );
            }
        }
    }

    /// Query incomplete storage request metadata using runtime API
    fn query_incomplete_storage_request(
        &self,
        file_key: H256,
    ) -> Result<
        pallet_file_system_runtime_api::IncompleteStorageRequestMetadataResponse<
            Runtime::AccountId,
            shc_common::types::BucketId<Runtime>,
            shc_common::types::StorageDataUnit<Runtime>,
            Runtime::Hash,
            shc_common::types::BackupStorageProviderId<Runtime>,
        >,
        FishermanServiceError,
    > {
        trace!(
            target: LOG_TARGET,
            "🎣 Querying incomplete storage request for file key: {:?}",
            file_key
        );

        // Get the best block hash
        let best_block_hash = self.client.info().best_hash;
        trace!(
            target: LOG_TARGET,
            "🎣 Using best block hash: {:?}",
            best_block_hash
        );

        // Use runtime API to query the metadata (decoding happens in runtime context with externalities)
        let metadata = self
            .client
            .runtime_api()
            .query_incomplete_storage_request_metadata(best_block_hash, file_key)
            .map_err(|e| {
                trace!(
                    target: LOG_TARGET,
                    "🎣 Runtime API error: {:?}",
                    e
                );
                FishermanServiceError::Client(format!("Runtime API error: {:?}", e))
            })?
            .map_err(|e| {
                trace!(
                    target: LOG_TARGET,
                    "🎣 Failed to query incomplete storage request: {:?}",
                    e
                );
                match e {
                    pallet_file_system_runtime_api::QueryIncompleteStorageRequestMetadataError::StorageNotFound => {
                        FishermanServiceError::StorageNotFound
                    }
                    _ => FishermanServiceError::Client(format!("Failed to query metadata: {:?}", e))
                }
            })?;

        trace!(
            target: LOG_TARGET,
            "🎣 Successfully retrieved IncompleteStorageRequestMetadata: pending_bucket_removal={}, pending_bsp_removals count={}",
            metadata.pending_bucket_removal,
            metadata.pending_bsp_removals.len()
        );

        Ok(metadata)
    }

    /// Get file key changes between two blocks for a specific target.
    ///
    /// Note:
    /// - `from_block` is excluded from being processed.
    /// - `target` is either a BSP id or a Bucket id to delete the file from
    pub async fn get_file_key_changes_since_block(
        &self,
        from_block: BlockNumber<Runtime>,
        target: FileDeletionTarget<Runtime>,
    ) -> Result<Vec<FileKeyChange>, FishermanServiceError> {
        // Get the current best block
        let best_block_info = self.client.info();
        let best_block_number: BlockNumber<Runtime> = best_block_info.best_number.into();

        debug!(
            target: LOG_TARGET,
            "🎣 Fetching file key changes from block {} to {}", from_block, best_block_number
        );

        // Track file key states
        // TODO: Add proper memory management and block range limits to prevent OOM
        let mut file_key_states: HashMap<Hash, FileKeyOperation> = HashMap::new();

        // Process blocks from `from_block` (excluding) to `best_block_number`
        let mut block_num = from_block.saturating_add(One::one());
        while block_num <= best_block_number {
            // Get block hash using HeaderBackend trait method
            let block_num_u32: u32 = block_num.saturated_into();
            let block_hash = HeaderBackend::hash(&*self.client, block_num_u32)
                .map_err(|e| FishermanServiceError::Client(e.to_string()))?
                .ok_or_else(|| {
                    FishermanServiceError::Client(format!("Block {} not found", block_num))
                })?;

            // Get events at this block
            let events = get_events_at_block::<Runtime>(&self.client, &block_hash)?;

            // Process ProofsDealer events for file key changes
            for event_record in events.iter() {
                let event: Result<StorageEnableEvents<Runtime>, _> =
                    event_record.event.clone().try_into();
                let event = match event {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                match (event, &target) {
                    // Process BSP mutations from MutationsAppliedForProvider events
                    (
                        StorageEnableEvents::ProofsDealer(
                            pallet_proofs_dealer::Event::MutationsAppliedForProvider {
                                provider_id,
                                mutations,
                                ..
                            },
                        ),
                        FileDeletionTarget::BspId(target_bsp_id),
                    ) if &provider_id == target_bsp_id => {
                        self.process_bsp_mutations(
                            &mutations,
                            &target_bsp_id,
                            &mut file_key_states,
                        );
                    }
                    // Process MSP/bucket mutations from MutationsApplied events
                    (
                        StorageEnableEvents::ProofsDealer(
                            pallet_proofs_dealer::Event::MutationsApplied {
                                mutations,
                                event_info,
                                ..
                            },
                        ),
                        FileDeletionTarget::BucketId(target_bucket_id),
                    ) => {
                        self.process_msp_bucket_mutations(
                            &mutations,
                            &target_bucket_id,
                            event_info,
                            &mut file_key_states,
                        );
                    }
                    _ => {}
                }
            }

            // Increment block number for next iteration
            block_num = block_num.saturating_add(One::one());
        }

        // Convert HashMap to Vec<FileKeyChange>
        let changes: Vec<FileKeyChange> = file_key_states
            .into_iter()
            .map(|(file_key, operation)| FileKeyChange {
                file_key,
                operation,
            })
            .collect();

        trace!(
            target: LOG_TARGET,
            "🎣 Found {} file key changes for provider {:?} between blocks {} and {}",
            changes.len(),
            target,
            from_block,
            best_block_number
        );

        Ok(changes)
    }

    /// Process BSP mutations from MutationsAppliedForProvider events
    fn process_bsp_mutations(
        &self,
        mutations: &[(Hash, shc_common::types::TrieMutation)],
        target_bsp_id: &Hash,
        file_key_states: &mut HashMap<Hash, FileKeyOperation>,
    ) {
        // Process mutations
        for (file_key, mutation) in mutations {
            match mutation {
                shc_common::types::TrieMutation::Add(add_mutation) => {
                    // Try to decode the value as FileMetadata
                    if let Ok(metadata) =
                        shc_common::types::FileMetadata::decode(&mut &add_mutation.value[..])
                    {
                        file_key_states.insert(*file_key, FileKeyOperation::Add(metadata));
                    } else {
                        debug!(
                            target: LOG_TARGET,
                            "Failed to decode FileMetadata from mutation value for file key: {:?}",
                            file_key
                        );
                    }
                }
                shc_common::types::TrieMutation::Remove(_) => {
                    file_key_states.insert(*file_key, FileKeyOperation::Remove);
                }
            }
        }

        debug!(
            target: LOG_TARGET,
            "Processed {} BSP mutations for provider {:?}",
            mutations.len(),
            target_bsp_id
        );
    }

    /// Process MSP/bucket mutations from MutationsApplied events
    fn process_msp_bucket_mutations(
        &self,
        mutations: &[(Hash, shc_common::types::TrieMutation)],
        target_bucket_id: &shc_common::types::BucketId<Runtime>,
        event_info: Option<Vec<u8>>,
        file_key_states: &mut HashMap<Hash, FileKeyOperation>,
    ) {
        // Check that event_info contains bucket ID
        let Some(event_info) = event_info else {
            error!(
                target: LOG_TARGET,
                "MutationsApplied event with `None` event info, when it is expected to contain the BucketId of the bucket that was mutated."
            );
            return;
        };

        // Decode bucket ID directly from event info
        let bucket_id = match shc_common::types::BucketId::<Runtime>::decode(&mut &event_info[..]) {
            Ok(bucket_id) => bucket_id,
            Err(e) => {
                error!(
                    target: LOG_TARGET,
                    "Failed to decode BucketId from event info: {:?}",
                    e
                );
                return;
            }
        };

        // Check if bucket ID matches the target bucket
        if &bucket_id != target_bucket_id {
            debug!(
                target: LOG_TARGET,
                "Bucket [0x{:x}] is not the target bucket [0x{:x}]. Skipping mutations.",
                bucket_id,
                target_bucket_id
            );
            return;
        }

        // Process mutations
        for (file_key, mutation) in mutations {
            match mutation {
                shc_common::types::TrieMutation::Add(add_mutation) => {
                    // Try to decode the value as FileMetadata
                    if let Ok(metadata) =
                        shc_common::types::FileMetadata::decode(&mut &add_mutation.value[..])
                    {
                        file_key_states.insert(*file_key, FileKeyOperation::Add(metadata));
                    } else {
                        debug!(
                            target: LOG_TARGET,
                            "Failed to decode FileMetadata from mutation value for file key: {:?}",
                            file_key
                        );
                    }
                }
                shc_common::types::TrieMutation::Remove(_) => {
                    file_key_states.insert(*file_key, FileKeyOperation::Remove);
                }
            }
        }

        debug!(
            target: LOG_TARGET,
            "Processed {} MSP/bucket mutations for bucket [0x{:x}]",
            mutations.len(),
            bucket_id
        );
    }
}

impl<Runtime: StorageEnableRuntime> Actor for FishermanService<Runtime> {
    type Message = FishermanServiceCommand<Runtime>;
    type EventLoop = FishermanServiceEventLoop<Runtime>;
    type EventBusProvider = FishermanServiceEventBusProvider;

    fn handle_message(
        &mut self,
        message: Self::Message,
    ) -> impl std::future::Future<Output = ()> + Send {
        // Extract command name before moving message into match
        let command_name = message.command_name();
        let metrics = self.metrics.clone();

        async move {
            // Start timer for command processing
            let start = std::time::Instant::now();

            // Track command success/failure for metrics
            let mut command_succeeded = true;

            match message {
                FishermanServiceCommand::GetFileKeyChangesSinceBlock {
                    from_block,
                    provider,
                    callback,
                } => {
                    debug!(
                        target: LOG_TARGET,
                        "🎣 GetFileKeyChangesSinceBlock from block {} for provider {:?}",
                        from_block,
                        provider
                    );

                    let result = self
                        .get_file_key_changes_since_block(from_block, provider)
                        .await;

                    // Track if the business logic failed
                    if result.is_err() {
                        command_succeeded = false;
                    }

                    // Send the result back through the callback
                    if let Err(_) = callback.send(result) {
                        warn!(
                            target: LOG_TARGET,
                            "Failed to send GetFileKeyChangesSinceBlock response - receiver dropped"
                        );
                    }
                }
                FishermanServiceCommand::QueryIncompleteStorageRequest { file_key, callback } => {
                    debug!(
                        target: LOG_TARGET,
                        "🎣 QueryIncompleteStorageRequest for file key [{:x}]",
                        file_key
                    );

                    let result = self.query_incomplete_storage_request(file_key);

                    // Track if the business logic failed
                    if result.is_err() {
                        command_succeeded = false;
                    }

                    // Send the result back through the callback
                    if let Err(_) = callback.send(result) {
                        warn!(
                            target: LOG_TARGET,
                            "Failed to send QueryIncompleteStorageRequest response - receiver dropped"
                        );
                    }
                }
            }

            // Record command completion
            let status = if command_succeeded {
                STATUS_SUCCESS
            } else {
                STATUS_FAILURE
            };
            observe_histogram!(metrics: metrics.as_ref(), command_processing_seconds, labels: &[command_name, status], start.elapsed().as_secs_f64());
        }
    }

    fn get_event_bus_provider(&self) -> &Self::EventBusProvider {
        &self.event_bus_provider
    }
}

/// Event loop for the FishermanService actor
///
/// This runs the main monitoring logic of the fisherman service,
/// watching for file deletion requests and processing them by
/// starting [`ProcessFileDeletionRequest`] tasks.
pub struct FishermanServiceEventLoop<Runtime: StorageEnableRuntime> {
    service: FishermanService<Runtime>,
    receiver: sc_utils::mpsc::TracingUnboundedReceiver<FishermanServiceCommand<Runtime>>,
    permit_release_receiver: mpsc::UnboundedReceiver<BatchDeletionPermitReleased>,
}

impl<Runtime: StorageEnableRuntime> ActorEventLoop<FishermanService<Runtime>>
    for FishermanServiceEventLoop<Runtime>
{
    fn new(
        actor: FishermanService<Runtime>,
        receiver: sc_utils::mpsc::TracingUnboundedReceiver<FishermanServiceCommand<Runtime>>,
    ) -> Self {
        // Create permit release channel and wire sender into actor.
        let (permit_release_sender, permit_release_receiver) = mpsc::unbounded_channel();

        let mut actor = actor;
        actor.permit_release_sender = permit_release_sender;

        Self {
            service: actor,
            receiver,
            permit_release_receiver,
        }
    }

    /// Runs the Fisherman service event loop and drives the batch deletion scheduler.
    ///
    /// The loop is fully event-driven:
    /// - A **timer** fires when `next_scheduled_run` is reached, calling
    ///   `try_start_batch_deletion_cycle()`.
    /// - A **permit-drop notification** is received when a batch completes (success, error, or
    ///   early return). This updates `next_scheduled_run`:
    ///   - `did_work = true`: `now + batch_cooldown_duration`
    ///   - `did_work = false`: keep a fast cadence on the first consecutive `false` (cooldown),
    ///     and only back off to `idle_poll_interval_duration` after the last [`CONSECUTIVE_NO_WORK_BATCHES_THRESHOLD`]
    ///     consecutive `false` signals.
    /// - **Commands** are handled as with other actor event loops.
    /// - A **health check** is performed every `health_check_interval_duration` seconds to ensure
    ///   the service is still running.
    async fn run(mut self) {
        info!(target: LOG_TARGET, "🎣 Fisherman service event loop started");

        // Immediate first attempt.
        self.service.next_scheduled_run = TokioInstant::now();

        let mut health_check_interval = interval(Duration::from_secs(300)); // 5 minutes
        health_check_interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                // First entry point. The first time we enter the loop, we'll immediately start a batch deletion cycle.
                // Then every time we reach the next scheduled run, we'll start a new batch deletion cycle.
                // self.service.handle_batch_deletion_permit_released() will update the next scheduled run.
                // If for whatever reason `self.service.next_scheduled_run` is in the past, this will immediately start
                // a new batch deletion cycle.
                _ = time::sleep_until(self.service.next_scheduled_run) => {
                    self.service.try_start_batch_deletion_cycle();
                }

                // When a batch deletion cycle completes (because the permit guard was dropped), we update the next
                // scheduled run.
                Some(msg) = self.permit_release_receiver.recv() => {
                    self.service.handle_batch_deletion_permit_released(msg);
                }

                // Handle commands as with other actor event loops.
                maybe_cmd = self.receiver.next() => {
                    match maybe_cmd {
                        Some(cmd) => self.service.handle_message(cmd).await,
                        None => {
                            warn!(target: LOG_TARGET, "Command stream ended");
                            break;
                        }
                    }
                }

                // Perform a health check every 5 minutes to ensure the service is still running.
                _ = health_check_interval.tick() => {
                    info!(target: LOG_TARGET, "🎣 Fisherman service health check");
                }
            }
        }

        info!(target: LOG_TARGET, "🎣 Fisherman service event loop terminated");
    }
}
