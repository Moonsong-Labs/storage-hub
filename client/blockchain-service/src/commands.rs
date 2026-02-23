use anyhow::Result;
use async_trait::async_trait;
use log::{debug, warn};
use sc_network::Multiaddr;
use shc_common::{
    blockchain_utils::decode_module_error,
    traits::{KeyTypeOperations, StorageEnableRuntime},
    types::StorageEnableEvents,
};
use sp_api::ApiError;
use sp_runtime::DispatchError;

use pallet_file_system_runtime_api::{
    IsStorageRequestOpenToVolunteersError, QueryBspConfirmChunksToProveForFileError,
    QueryBspsVolunteeredForFileError, QueryFileEarliestVolunteerTickError,
    QueryMspConfirmChunksToProveForFileError,
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
    AccountId, BackupStorageProviderId, Balance, BlockNumber, BucketId, ChunkId, CustomChallenge,
    FileKey, ForestLeaf, MainStorageProviderId, ProofsDealerProviderId, ProviderId,
    RandomnessOutput, StorageDataUnit, StorageHubEventsVec, StorageProviderId, TickNumber,
};
use shc_forest_manager::traits::ForestStorageHandler;

use crate::{
    capacity_manager::CapacityRequestData,
    events::NewStorageRequest,
    handler::BlockchainService,
    transaction_manager::wait_for_transaction_status,
    types::{
        ConfirmStoringRequest, Extrinsic, ExtrinsicResult, FileDeletionRequest,
        FileKeyStatusUpdate, MinimalBlockInfo, RespondStorageRequest, RetryStrategy,
        SendExtrinsicOptions, StatusToWait, StopStoringForInsolventUserRequest, SubmitProofRequest,
        SubmittedExtrinsicInfo, WatchTransactionError,
    },
};

const LOG_TARGET: &str = "blockchain-service-interface";

/// Commands that can be sent to the BlockchainService actor.
#[actor_command(
    service = BlockchainService<FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static, Runtime: StorageEnableRuntime>,
    default_mode = "ImmediateResponse",
    default_inner_channel_type = tokio::sync::oneshot::Receiver,
)]
pub enum BlockchainServiceCommand<Runtime: StorageEnableRuntime> {
    #[command(success_type = SubmittedExtrinsicInfo<Runtime>)]
    SendExtrinsic {
        call: Runtime::Call,
        options: SendExtrinsicOptions,
    },
    #[command(success_type = Extrinsic<Runtime>)]
    GetExtrinsicFromBlock {
        block_hash: Runtime::Hash,
        extrinsic_hash: Runtime::Hash,
    },
    #[command(success_type = MinimalBlockInfo<Runtime>)]
    GetBestBlockInfo,
    #[command(mode = "AsyncResponse")]
    WaitForBlock { block_number: BlockNumber<Runtime> },
    #[command(mode = "AsyncResponse")]
    WaitForNumBlocks {
        number_of_blocks: BlockNumber<Runtime>,
    },
    #[command(mode = "AsyncResponse", error_type = ApiError)]
    WaitForTick { tick_number: TickNumber<Runtime> },
    #[command(success_type = bool, error_type = IsStorageRequestOpenToVolunteersError)]
    IsStorageRequestOpenToVolunteers { file_key: Runtime::Hash },
    #[command(success_type = BlockNumber<Runtime>, error_type = QueryFileEarliestVolunteerTickError)]
    QueryFileEarliestVolunteerTick {
        bsp_id: ProviderId<Runtime>,
        file_key: Runtime::Hash,
    },
    #[command(success_type = BlockNumber<Runtime>, error_type = QueryEarliestChangeCapacityBlockError)]
    QueryEarliestChangeCapacityBlock { bsp_id: ProviderId<Runtime> },
    #[command(success_type = <<Runtime as StorageEnableRuntime>::Signature as KeyTypeOperations>::Public)]
    GetNodePublicKey,
    #[command(success_type = Vec<ChunkId>, error_type = QueryBspConfirmChunksToProveForFileError)]
    QueryBspConfirmChunksToProveForFile {
        bsp_id: ProofsDealerProviderId<Runtime>,
        file_key: Runtime::Hash,
    },
    #[command(success_type = Vec<ChunkId>, error_type = QueryMspConfirmChunksToProveForFileError)]
    QueryMspConfirmChunksToProveForFile {
        msp_id: ProofsDealerProviderId<Runtime>,
        file_key: Runtime::Hash,
    },
    #[command(success_type = bool, error_type = QueryBspsVolunteeredForFileError)]
    QueryBspVolunteeredForFile {
        bsp_id: BackupStorageProviderId<Runtime>,
        file_key: Runtime::Hash,
    },
    #[command(success_type = Vec<Multiaddr>, error_type = QueryProviderMultiaddressesError)]
    QueryProviderMultiaddresses { provider_id: ProviderId<Runtime> },
    QueueSubmitProofRequest {
        request: SubmitProofRequest<Runtime>,
    },
    QueueConfirmBspRequest {
        request: ConfirmStoringRequest<Runtime>,
    },
    #[command(mode = "FireAndForget")]
    QueueMspRespondStorageRequest {
        request: RespondStorageRequest<Runtime>,
    },
    QueueStopStoringForInsolventUserRequest {
        request: StopStoringForInsolventUserRequest<Runtime>,
    },
    #[command(success_type = Vec<ForestLeaf<Runtime>>, error_type = ApiError)]
    QueryChallengesFromSeed {
        seed: RandomnessOutput<Runtime>,
        provider_id: ProofsDealerProviderId<Runtime>,
        count: u32,
    },
    #[command(success_type = Vec<ForestLeaf<Runtime>>, error_type = ApiError)]
    QueryForestChallengesFromSeed {
        seed: RandomnessOutput<Runtime>,
        provider_id: ProofsDealerProviderId<Runtime>,
    },
    #[command(success_type = BlockNumber<Runtime>, error_type = GetProofSubmissionRecordError)]
    QueryLastTickProviderSubmittedProof {
        provider_id: ProofsDealerProviderId<Runtime>,
    },
    #[command(success_type = BlockNumber<Runtime>, error_type = GetChallengePeriodError)]
    QueryChallengePeriod {
        provider_id: ProofsDealerProviderId<Runtime>,
    },
    #[command(success_type = BlockNumber<Runtime>, error_type = GetProofSubmissionRecordError)]
    QueryNextChallengeTickForProvider {
        provider_id: ProofsDealerProviderId<Runtime>,
    },
    #[command(success_type = BlockNumber<Runtime>, error_type = ApiError)]
    QueryLastCheckpointChallengeTick,
    #[command(success_type = Vec<CustomChallenge<Runtime>>, error_type = GetCheckpointChallengesError)]
    QueryLastCheckpointChallenges { tick: BlockNumber<Runtime> },
    #[command(success_type = Runtime::Hash, error_type = GetBspInfoError)]
    QueryProviderForestRoot { provider_id: ProviderId<Runtime> },
    #[command(success_type = StorageDataUnit<Runtime>, error_type = QueryStorageProviderCapacityError)]
    QueryStorageProviderCapacity { provider_id: ProviderId<Runtime> },
    #[command(success_type = StorageDataUnit<Runtime>, error_type = QueryAvailableStorageCapacityError)]
    QueryAvailableStorageCapacity { provider_id: ProviderId<Runtime> },
    #[command(success_type = Option<StorageProviderId<Runtime>>)]
    QueryStorageProviderId {
        maybe_node_pub_key:
            Option<<<Runtime as StorageEnableRuntime>::Signature as KeyTypeOperations>::Public>,
    },
    #[command(success_type = Vec<AccountId<Runtime>>, error_type = GetUsersWithDebtOverThresholdError)]
    QueryUsersWithDebt {
        provider_id: ProviderId<Runtime>,
        min_debt: Balance<Runtime>,
    },
    #[command(success_type = Option<Balance<Runtime>>)]
    QueryWorstCaseScenarioSlashableAmount { provider_id: ProviderId<Runtime> },
    #[command(success_type = Balance<Runtime>)]
    QuerySlashAmountPerMaxFileSize,
    #[command(success_type = Option<MainStorageProviderId<Runtime>>, error_type = QueryMspIdOfBucketIdError)]
    QueryMspIdOfBucketId { bucket_id: BucketId<Runtime> },
    QueueFileDeletionRequest {
        request: FileDeletionRequest<Runtime>,
    },
    #[command(mode = "AsyncResponse")]
    IncreaseCapacity {
        request: CapacityRequestData<Runtime>,
    },
    #[command(success_type = Vec<BucketId<Runtime>>, error_type = QueryBucketsOfUserStoredByMspError)]
    QueryBucketsOfUserStoredByMsp {
        msp_id: ProviderId<Runtime>,
        user: AccountId<Runtime>,
    },
    #[command(success_type = ())]
    RegisterBspDistributing {
        file_key: FileKey,
        bsp_id: BackupStorageProviderId<Runtime>,
    },
    #[command(success_type = ())]
    UnregisterBspDistributing {
        file_key: FileKey,
        bsp_id: BackupStorageProviderId<Runtime>,
    },
    /// Query pending storage requests for the MSP.
    /// If `file_keys` is provided, only query those specific storage requests from storage.
    /// If `file_keys` is None, returns all pending storage requests via runtime API.
    #[command(success_type = Vec<NewStorageRequest<Runtime>>)]
    QueryPendingStorageRequests {
        maybe_file_keys: Option<Vec<FileKey>>,
    },
    /// Query the maximum amount of storage requests that can be confirmed by a BSP in one batch.
    #[command(success_type = u32)]
    QueryMaxBatchConfirmStorageRequests,
    /// Query the maximum number of file keys an MSP can respond to per batch call.
    #[command(success_type = u32)]
    QueryMaxMspRespondFileKeys,
    /// Add a file key to pending volunteer tracking.
    ///
    /// Called before submitting a volunteer tx to track that the volunteer is pending.
    #[command(mode = "FireAndForget")]
    AddPendingVolunteerFileKey { file_key: FileKey },
    /// Remove a file key from pending volunteer tracking.
    ///
    /// Called after volunteer is verified on-chain or fails.
    #[command(mode = "FireAndForget")]
    RemovePendingVolunteerFileKey { file_key: FileKey },
    /// Set the terminal status of a file key in the MSP upload pipeline.
    ///
    /// Used by tasks to update the status of a file key after processing.
    /// Only terminal statuses are allowed—`Processing` is set exclusively by the
    /// blockchain service when emitting [`NewStorageRequest`] events.
    ///
    /// See [`FileKeyStatusUpdate`] for available statuses.
    #[command(mode = "FireAndForget")]
    SetFileKeyStatus {
        file_key: FileKey,
        status: FileKeyStatusUpdate,
    },
    /// Remove a file key from the status tracking.
    ///
    /// Used by tasks to enable retry on the next block cycle:
    /// - After proof errors (to retry with regenerated proofs)
    /// - After extrinsic submission failures (may be transient)
    #[command(mode = "FireAndForget")]
    RemoveFileKeyStatus { file_key: FileKey },
    /// Pop up to N storage requests pending confirmation from the pending queue.
    ///
    /// Returns the items without filtering so the task is responsible for filtering stale requests.
    /// Items are removed from the queue so the task is responsible for re-queuing them if needed.
    #[command(success_type = Vec<ConfirmStoringRequest<Runtime>>)]
    PopConfirmStoringRequests { count: u32 },
    /// Filter confirm storing requests without re-queuing pending volunteer requests.
    ///
    /// Takes a list of confirm storing requests and returns two lists:
    /// - First: Requests ready for confirmation (BSP has volunteered but not yet confirmed)
    /// - Second: Requests with pending volunteer transactions (not yet on-chain)
    ///
    /// This command does NOT re-queue pending volunteer requests.
    #[command(success_type = (Vec<ConfirmStoringRequest<Runtime>>, Vec<ConfirmStoringRequest<Runtime>>))]
    FilterConfirmStoringRequests {
        requests: Vec<ConfirmStoringRequest<Runtime>>,
    },
}

/// Interface for interacting with the BlockchainService actor.
#[async_trait]
pub trait BlockchainServiceCommandInterfaceExt<Runtime: StorageEnableRuntime>:
    BlockchainServiceCommandInterface<Runtime>
{
    /// Helper function to check if an extrinsic failed or succeeded in a block.
    fn extrinsic_result(extrinsic: Extrinsic<Runtime>) -> Result<ExtrinsicResult>;

    /// Helper function to submit an extrinsic with a retry strategy. Returns when the extrinsic is
    /// included in a block or when the retry strategy is exhausted.
    async fn submit_extrinsic_with_retry(
        &self,
        call: impl Into<Runtime::Call> + Send,
        options: SendExtrinsicOptions,
        retry_strategy: RetryStrategy,
        with_events: bool,
    ) -> Result<Option<StorageHubEventsVec<Runtime>>>;
}

/// Implement the BlockchainServiceInterface for the ActorHandle<BlockchainService>.
#[async_trait]
impl<FSH, Runtime> BlockchainServiceCommandInterfaceExt<Runtime>
    for ActorHandle<BlockchainService<FSH, Runtime>>
where
    FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    fn extrinsic_result(extrinsic: Extrinsic<Runtime>) -> Result<ExtrinsicResult> {
        for ev in extrinsic.events {
            match ev.event.into() {
                StorageEnableEvents::System(frame_system::Event::ExtrinsicFailed {
                    dispatch_error,
                    dispatch_info,
                }) => {
                    return Ok(ExtrinsicResult::Failure {
                        dispatch_info,
                        dispatch_error,
                    });
                }
                StorageEnableEvents::System(frame_system::Event::ExtrinsicSuccess {
                    dispatch_info,
                }) => {
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
    ) -> Result<Option<StorageHubEventsVec<Runtime>>> {
        let call = call.into();

        // Execute the extrinsic without any tip or specific nonce the first time around.
        let mut tip = retry_strategy.compute_tip(0);
        let mut nonce = None;

        for retry_count in 0..=retry_strategy.max_retries {
            debug!(target: LOG_TARGET, "Submitting transaction {:?} with tip {}", call, tip);

            let extrinsic_options =
                SendExtrinsicOptions::new(options.timeout(), options.module(), options.method())
                    .with_tip(tip as u128)
                    .with_nonce(nonce);

            let submitted_ext_info = self.send_extrinsic(call.clone(), extrinsic_options).await?;

            // Wait for transaction to be included in a block
            let result = wait_for_transaction_status(
                submitted_ext_info.nonce,
                submitted_ext_info.status_subscription.clone(),
                StatusToWait::InBlock,
                options.timeout(),
            )
            .await;

            match result {
                Ok(block_hash) => {
                    // Always fetch the extrinsic to check if it succeeded or failed on-chain
                    let extrinsic = self
                        .get_extrinsic_from_block(block_hash, submitted_ext_info.hash)
                        .await?;

                    // Check the extrinsic result and log appropriately
                    match Self::extrinsic_result(extrinsic.clone()) {
                        Ok(ExtrinsicResult::Success { .. }) => {
                            debug!(
                                target: LOG_TARGET,
                                "Transaction {:?} succeeded in block {:?}",
                                call,
                                block_hash
                            );
                        }
                        Ok(ExtrinsicResult::Failure { dispatch_error, .. }) => {
                            // Try to decode module errors to get human-readable error names
                            let error_description = match &dispatch_error {
                                DispatchError::Module(module_error) => {
                                    match decode_module_error::<Runtime>(module_error.clone()) {
                                        Ok(decoded) => format!("{:?}", decoded),
                                        Err(_) => format!("{:?}", dispatch_error),
                                    }
                                }
                                _ => format!("{:?}", dispatch_error),
                            };
                            warn!(
                                target: LOG_TARGET,
                                "Transaction {:?} failed on-chain with dispatch error: {}",
                                call,
                                error_description
                            );
                        }
                        Err(e) => {
                            warn!(
                                target: LOG_TARGET,
                                "Transaction {:?} included in block but could not determine result: {:?}",
                                call,
                                e
                            );
                        }
                    }

                    if with_events {
                        return Ok(Some(extrinsic.events));
                    } else {
                        return Ok(None);
                    }
                }
                Err(err) => {
                    warn!(
                        target: LOG_TARGET,
                        "Transaction {:?} (hash: {:?}) failed on attempt {}/{}: {:?}",
                        call,
                        submitted_ext_info.hash,
                        retry_count + 1,
                        retry_strategy.max_retries + 1,
                        err
                    );

                    if let Some(ref should_retry) = retry_strategy.should_retry {
                        if !should_retry(err.clone()).await {
                            return Err(anyhow::anyhow!("Exhausted retry strategy"));
                        }
                    }

                    if let WatchTransactionError::Timeout = err {
                        // Increase the tip to incentivise the collators to include the transaction in a block with priority
                        tip = retry_strategy.compute_tip(retry_count + 1);
                        // Reuse the same nonce since the transaction was not included in a block.
                        nonce = Some(submitted_ext_info.nonce);

                        // Log warning if this is not the last retry.
                        if retry_count < retry_strategy.max_retries {
                            warn!(target: LOG_TARGET, "Retrying with increased tip {} and nonce {}", tip, submitted_ext_info.nonce);
                        }
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Exhausted retry strategy"))
    }
}
