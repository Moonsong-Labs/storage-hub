use anyhow::Result;
use async_trait::async_trait;
use serde_json::Number;
use sp_core::H256;

use pallet_file_system_runtime_api::QueryFileEarliestVolunteerBlockError;
use shc_actors_framework::actor::ActorHandle;
use shc_common::types::BlockNumber;

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
        block_number: BlockNumber,
        callback: tokio::sync::oneshot::Sender<tokio::sync::oneshot::Receiver<()>>,
    },
    QueryFileEarliestVolunteerBlock {
        bsp_id: sp_core::sr25519::Public,
        file_key: H256,
        callback:
            tokio::sync::oneshot::Sender<Result<BlockNumber, QueryFileEarliestVolunteerBlockError>>,
    },
    GetNodePublicKey {
        callback: tokio::sync::oneshot::Sender<sp_core::sr25519::Public>,
    },
}

/// Interface for interacting with the BlockchainService actor.
#[async_trait]
pub trait BlockchainServiceInterface {
    /// Send an extrinsic to the runtime.
    async fn send_extrinsic(
        &self,
        call: impl Into<storage_hub_runtime::RuntimeCall> + Send,
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
    async fn wait_for_block(&self, block_number: BlockNumber) -> Result<()>;

    /// Query the earliest block number that a file was volunteered for storage.
    async fn query_file_earliest_volunteer_block(
        &self,
        bsp_id: sp_core::sr25519::Public,
        file_key: H256,
    ) -> Result<BlockNumber, QueryFileEarliestVolunteerBlockError>;

    /// Get the node's public key.
    async fn get_node_public_key(&self) -> sp_core::sr25519::Public;
}

/// Implement the BlockchainServiceInterface for the ActorHandle<BlockchainService>.
#[async_trait]
impl BlockchainServiceInterface for ActorHandle<BlockchainService> {
    async fn send_extrinsic(
        &self,
        call: impl Into<storage_hub_runtime::RuntimeCall> + Send,
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

    async fn wait_for_block(&self, block_number: BlockNumber) -> Result<()> {
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

    async fn query_file_earliest_volunteer_block(
        &self,
        bsp_id: sp_core::sr25519::Public,
        file_key: H256,
    ) -> Result<BlockNumber, QueryFileEarliestVolunteerBlockError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        // Build command to send to blockchain service.
        let message = BlockchainServiceCommand::QueryFileEarliestVolunteerBlock {
            bsp_id,
            file_key,
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    /// Get the node's public key.
    async fn get_node_public_key(&self) -> sp_core::sr25519::Public {
        let (callback, rx) = tokio::sync::oneshot::channel();
        // Build command to send to blockchain service.
        let message = BlockchainServiceCommand::GetNodePublicKey { callback };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }
}
