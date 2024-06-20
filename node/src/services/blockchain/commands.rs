use anyhow::Result;
use serde_json::Number;
use shc_actors_framework::actor::ActorHandle;
use sp_core::{H256, U256};

use super::{
    handler::BlockchainService,
    transaction::SubmittedTransaction,
    types::{Extrinsic, ExtrinsicResult},
};

/// Commands that can be sent to the BlockchainService actor.
#[derive(Debug)]
pub enum BlockchainServiceCommand {
    SendExtrinsic {
        call: storage_hub_runtime::RuntimeCall,
        callback: tokio::sync::oneshot::Sender<Result<SubmittedTransaction>>,
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
    WaitForBlock {
        block_number: U256,
        callback: tokio::sync::oneshot::Sender<tokio::sync::oneshot::Receiver<()>>,
    },
}

/// Interface for interacting with the BlockchainService actor.
pub trait BlockchainServiceInterface {
    /// Send an extrinsic to the runtime.
    async fn send_extrinsic(
        &self,
        call: impl Into<storage_hub_runtime::RuntimeCall>,
    ) -> Result<SubmittedTransaction>;

    /// Get an extrinsic from a block.
    async fn get_extrinsic_from_block(
        &self,
        block_hash: H256,
        extrinsic_hash: H256,
    ) -> Result<Extrinsic>;

    /// Unwatch an extrinsic.
    async fn unwatch_extrinsic(&self, subscription_id: Number) -> Result<()>;

    /// Helper function to check if an extrinsic failed or succeeded in a block.
    fn extrinsic_result(extrinsic: Extrinsic) -> Result<ExtrinsicResult>;

    /// Wait for a block number.
    async fn wait_for_block(&self, block_number: U256) -> Result<()>;
}

/// Implement the BlockchainServiceInterface for the ActorHandle<BlockchainService>.
impl BlockchainServiceInterface for ActorHandle<BlockchainService> {
    async fn send_extrinsic(
        &self,
        call: impl Into<storage_hub_runtime::RuntimeCall>,
    ) -> Result<SubmittedTransaction> {
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

    fn extrinsic_result(extrinsic: Extrinsic) -> Result<ExtrinsicResult> {
        for ev in extrinsic.events {
            match ev.event {
                storage_hub_runtime::RuntimeEvent::System(
                    frame_system::Event::ExtrinsicFailed {
                        dispatch_error,
                        dispatch_info,
                    },
                ) => {
                    return Ok(ExtrinsicResult::Failure {
                        dispatch_info,
                        dispatch_error,
                    });
                }
                storage_hub_runtime::RuntimeEvent::System(
                    frame_system::Event::ExtrinsicSuccess { dispatch_info },
                ) => {
                    return Ok(ExtrinsicResult::Success { dispatch_info });
                }
                _ => {}
            }
        }

        Err(anyhow::anyhow!(
            "Extrinsic does not contain an ExtrinsicFailed event."
        ))
    }

    async fn wait_for_block(&self, block_number: U256) -> Result<()> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        // Build command to send to blockchain service.
        let message = BlockchainServiceCommand::WaitForBlock {
            block_number,
            callback,
        };
        self.send(message).await;
        let rx = rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.");
        rx.await.expect("Failed to wait for block");
        Ok(())
    }
}
