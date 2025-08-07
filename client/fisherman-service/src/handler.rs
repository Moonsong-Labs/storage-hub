use futures::stream::{self, StreamExt};
use log::{debug, error, info, warn};
use sc_client_api::BlockchainEvents;
use shc_common::{blockchain_utils::EventsRetrievalError, traits::StorageEnableRuntime};
use sp_runtime::traits::Header;
use std::sync::Arc;
use thiserror::Error;

use shc_actors_framework::actor::{Actor, ActorEventLoop};
use shc_common::types::{BlockNumber, ParachainClient};
use sp_core::H256;

use crate::events::FishermanServiceEventBusProvider;

pub(crate) const LOG_TARGET: &str = "fisherman-service";

/// Commands that can be sent to the FishermanService actor
#[derive(Debug)]
pub enum FishermanServiceCommand {}

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
pub struct FishermanService<Runtime: StorageEnableRuntime> {
    /// Substrate client for blockchain interaction
    client: Arc<ParachainClient<Runtime::RuntimeApi>>,
    /// Last processed block number to avoid reprocessing
    last_processed_block: Option<BlockNumber>,
    /// Event bus provider for emitting fisherman events
    event_bus_provider: FishermanServiceEventBusProvider,
}

impl<Runtime: StorageEnableRuntime> FishermanService<Runtime> {
    /// Create a new FishermanService instance
    pub fn new(client: Arc<ParachainClient<Runtime::RuntimeApi>>) -> Self {
        Self {
            client,
            last_processed_block: None,
            event_bus_provider: FishermanServiceEventBusProvider::new(),
        }
    }

    /// Monitor new blocks for file deletion request events
    async fn monitor_block(
        &mut self,
        block_number: BlockNumber,
        block_hash: H256,
    ) -> Result<(), FishermanServiceError> {
        debug!(target: LOG_TARGET, "🎣 Monitoring block #{}: {}", block_number, block_hash);

        // TODO: Emit ProcessFileDeletionRequest event

        self.last_processed_block = Some(block_number);
        Ok(())
    }
}

/// Implement the Actor trait for FishermanService
impl<Runtime: StorageEnableRuntime> Actor for FishermanService<Runtime> {
    type Message = FishermanServiceCommand;
    type EventLoop = FishermanServiceEventLoop<Runtime>;
    type EventBusProvider = FishermanServiceEventBusProvider;

    fn handle_message(
        &mut self,
        _message: Self::Message,
    ) -> impl std::future::Future<Output = ()> + Send {
        async move {
            // TODO: handle fisherman catch up command
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
pub struct FishermanServiceEventLoop<Runtime: StorageEnableRuntime> {
    service: FishermanService<Runtime>,
    receiver: sc_utils::mpsc::TracingUnboundedReceiver<FishermanServiceCommand>,
}

impl<Runtime: StorageEnableRuntime> ActorEventLoop<FishermanService<Runtime>>
    for FishermanServiceEventLoop<Runtime>
{
    fn new(
        actor: FishermanService<Runtime>,
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
