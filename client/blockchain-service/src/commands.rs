use anyhow::Result;
use async_trait::async_trait;
use log::{debug, warn};
use sc_network::Multiaddr;
use serde_json::Number;
use shc_forest_manager::traits::ForestStorageHandler;
use sp_api::ApiError;
use sp_core::H256;

use pallet_file_system_runtime_api::{
    IsStorageRequestOpenToVolunteersError, QueryBspConfirmChunksToProveForFileError,
    QueryFileEarliestVolunteerTickError, QueryMspConfirmChunksToProveForFileError,
};
use pallet_payment_streams_runtime_api::GetUsersWithDebtOverThresholdError;
use pallet_proofs_dealer_runtime_api::{
    GetChallengePeriodError, GetCheckpointChallengesError, GetProofSubmissionRecordError,
};
use pallet_storage_providers_runtime_api::{
    GetBspInfoError, QueryAvailableStorageCapacityError, QueryEarliestChangeCapacityBlockError,
    QueryMspIdOfBucketIdError, QueryProviderMultiaddressesError, QueryStorageProviderCapacityError,
};
use shc_actors_framework::actor::ActorHandle;
use shc_common::types::{
    BlockNumber, BucketId, ChunkId, CustomChallenge, ForestLeaf, MainStorageProviderId,
    ProofsDealerProviderId, ProviderId, RandomnessOutput, StorageHubEventsVec, StorageProviderId,
    TickNumber,
};
use storage_hub_runtime::{AccountId, Balance, StorageDataUnit};

use crate::{
    handler::BlockchainService,
    transaction::SubmittedTransaction,
    types::{
        ConfirmStoringRequest, Extrinsic, ExtrinsicResult, FileDeletionRequest, MinimalBlockInfo,
        RespondStorageRequest, RetryStrategy, SendExtrinsicOptions,
        StopStoringForInsolventUserRequest, SubmitProofRequest, WatchTransactionError,
    },
};

const LOG_TARGET: &str = "blockchain-service-interface";

/// Commands that can be sent to the BlockchainService actor.
pub enum BlockchainServiceCommand {
    SendExtrinsic {
        call: storage_hub_runtime::RuntimeCall,
        options: SendExtrinsicOptions,
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
    GetBestBlockInfo {
        callback: tokio::sync::oneshot::Sender<MinimalBlockInfo>,
    },
    WaitForBlock {
        block_number: BlockNumber,
        callback: tokio::sync::oneshot::Sender<tokio::sync::oneshot::Receiver<()>>,
    },
    WaitForTick {
        tick_number: TickNumber,
        callback:
            tokio::sync::oneshot::Sender<tokio::sync::oneshot::Receiver<Result<(), ApiError>>>,
    },
    IsStorageRequestOpenToVolunteers {
        file_key: H256,
        callback: tokio::sync::oneshot::Sender<Result<bool, IsStorageRequestOpenToVolunteersError>>,
    },
    QueryFileEarliestVolunteerTick {
        bsp_id: ProviderId,
        file_key: H256,
        callback:
            tokio::sync::oneshot::Sender<Result<BlockNumber, QueryFileEarliestVolunteerTickError>>,
    },
    QueryEarliestChangeCapacityBlock {
        bsp_id: ProviderId,
        callback: tokio::sync::oneshot::Sender<
            Result<BlockNumber, QueryEarliestChangeCapacityBlockError>,
        >,
    },
    GetNodePublicKey {
        callback: tokio::sync::oneshot::Sender<sp_core::sr25519::Public>,
    },
    QueryBspConfirmChunksToProveForFile {
        bsp_id: ProofsDealerProviderId,
        file_key: H256,
        callback: tokio::sync::oneshot::Sender<
            Result<Vec<ChunkId>, QueryBspConfirmChunksToProveForFileError>,
        >,
    },
    QueryMspConfirmChunksToProveForFile {
        msp_id: ProofsDealerProviderId,
        file_key: H256,
        callback: tokio::sync::oneshot::Sender<
            Result<Vec<ChunkId>, QueryMspConfirmChunksToProveForFileError>,
        >,
    },
    QueryProviderMultiaddresses {
        provider_id: ProviderId,
        callback:
            tokio::sync::oneshot::Sender<Result<Vec<Multiaddr>, QueryProviderMultiaddressesError>>,
    },
    QueueSubmitProofRequest {
        request: SubmitProofRequest,
        callback: tokio::sync::oneshot::Sender<Result<()>>,
    },
    QueueConfirmBspRequest {
        request: ConfirmStoringRequest,
        callback: tokio::sync::oneshot::Sender<Result<()>>,
    },
    QueueMspRespondStorageRequest {
        request: RespondStorageRequest,
        callback: tokio::sync::oneshot::Sender<Result<()>>,
    },
    QueueStopStoringForInsolventUserRequest {
        request: StopStoringForInsolventUserRequest,
        callback: tokio::sync::oneshot::Sender<Result<()>>,
    },
    QueryChallengesFromSeed {
        seed: RandomnessOutput,
        provider_id: ProofsDealerProviderId,
        count: u32,
        callback: tokio::sync::oneshot::Sender<Result<Vec<ForestLeaf>, ApiError>>,
    },
    QueryForestChallengesFromSeed {
        seed: RandomnessOutput,
        provider_id: ProofsDealerProviderId,
        callback: tokio::sync::oneshot::Sender<Result<Vec<ForestLeaf>, ApiError>>,
    },
    QueryLastTickProviderSubmittedProof {
        provider_id: ProofsDealerProviderId,
        callback: tokio::sync::oneshot::Sender<Result<BlockNumber, GetProofSubmissionRecordError>>,
    },
    QueryChallengePeriod {
        provider_id: ProofsDealerProviderId,
        callback: tokio::sync::oneshot::Sender<Result<BlockNumber, GetChallengePeriodError>>,
    },
    QueryNextChallengeTickForProvider {
        provider_id: ProofsDealerProviderId,
        callback: tokio::sync::oneshot::Sender<Result<BlockNumber, GetProofSubmissionRecordError>>,
    },
    QueryLastCheckpointChallengeTick {
        callback: tokio::sync::oneshot::Sender<Result<BlockNumber, ApiError>>,
    },
    QueryLastCheckpointChallenges {
        tick: BlockNumber,
        callback: tokio::sync::oneshot::Sender<
            Result<Vec<CustomChallenge>, GetCheckpointChallengesError>,
        >,
    },
    QueryProviderForestRoot {
        provider_id: ProviderId,
        callback: tokio::sync::oneshot::Sender<Result<H256, GetBspInfoError>>,
    },
    QueryStorageProviderCapacity {
        provider_id: ProviderId,
        callback: tokio::sync::oneshot::Sender<
            Result<StorageDataUnit, QueryStorageProviderCapacityError>,
        >,
    },
    QueryAvailableStorageCapacity {
        provider_id: ProviderId,
        callback: tokio::sync::oneshot::Sender<
            Result<StorageDataUnit, QueryAvailableStorageCapacityError>,
        >,
    },
    QueryStorageProviderId {
        maybe_node_pub_key: Option<sp_core::sr25519::Public>,
        callback: tokio::sync::oneshot::Sender<Result<Option<StorageProviderId>>>,
    },
    QueryUsersWithDebt {
        provider_id: ProviderId,
        min_debt: Balance,
        callback: tokio::sync::oneshot::Sender<
            Result<Vec<AccountId>, GetUsersWithDebtOverThresholdError>,
        >,
    },
    QueryWorstCaseScenarioSlashableAmount {
        provider_id: ProviderId,
        callback: tokio::sync::oneshot::Sender<Result<Option<Balance>>>,
    },
    QuerySlashAmountPerMaxFileSize {
        callback: tokio::sync::oneshot::Sender<Result<Balance>>,
    },
    QueryMspIdOfBucketId {
        bucket_id: BucketId,
        callback: tokio::sync::oneshot::Sender<
            Result<Option<MainStorageProviderId>, QueryMspIdOfBucketIdError>,
        >,
    },
    ReleaseForestRootWriteLock {
        forest_root_write_tx: tokio::sync::oneshot::Sender<()>,
        callback: tokio::sync::oneshot::Sender<Result<()>>,
    },
    QueueFileDeletionRequest {
        request: FileDeletionRequest,
        callback: tokio::sync::oneshot::Sender<Result<()>>,
    },
}

/// Interface for interacting with the BlockchainService actor.
#[async_trait]
pub trait BlockchainServiceInterface {
    /// Send an extrinsic to the runtime.
    async fn send_extrinsic(
        &self,
        call: impl Into<storage_hub_runtime::RuntimeCall> + Send,
        options: SendExtrinsicOptions,
    ) -> Result<SubmittedTransaction>;

    /// Get an extrinsic from a block.
    async fn get_extrinsic_from_block(
        &self,
        block_hash: H256,
        extrinsic_hash: H256,
    ) -> Result<Extrinsic>;

    /// Unwatch an extrinsic.
    async fn unwatch_extrinsic(&self, subscription_id: Number) -> Result<()>;

    /// Wait for a block number.
    async fn wait_for_block(&self, block_number: BlockNumber) -> Result<()>;

    /// Wait for a tick number.
    async fn wait_for_tick(&self, tick_number: TickNumber) -> Result<(), ApiError>;

    /// Determine if a storage request is still open to volunteers.
    async fn is_storage_request_open_to_volunteers(
        &self,
        file_key: H256,
    ) -> Result<bool, IsStorageRequestOpenToVolunteersError>;

    /// Query the earliest tick number that a file was volunteered for storage.
    async fn query_file_earliest_volunteer_tick(
        &self,
        bsp_id: ProofsDealerProviderId,
        file_key: H256,
    ) -> Result<BlockNumber, QueryFileEarliestVolunteerTickError>;

    async fn query_earliest_change_capacity_block(
        &self,
        bsp_id: ProviderId,
    ) -> Result<BlockNumber, QueryEarliestChangeCapacityBlockError>;

    /// Get the node's public key.
    async fn get_node_public_key(&self) -> sp_core::sr25519::Public;

    /// Query the chunks that a BSP needs to confirm for a file.
    async fn query_bsp_confirm_chunks_to_prove_for_file(
        &self,
        bsp_id: ProofsDealerProviderId,
        file_key: H256,
    ) -> Result<Vec<ChunkId>, QueryBspConfirmChunksToProveForFileError>;

    /// Query the chunks that a MSP needs to confirm for a file.
    async fn query_msp_confirm_chunks_to_prove_for_file(
        &self,
        msp_id: ProofsDealerProviderId,
        file_key: H256,
    ) -> Result<Vec<ChunkId>, QueryMspConfirmChunksToProveForFileError>;

    /// Query the a Provider's multiaddresses.
    async fn query_provider_multiaddresses(
        &self,
        provider_id: ProviderId,
    ) -> Result<Vec<Multiaddr>, QueryProviderMultiaddressesError>;

    /// Queue a SubmitProofRequest to be processed.
    async fn queue_submit_proof_request(&self, request: SubmitProofRequest) -> Result<()>;

    /// Queue a ConfirmBspRequest to be processed.
    async fn queue_confirm_bsp_request(&self, request: ConfirmStoringRequest) -> Result<()>;

    // Queue a BspStopStoringForInsolventUserRequest to be processed.
    async fn queue_stop_storing_for_insolvent_user_request(
        &self,
        request: StopStoringForInsolventUserRequest,
    ) -> Result<()>;

    /// Queue a RespondStoringRequest to be processed.
    async fn queue_msp_respond_storage_request(&self, request: RespondStorageRequest)
        -> Result<()>;

    /// Queue a FileDeletionRequest to be processed.
    async fn queue_file_deletion_request(&self, request: FileDeletionRequest) -> Result<()>;

    /// Query the challenges that a Provider needs to submit for a given seed.
    async fn query_challenges_from_seed(
        &self,
        seed: RandomnessOutput,
        provider_id: ProofsDealerProviderId,
        count: u32,
    ) -> Result<Vec<ForestLeaf>, ApiError>;

    /// Query the forest challenges that a Provider needs to submit for a given seed.
    /// This is the same as the `query_challenges_from_seed` method, but it does not
    /// require specifying the `count`, as the runtime will know how many challenges
    /// to generate.
    async fn query_forest_challenges_from_seed(
        &self,
        seed: RandomnessOutput,
        provider_id: ProofsDealerProviderId,
    ) -> Result<Vec<ForestLeaf>, ApiError>;

    /// Query the last tick that a Provider submitted a proof for.
    async fn query_last_tick_provider_submitted_proof(
        &self,
        provider_id: ProofsDealerProviderId,
    ) -> Result<BlockNumber, GetProofSubmissionRecordError>;

    /// Query the challenge period for a given Provider.
    async fn query_challenge_period(
        &self,
        provider_id: ProofsDealerProviderId,
    ) -> Result<BlockNumber, GetChallengePeriodError>;

    /// Query the next challenge tick for a given Provider.
    async fn get_next_challenge_tick_for_provider(
        &self,
        provider_id: ProofsDealerProviderId,
    ) -> Result<BlockNumber, GetProofSubmissionRecordError>;

    /// Query the last checkpoint tick.
    async fn query_last_checkpoint_challenge_tick(&self) -> Result<BlockNumber, ApiError>;

    /// Query the checkpoint challenges for a given tick.
    async fn query_last_checkpoint_challenges(
        &self,
        tick: BlockNumber,
    ) -> Result<Vec<CustomChallenge>, GetCheckpointChallengesError>;

    /// Query the Merkle Patricia Forest root for a given Provider.
    async fn query_provider_forest_root(
        &self,
        provider_id: ProviderId,
    ) -> Result<H256, GetBspInfoError>;

    /// Query the storage capacity for a Provider.
    async fn query_storage_provider_capacity(
        &self,
        provider_id: ProviderId,
    ) -> Result<StorageDataUnit, QueryStorageProviderCapacityError>;

    /// Query the available storage capacity for a Provider.
    async fn query_available_storage_capacity(
        &self,
        provider_id: ProviderId,
    ) -> Result<StorageDataUnit, QueryAvailableStorageCapacityError>;

    /// Query the ProviderId for a given account. If no account is provided, the node's account is
    /// used.
    async fn query_storage_provider_id(
        &self,
        maybe_node_pub_key: Option<sp_core::sr25519::Public>,
    ) -> Result<Option<StorageProviderId>>;

    async fn query_users_with_debt(
        &self,
        provider_id: ProviderId,
        min_debt: Balance,
    ) -> Result<Vec<AccountId>, GetUsersWithDebtOverThresholdError>;

    async fn query_worst_case_scenario_slashable_amount(
        &self,
        provider_id: ProviderId,
    ) -> Result<Option<Balance>>;

    async fn query_slash_amount_per_max_file_size(&self) -> Result<Balance>;

    /// Helper function to check if an extrinsic failed or succeeded in a block.
    fn extrinsic_result(extrinsic: Extrinsic) -> Result<ExtrinsicResult>;

    /// Helper function to submit an extrinsic with a retry strategy. Returns when the extrinsic is
    /// included in a block or when the retry strategy is exhausted.
    async fn submit_extrinsic_with_retry(
        &self,
        call: impl Into<storage_hub_runtime::RuntimeCall> + Send,
        retry_strategy: RetryStrategy,
        with_events: bool,
    ) -> Result<Option<StorageHubEventsVec>>;

    /// Helper function to get the MSP ID of a bucket ID.
    async fn query_msp_id_of_bucket_id(
        &self,
        bucket_id: BucketId,
    ) -> Result<Option<MainStorageProviderId>, QueryMspIdOfBucketIdError>;

    /// Helper function to release the Forest root write lock.
    async fn release_forest_root_write_lock(
        &self,
        forest_root_write_tx: tokio::sync::oneshot::Sender<()>,
    ) -> Result<()>;
}

/// Implement the BlockchainServiceInterface for the ActorHandle<BlockchainService>.
#[async_trait]
impl<FSH> BlockchainServiceInterface for ActorHandle<BlockchainService<FSH>>
where
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
{
    async fn send_extrinsic(
        &self,
        call: impl Into<storage_hub_runtime::RuntimeCall> + Send,
        options: SendExtrinsicOptions,
    ) -> Result<SubmittedTransaction> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        // Build command to send to blockchain service.
        let message = BlockchainServiceCommand::SendExtrinsic {
            call: call.into(),
            options,
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

    async fn wait_for_tick(&self, tick_number: TickNumber) -> Result<(), ApiError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        // Build command to send to blockchain service.
        let message = BlockchainServiceCommand::WaitForTick {
            tick_number,
            callback,
        };
        self.send(message).await;
        let rx = rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.");
        rx.await.expect("Failed to wait for tick")
    }

    async fn is_storage_request_open_to_volunteers(
        &self,
        file_key: H256,
    ) -> Result<bool, IsStorageRequestOpenToVolunteersError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        // Build command to send to blockchain service.
        let message =
            BlockchainServiceCommand::IsStorageRequestOpenToVolunteers { file_key, callback };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn query_file_earliest_volunteer_tick(
        &self,
        bsp_id: ProviderId,
        file_key: H256,
    ) -> Result<BlockNumber, QueryFileEarliestVolunteerTickError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        // Build command to send to blockchain service.
        let message = BlockchainServiceCommand::QueryFileEarliestVolunteerTick {
            bsp_id,
            file_key,
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn query_earliest_change_capacity_block(
        &self,
        bsp_id: ProviderId,
    ) -> Result<BlockNumber, QueryEarliestChangeCapacityBlockError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message =
            BlockchainServiceCommand::QueryEarliestChangeCapacityBlock { bsp_id, callback };
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
        bsp_id: ProofsDealerProviderId,
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

    async fn query_msp_confirm_chunks_to_prove_for_file(
        &self,
        msp_id: ProofsDealerProviderId,
        file_key: H256,
    ) -> Result<Vec<ChunkId>, QueryMspConfirmChunksToProveForFileError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        // Build command to send to blockchain service.
        let message = BlockchainServiceCommand::QueryMspConfirmChunksToProveForFile {
            msp_id,
            file_key,
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn query_provider_multiaddresses(
        &self,
        provider_id: ProviderId,
    ) -> Result<Vec<Multiaddr>, QueryProviderMultiaddressesError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QueryProviderMultiaddresses {
            provider_id,
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

    async fn queue_msp_respond_storage_request(
        &self,
        request: RespondStorageRequest,
    ) -> Result<()> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QueueMspRespondStorageRequest { request, callback };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn queue_file_deletion_request(&self, request: FileDeletionRequest) -> Result<()> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QueueFileDeletionRequest { request, callback };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn queue_stop_storing_for_insolvent_user_request(
        &self,
        request: StopStoringForInsolventUserRequest,
    ) -> Result<()> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message =
            BlockchainServiceCommand::QueueStopStoringForInsolventUserRequest { request, callback };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn query_challenges_from_seed(
        &self,
        seed: RandomnessOutput,
        provider_id: ProofsDealerProviderId,
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
        provider_id: ProofsDealerProviderId,
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
        provider_id: ProofsDealerProviderId,
    ) -> Result<BlockNumber, GetProofSubmissionRecordError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QueryLastTickProviderSubmittedProof {
            provider_id,
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn query_challenge_period(
        &self,
        provider_id: ProofsDealerProviderId,
    ) -> Result<BlockNumber, GetChallengePeriodError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QueryChallengePeriod {
            provider_id,
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn get_next_challenge_tick_for_provider(
        &self,
        provider_id: ProofsDealerProviderId,
    ) -> Result<BlockNumber, GetProofSubmissionRecordError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QueryNextChallengeTickForProvider {
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
    ) -> Result<Vec<CustomChallenge>, GetCheckpointChallengesError> {
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

    async fn query_storage_provider_id(
        &self,
        maybe_node_pub_key: Option<sp_core::sr25519::Public>,
    ) -> Result<Option<StorageProviderId>> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QueryStorageProviderId {
            maybe_node_pub_key,
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn query_storage_provider_capacity(
        &self,
        provider_id: ProviderId,
    ) -> Result<StorageDataUnit, QueryStorageProviderCapacityError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QueryStorageProviderCapacity {
            provider_id,
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn query_available_storage_capacity(
        &self,
        provider_id: ProviderId,
    ) -> Result<StorageDataUnit, QueryAvailableStorageCapacityError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QueryAvailableStorageCapacity {
            provider_id,
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn query_users_with_debt(
        &self,
        provider_id: ProviderId,
        min_debt: Balance,
    ) -> Result<Vec<AccountId>, GetUsersWithDebtOverThresholdError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QueryUsersWithDebt {
            provider_id,
            min_debt,
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn query_worst_case_scenario_slashable_amount(
        &self,
        provider_id: ProviderId,
    ) -> Result<Option<Balance>> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QueryWorstCaseScenarioSlashableAmount {
            provider_id,
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn query_slash_amount_per_max_file_size(&self) -> Result<Balance> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QuerySlashAmountPerMaxFileSize { callback };
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

    async fn submit_extrinsic_with_retry(
        &self,
        call: impl Into<storage_hub_runtime::RuntimeCall> + Send,
        retry_strategy: RetryStrategy,
        with_events: bool,
    ) -> Result<Option<StorageHubEventsVec>> {
        let call = call.into();

        // Execute the extrinsic without any tip or specific nonce the first time around.
        let mut tip = retry_strategy.compute_tip(0);
        let mut nonce = None;

        for retry_count in 0..=retry_strategy.max_retries {
            debug!(target: LOG_TARGET, "Submitting transaction {:?} with tip {}", call, tip);

            let extrinsic_options = SendExtrinsicOptions::new()
                .with_tip(tip as u128)
                .with_nonce(nonce);

            let mut transaction = self
                .send_extrinsic(call.clone(), extrinsic_options)
                .await?
                .with_timeout(retry_strategy.timeout);

            let result: Result<Option<StorageHubEventsVec>, _> = if with_events {
                transaction
                    .watch_for_success_with_events(&self)
                    .await
                    .map(Some)
            } else {
                transaction.watch_for_success(&self).await.map(|_| None)
            };

            match result {
                Ok(maybe_events) => {
                    debug!(target: LOG_TARGET, "Transaction with hash {:?} succeeded", transaction.hash());
                    return Ok(maybe_events);
                }
                Err(err) => {
                    warn!(target: LOG_TARGET, "Transaction failed: {:?}", err);

                    if let Some(ref should_retry) = retry_strategy.should_retry {
                        if !should_retry(err.clone()).await {
                            return Err(anyhow::anyhow!("Exhausted retry strategy"));
                        }
                    }

                    warn!(target: LOG_TARGET, "Failed to submit transaction with hash {:?}, attempt #{}", transaction.hash(), retry_count + 1);

                    // TODO: Add pending transaction pool implementation to be able to resubmit transactions with nonces lower than the current one to avoid this transaction from being stuck.
                    if let WatchTransactionError::Timeout = err {
                        // Increase the tip to incentivise the collators to include the transaction in a block with priority
                        tip = retry_strategy.compute_tip(retry_count + 1);
                        // Reuse the same nonce since the transaction was not included in a block.
                        nonce = Some(transaction.nonce());

                        // Log warning if this is not the last retry.
                        if retry_count < retry_strategy.max_retries {
                            warn!(target: LOG_TARGET, "Retrying with increased tip {} and nonce {}", tip, transaction.nonce());
                        }
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Exhausted retry strategy"))
    }

    async fn query_msp_id_of_bucket_id(
        &self,
        bucket_id: BucketId,
    ) -> Result<Option<MainStorageProviderId>, QueryMspIdOfBucketIdError> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::QueryMspIdOfBucketId {
            bucket_id,
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }

    async fn release_forest_root_write_lock(
        &self,
        forest_root_write_tx: tokio::sync::oneshot::Sender<()>,
    ) -> Result<()> {
        let (callback, rx) = tokio::sync::oneshot::channel();
        let message = BlockchainServiceCommand::ReleaseForestRootWriteLock {
            forest_root_write_tx,
            callback,
        };
        self.send(message).await;
        rx.await.expect("Failed to receive response from BlockchainService. Probably means BlockchainService has crashed.")
    }
}
