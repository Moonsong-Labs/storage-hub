use anyhow::Result;
use serde_json::Number;
use sp_core::H256;
use storage_hub_infra::actor::ActorHandle;

use super::{handler::BlockchainService, types::Extrinsic};

/// Commands that can be sent to the BlockchainService actor.
#[derive(Debug)]
pub enum BlockchainServiceCommand {
    SendExtrinsic {
        call: storage_hub_runtime::RuntimeCall,
        callback: tokio::sync::oneshot::Sender<tokio::sync::mpsc::Receiver<String>>,
    },
    GetExtrinsicFromBlock {
        block_hash: H256,
        extrinsic_hash: H256,
        callback: tokio::sync::oneshot::Sender<Result<Extrinsic>>,
    },
    UnwatchExtrinsic {
        subscription_id: Number,
        callback: tokio::sync::oneshot::Sender<Result<()>>,
    },
}

/// Interface for interacting with the BlockchainService actor.
pub trait BlockchainServiceInterface {
    /// Send an extrinsic to the runtime.
    async fn send_extrinsic(
        &self,
        call: impl Into<storage_hub_runtime::RuntimeCall>,
    ) -> tokio::sync::mpsc::Receiver<String>;

    /// Get an extrinsic from a block.
    async fn get_extrinsic_from_block(
        &self,
        block_hash: H256,
        extrinsic_hash: H256,
    ) -> Result<Extrinsic>;

    /// Unwatch an extrinsic.
    async fn unwatch_extrinsic(&self, subscription_id: Number) -> Result<()>;
}

/// Implement the BlockchainServiceInterface for the ActorHandle<BlockchainService>.
impl BlockchainServiceInterface for ActorHandle<BlockchainService> {
    async fn send_extrinsic(
        &self,
        call: impl Into<storage_hub_runtime::RuntimeCall>,
    ) -> tokio::sync::mpsc::Receiver<String> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        // Build command to send to blockchain service.
        let message = BlockchainServiceCommand::SendExtrinsic {
            call: call.into(),
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn get_extrinsic_from_block(
        &self,
        block_hash: H256,
        extrinsic_hash: H256,
    ) -> Result<Extrinsic> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        // Build command to send to blockchain service.
        let message = BlockchainServiceCommand::GetExtrinsicFromBlock {
            block_hash,
            extrinsic_hash,
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn unwatch_extrinsic(&self, subscription_id: Number) -> Result<()> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        // Build command to send to blockchain service.
        let message = BlockchainServiceCommand::UnwatchExtrinsic {
            subscription_id,
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }
}
