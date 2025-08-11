use anyhow::Result;
use async_trait::async_trait;
use log::{debug, warn};
use sc_network::Multiaddr;
use serde_json::Number;
use shc_common::traits::{KeyTypeOperations, StorageEnableRuntime};
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
    GetBspInfoError, QueryAvailableStorageCapacityError, QueryBucketsOfUserStoredByMspError,
    QueryEarliestChangeCapacityBlockError, QueryMspIdOfBucketIdError,
    QueryProviderMultiaddressesError, QueryStorageProviderCapacityError,
};
use shc_actors_derive::actor_command;
use shc_actors_framework::actor::ActorHandle;
use shc_common::types::{
    BlockNumber, BucketId, ChunkId, CustomChallenge, ForestLeaf, MainStorageProviderId,
    ProofsDealerProviderId, ProviderId, RandomnessOutput, StorageHubEventsVec, StorageProviderId,
    TickNumber,
};
use shc_forest_manager::traits::ForestStorageHandler;
use storage_hub_runtime::{AccountId, Balance, StorageDataUnit};

use crate::{
    capacity_manager::CapacityRequestData,
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
#[actor_command(
    service = BlockchainService<FSH: ForestStorageHandler + Clone + Send + Sync + 'static, Runtime: StorageEnableRuntime>,
    default_mode = "ImmediateResponse",
    default_inner_channel_type = tokio::sync::oneshot::Receiver,
)]
pub enum BlockchainServiceCommand<Runtime: StorageEnableRuntime> {
    #[command(success_type = SubmittedTransaction)]
    SendExtrinsic {
        call: Runtime::Call,
        options: SendExtrinsicOptions,
    },
    #[command(success_type = Extrinsic)]
    GetExtrinsicFromBlock {
        block_hash: H256,
        extrinsic_hash: H256,
    },
    UnwatchExtrinsic {
        subscription_id: Number,
    },
    #[command(success_type = MinimalBlockInfo)]
    GetBestBlockInfo,
    #[command(mode = "AsyncResponse")]
    WaitForBlock {
        block_number: BlockNumber,
    },
    #[command(mode = "AsyncResponse")]
    WaitForNumBlocks {
        number_of_blocks: BlockNumber,
    },
    #[command(mode = "AsyncResponse", error_type = ApiError)]
    WaitForTick {
        tick_number: TickNumber,
    },
    #[command(success_type = bool, error_type = IsStorageRequestOpenToVolunteersError)]
    IsStorageRequestOpenToVolunteers {
        file_key: H256,
    },
    #[command(success_type = BlockNumber, error_type = QueryFileEarliestVolunteerTickError)]
    QueryFileEarliestVolunteerTick {
        bsp_id: ProviderId,
        file_key: H256,
    },
    #[command(success_type = BlockNumber, error_type = QueryEarliestChangeCapacityBlockError)]
    QueryEarliestChangeCapacityBlock {
        bsp_id: ProviderId,
    },
    #[command(success_type = <<Runtime as StorageEnableRuntime>::Signature as KeyTypeOperations>::Public)]
    GetNodePublicKey,
    #[command(success_type = Vec<ChunkId>, error_type = QueryBspConfirmChunksToProveForFileError)]
    QueryBspConfirmChunksToProveForFile {
        bsp_id: ProofsDealerProviderId,
        file_key: H256,
    },
    #[command(success_type = Vec<ChunkId>, error_type = QueryMspConfirmChunksToProveForFileError)]
    QueryMspConfirmChunksToProveForFile {
        msp_id: ProofsDealerProviderId,
        file_key: H256,
    },
    #[command(success_type = Vec<Multiaddr>, error_type = QueryProviderMultiaddressesError)]
    QueryProviderMultiaddresses {
        provider_id: ProviderId,
    },
    QueueSubmitProofRequest {
        request: SubmitProofRequest,
    },
    QueueConfirmBspRequest {
        request: ConfirmStoringRequest,
    },
    QueueMspRespondStorageRequest {
        request: RespondStorageRequest,
    },
    QueueStopStoringForInsolventUserRequest {
        request: StopStoringForInsolventUserRequest,
    },
    #[command(success_type = Vec<ForestLeaf>, error_type = ApiError)]
    QueryChallengesFromSeed {
        seed: RandomnessOutput,
        provider_id: ProofsDealerProviderId,
        count: u32,
    },
    #[command(success_type = Vec<ForestLeaf>, error_type = ApiError)]
    QueryForestChallengesFromSeed {
        seed: RandomnessOutput,
        provider_id: ProofsDealerProviderId,
    },
    #[command(success_type = BlockNumber, error_type = GetProofSubmissionRecordError)]
    QueryLastTickProviderSubmittedProof {
        provider_id: ProofsDealerProviderId,
    },
    #[command(success_type = BlockNumber, error_type = GetChallengePeriodError)]
    QueryChallengePeriod {
        provider_id: ProofsDealerProviderId,
    },
    #[command(success_type = BlockNumber, error_type = GetProofSubmissionRecordError)]
    QueryNextChallengeTickForProvider {
        provider_id: ProofsDealerProviderId,
    },
    #[command(success_type = BlockNumber, error_type = ApiError)]
    QueryLastCheckpointChallengeTick,
    #[command(success_type = Vec<CustomChallenge>, error_type = GetCheckpointChallengesError)]
    QueryLastCheckpointChallenges {
        tick: BlockNumber,
    },
    #[command(success_type = H256, error_type = GetBspInfoError)]
    QueryProviderForestRoot {
        provider_id: ProviderId,
    },
    #[command(success_type = StorageDataUnit, error_type = QueryStorageProviderCapacityError)]
    QueryStorageProviderCapacity {
        provider_id: ProviderId,
    },
    #[command(success_type = StorageDataUnit, error_type = QueryAvailableStorageCapacityError)]
    QueryAvailableStorageCapacity {
        provider_id: ProviderId,
    },
    #[command(success_type = Option<StorageProviderId>)]
    QueryStorageProviderId {
        maybe_node_pub_key:
            Option<<<Runtime as StorageEnableRuntime>::Signature as KeyTypeOperations>::Public>,
    },
    #[command(success_type = Vec<AccountId>, error_type = GetUsersWithDebtOverThresholdError)]
    QueryUsersWithDebt {
        provider_id: ProviderId,
        min_debt: Balance,
    },
    #[command(success_type = Option<Balance>)]
    QueryWorstCaseScenarioSlashableAmount {
        provider_id: ProviderId,
    },
    #[command(success_type = Balance)]
    QuerySlashAmountPerMaxFileSize,
    #[command(success_type = Option<MainStorageProviderId>, error_type = QueryMspIdOfBucketIdError)]
    QueryMspIdOfBucketId {
        bucket_id: BucketId,
    },
    ReleaseForestRootWriteLock {
        forest_root_write_tx: tokio::sync::oneshot::Sender<()>,
    },
    QueueFileDeletionRequest {
        request: FileDeletionRequest,
    },
    #[command(mode = "AsyncResponse")]
    IncreaseCapacity {
        request: CapacityRequestData,
    },
    #[command(success_type = Vec<BucketId>, error_type = QueryBucketsOfUserStoredByMspError)]
    QueryBucketsOfUserStoredByMsp {
        msp_id: ProviderId,
        user: AccountId,
    },
}

/// Interface for interacting with the BlockchainService actor.
#[async_trait]
pub trait BlockchainServiceCommandInterfaceExt<Runtime: StorageEnableRuntime>:
    BlockchainServiceCommandInterface<Runtime>
{
    /// Helper function to check if an extrinsic failed or succeeded in a block.
    fn extrinsic_result(extrinsic: Extrinsic) -> Result<ExtrinsicResult>;

    /// Helper function to submit an extrinsic with a retry strategy. Returns when the extrinsic is
    /// included in a block or when the retry strategy is exhausted.
    async fn submit_extrinsic_with_retry(
        &self,
        call: impl Into<Runtime::Call> + Send,
        options: SendExtrinsicOptions,
        retry_strategy: RetryStrategy,
        with_events: bool,
    ) -> Result<Option<StorageHubEventsVec>>;
}

/// Implement the BlockchainServiceInterface for the ActorHandle<BlockchainService>.
#[async_trait]
impl<FSH, Runtime> BlockchainServiceCommandInterfaceExt<Runtime>
    for ActorHandle<BlockchainService<FSH, Runtime>>
where
    FSH: ForestStorageHandler + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
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
        call: impl Into<Runtime::Call> + Send,
        options: SendExtrinsicOptions,
        retry_strategy: RetryStrategy,
        with_events: bool,
    ) -> Result<Option<StorageHubEventsVec>> {
        let call = call.into();

        // Execute the extrinsic without any tip or specific nonce the first time around.
        let mut tip = retry_strategy.compute_tip(0);
        let mut nonce = None;

        for retry_count in 0..=retry_strategy.max_retries {
            debug!(target: LOG_TARGET, "Submitting transaction {:?} with tip {}", call, tip);

            let extrinsic_options = SendExtrinsicOptions::new(options.timeout())
                .with_tip(tip as u128)
                .with_nonce(nonce);

            let mut transaction = self.send_extrinsic(call.clone(), extrinsic_options).await?;

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
}
