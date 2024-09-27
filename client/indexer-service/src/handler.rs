use diesel_async::AsyncConnection;
use futures::prelude::*;
use log::{error, info};
use std::sync::Arc;
use thiserror::Error;

use sc_client_api::{BlockBackend, BlockchainEvents};
use sp_core::H256;
use sp_runtime::traits::Header;

use shc_actors_framework::actor::{Actor, ActorEventLoop};
use shc_common::types::{BlockNumber, ParachainClient};
use shc_indexer_db::{models::ServiceState, DbConnection, DbPool};

pub(crate) const LOG_TARGET: &str = "indexer-service";

// Since the indexed data should be used directly from the database,
// we don't need to implement commands.
#[derive(Debug)]
pub enum IndexerServiceCommand {}

// The IndexerService actor
pub struct IndexerService {
    client: Arc<ParachainClient>,
    db_pool: DbPool,
}

// Implement the Actor trait for IndexerService
impl Actor for IndexerService {
    type Message = IndexerServiceCommand;
    type EventLoop = IndexerServiceEventLoop;
    type EventBusProvider = (); // We're not using an event bus for now

    fn handle_message(
        &mut self,
        message: Self::Message,
    ) -> impl std::future::Future<Output = ()> + Send {
        async move {
            match message {
                // No commands for now
            }
        }
    }

    fn get_event_bus_provider(&self) -> &Self::EventBusProvider {
        &()
    }
}

// Implement methods for IndexerService
impl IndexerService {
    pub fn new(client: Arc<ParachainClient>, db_pool: DbPool) -> Self {
        Self { client, db_pool }
    }

    async fn handle_finality_notification<Block>(
        &mut self,
        notification: sc_client_api::FinalityNotification<Block>,
    ) -> Result<(), HandleFinalityNotificationError>
    where
        Block: sp_runtime::traits::Block,
        Block::Header: Header<Number = BlockNumber>,
    {
        let finalized_block_hash = notification.hash;
        let finalized_block_number = *notification.header.number();

        info!(target: LOG_TARGET, "Finality notification (#{}): {}", finalized_block_number, finalized_block_hash);

        let mut db_conn = self.db_pool.get().await?;

        let service_state = shc_indexer_db::models::ServiceState::get(&mut db_conn).await?;

        for block_number in
            (service_state.last_processed_block as BlockNumber + 1)..=finalized_block_number
        {
            let block_hash = self
                .client
                .block_hash(block_number)?
                .ok_or(HandleFinalityNotificationError::BlockHashNotFound)?;
            self.index_block(&mut db_conn, block_number as BlockNumber, block_hash)
                .await?;
        }

        Ok(())
    }

    async fn index_block<'a>(
        &self,
        conn: &mut DbConnection<'a>,
        block_number: BlockNumber,
        block_hash: H256,
    ) -> Result<(), IndexBlockError> {
        info!(target: LOG_TARGET, "Indexing block #{}: {}", block_number, block_hash);

        // TODO: Process relevant block events

        conn.transaction::<(), IndexBlockError, _>(move |conn| {
            Box::pin(async move {
                ServiceState::update(conn, block_number as i64).await?;

                // TODO: Add here everything else that we want to update for this block

                Ok(())
            })
        })
        .await?;

        Ok(())
    }
}

// Define the EventLoop for IndexerService
pub struct IndexerServiceEventLoop {
    receiver: sc_utils::mpsc::TracingUnboundedReceiver<IndexerServiceCommand>,
    actor: IndexerService,
}

enum MergedEventLoopMessage<Block>
where
    Block: sp_runtime::traits::Block,
{
    Command(IndexerServiceCommand),
    FinalityNotification(sc_client_api::FinalityNotification<Block>),
}

// Implement ActorEventLoop for IndexerServiceEventLoop
impl ActorEventLoop<IndexerService> for IndexerServiceEventLoop {
    fn new(
        actor: IndexerService,
        receiver: sc_utils::mpsc::TracingUnboundedReceiver<IndexerServiceCommand>,
    ) -> Self {
        Self { actor, receiver }
    }

    async fn run(mut self) {
        info!(target: LOG_TARGET, "IndexerService starting up!");

        let finality_notification_stream = self.actor.client.finality_notification_stream();

        let mut merged_stream = stream::select(
            self.receiver.map(MergedEventLoopMessage::Command),
            finality_notification_stream.map(MergedEventLoopMessage::FinalityNotification),
        );

        while let Some(message) = merged_stream.next().await {
            match message {
                MergedEventLoopMessage::Command(command) => {
                    self.actor.handle_message(command).await;
                }
                MergedEventLoopMessage::FinalityNotification(notification) => {
                    self.actor
                        .handle_finality_notification(notification)
                        .await
                        .unwrap_or_else(|e| {
                            error!(target: LOG_TARGET, "Failed to handle finality notification: {}", e);
                        });
                }
            }
        }

        info!(target: LOG_TARGET, "IndexerService shutting down.");
    }
}

#[derive(Error, Debug)]
pub enum IndexBlockError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] diesel::result::Error),
}

#[derive(Error, Debug)]
pub enum HandleFinalityNotificationError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] diesel::result::Error),
    #[error("Block hash not found")]
    BlockHashNotFound,
    #[error("Index block error: {0}")]
    IndexBlockError(#[from] IndexBlockError),
    #[error("Client error: {0}")]
    ClientError(#[from] sp_blockchain::Error),
    #[error("Pool run error: {0}")]
    PoolRunError(#[from] diesel_async::pooled_connection::bb8::RunError),
}
