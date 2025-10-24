use codec::Decode;
use futures::stream::{self, StreamExt};
use log::{debug, error, info, trace, warn};
use pallet_file_system_runtime_api::FileSystemApi;
use sc_client_api::{BlockImportNotification, BlockchainEvents, HeaderBackend};
use sp_api::ProvideRuntimeApi;
use sp_core::H256;
use sp_runtime::traits::{Header, One, SaturatedConversion, Saturating};
use std::{collections::HashMap, sync::Arc};

use shc_actors_framework::actor::{Actor, ActorEventLoop};
use shc_common::{
    blockchain_utils::get_events_at_block,
    traits::StorageEnableRuntime,
    types::{
        BackupStorageProviderId, BlockNumber, BucketId, FileOperation, FileOperationIntention,
        OffchainSignature, OpaqueBlock, ParachainClient, StorageEnableEvents,
    },
};
use shc_indexer_db::{
    models::{BspFile, File, FileDeletionType},
    DbPool, OnchainBspId,
};
use shp_types::Hash;

use crate::{
    commands::{
        FileDeletionData, FishermanServiceCommand, FishermanServiceError, PendingDeletionsGrouped,
    },
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
    client: Arc<ParachainClient<Runtime::RuntimeApi>>,
    /// Last processed block number to avoid reprocessing
    last_processed_block: BlockNumber<Runtime>,
    /// Event bus provider for emitting fisherman events
    event_bus_provider: FishermanServiceEventBusProvider<Runtime>,
    /// The minimum number of blocks behind the current best block to consider the fisherman out of sync
    sync_mode_min_blocks_behind: BlockNumber<Runtime>,
    /// Maximum number of incomplete storage requests to process during initial sync
    incomplete_sync_max: u32,
    /// Page size for incomplete storage request pagination
    incomplete_sync_page_size: u32,
    /// Indexer database pool for querying pending file deletions
    indexer_db_pool: DbPool,
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
        client: Arc<ParachainClient<Runtime::RuntimeApi>>,
        incomplete_sync_max: u32,
        incomplete_sync_page_size: u32,
        sync_mode_min_blocks_behind: u32,
        indexer_db_pool: DbPool,
    ) -> Self {
        Self {
            client,
            last_processed_block: 0u32.into(),
            event_bus_provider: FishermanServiceEventBusProvider::<Runtime>::new(),
            sync_mode_min_blocks_behind: sync_mode_min_blocks_behind.into(),
            incomplete_sync_max,
            incomplete_sync_page_size,
            indexer_db_pool,
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
    async fn monitor_block(
        &mut self,
        block_number: BlockNumber<Runtime>,
        block_hash: Runtime::Hash,
    ) -> Result<(), FishermanServiceError> {
        debug!(target: LOG_TARGET, "🎣 Monitoring block #{}: {}", block_number, block_hash);

        // Check if we just came out of syncing mode
        // On initial startup, last_processed_block is 0, so if current block > sync_mode_min_blocks_behind,
        // we trigger the sync. This matches the blockchain service behavior.
        if block_number.saturating_sub(self.last_processed_block) > self.sync_mode_min_blocks_behind
        {
            info!(target: LOG_TARGET, "🎣 Handling coming out of sync mode (synced to #{}: {})", block_number, block_hash);
            if let Err(e) = self.initial_incomplete_requests_sync().await {
                error!(target: LOG_TARGET, "Failed initial incomplete requests sync: {:?}", e);
            }
        }

        let events = get_events_at_block::<Runtime>(&self.client, &block_hash)?;

        for event_record in events.iter() {
            let event: Result<StorageEnableEvents<Runtime>, _> =
                event_record.event.clone().try_into();
            let event = match event {
                Ok(e) => e,
                Err(e) => {
                    warn!(
                        target: LOG_TARGET,
                        "Failed to decode event: {:?}",
                        e
                    );
                    continue;
                }
            };
            match event {
                StorageEnableEvents::FileSystem(
                    pallet_file_system::Event::FileDeletionRequested {
                        signed_delete_intention,
                        signature,
                    },
                ) if signed_delete_intention.operation == FileOperation::Delete => {
                    info!(
                        target: LOG_TARGET,
                        "🎣 Found FileDeletionRequested event for file key: {:?}",
                        signed_delete_intention.file_key
                    );

                    let event = crate::events::ProcessFileDeletionRequest {
                        signed_file_operation_intention: signed_delete_intention,
                        signature,
                    };

                    self.emit(event);
                }
                StorageEnableEvents::FileSystem(
                    pallet_file_system::Event::IncompleteStorageRequest { file_key },
                ) => {
                    info!(
                        target: LOG_TARGET,
                        "🎣 Found IncompleteStorageRequest event for file key: {:?}",
                        file_key
                    );

                    let event = crate::events::ProcessIncompleteStorageRequest {
                        file_key: file_key.into(),
                    };

                    self.emit(event);
                }
                _ => {}
            }
        }

        self.last_processed_block = block_number;
        Ok(())
    }

    /// Perform initial catch-up for incomplete storage requests
    async fn initial_incomplete_requests_sync(&mut self) -> Result<(), FishermanServiceError> {
        info!(target: LOG_TARGET, "🎣 Starting initial incomplete storage requests sync");

        let page_size = self.incomplete_sync_page_size;
        let cap = self.incomplete_sync_max;
        let mut processed: u32 = 0;
        let mut cursor: Option<Runtime::Hash> = None;

        let best_block_hash = self.client.info().best_hash;

        'sync_loop: while processed < cap {
            // Call the runtime API to get a page of incomplete storage request keys
            let keys = self
                .client
                .runtime_api()
                .list_incomplete_storage_request_keys(best_block_hash, cursor, page_size)
                .map_err(|e| {
                    FishermanServiceError::Client(format!("Runtime API error: {:?}", e))
                })?;

            if keys.is_empty() {
                break;
            }

            let page_count = keys.len();
            debug!(
                target: LOG_TARGET,
                "🎣 Processing page of {} incomplete storage requests",
                page_count
            );

            for key in &keys {
                // Emit the event for each key
                // TODO: Emit batch of file keys per BSP/Bucket
                self.emit(crate::events::ProcessIncompleteStorageRequest {
                    file_key: (*key).into(),
                });

                processed = processed.saturating_add(1);

                // Check if we've hit the cap
                if processed >= cap {
                    info!(
                        target: LOG_TARGET,
                        "🎣 Initial incomplete requests sync reached cap: {}",
                        cap
                    );
                    break 'sync_loop;
                }
            }

            // Advance cursor to last processed key
            cursor = keys.last().cloned();
        }

        info!(target: LOG_TARGET, "🎣 Completed initial incomplete storage requests sync - processed {} requests", processed);

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

    /// Get pending deletions grouped by BSP and Bucket.
    ///
    /// Queries the indexer database for files marked with `deletion_status = InProgress`,
    /// filtered by the specified deletion type.
    ///
    /// # Parameters
    /// * `deletion_type` - Type of deletion to query ([`FileDeletionType::User`] or [`FileDeletionType::Incomplete`])
    /// * `bucket_id` - Optional filter to only return files from a specific bucket
    /// * `bsp_id` - Optional filter to only return files from a specific BSP
    /// * `limit` - Maximum number of files to return (default: 1000)
    /// * `offset` - Number of files to skip for pagination (default: 0)
    async fn get_pending_deletions(
        &self,
        deletion_type: FileDeletionType,
        bucket_id: Option<BucketId<Runtime>>,
        bsp_id: Option<BackupStorageProviderId<Runtime>>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<PendingDeletionsGrouped<Runtime>, FishermanServiceError> {
        trace!(
            target: LOG_TARGET,
            "🎣 Fetching pending {:?} deletions (bucket_id: {:?}, bsp_id: {:?}, limit: {:?}, offset: {:?})",
            deletion_type, bucket_id, bsp_id, limit, offset
        );

        // Clone connection pools for parallel tasks
        let pool_for_bucket = self.indexer_db_pool.clone();
        let pool_for_bsp = self.indexer_db_pool.clone();

        // Execute both pipelines concurrently: each queries + converts its own data
        let bucket_task = async move {
            // Get DB connection for concurrent query
            let mut bucket_conn = pool_for_bucket.get().await.map_err(|e| {
                FishermanServiceError::Client(format!(
                    "Failed to get bucket DB connection: {:?}",
                    e
                ))
            })?;

            // Convert bucket_id from Runtime type to DB type
            let bucket_id_bytes = bucket_id.as_ref().map(|id| id.as_ref() as &[u8]);

            // Query bucket files from DB
            let bucket_files = File::get_files_pending_deletion_grouped_by_bucket(
                &mut bucket_conn,
                deletion_type,
                bucket_id_bytes,
                limit,
                offset,
            )
            .await?;

            drop(bucket_conn);

            // Convert bucket files to Runtime types
            Self::convert_bucket_files_to_runtime(bucket_files)
        };

        let bsp_task = async move {
            // Get DB connection for concurrent query
            let mut bsp_conn = pool_for_bsp.get().await.map_err(|e| {
                FishermanServiceError::Client(format!("Failed to get BSP DB connection: {:?}", e))
            })?;

            // Convert bsp_id from Runtime type to DB type
            let bsp_id_db = bsp_id.map(OnchainBspId::new);

            // Query BSP files from DB
            let bsp_files = BspFile::get_files_pending_deletion_grouped_by_bsp(
                &mut bsp_conn,
                deletion_type,
                bsp_id_db,
                limit,
                offset,
            )
            .await?;

            drop(bsp_conn);

            // Convert BSP files to Runtime types
            Self::convert_bsp_files_to_runtime(bsp_files)
        };

        // Execute both pipelines concurrently
        let (bucket_deletions, bsp_deletions) = tokio::try_join!(bucket_task, bsp_task)?;

        debug!(
            target: LOG_TARGET,
            "🎣 Found {} bucket groups and {} BSP groups with pending {:?} deletions",
            bucket_deletions.len(),
            bsp_deletions.len(),
            deletion_type
        );

        Ok(PendingDeletionsGrouped {
            bsp_deletions,
            bucket_deletions,
        })
    }

    /// Convert DB bucket files to Runtime types.
    ///
    /// Transforms the database representation to runtime types and converts
    /// individual files to [`FileDeletionData`].
    fn convert_bucket_files_to_runtime(
        db_files: HashMap<Vec<u8>, Vec<File>>,
    ) -> Result<HashMap<BucketId<Runtime>, Vec<FileDeletionData<Runtime>>>, FishermanServiceError>
    {
        let mut result = HashMap::new();

        for (bucket_id_bytes, files) in db_files {
            // Convert bucket ID from database type to Runtime type using SCALE codec
            let bucket_id =
                BucketId::<Runtime>::decode(&mut bucket_id_bytes.as_slice()).map_err(|e| {
                    FishermanServiceError::DecodingError(format!(
                        "Failed to decode bucket ID: {:?}",
                        e
                    ))
                })?;

            // Convert files
            let file_data: Result<Vec<_>, _> = files
                .into_iter()
                .map(|file| Self::convert_file_to_deletion_data(file))
                .collect();

            result.insert(bucket_id, file_data?);
        }

        Ok(result)
    }

    /// Convert DB BSP files to Runtime types.
    ///
    /// Transforms the database representation to runtime types and converts
    /// individual files to [`FileDeletionData`].
    fn convert_bsp_files_to_runtime(
        db_files: HashMap<OnchainBspId, Vec<File>>,
    ) -> Result<
        HashMap<BackupStorageProviderId<Runtime>, Vec<FileDeletionData<Runtime>>>,
        FishermanServiceError,
    > {
        let mut result = HashMap::new();

        for (bsp_id, files) in db_files {
            // Convert BSP ID from database type to Runtime type
            let provider_id = bsp_id.into_h256();

            // Convert files
            let file_data: Result<Vec<_>, _> = files
                .into_iter()
                .map(|file| Self::convert_file_to_deletion_data(file))
                .collect();

            result.insert(provider_id, file_data?);
        }

        Ok(result)
    }

    /// Convert single DB File to [`FileDeletionData`].
    ///
    /// Handles conversion of all file metadata and decodes signatures for user deletions.
    /// For user deletions, reconstructs the [`FileOperationIntention`] from the file key.
    fn convert_file_to_deletion_data(
        file: File,
    ) -> Result<FileDeletionData<Runtime>, FishermanServiceError> {
        // Convert file key from database type to Runtime type using SCALE codec
        let file_key = Runtime::Hash::decode(&mut file.file_key.as_slice()).map_err(|e| {
            FishermanServiceError::DecodingError(format!("Failed to decode file key: {:?}", e))
        })?;

        // Convert to FileMetadata
        let file_metadata = file
            .to_file_metadata(file.onchain_bucket_id.clone())
            .map_err(|e| FishermanServiceError::DecodingError(e))?;

        // Decode signature if present (user deletions)
        let (signature, signed_intention) = if let Some(sig_bytes) = &file.deletion_signature {
            // Decode signature from SCALE-encoded bytes
            let signature =
                OffchainSignature::<Runtime>::decode(&mut &sig_bytes[..]).map_err(|e| {
                    FishermanServiceError::DecodingError(format!(
                        "Failed to decode signature: {:?}",
                        e
                    ))
                })?;

            // Reconstruct FileOperationIntention
            let intention = FileOperationIntention {
                file_key,
                operation: FileOperation::Delete,
            };

            (Some(signature), Some(intention))
        } else {
            // No signature for incomplete deletions
            (None, None)
        };

        Ok(FileDeletionData {
            file_key,
            file_metadata,
            signature,
            signed_intention,
        })
    }
}

impl<Runtime: StorageEnableRuntime> Actor for FishermanService<Runtime> {
    type Message = FishermanServiceCommand<Runtime>;
    type EventLoop = FishermanServiceEventLoop<Runtime>;
    type EventBusProvider = FishermanServiceEventBusProvider<Runtime>;

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
                FishermanServiceCommand::GetPendingDeletions {
                    deletion_type,
                    bucket_id,
                    bsp_id,
                    limit,
                    offset,
                    callback,
                } => {
                    debug!(
                        target: LOG_TARGET,
                        "🎣 GetPendingDeletions command received (deletion_type: {:?}, bucket_id: {:?}, bsp_id: {:?}, limit: {:?}, offset: {:?})",
                        deletion_type, bucket_id, bsp_id, limit, offset
                    );

                    let result = self
                        .get_pending_deletions(deletion_type, bucket_id, bsp_id, limit, offset)
                        .await;

                    // Send the result back through the callback
                    if let Err(_) = callback.send(result) {
                        warn!(
                            target: LOG_TARGET,
                            "Failed to send GetPendingDeletions response - receiver dropped"
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
