use std::time::Duration;

use anyhow::{anyhow, Result};
use tokio::{
    select,
    sync::{mpsc::Receiver, oneshot},
    time::{self, sleep},
};
use tracing::info;

use crate::{blockchain::events::ChallengeRequest, Actor, ActorEventLoop, ActorHandle};

use super::events::BlockchainEventBusProvider;

#[derive(Debug)]
pub struct RegisteredBsp {
    pub dummy: String,
}

/// Event loop for the P2PModule actor.
pub struct BlockchainEventLoop {
    receiver: Receiver<BlockchainModuleCommand>,
    actor: BlockchainModule,
}

impl ActorEventLoop<BlockchainModule> for BlockchainEventLoop {
    fn new(actor: BlockchainModule, receiver: Receiver<BlockchainModuleCommand>) -> Self {
        Self { actor, receiver }
    }

    async fn run(&mut self) {
        info!("BlockchainModule starting up");
        let mut interval = time::interval(Duration::from_secs(1));
        let mut challenge_count = 0;

        loop {
            select! {
                _ = interval.tick() => {
                    challenge_count += 1;
                    info!("BlockchainModule tick");
                    self.actor.emit(ChallengeRequest {challenge: format!("challenge {}", challenge_count) }).unwrap();
                },
                message = self.receiver.recv() => {
                    let message = message.ok_or_else(|| anyhow!("Command invalid!")).unwrap();
                    self.actor.handle_message(message).await;
                },
            }
        }
    }
}

#[derive(Debug)]
pub enum BlockchainModuleCommand {
    /// Get registered BSP list.
    RegisteredBsps {
        channel: oneshot::Sender<Result<Vec<RegisteredBsp>>>,
    },
}

pub struct BlockchainModule {
    event_bus_provider: BlockchainEventBusProvider,
}

impl Actor for BlockchainModule {
    type Message = BlockchainModuleCommand;
    type EventLoop = BlockchainEventLoop;
    type EventBusProvider = BlockchainEventBusProvider;

    async fn handle_message(&mut self, command: Self::Message) {
        match command {
            BlockchainModuleCommand::RegisteredBsps { channel } => {
                let dummy_result = vec![RegisteredBsp {
                    dummy: "dummy".to_string(),
                }];

                channel
                    .send(Ok(dummy_result))
                    .map_err(|_| anyhow!("Failed to send get registered bsps command response"))
                    .unwrap();
            }
        }
    }

    fn get_event_bus_provider(&self) -> &Self::EventBusProvider {
        &self.event_bus_provider
    }
}

impl BlockchainModule {
    /// Creates a BlockchainModule instance that can be used to interact with the blockchain.
    pub fn new() -> Result<BlockchainModule> {
        Ok(BlockchainModule {
            event_bus_provider: BlockchainEventBusProvider::new(),
        })
    }
}

impl ActorHandle<BlockchainModule> {
    /// Get registered BSP list.
    pub async fn registered_bsps(&self) -> Result<Vec<RegisteredBsp>> {
        let (channel, receiver) = oneshot::channel();
        self.send(BlockchainModuleCommand::RegisteredBsps { channel })
            .await;
        receiver.await?
    }
}
