use futures::stream::{self, StreamExt};
use log::{debug, error, info, warn};
use sc_client_api::{BlockchainEvents, HeaderBackend};
use shc_common::{
    blockchain_utils::{get_events_at_block, EventsRetrievalError},
    traits::{StorageEnableApiCollection, StorageEnableRuntimeApi},
    types::FileOperationIntention,
};
use sp_runtime::{traits::Header, MultiSignature};
use std::sync::Arc;
    types::{FileOperation, FileOperationIntention},
};
use sp_runtime::{traits::Header, MultiSignature};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;

use shc_actors_framework::actor::{Actor, ActorEventLoop};
use shc_common::types::{BlockNumber, ParachainClient};
use sp_core::H256;
use sp_core::{Encode, H256};

use crate::events::FishermanServiceEventBusProvider;

pub(crate) const LOG_TARGET: &str = "fisherman-service";

/// Represents an operation that occurred on a file key
#[derive(Debug, Clone)]
pub enum FileKeyOperation {
    /// File key was added with optional metadata (Some when available, None when pending)
    Add(Option<shc_common::types::FileMetadata>),
    /// File key was removed
    Remove,
}

/// Represents a change to a file key between blocks
#[derive(Debug, Clone)]
pub struct FileKeyChange {
    /// The file key that changed
    pub file_key: Vec<u8>,
    /// The operation that was applied
    pub operation: FileKeyOperation,
}

/// Commands that can be sent to the FishermanService actor
#[derive(Debug)]
pub enum FishermanServiceCommand {
    /// Process a file deletion request by constructing proof of inclusion
    /// from Bucket/BSP forest and submitting it to the blockchain
    ProcessFileDeletionRequest {
        /// The file key from the signed intention
        signed_file_operation_intention: FileOperationIntention,
        /// The signed intention
        signature: MultiSignature,
    },
    /// Get file key changes since a specific block for a given provider
    GetFileKeyChangesSinceBlock {
        /// The starting block (exclusive) - changes will be tracked from this block + 1
        from_block: BlockNumber,
        /// The provider to track changes for (BSP ID or Bucket ID)
        provider: crate::events::FileDeletionTarget,
        /// Response channel for the file key changes
        response_tx:
            tokio::sync::oneshot::Sender<Result<Vec<FileKeyChange>, FishermanServiceError>>,
    },
}

/// Errors that can occur in the fisherman service
#[derive(Error, Debug)]
pub enum FishermanServiceError {
    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),
    #[error("Blockchain client error: {0}")]
    Client(String),
    #[error("Events retrieval error: {0}")]
    EventsRetrieval(#[from] EventsRetrievalError),
}

/// The main FishermanService actor
///
/// This service monitors the StorageHub blockchain for file deletion requests,
/// constructs proofs of inclusion from Bucket/BSP forests, and submits these proofs
/// to the StorageHub protocol to permissionlessly mutate (delete the file key) the merkle forest on chain.
pub struct FishermanService<RuntimeApi> {
    /// Substrate client for blockchain interaction
    client: Arc<ParachainClient<RuntimeApi>>,
    /// Last processed block number to avoid reprocessing
    last_processed_block: Option<BlockNumber>,
    /// Event bus provider for emitting fisherman events
    event_bus_provider: FishermanServiceEventBusProvider,
}

impl<RuntimeApi> FishermanService<RuntimeApi> {
    /// Create a new FishermanService instance
    pub fn new(client: Arc<ParachainClient<RuntimeApi>>) -> Self {
        Self {
            client,
            last_processed_block: None,
            event_bus_provider: FishermanServiceEventBusProvider::new(),
        }
    }

    /// Monitor new blocks for file deletion request events
    pub async fn monitor_block(
        &mut self,
        block_number: BlockNumber,
        block_hash: H256,
    ) -> Result<(), FishermanServiceError>
    where
        RuntimeApi: StorageEnableRuntimeApi,
        RuntimeApi::RuntimeApi: StorageEnableApiCollection,
    {
        debug!(target: LOG_TARGET, "🎣 Monitoring block #{}: {}", block_number, block_hash);

        let events = get_events_at_block(&self.client, &block_hash)?;

        for event_record in events.iter() {
            let event: Result<storage_hub_runtime::RuntimeEvent, _> =
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
                storage_hub_runtime::RuntimeEvent::FileSystem(
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
                _ => {}
            }
        }

        self.last_processed_block = Some(block_number);
        Ok(())
    }

    /// Get file key changes between two blocks for a specific provider
    pub async fn get_file_key_changes_since_block(
        &self,
        from_block: BlockNumber,
        provider: crate::events::FileDeletionTarget,
    ) -> Result<Vec<FileKeyChange>, FishermanServiceError>
    where
        RuntimeApi: StorageEnableRuntimeApi,
        RuntimeApi::RuntimeApi: StorageEnableApiCollection,
    {
        // Get the current best block
        let best_block_info = self.client.info();
        let best_block_number = best_block_info.best_number;

        debug!(
            target: LOG_TARGET,
            "🎣 Fetching file key changes from block {} to {}", from_block, best_block_number
        );

        // Track file key states
        // TODO: Add proper memory management and block range limits to prevent OOM
        let mut file_key_states: HashMap<Vec<u8>, FileKeyOperation> = HashMap::new();

        // Track file metadata from NewStorageRequest events
        let mut file_metadata_cache: HashMap<Vec<u8>, shc_common::types::FileMetadata> =
            HashMap::new();

        // Process blocks from from_block + 1 to best_block
        for block_num in (from_block + 1)..=best_block_number {
            // Get block hash
            let block_hash = self
                .client
                .hash(block_num.into())
                .map_err(|e| FishermanServiceError::Client(e.to_string()))?
                .ok_or_else(|| {
                    FishermanServiceError::Client(format!("Block {} not found", block_num))
                })?;

            // Get events at this block
            let events = get_events_at_block(&self.client, &block_hash)?;

            // Process events for file key changes
            for event_record in events.iter() {
                let event: Result<storage_hub_runtime::RuntimeEvent, _> =
                    event_record.event.clone().try_into();
                let event = match event {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                match event {
                    // Track new storage requests
                    storage_hub_runtime::RuntimeEvent::FileSystem(
                        pallet_file_system::Event::NewStorageRequest {
                            who,
                            file_key,
                            bucket_id,
                            location,
                            fingerprint,
                            size,
                            ..
                        },
                    ) => {
                        // Cache metadata based on provider type
                        match &provider {
                            crate::events::FileDeletionTarget::BspId(_) => {
                                // For BSP providers, always cache metadata (any file might be stored by our BSP)
                                if let Ok(metadata) = shc_common::types::FileMetadata::new(
                                    who.encode(),
                                    bucket_id.as_bytes().to_vec(),
                                    location.to_vec(),
                                    size,
                                    fingerprint.as_bytes().into(),
                                ) {
                                    file_metadata_cache
                                        .insert(file_key.as_ref().to_vec(), metadata);
                                }
                            }
                            crate::events::FileDeletionTarget::BucketId(target_bucket_id) => {
                                // For bucket providers, only cache if it's our bucket
                                if &bucket_id == target_bucket_id {
                                    if let Ok(metadata) = shc_common::types::FileMetadata::new(
                                        who.encode(),
                                        bucket_id.as_bytes().to_vec(),
                                        location.to_vec(),
                                        size,
                                        fingerprint.as_bytes().into(),
                                    ) {
                                        file_metadata_cache
                                            .insert(file_key.as_ref().to_vec(), metadata);
                                    }
                                }
                            }
                        }
                    }
                    // Track BSP confirmations
                    storage_hub_runtime::RuntimeEvent::FileSystem(
                        pallet_file_system::Event::BspConfirmedStoring {
                            bsp_id,
                            confirmed_file_keys,
                            ..
                        },
                    ) => {
                        if let crate::events::FileDeletionTarget::BspId(target_bsp_id) = &provider {
                            if &bsp_id == target_bsp_id {
                                // For BSP confirmations, check metadata cache first
                                for file_key in confirmed_file_keys.iter() {
                                    let operation = if let Some(metadata) =
                                        file_metadata_cache.get(file_key.as_ref())
                                    {
                                        FileKeyOperation::Add(Some(metadata.clone()))
                                    } else {
                                        FileKeyOperation::Add(None)
                                    };
                                    file_key_states.insert(file_key.as_ref().to_vec(), operation);
                                }
                                debug!(
                                    target: LOG_TARGET,
                                    "Added {} BSP confirmed file keys",
                                    confirmed_file_keys.len()
                                );
                            }
                        }
                    }
                    // Track BSP stop storing
                    storage_hub_runtime::RuntimeEvent::FileSystem(
                        pallet_file_system::Event::BspConfirmStoppedStoring {
                            bsp_id,
                            file_key,
                            ..
                        },
                    ) => {
                        if let crate::events::FileDeletionTarget::BspId(target_bsp_id) = &provider {
                            if &bsp_id == target_bsp_id {
                                file_key_states
                                    .insert(file_key.as_ref().to_vec(), FileKeyOperation::Remove);
                            }
                        }
                    }
                    // Track successful proof submissions for pending deletions
                    storage_hub_runtime::RuntimeEvent::FileSystem(
                        pallet_file_system::Event::ProofSubmittedForPendingFileDeletionRequest {
                            file_key,
                            ..
                        },
                    ) => {
                        // This confirms the file was deleted
                        file_key_states
                            .insert(file_key.as_ref().to_vec(), FileKeyOperation::Remove);
                        // Remove from metadata cache
                        file_metadata_cache.remove(file_key.as_ref());
                    }
                    // Track MSP accepted storage requests
                    storage_hub_runtime::RuntimeEvent::FileSystem(
                        pallet_file_system::Event::MspAcceptedStorageRequest { file_key, .. },
                    ) => {
                        // For bucket providers, check if we have cached metadata
                        if let crate::events::FileDeletionTarget::BucketId(_) = &provider {
                            // If metadata exists in cache, it means this file is for our bucket
                            // (we only cache metadata for our target bucket)
                            if let Some(metadata) = file_metadata_cache.get(file_key.as_ref()) {
                                file_key_states.insert(
                                    file_key.as_ref().to_vec(),
                                    FileKeyOperation::Add(Some(metadata.clone())),
                                );
                                debug!(
                                    target: LOG_TARGET,
                                    "Added MSP accepted file key with cached metadata"
                                );
                            }
                            // If no metadata in cache, this file is not for our bucket, so we skip it
                        }
                    }
                    // Track insolvent user file removals
                    storage_hub_runtime::RuntimeEvent::FileSystem(
                        pallet_file_system::Event::SpStopStoringInsolventUser {
                            sp_id,
                            file_key,
                            ..
                        },
                    ) => {
                        match &provider {
                            crate::events::FileDeletionTarget::BspId(target_bsp_id) => {
                                // Convert AccountId32 to H256
                                if &H256::from_slice(sp_id.as_ref()) == target_bsp_id {
                                    file_key_states.insert(
                                        file_key.as_ref().to_vec(),
                                        FileKeyOperation::Remove,
                                    );
                                }
                            }
                            crate::events::FileDeletionTarget::BucketId(_) => {
                                // This also affects bucket storage
                                file_key_states
                                    .insert(file_key.as_ref().to_vec(), FileKeyOperation::Remove);
                            }
                        }
                    }
                    // TODO: Track new file deletion completion events once they are implemented
                    _ => {}
                }
            }
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
}

/// Implement the Actor trait for FishermanService
impl<RuntimeApi> Actor for FishermanService<RuntimeApi>
where
    RuntimeApi: StorageEnableRuntimeApi + Send + 'static,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection + Send,
{
    type Message = FishermanServiceCommand;
    type EventLoop = FishermanServiceEventLoop<RuntimeApi>;
    type EventBusProvider = FishermanServiceEventBusProvider;

    fn handle_message(
        &mut self,
        message: Self::Message,
    ) -> impl std::future::Future<Output = ()> + Send {
        async move {
            match message {
                FishermanServiceCommand::ProcessFileDeletionRequest {
                    signed_file_operation_intention,
                    signature,
                } if signed_file_operation_intention.operation == FileOperation::Delete => {
                    info!(
                        target: LOG_TARGET,
                        "🎣 ProcessFileDeletionRequest received for file key {:?}",
                        signed_file_operation_intention.file_key
                    );

                    let event = crate::events::ProcessFileDeletionRequest {
                        signed_file_operation_intention,
                        signature,
                    };

                    self.emit(event);
                }
                FishermanServiceCommand::GetFileKeyChangesSinceBlock {
                    from_block,
                    provider,
                    response_tx,
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

                    if let Err(_) = response_tx.send(result) {
                        warn!(
                            target: LOG_TARGET,
                            "Failed to send GetFileKeyChangesSinceBlock response - receiver dropped"
                        );
                    }
                }
                _ => {
                    warn!(target: LOG_TARGET, "Received unsupported command: {:?}", message);
                }
            }
        }
    }

    fn get_event_bus_provider(&self) -> &Self::EventBusProvider {
        &self.event_bus_provider
    }
}

/// Messages that can be received in the event loop
enum MergedEventLoopMessage<Block>
where
    Block: sp_runtime::traits::Block,
{
    Command(FishermanServiceCommand),
    BlockImportNotification(sc_client_api::BlockImportNotification<Block>),
}

/// Event loop for the FishermanService actor
///
/// This runs the main monitoring logic of the fisherman service,
/// watching for file deletion requests and processing them by
/// starting [`ProcessFileDeletionRequest`] tasks.
pub struct FishermanServiceEventLoop<RuntimeApi> {
    service: FishermanService<RuntimeApi>,
    receiver: sc_utils::mpsc::TracingUnboundedReceiver<FishermanServiceCommand>,
}

impl<RuntimeApi> ActorEventLoop<FishermanService<RuntimeApi>>
    for FishermanServiceEventLoop<RuntimeApi>
where
    RuntimeApi: StorageEnableRuntimeApi + Send + 'static,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection + Send,
{
    fn new(
        actor: FishermanService<RuntimeApi>,
        receiver: sc_utils::mpsc::TracingUnboundedReceiver<FishermanServiceCommand>,
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

                            if let Err(e) = self.service.monitor_block(block_number, block_hash).await {
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
