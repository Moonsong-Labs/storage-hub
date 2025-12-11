use codec::Decode;
use futures::stream::{self, StreamExt};
use log::{debug, error, info, trace, warn};
use pallet_file_system_runtime_api::FileSystemApi;
use sc_client_api::{BlockImportNotification, BlockchainEvents, HeaderBackend};
use sp_api::ProvideRuntimeApi;
use sp_core::H256;
use sp_runtime::traits::{Header, One, SaturatedConversion, Saturating};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;

use shc_actors_framework::actor::{Actor, ActorEventLoop};
use shc_common::{
    blockchain_utils::get_events_at_block,
    traits::StorageEnableRuntime,
    types::{BlockNumber, OpaqueBlock, StorageEnableEvents, StorageHubClient},
};
use shc_indexer_db::models::FileDeletionType;
use shp_types::Hash;

use crate::{
    commands::{FishermanServiceCommand, FishermanServiceError},
    events::{FileDeletionTarget, FishermanServiceEventBusProvider},
};

pub(crate) const LOG_TARGET: &str = "fisherman-service";

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
    /// Track last deletion type processed (for alternating User/Incomplete)
    last_deletion_type: Option<FileDeletionType>,
    /// Duration between batch deletion processing cycles.
    ///
    /// This value must be greater than the expected average block time to avoid processing a new batch deletion cycle at every block.
    /// For example, a block time of 6 seconds and a batch interval of 5 seconds will result in a new batch deletion cycle being processed every block since the time
    /// would have elapsed since the last batch (see. [`monitor_block`](Self::monitor_block)). It is still up to you to decide whether you want this behaviour or not.
    batch_interval_duration: Duration,
    /// Timestamp of last batch emission.
    last_batch_time: Option<Instant>,
    /// Maximum number of files to process per batch deletion cycle
    batch_deletion_limit: u64,
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
        batch_deletion_limit: u64,
    ) -> Self {
        Self {
            client,
            event_bus_provider: FishermanServiceEventBusProvider::new(),
            batch_processing_semaphore: Arc::new(Semaphore::new(1)),
            last_deletion_type: None,
            batch_interval_duration: Duration::from_secs(batch_interval_seconds),
            last_batch_time: None,
            batch_deletion_limit,
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

    /// Monitor new blocks for file deletion request events
    ///
    /// Emits a [`BatchFileDeletions`](`crate::events::BatchFileDeletions`) event if a new batch deletion cycle should be attempted every [`batch_interval_duration`](`Self::batch_interval_duration`) seconds.
    /// Interchanges between [`User`](`FileDeletionType::User`) and [`Incomplete`](`FileDeletionType::Incomplete`) deletion types across batch cycles.
    ///
    /// The [`batch_processing_semaphore`](`Self::batch_processing_semaphore`) permit is passed through the [`BatchFileDeletions`](`crate::events::BatchFileDeletions`) event and automatically released when the event handler completes or fails.
    /// This semaphore ensures that only a single batch deletion cycle is running at a time, preventing any possibility of colliding deletions with this node's own deletions.
    async fn monitor_block(
        &mut self,
        block_number: BlockNumber<Runtime>,
        block_hash: Runtime::Hash,
    ) -> Result<(), FishermanServiceError> {
        info!(target: LOG_TARGET, "🎣 Monitoring block #{}: {}", block_number, block_hash);

        // Check if enough time has elapsed since last batch
        let should_attempt_new_batch_deletion = match self.last_batch_time {
            None => true,
            Some(last_time) => last_time.elapsed() >= self.batch_interval_duration,
        };

        if should_attempt_new_batch_deletion {
            // Try to acquire the semaphore permit (non-blocking)
            // The semaphore prevents running multiple batch deletion cycles concurrently, preventing any possibility of colliding deletions with this node's own deletions.
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
                        "🎣 Batch interval reached, emitting BatchFileDeletions event for {:?} deletions",
                        deletion_type
                    );

                    // Update state service state
                    self.last_deletion_type = Some(deletion_type);
                    self.last_batch_time = Some(Instant::now());

                    // Wrap permit in Arc to satisfy Clone requirement for events
                    // The permit will be held by the event handler for its lifetime,
                    // automatically releasing when the handler completes or fails
                    let permit_wrapper = Arc::new(permit);

                    // Emit event to trigger batch processing
                    self.emit(crate::events::BatchFileDeletions {
                        deletion_type,
                        batch_deletion_limit: self.batch_deletion_limit,
                        permit: permit_wrapper,
                    });
                }
                Err(_) => {
                    // The permit will eventually be released, so we do nothing here and will retry next block.
                    // This is a warning because it could indicate a bug if the permit is held for too long, or indefinitely for that matter.
                    debug!(
                        target: LOG_TARGET,
                        "🎣 Batch interval reached but semaphore permit is held (previous batch still processing), will retry next block"
                    );
                }
            }
        }

        Ok(())
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
                "Bucket [{:?}] is not the target bucket [{:?}]. Skipping mutations.",
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
            "Processed {} MSP/bucket mutations for bucket {:?}",
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
        async move {
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
                        "🎣 QueryIncompleteStorageRequest for file key {:?}",
                        file_key
                    );

                    let result = self.query_incomplete_storage_request(file_key);

                    // Send the result back through the callback
                    if let Err(_) = callback.send(result) {
                        warn!(
                            target: LOG_TARGET,
                            "Failed to send QueryIncompleteStorageRequest response - receiver dropped"
                        );
                    }
                }
            }
        }
    }

    fn get_event_bus_provider(&self) -> &Self::EventBusProvider {
        &self.event_bus_provider
    }
}

/// Messages that can be received in the event loop
enum MergedEventLoopMessage<Runtime: StorageEnableRuntime> {
    Command(FishermanServiceCommand<Runtime>),
    BlockImportNotification(BlockImportNotification<OpaqueBlock>),
}

/// Event loop for the FishermanService actor
///
/// This runs the main monitoring logic of the fisherman service,
/// watching for file deletion requests and processing them by
/// starting [`ProcessFileDeletionRequest`] tasks.
pub struct FishermanServiceEventLoop<Runtime: StorageEnableRuntime> {
    service: FishermanService<Runtime>,
    receiver: sc_utils::mpsc::TracingUnboundedReceiver<FishermanServiceCommand<Runtime>>,
}

impl<Runtime: StorageEnableRuntime> ActorEventLoop<FishermanService<Runtime>>
    for FishermanServiceEventLoop<Runtime>
{
    fn new(
        actor: FishermanService<Runtime>,
        receiver: sc_utils::mpsc::TracingUnboundedReceiver<FishermanServiceCommand<Runtime>>,
    ) -> Self {
        Self {
            service: actor,
            receiver,
        }
    }

    async fn run(mut self) {
        info!(target: LOG_TARGET, "🎣 Fisherman service event loop started");

        // Get import notification stream (not finality stream) to monitor all blocks
        let import_notification_stream = self.service.client.import_notification_stream();

        // Create merged stream for commands and block notifications
        let mut merged_stream = stream::select(
            self.receiver.map(MergedEventLoopMessage::Command),
            import_notification_stream.map(MergedEventLoopMessage::BlockImportNotification),
        );

        loop {
            tokio::select! {
                // Process merged stream
                message = merged_stream.next() => {
                    match message {
                        Some(MergedEventLoopMessage::Command(cmd)) => {
                            self.service.handle_message(cmd).await;
                        }
                        Some(MergedEventLoopMessage::BlockImportNotification(notification)) => {
                            let block_number = *notification.header.number();
                            let block_hash = notification.hash;

                            // Only process new best blocks
                            if !notification.is_new_best {
                                continue;
                            }

                            if let Err(e) = self
                                .service
                                .monitor_block(block_number.into(), block_hash)
                                .await
                            {
                                error!(target: LOG_TARGET, "Failed to monitor block: {:?}", e);
                            }
                        }
                        None => {
                            warn!(target: LOG_TARGET, "Stream ended");
                            break;
                        }
                    }
                }

                // Periodic health check
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(300)) => {
                    info!(target: LOG_TARGET, "🎣 Fisherman service health check - running normally");
                }
            }
        }

        info!(target: LOG_TARGET, "🎣 Fisherman service event loop terminated");
    }
}
