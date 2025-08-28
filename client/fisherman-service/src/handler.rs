use codec::Decode;
use futures::stream::{self, StreamExt};
use log::{debug, error, info, warn};
use pallet_file_system_runtime_api::FileSystemApi;
use sc_client_api::{BlockImportNotification, BlockchainEvents, HeaderBackend};
use shc_common::types::{FileOperation, OpaqueBlock, StorageEnableEvents};
use shc_common::{blockchain_utils::get_events_at_block, traits::StorageEnableRuntime};
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::{Header, One, SaturatedConversion, Saturating};
use std::{collections::HashMap, sync::Arc};

use shc_actors_framework::actor::{Actor, ActorEventLoop};
use shc_common::types::{BlockNumber, ParachainClient};
use shp_types::Hash;

use crate::{
    commands::{FishermanServiceCommand, FishermanServiceError},
    events::FishermanServiceEventBusProvider,
};

pub(crate) const LOG_TARGET: &str = "fisherman-service";

/// Represents an operation that occurred on a file key
#[derive(Debug, Clone)]
pub enum FileKeyOperation {
    /// File key was added with complete metadata
    Add(shc_common::types::FileMetadata),
    /// File key was removed
    Remove,
}

/// Represents a change to a file key between blocks
#[derive(Debug, Clone)]
pub struct FileKeyChange {
    /// The file key that changed
    pub file_key: Hash,
    /// The operation that was applied
    pub operation: FileKeyOperation,
}

/// The main FishermanService actor
///
/// This service monitors the StorageHub blockchain for file deletion requests,
/// constructs proofs of inclusion from Bucket/BSP forests, and submits these proofs
/// to the StorageHub protocol to permissionlessly mutate (delete the file key) the merkle forest on chain.
pub struct FishermanService<Runtime: StorageEnableRuntime> {
    /// Substrate client for blockchain interaction
    client: Arc<ParachainClient<Runtime::RuntimeApi>>,
    /// Last processed block number to avoid reprocessing
    last_processed_block: Option<BlockNumber<Runtime>>,
    /// Event bus provider for emitting fisherman events
    event_bus_provider: FishermanServiceEventBusProvider<Runtime>,
}

impl<Runtime: StorageEnableRuntime> FishermanService<Runtime> {
    /// Create a new FishermanService instance
    pub fn new(client: Arc<ParachainClient<Runtime::RuntimeApi>>) -> Self {
        Self {
            client,
            last_processed_block: None,
            event_bus_provider: FishermanServiceEventBusProvider::<Runtime>::new(),
        }
    }

    /// Monitor new blocks for file deletion request events
    async fn monitor_block(
        &mut self,
        block_number: BlockNumber<Runtime>,
        block_hash: Runtime::Hash,
    ) -> Result<(), FishermanServiceError> {
        debug!(target: LOG_TARGET, "🎣 Monitoring block #{}: {}", block_number, block_hash);

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
                    pallet_file_system::Event::StorageRequestExpired { file_key },
                ) => {
                    info!(
                        target: LOG_TARGET,
                        "🎣 Found StorageRequestExpired event for file key: {:?}",
                        file_key
                    );

                    let event = crate::events::ProcessIncompleteStorageRequest {
                        file_key: file_key.into(),
                    };

                    self.emit(event);
                }
                StorageEnableEvents::FileSystem(
                    pallet_file_system::Event::StorageRequestRevoked { file_key },
                ) => {
                    info!(
                        target: LOG_TARGET,
                        "🎣 Found StorageRequestRevoked event for file key: {:?}",
                        file_key
                    );

                    let event = crate::events::ProcessIncompleteStorageRequest {
                        file_key: file_key.into(),
                    };

                    self.emit(event);
                }
                StorageEnableEvents::FileSystem(
                    pallet_file_system::Event::StorageRequestRejected { file_key, .. },
                ) => {
                    info!(
                        target: LOG_TARGET,
                        "🎣 Found StorageRequestRejected event for file key: {:?}",
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

        self.last_processed_block = Some(block_number);
        Ok(())
    }

    /// Get file key changes between two blocks for a specific provider.
    ///
    /// Note:
    /// - `from_block` is excluded from being processed.
    /// - `provider` is either a BSP id or a Bucket id
    pub async fn get_file_key_changes_since_block(
        &self,
        from_block: BlockNumber<Runtime>,
        provider: crate::events::FileDeletionTarget<Runtime>,
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
            let num: u32 = (block_num.into()).as_u64().saturated_into();
            // Get block hash
            let block_hash = self
                .client
                .hash(num)
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

                match event {
                    // Process BSP mutations from MutationsAppliedForProvider events
                    StorageEnableEvents::ProofsDealer(
                        pallet_proofs_dealer::Event::MutationsAppliedForProvider {
                            provider_id,
                            mutations,
                            ..
                        },
                    ) => {
                        if let crate::events::FileDeletionTarget::BspId(target_bsp_id) = &provider {
                            self.process_bsp_mutations(
                                &mutations,
                                target_bsp_id,
                                &provider_id,
                                &mut file_key_states,
                            );
                        }
                    }
                    // Process MSP/bucket mutations from MutationsApplied events
                    StorageEnableEvents::ProofsDealer(
                        pallet_proofs_dealer::Event::MutationsApplied {
                            mutations,
                            event_info,
                            ..
                        },
                    ) => {
                        if let crate::events::FileDeletionTarget::BucketId(target_bucket_id) =
                            &provider
                        {
                            self.process_msp_bucket_mutations(
                                &block_hash,
                                &mutations,
                                target_bucket_id,
                                event_info,
                                &mut file_key_states,
                            );
                        }
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

        info!(
            target: LOG_TARGET,
            "🎣 Found {} file key changes for provider {:?} between blocks {} and {}",
            changes.len(),
            provider,
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
        provider_id: &Hash,
        file_key_states: &mut HashMap<Hash, FileKeyOperation>,
    ) {
        // Check if the provider_id matches the target BSP
        if provider_id != target_bsp_id {
            debug!(
                target: LOG_TARGET,
                "Provider ID [{:?}] is not the target BSP ID [{:?}]. Skipping mutations.",
                provider_id,
                target_bsp_id
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
            "Processed {} BSP mutations for provider {:?}",
            mutations.len(),
            provider_id
        );
    }

    /// Process MSP/bucket mutations from MutationsApplied events
    fn process_msp_bucket_mutations(
        &self,
        block_hash: &Runtime::Hash,
        mutations: &[(Hash, shc_common::types::TrieMutation)],
        target_bucket_id: &shc_common::types::BucketId<Runtime>,
        event_info: Option<Vec<u8>>,
        file_key_states: &mut HashMap<Hash, FileKeyOperation>,
    ) {
        // Check that event_info contains bucket ID
        if event_info.is_none() {
            error!(
                target: LOG_TARGET,
                "MutationsApplied event with `None` event info, when it is expected to contain the BucketId of the bucket that was mutated."
            );
            return;
        }
        let event_info = event_info.expect("Just checked that this is not None; qed");

        // Decode bucket ID from event info
        let bucket_id = match self
            .client
            .runtime_api()
            .decode_generic_apply_delta_event_info(*block_hash, event_info)
            .map_err(|e| {
                error!(
                    target: LOG_TARGET,
                    "Error while calling runtime API to decode BucketId from event info: {:?}",
                    e
                );
            })
            .and_then(|res| {
                res.map_err(|e| {
                    error!(
                        target: LOG_TARGET,
                        "Failed to decode BucketId from event info: {:?}",
                        e
                    );
                })
            }) {
            Ok(bucket_id) => bucket_id,
            Err(_) => return,
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

/// Implement the Actor trait for FishermanService
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

                            // TODO: Only monitor block if it is the new best block

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
