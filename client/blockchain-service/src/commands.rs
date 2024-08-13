use anyhow::Result;
use async_trait::async_trait;
use pallet_proofs_dealer_runtime_api::{
    GetCheckpointChallengesError, GetLastTickProviderSubmittedProofError,
};
use pallet_storage_providers_runtime_api::GetBspInfoError;
use serde_json::Number;
use sp_api::ApiError;
use sp_core::H256;

use pallet_file_system_runtime_api::{
    QueryBspConfirmChunksToProveForFileError, QueryFileEarliestVolunteerBlockError,
};
use shc_actors_framework::actor::ActorHandle;
use shc_common::types::{
    BlockNumber, ChunkId, ForestLeaf, ProviderId, RandomnessOutput, TrieRemoveMutation,
};

use crate::handler::ConfirmStoringRequest;
use crate::handler::SubmitProofRequest;

use super::{
    handler::BlockchainService,
    transaction::SubmittedTransaction,
    types::{Extrinsic, ExtrinsicResult},
};

/// Commands that can be sent to the BlockchainService actor.
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
    QueryBspConfirmChunksToProveForFile {
        bsp_id: sp_core::sr25519::Public,
        file_key: H256,
        callback: tokio::sync::oneshot::Sender<
            Result<Vec<ChunkId>, QueryBspConfirmChunksToProveForFileError>,
        >,
    },
    // TODO: update this with proper types
    QueueSubmitProofRequest {
        request: SubmitProofRequest,
        callback: tokio::sync::oneshot::Sender<Result<()>>,
    },
    QueueConfirmBspRequest {
        request: ConfirmStoringRequest,
        callback: tokio::sync::oneshot::Sender<Result<()>>,
    },
    QueryChallengesFromSeed {
        seed: RandomnessOutput,
        provider_id: ProviderId,
        count: u32,
        callback: tokio::sync::oneshot::Sender<Result<Vec<ForestLeaf>, ApiError>>,
    },
    QueryForestChallengesFromSeed {
        seed: RandomnessOutput,
        provider_id: ProviderId,
        callback: tokio::sync::oneshot::Sender<Result<Vec<ForestLeaf>, ApiError>>,
    },
    QueryLastTickProviderSubmittedProof {
        provider_id: ProviderId,
        callback: tokio::sync::oneshot::Sender<
            Result<BlockNumber, GetLastTickProviderSubmittedProofError>,
        >,
    },
    QueryLastCheckpointChallengeTick {
        callback: tokio::sync::oneshot::Sender<Result<BlockNumber, ApiError>>,
    },
    QueryLastCheckpointChallenges {
        tick: BlockNumber,
        callback: tokio::sync::oneshot::Sender<
            Result<Vec<(ForestLeaf, Option<TrieRemoveMutation>)>, GetCheckpointChallengesError>,
        >,
    },
    QueryProviderForestRoot {
        provider_id: ProviderId,
        callback: tokio::sync::oneshot::Sender<Result<H256, GetBspInfoError>>,
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

    /// Query the chunks that a BSP needs to confirm for a file.
    async fn query_bsp_confirm_chunks_to_prove_for_file(
        &self,
        bsp_id: sp_core::sr25519::Public,
        file_key: H256,
    ) -> Result<Vec<ChunkId>, QueryBspConfirmChunksToProveForFileError>;

    // Queue a SubmitProofRequest to be processed.
    async fn queue_submit_proof_request(&self, request: SubmitProofRequest) -> Result<()>;

    // Queue a ConfirmBspRequest to be processed.
    async fn queue_confirm_bsp_request(&self, request: ConfirmStoringRequest) -> Result<()>;

    /// Query the challenges that a Provider needs to submit for a given seed.
    async fn query_challenges_from_seed(
        &self,
        seed: RandomnessOutput,
        provider_id: ProviderId,
        count: u32,
    ) -> Result<Vec<ForestLeaf>, ApiError>;

    /// Query the forest challenges that a Provider needs to submit for a given seed.
    /// This is the same as the `query_challenges_from_seed` method, but it does not
    /// require specifying the `count`, as the runtime will know how many challenges
    /// to generate.
    async fn query_forest_challenges_from_seed(
        &self,
        seed: RandomnessOutput,
        provider_id: ProviderId,
    ) -> Result<Vec<ForestLeaf>, ApiError>;

    /// Query the last tick that a Provider submitted a proof for.
    async fn query_last_tick_provider_submitted_proof(
        &self,
        provider_id: ProviderId,
    ) -> Result<BlockNumber, GetLastTickProviderSubmittedProofError>;

    /// Query the last checkpoint tick.
    async fn query_last_checkpoint_challenge_tick(&self) -> Result<BlockNumber, ApiError>;

    /// Query the checkpoint challenges for a given tick.
    async fn query_last_checkpoint_challenges(
        &self,
        tick: BlockNumber,
    ) -> Result<Vec<(ForestLeaf, Option<TrieRemoveMutation>)>, GetCheckpointChallengesError>;

    /// Query the Merkle Patricia Forest root for a given Provider.
    async fn query_provider_forest_root(
        &self,
        provider_id: ProviderId,
    ) -> Result<H256, GetBspInfoError>;
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

    async fn query_bsp_confirm_chunks_to_prove_for_file(
        &self,
        bsp_id: sp_core::sr25519::Public,
        file_key: H256,
    ) -> Result<Vec<ChunkId>, QueryBspConfirmChunksToProveForFileError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        // Build command to send to blockchain service.
        let message = BlockchainServiceCommand::QueryBspConfirmChunksToProveForFile {
            bsp_id,
            file_key,
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn queue_submit_proof_request(&self, request: SubmitProofRequest) -> Result<()> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QueueSubmitProofRequest { request, callback };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn queue_confirm_bsp_request(&self, request: ConfirmStoringRequest) -> Result<()> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QueueConfirmBspRequest { request, callback };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn query_challenges_from_seed(
        &self,
        seed: RandomnessOutput,
        provider_id: ProviderId,
        count: u32,
    ) -> Result<Vec<ForestLeaf>, ApiError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        // Build command to send to blockchain service.
        let message = BlockchainServiceCommand::QueryChallengesFromSeed {
            seed,
            provider_id,
            count,
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn query_forest_challenges_from_seed(
        &self,
        seed: RandomnessOutput,
        provider_id: ProviderId,
    ) -> Result<Vec<ForestLeaf>, ApiError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QueryForestChallengesFromSeed {
            seed,
            provider_id,
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn query_last_tick_provider_submitted_proof(
        &self,
        provider_id: ProviderId,
    ) -> Result<BlockNumber, GetLastTickProviderSubmittedProofError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QueryLastTickProviderSubmittedProof {
            provider_id,
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn query_last_checkpoint_challenge_tick(&self) -> Result<BlockNumber, ApiError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QueryLastCheckpointChallengeTick { callback };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn query_last_checkpoint_challenges(
        &self,
        tick: BlockNumber,
    ) -> Result<Vec<(ForestLeaf, Option<TrieRemoveMutation>)>, GetCheckpointChallengesError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QueryLastCheckpointChallenges { tick, callback };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn query_provider_forest_root(
        &self,
        provider_id: ProviderId,
    ) -> Result<H256, GetBspInfoError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QueryProviderForestRoot {
            provider_id,
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }
}
