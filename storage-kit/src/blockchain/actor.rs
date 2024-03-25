use anyhow::Result;

use crate::{Actor, EventLoop};

#[derive(Debug)]
pub enum BlockchainModuleCommand {
}

pub struct BlockchainModule {
}

impl Actor for BlockchainModule {
	type Message = BlockchainModuleCommand;
	type EventLoop = EventLoop<Self>;

	async fn handle_message(&mut self, command: Self::Message) {
		match command {
		}
	}
}

impl BlockchainModule {
	/// Creates a BlockchainModule instance that can be used to interact with the blockchain.
	pub fn new() -> Result<BlockchainModule> {
		Ok(BlockchainModule {
		})
	}
}
