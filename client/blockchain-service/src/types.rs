use log::{debug, info, warn};
use std::{
    cmp::{min, Ordering},
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    future::Future,
    pin::Pin,
    time::Duration,
};

use codec::{Decode, Encode};
use frame_system::DispatchEventInfo;
use sc_client_api::BlockImportNotification;
use sc_transaction_pool_api::TransactionStatus;
use shc_common::{
    traits::StorageEnableRuntime,
    types::{
        BackupStorageProviderId, BlockNumber, BucketId, CustomChallenge, FileKey, HasherOutT,
        MainStorageProviderId, MerkleTrieHash, OpaqueBlock, ProofsDealerProviderId,
        RandomnessOutput, RejectedStorageRequestReason, StorageDataUnit, StorageHubEventsVec,
        StorageProofsMerkleTrieLayout, StorageProviderId,
    },
};
use sp_blockchain::{HashAndNumber, TreeRoute};
use sp_runtime::{
    traits::{Header, Zero},
    DispatchError, SaturatedConversion,
};

use crate::{
    commands::BlockchainServiceCommandInterfaceExt, handler::LOG_TARGET,
    transaction_manager::wait_for_transaction_status,
};

/// A struct that holds the information to submit a storage proof.
///
/// This struct is used as an item in the `pending_submit_proof_requests` queue.
#[derive(Debug, Clone, Encode, Decode)]
pub struct SubmitProofRequest<Runtime: StorageEnableRuntime> {
    pub provider_id: ProofsDealerProviderId<Runtime>,
    pub tick: BlockNumber<Runtime>,
    pub seed: RandomnessOutput<Runtime>,
    pub forest_challenges: Vec<MerkleTrieHash<Runtime>>,
    pub checkpoint_challenges: Vec<CustomChallenge<Runtime>>,
}

impl<Runtime: StorageEnableRuntime> SubmitProofRequest<Runtime> {
    pub fn new(
        provider_id: ProofsDealerProviderId<Runtime>,
        tick: BlockNumber<Runtime>,
        seed: RandomnessOutput<Runtime>,
        forest_challenges: Vec<MerkleTrieHash<Runtime>>,
        checkpoint_challenges: Vec<CustomChallenge<Runtime>>,
    ) -> Self {
        Self {
            provider_id,
            tick,
            seed,
            forest_challenges,
            checkpoint_challenges,
        }
    }
}

impl<Runtime: StorageEnableRuntime> Ord for SubmitProofRequest<Runtime> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.tick.cmp(&other.tick)
    }
}

impl<Runtime: StorageEnableRuntime> PartialOrd for SubmitProofRequest<Runtime> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Two `SubmitProofRequest`s are considered equal if they have the same `tick` and `provider_id`.
// This helps to identify and remove duplicate requests from the queue.
impl<Runtime: StorageEnableRuntime> PartialEq for SubmitProofRequest<Runtime> {
    fn eq(&self, other: &Self) -> bool {
        self.tick == other.tick && self.provider_id == other.provider_id
    }
}

impl<Runtime: StorageEnableRuntime> Eq for SubmitProofRequest<Runtime> {}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ConfirmStoringRequest<Runtime: StorageEnableRuntime> {
    pub file_key: MerkleTrieHash<Runtime>,
    pub try_count: u32,
}

impl<Runtime: StorageEnableRuntime> ConfirmStoringRequest<Runtime> {
    pub fn new(file_key: MerkleTrieHash<Runtime>) -> Self {
        Self {
            file_key,
            try_count: 0,
        }
    }

    pub fn increment_try_count(&mut self) {
        self.try_count += 1;
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub enum MspRespondStorageRequest {
    Accept,
    Reject(RejectedStorageRequestReason),
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct RespondStorageRequest<Runtime: StorageEnableRuntime> {
    pub file_key: MerkleTrieHash<Runtime>,
    pub response: MspRespondStorageRequest,
    pub try_count: u32,
}

impl<Runtime: StorageEnableRuntime> RespondStorageRequest<Runtime> {
    pub fn new(file_key: MerkleTrieHash<Runtime>, response: MspRespondStorageRequest) -> Self {
        Self {
            file_key,
            response,
            try_count: 0,
        }
    }

    pub fn increment_try_count(&mut self) {
        self.try_count += 1;
    }
}

/// A struct that holds the information to stop storing all files from an insolvent user.
/// (Which is only the user's account ID).
///
/// This struct is used as an item in the `pending_stop_storing_for_insolvent_user_requests` queue.
#[derive(Debug, Clone, Encode, Decode)]
pub struct StopStoringForInsolventUserRequest<Runtime: StorageEnableRuntime> {
    pub user: Runtime::AccountId,
}

impl<Runtime: StorageEnableRuntime> StopStoringForInsolventUserRequest<Runtime> {
    pub fn new(user: Runtime::AccountId) -> Self {
        Self { user }
    }
}

/// A struct that holds the information to delete a file from storage.
///
/// This struct is used as an item in the `pending_file_deletion_requests` queue.
#[derive(Debug, Clone, Encode, Decode)]
pub struct FileDeletionRequest<Runtime: StorageEnableRuntime> {
    pub user: Runtime::AccountId,
    pub file_key: MerkleTrieHash<Runtime>,
    pub file_size: StorageDataUnit<Runtime>,
    pub bucket_id: BucketId<Runtime>,
    pub msp_id: ProofsDealerProviderId<Runtime>,
    pub proof_of_inclusion: bool,
    pub try_count: u32,
}

impl<Runtime: StorageEnableRuntime> FileDeletionRequest<Runtime> {
    pub fn new(
        user: Runtime::AccountId,
        file_key: MerkleTrieHash<Runtime>,
        file_size: StorageDataUnit<Runtime>,
        bucket_id: BucketId<Runtime>,
        msp_id: ProofsDealerProviderId<Runtime>,
        proof_of_inclusion: bool,
    ) -> Self {
        Self {
            user,
            file_key,
            file_size,
            bucket_id,
            msp_id,
            proof_of_inclusion,
            try_count: 0,
        }
    }

    pub fn increment_try_count(&mut self) {
        self.try_count += 1;
    }
}

/// Extrinsic struct.
///
/// This struct represents an extrinsic in the blockchain.
#[derive(Debug, Clone)]
pub struct Extrinsic<Runtime: StorageEnableRuntime> {
    /// Extrinsic hash.
    pub hash: Runtime::Hash,
    /// Block hash.
    pub block_hash: Runtime::Hash,
    /// Events vector.
    pub events: StorageHubEventsVec<Runtime>,
}

/// Information about a submitted extrinsic.
///
/// This struct is returned by `send_extrinsic()` and contains basic information
/// about the submitted transaction. The transaction is automatically watched
/// in the background by a spawned watcher task.
#[derive(Debug, Clone)]
pub struct SubmittedExtrinsicInfo<Runtime: StorageEnableRuntime> {
    /// Hash of the submitted extrinsic.
    pub hash: Runtime::Hash,
    /// The nonce of the extrinsic.
    pub nonce: u32,
    /// Status subscription receiver for tracking transaction lifecycle.
    /// Subscribe to this to get notified of status changes (Ready, InBlock, Finalized, etc.)
    pub status_subscription:
        tokio::sync::watch::Receiver<TransactionStatus<Runtime::Hash, Runtime::Hash>>,
}

impl<Runtime: StorageEnableRuntime> SubmittedExtrinsicInfo<Runtime> {
    /// Wait for the transaction to be included in a block and verify it succeeded.
    ///
    /// This method waits for the transaction to reach InBlock status, then fetches the
    /// extrinsic from the block and checks if it succeeded or failed by examining the
    /// ExtrinsicSuccess/ExtrinsicFailed events.
    ///
    /// Returns an error if:
    /// - The transaction fails to be included in a block (timeout, dropped, invalid, etc.)
    /// - The transaction is included but the extrinsic failed (ExtrinsicFailed event)
    pub async fn watch_for_success<T>(self, blockchain: &T) -> anyhow::Result<()>
    where
        T: BlockchainServiceCommandInterfaceExt<Runtime>,
    {
        // Wait for InBlock status with a reasonable timeout
        let block_hash = wait_for_transaction_status(
            self.nonce,
            self.status_subscription,
            StatusToWait::InBlock,
            std::time::Duration::from_secs(60),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Transaction failed to be included in block: {:?}", e))?;

        // Fetch the extrinsic from the block to check if it succeeded
        let extrinsic = blockchain
            .get_extrinsic_from_block(block_hash, self.hash)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to get extrinsic from block after InBlock status: {:?}",
                    e
                )
            })?;

        // Check if the extrinsic was successful by examining the events
        let extrinsic_result = T::extrinsic_result(extrinsic.clone()).map_err(|e| {
            anyhow::anyhow!(
                "Extrinsic does not contain an ExtrinsicFailed nor ExtrinsicSuccess event: {:?}",
                e
            )
        })?;

        match extrinsic_result {
            ExtrinsicResult::Success { dispatch_info } => {
                info!(
                    target: LOG_TARGET,
                    "Extrinsic with nonce {} succeeded with dispatch info: {:?}",
                    self.nonce,
                    dispatch_info
                );
                debug!(target: LOG_TARGET, "Extrinsic events: {:?}", extrinsic.events);
                Ok(())
            }
            ExtrinsicResult::Failure {
                dispatch_error,
                dispatch_info,
            } => {
                log::error!(
                    target: LOG_TARGET,
                    "Extrinsic with nonce {} failed with dispatch error: {:?}, dispatch info: {:?}",
                    self.nonce,
                    dispatch_error,
                    dispatch_info
                );
                Err(anyhow::anyhow!(
                    "Extrinsic failed: dispatch_error={:?}, dispatch_info={:?}",
                    dispatch_error,
                    dispatch_info
                ))
            }
        }
    }

    /// Wait for the transaction to be finalized.
    ///
    /// This is a convenience method that waits for the transaction to reach Finalized status.
    /// Returns an error if the transaction fails or times out.
    /// TODO: Add a timeout parameter.
    pub async fn watch_for_finalization(self) -> anyhow::Result<()> {
        wait_for_transaction_status(
            self.nonce,
            self.status_subscription,
            StatusToWait::Finalized,
            std::time::Duration::from_secs(60),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Transaction failed: {:?}", e))?;

        Ok(())
    }
}

/// ExtrinsicResult enum.
///
/// This enum represents the result of an extrinsic execution. It can be either a success or a failure.
pub enum ExtrinsicResult {
    /// Success variant.
    ///
    /// This variant represents a successful extrinsic execution.
    Success {
        /// Dispatch info.
        dispatch_info: DispatchEventInfo,
    },
    /// Failure variant.
    ///
    /// This variant represents a failed extrinsic execution.
    Failure {
        /// Dispatch error.
        dispatch_error: DispatchError,
        /// Dispatch info.
        dispatch_info: DispatchEventInfo,
    },
}

/// Options for [`send_extrinsic`](crate::BlockchainService::send_extrinsic).
///
/// You can safely use [`SendExtrinsicOptions::default`] to create a new instance of `SendExtrinsicOptions`.
#[derive(Debug)]
pub struct SendExtrinsicOptions {
    /// Tip to add to the transaction to incentivize the collator to include the transaction in a block.
    tip: u128,
    /// Optionally override the nonce to use when sending the transaction.
    nonce: Option<u32>,
    /// Maximum time to wait for a response before assuming the extrinsic submission has failed.
    timeout: Duration,
    /// The module (pallet) that the extrinsic belongs to. For instance, when sending a system_remark
    /// extrinsic, the module would be "system".
    module: Option<String>,
    /// The method that the extrinsic is calling. For instance, when sending a system_remark
    /// extrinsic, the method would be "remark".
    method: Option<String>,
}

impl SendExtrinsicOptions {
    pub fn new(timeout: Duration, module: Option<String>, method: Option<String>) -> Self {
        Self {
            tip: 0u128,
            nonce: None,
            timeout,
            module,
            method,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_tip(mut self, tip: u128) -> Self {
        self.tip = tip;
        self
    }

    pub fn with_nonce(mut self, nonce: Option<u32>) -> Self {
        self.nonce = nonce;
        self
    }

    pub fn with_module(mut self, module: Option<String>) -> Self {
        self.module = module;
        self
    }

    pub fn with_method(mut self, method: Option<String>) -> Self {
        self.method = method;
        self
    }

    pub fn tip(&self) -> u128 {
        self.tip
    }

    pub fn nonce(&self) -> Option<u32> {
        self.nonce
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn module(&self) -> Option<String> {
        self.module.clone()
    }

    pub fn method(&self) -> Option<String> {
        self.method.clone()
    }
}

impl Default for SendExtrinsicOptions {
    fn default() -> Self {
        Self {
            tip: 0u128,
            nonce: None,
            timeout: Duration::from_secs(60),
            module: None,
            method: None,
        }
    }
}

/// A struct which defines a submit extrinsic retry strategy. This defines a simple strategy when
/// sending and extrinsic. It will retry a maximum number of times ([Self::max_retries]).
/// If the extrinsic is not included in a block within a certain time frame [`Self::timeout`] it is
/// considered a failure.
/// The tip will increase with each retry, up to a maximum tip of [`Self::max_tip`].
/// The tip series (with the exception of the first try which is 0) is a geometric progression with
/// a multiplier of [`Self::base_multiplier`].
/// The final tip for each retry is calculated as:
/// [`Self::max_tip`] * (([`Self::base_multiplier`] ^ (retry_count / [`Self::max_retries`]) - 1) /
/// ([`Self::base_multiplier`] - 1)).
/// An optional check function can be provided to determine if the extrinsic should be retried,
/// aborting early if the function returns false.
pub struct RetryStrategy {
    /// Maximum number of retries after which the extrinsic submission will be considered failed.
    pub max_retries: u32,
    /// Maximum tip to be paid for the extrinsic submission. The progression follows an exponential
    /// backoff strategy.
    pub max_tip: u128,
    /// Base multiplier for the tip calculation. This is the base of the geometric progression.
    /// A higher value will make tips grow faster.
    pub base_multiplier: f64,
    /// An optional check function to determine if the extrinsic should be retried.
    ///
    /// If this is provided, the function will be called before each retry to determine if the
    /// extrinsic should be retried or the submission should be considered failed. If this is not
    /// provided, the extrinsic will be retried until [`Self::max_retries`] is reached.
    ///
    /// Additionally, the function will receive the [`WatchTransactionError`] as an argument, to
    /// help determine if the extrinsic should be retried or not.
    pub should_retry: Option<
        Box<dyn Fn(WatchTransactionError) -> Pin<Box<dyn Future<Output = bool> + Send>> + Send>,
    >,
}

impl RetryStrategy {
    /// Creates a new `RetryStrategy` instance.
    pub fn new(max_retries: u32, max_tip: u128, base_multiplier: f64) -> Self {
        Self {
            max_retries,
            max_tip,
            base_multiplier,
            should_retry: None,
        }
    }

    /// Set the maximum number of times to retry sending the extrinsic.
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set the maximum tip for the extrinsic.
    ///
    /// As the number of times the extrinsic is retried increases, the tip will increase
    /// exponentially, up to this maximum tip.
    pub fn with_max_tip(mut self, max_tip: u128) -> Self {
        self.max_tip = max_tip;
        self
    }

    /// The base multiplier for the exponential backoff.
    ///
    /// A higher value will make the exponential backoff more aggressive, making the tip
    /// increase quicker.
    pub fn with_base_multiplier(mut self, base_multiplier: f64) -> Self {
        self.base_multiplier = base_multiplier;
        self
    }

    /// Set a function to determine if the extrinsic should be retried.
    ///
    /// If this function is provided, it will be called before each retry to determine if the
    /// extrinsic should be retried or the submission should be considered failed. If this function
    /// is not provided, the extrinsic will be retried until [`Self::max_retries`] is reached.
    ///
    /// Additionally, the function will receive the [`WatchTransactionError`] as an argument, to
    /// help determine if the extrinsic should be retried or not.
    pub fn with_should_retry(
        mut self,
        should_retry: Option<
            Box<dyn Fn(WatchTransactionError) -> Pin<Box<dyn Future<Output = bool> + Send>> + Send>,
        >,
    ) -> Self {
        self.should_retry = should_retry;
        self
    }

    /// Sets [`Self::should_retry`] to retry only if the extrinsic submission times out.
    ///
    /// This is the recommended retry strategy for extrinsic submissions where the extrinsic
    /// itself is idempotent or safe to retry. Timeouts ([`WatchTransactionError::Timeout`])
    /// are transient errors that may be resolved by retrying (e.g., network congestion,
    /// collator unavailability).
    ///
    /// See [`WatchTransactionError`] for the full list of possible errors.
    pub fn retry_only_if_timeout(mut self) -> Self {
        self.should_retry = Some(Box::new(|error| {
            Box::pin(async move {
                match error {
                    WatchTransactionError::Timeout => true,
                    _ => false,
                }
            })
        }));
        self
    }

    /// Computes the tip for the given retry count.
    ///
    /// The formula for the tip is:
    /// [`Self::max_tip`] * (([`Self::base_multiplier`] ^ (retry_count / [`Self::max_retries`]) - 1) /
    /// ([`Self::base_multiplier`] - 1)).
    pub fn compute_tip(&self, retry_count: u32) -> u128 {
        // Ensure the retry_count is within the bounds of max_retries
        let retry_count = min(retry_count, self.max_retries);

        // Calculate the geometric progression factor for this retry_count
        let factor = (self
            .base_multiplier
            .powf(retry_count as f64 / self.max_retries as f64)
            - 1.0)
            / (self.base_multiplier - 1.0);

        // Final tip formula for each retry, scaled to max_tip
        let tip = self.max_tip as f64 * factor;
        tip as u128
    }
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            max_retries: 5,
            max_tip: 0,
            base_multiplier: 2.0,
            should_retry: None,
        }
    }
}

/// Status to wait for when monitoring a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusToWait {
    /// Wait for the transaction to be included in a block.
    InBlock,
    /// Wait for the transaction to be finalized.
    Finalized,
}

/// Errors that can occur while watching a transaction's lifecycle.
///
/// These errors are used by the retry mechanism in [`RetryStrategy`] to determine
/// whether to retry an extrinsic submission. By default, all errors cause a retry
/// (up to `max_retries`).
#[derive(thiserror::Error, Debug, Clone)]
pub enum WatchTransactionError {
    /// The transaction was not included in a block within the timeout period.
    ///
    /// This is a transient error that may be resolved by retrying.
    #[error("Timeout waiting for transaction to be included in a block")]
    Timeout,
    #[error("Transaction not found in the manager")]
    TransactionNotFound,
    #[error("Transaction hash does not match the hash in the manager")]
    TransactionHashMismatch,
    #[error("Transaction watcher channel closed")]
    WatcherChannelClosed,
    #[error("Transaction failed. DispatchError: {dispatch_error}, DispatchInfo: {dispatch_info}")]
    TransactionFailed {
        dispatch_error: String,
        dispatch_info: String,
    },
    #[error("Unexpected error: {0}")]
    Internal(String),
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum SubmitAndWatchError {
    #[error("Invalid Transaction: transaction is outdated (nonce {nonce})")]
    InvalidTransactionOutdated { nonce: u32 },
    #[error("RPC transport error: {message}")]
    RpcTransport { message: String },
    #[error("Malformed RPC response: {message}")]
    MalformedResponse { message: String },
    #[error("RPC error {code}: {message}")]
    RpcError {
        code: i64,
        message: String,
        data: Option<String>,
    },
}

/// Minimum block information needed to register what is the current best block
/// and detect reorgs.
#[derive(Debug, Clone, Encode, Decode, Copy)]
pub struct MinimalBlockInfo<Runtime: StorageEnableRuntime> {
    pub number: BlockNumber<Runtime>,
    pub hash: Runtime::Hash,
}

impl<Runtime: StorageEnableRuntime> From<&BlockImportNotification<OpaqueBlock>>
    for MinimalBlockInfo<Runtime>
{
    fn from(notification: &BlockImportNotification<OpaqueBlock>) -> Self {
        Self {
            number: (*notification.header.number()).into(),
            hash: notification.hash,
        }
    }
}

impl<Runtime: StorageEnableRuntime> From<BlockImportNotification<OpaqueBlock>>
    for MinimalBlockInfo<Runtime>
{
    fn from(notification: BlockImportNotification<OpaqueBlock>) -> Self {
        Self {
            number: (*notification.header.number()).into(),
            hash: notification.hash,
        }
    }
}

impl<Runtime: StorageEnableRuntime> Into<HashAndNumber<OpaqueBlock>> for MinimalBlockInfo<Runtime> {
    fn into(self) -> HashAndNumber<OpaqueBlock> {
        HashAndNumber {
            number: self.number.saturated_into(),
            hash: self.hash,
        }
    }
}

impl<Runtime: StorageEnableRuntime> From<HashAndNumber<OpaqueBlock>> for MinimalBlockInfo<Runtime> {
    fn from(hash_and_number: HashAndNumber<OpaqueBlock>) -> Self {
        Self {
            number: hash_and_number.number.into(),
            hash: hash_and_number.hash,
        }
    }
}

impl<Runtime: StorageEnableRuntime> Default for MinimalBlockInfo<Runtime> {
    fn default() -> Self {
        Self {
            number: Zero::zero(),
            hash: Default::default(),
        }
    }
}

/// When a new block is imported, the block is checked to see if it is one of the members
/// of this enum.
pub enum NewBlockNotificationKind<Runtime: StorageEnableRuntime> {
    /// The block is a new best block, built on top of the previous best block.
    ///
    /// - `last_best_block_processed`: The last best block that was processed by this node.
    ///   This is not necessarily the parent of `new_best_block`, since this node might be
    ///   coming out of syncing mode.
    /// - `new_best_block`: The new best block that was imported.
    /// - `tree_route`: The [`TreeRoute`] with `new_best_block` as the last element. The
    ///   length of the `tree_route` is determined by the number of blocks between the
    ///   `last_best_block_processed` and `new_best_block`, but if there are more than
    ///   `BlockchainServiceConfig::max_blocks_behind_to_catch_up_root_changes` blocks between the two, the route
    ///   will be trimmed to include the first `BlockchainServiceConfig::max_blocks_behind_to_catch_up_root_changes`
    ///   before the `new_best_block`.
    NewBestBlock {
        last_best_block_processed: MinimalBlockInfo<Runtime>,
        new_best_block: MinimalBlockInfo<Runtime>,
        tree_route: TreeRoute<OpaqueBlock>,
    },
    /// The block belongs to a fork that is not currently the best fork.
    NewNonBestBlock(MinimalBlockInfo<Runtime>),
    /// This block causes a reorg, i.e. it is the new best block, but the previous best block
    /// is not the parent of this one.
    ///
    /// The old best block (from the now non-best fork) is provided, as well as the new best block.
    /// The [`TreeRoute`] between the two (both included) is also provided, where `old_best_block`
    /// is the first element in the `tree_route`, and `new_best_block` is the last element.
    Reorg {
        old_best_block: MinimalBlockInfo<Runtime>,
        new_best_block: MinimalBlockInfo<Runtime>,
        tree_route: TreeRoute<OpaqueBlock>,
    },
}

/// The information needed to register a Forest Storage snapshot.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
pub struct ForestStorageSnapshotInfo<Runtime: StorageEnableRuntime> {
    /// The block number at which the Forest Storage snapshot was taken.
    ///
    /// i.e. the block number at which the Forest Storage changed from this snapshot
    /// version due to adding or removing files.
    pub block_number: BlockNumber<Runtime>,
    /// The Forest Storage snapshot hash.
    ///
    /// This is to uniquely identify the Forest Storage snapshot, as there could be
    /// snapshots at the same block number, but in different forks.
    pub block_hash: Runtime::Hash,
    /// The Forest Storage root when the snapshot was taken.
    ///
    /// This is used to identify the Forest Storage snapshot and retrieve it.
    pub forest_root: HasherOutT<StorageProofsMerkleTrieLayout>,
}

impl<Runtime: StorageEnableRuntime> PartialOrd for ForestStorageSnapshotInfo<Runtime> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Implements the `Ord` trait for `ForestStorageSnapshotInfo`.
///
/// This allows for a BTreeSet to be used to store Forest Storage snapshots.
impl<Runtime: StorageEnableRuntime> Ord for ForestStorageSnapshotInfo<Runtime> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Block number ordering is the first criteria.
        match self.block_number.cmp(&other.block_number) {
            std::cmp::Ordering::Less => std::cmp::Ordering::Less,
            std::cmp::Ordering::Greater => std::cmp::Ordering::Greater,
            std::cmp::Ordering::Equal => {
                // If the block numbers are equal, compare the block hashes.
                match self.block_hash.cmp(&other.block_hash) {
                    std::cmp::Ordering::Less => std::cmp::Ordering::Less,
                    std::cmp::Ordering::Greater => std::cmp::Ordering::Greater,
                    std::cmp::Ordering::Equal => {
                        // If the block hashes and block numbers are equal, the forest roots should be
                        // the same, because there can only be one snapshot at a given block number,
                        // for a given fork.
                        if self.forest_root != other.forest_root {
                            warn!(target: LOG_TARGET, "CRITICAL❗️❗️ This is a bug! Forest storage snapshot forest roots are not equal, for the same block number and hash. This should not happen. This is a bug. Please report it to the StorageHub team.");
                        }

                        std::cmp::Ordering::Equal
                    }
                }
            }
        }
    }
}

/// Info recorded for files being distributed to BSPs from an MSP.
///
/// Stores the BSPs for which there are tasks currently distributing the file,
/// and the BSPs for which the file has been confirmed to be stored.
#[derive(Debug, Clone)]
pub struct FileDistributionInfo<Runtime: StorageEnableRuntime> {
    /// The BSPs for which there are tasks currently distributing the file.
    pub(crate) bsps_distributing: BTreeSet<BackupStorageProviderId<Runtime>>,
    /// The BSPs for which the file has been confirmed to be stored.
    pub(crate) bsps_confirmed: BTreeSet<BackupStorageProviderId<Runtime>>,
}

impl<Runtime: StorageEnableRuntime> FileDistributionInfo<Runtime> {
    pub fn new() -> Self {
        Self {
            bsps_distributing: BTreeSet::new(),
            bsps_confirmed: BTreeSet::new(),
        }
    }
}
/// A struct that holds the information to handle a BSP.
///
/// This struct implements all the needed logic to manage BSP specific functionality.
#[derive(Debug)]
pub struct BspHandler<Runtime: StorageEnableRuntime> {
    /// The BSP ID.
    pub(crate) bsp_id: BackupStorageProviderId<Runtime>,
    /// Pending submit proof requests. Note: this is not kept in the persistent state because of
    /// various edge cases when restarting the node.
    pub(crate) pending_submit_proof_requests: BTreeSet<SubmitProofRequest<Runtime>>,
    /// A lock to prevent multiple tasks from writing to the runtime Forest root (send transactions) at the same time.
    ///
    /// This is a oneshot channel instead of a regular mutex because we want to "lock" in 1
    /// thread (Blockchain Service) and unlock it at the end of the spawned task. The alternative
    /// would be to send a [`MutexGuard`].
    pub(crate) forest_root_write_lock: Option<tokio::sync::oneshot::Receiver<()>>,
    /// A set of Forest Storage snapshots, ordered by block number and block hash.
    ///
    /// A BSP can have multiple Forest Storage snapshots.
    /// TODO: Remove this `allow(dead_code)` once we have implemented the Forest Storage snapshots.
    #[allow(dead_code)]
    pub(crate) forest_root_snapshots: BTreeSet<ForestStorageSnapshotInfo<Runtime>>,
}

impl<Runtime: StorageEnableRuntime> BspHandler<Runtime> {
    pub fn new(bsp_id: BackupStorageProviderId<Runtime>) -> Self {
        Self {
            bsp_id,
            pending_submit_proof_requests: BTreeSet::new(),
            forest_root_write_lock: None,
            forest_root_snapshots: BTreeSet::new(),
        }
    }
}
/// Status of a file key in the MSP upload pipeline.
///
/// Used to track processing state, prevent duplicate processing, and enable automatic retries.
///
/// ## Status Transitions
///
/// | Status        | Meaning                                    | Next Block Behavior                           |
/// | ------------- | ------------------------------------------ | ----------------------------------------------|
/// | `Processing`  | File key is in the pipeline                | **Skip** (already being handled)              |
/// | `Abandoned`   | Failed with non-proof dispatch error       | **Skip** (permanent failure)                  |
/// | *Not present* | New or retryable file key                  | **Process** (emit event, set to `Processing`) |
///
/// ## Retry Mechanism
///
/// File keys are **removed** from statuses to signal they should be re-processed:
/// - **Proof errors**: Removed to retry with regenerated proofs
/// - **Extrinsic submission timeouts**: Removed to retry (timeouts are transient)
/// - **Non-proof dispatch errors**: Marked as `Abandoned` (permanent failure, no retry)
///
/// ## Lifecycle Completion
///
/// File keys are **removed** from statuses when their storage request lifecycle is complete
/// (i.e., they no longer appear in pending storage requests). This cleanup happens automatically
/// during block processing, regardless of the file key's current status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileKeyStatus {
    /// File key is currently being processed (in the pipeline).
    ///
    /// Set **only** by the blockchain service when emitting a
    /// [`NewStorageRequest`](crate::events::NewStorageRequest) event for the file key.
    ///
    /// Tasks cannot set this status directly—use [`FileKeyStatusUpdate`] instead.
    #[default]
    Processing,
    /// File key failed with a non-proof-related dispatch error (permanent failure).
    ///
    /// Set when the extrinsic is included in a block but fails with a dispatch error
    /// that is NOT proof-related (e.g., authorization failures, invalid parameters,
    /// inconsistent runtime state). These are permanent failures not resolved by retrying.
    ///
    /// **Note:** Proof errors and extrinsic submission failures (timeout after retries)
    /// do NOT set this status—instead, they **remove** the file key from statuses to
    /// enable automatic retry on the next block.
    Abandoned,
}

/// Task-settable status for a file key, used via [`SetFileKeyStatus`](crate::commands::BlockchainServiceCommand::SetFileKeyStatus).
///
/// This is a subset of [`FileKeyStatus`] that excludes `Processing`, which is only
/// set by the blockchain service when emitting [`NewStorageRequest`](crate::events::NewStorageRequest) events.
///
/// This type-safe restriction ensures tasks can only transition file keys to appropriate
/// states, preventing accidental re-processing of already-handled requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKeyStatusUpdate {
    /// File key failed with a permanent error (non-proof dispatch error).
    Abandoned,
}

impl From<FileKeyStatusUpdate> for FileKeyStatus {
    fn from(status: FileKeyStatusUpdate) -> Self {
        match status {
            FileKeyStatusUpdate::Abandoned => FileKeyStatus::Abandoned,
        }
    }
}

/// A struct that holds the information to handle an MSP.
///
/// This struct implements all the needed logic to manage MSP specific functionality.
#[derive(Debug)]
pub struct MspHandler<Runtime: StorageEnableRuntime> {
    /// The MSP ID.
    pub(crate) msp_id: MainStorageProviderId<Runtime>,
    /// TODO: CHANGE THIS INTO MULTIPLE LOCKS, ONE FOR EACH BUCKET.
    ///
    /// A lock to prevent multiple tasks from writing to the runtime Forest root (send transactions) at the same time.
    ///
    /// This is a oneshot channel instead of a regular mutex because we want to "lock" in 1
    /// thread (Blockchain Service) and unlock it at the end of the spawned task. The alternative
    /// would be to send a [`MutexGuard`].
    pub(crate) forest_root_write_lock: Option<tokio::sync::oneshot::Receiver<()>>,
    /// A map of [`BucketId`] to the Forest Storage snapshots.
    ///
    /// Forest Storage snapshots are stored in a BTreeSet, ordered by block number and block hash.
    /// Each Bucket can have multiple Forest Storage snapshots.
    /// TODO: Remove this `allow(dead_code)` once we have implemented the Forest Storage snapshots.
    #[allow(dead_code)]
    pub(crate) forest_root_snapshots:
        BTreeMap<BucketId<Runtime>, BTreeSet<ForestStorageSnapshotInfo<Runtime>>>,
    /// A map of [`FileKey`] to the information needed to distribute the file to BSPs.
    ///
    /// This is used to keep track of the BSPs for which there are tasks currently distributing the file,
    /// and the BSPs for which the file has been confirmed to be stored.
    pub(crate) files_to_distribute: HashMap<FileKey, FileDistributionInfo<Runtime>>,
    /// Tracks the processing status of each file key in the upload pipeline.
    ///
    /// Used to prevent duplicate processing and enable retries for failed requests.
    /// The blockchain service checks this before emitting [`NewStorageRequest`] events,
    /// and tasks update it via commands as they process requests.
    pub(crate) file_key_statuses: HashMap<FileKey, FileKeyStatus>,
    /// In-memory FIFO queue for pending MSP respond storage requests.
    pub(crate) pending_respond_storage_requests: VecDeque<RespondStorageRequest<Runtime>>,
    /// HashSet tracking file keys currently in the pending respond storage request queue.
    /// Used for O(1) deduplication when queueing new requests.
    pub(crate) pending_respond_storage_request_file_keys: HashSet<MerkleTrieHash<Runtime>>,
}

impl<Runtime: StorageEnableRuntime> MspHandler<Runtime> {
    pub fn new(msp_id: MainStorageProviderId<Runtime>) -> Self {
        Self {
            msp_id,
            forest_root_write_lock: None,
            forest_root_snapshots: BTreeMap::new(),
            files_to_distribute: HashMap::new(),
            file_key_statuses: HashMap::new(),
            pending_respond_storage_requests: VecDeque::new(),
            pending_respond_storage_request_file_keys: HashSet::new(),
        }
    }
}

/// An enum that represents the managed provider, either a BSP or an MSP.
///
/// The enum variants hold the handler for the managed provider (see [`BspHandler`] and [`MspHandler`]).
#[derive(Debug)]
pub enum ManagedProvider<Runtime: StorageEnableRuntime> {
    Bsp(BspHandler<Runtime>),
    Msp(MspHandler<Runtime>),
}

impl<Runtime: StorageEnableRuntime> ManagedProvider<Runtime> {
    pub fn new(provider_id: StorageProviderId<Runtime>) -> Self {
        match provider_id {
            StorageProviderId::BackupStorageProvider(bsp_id) => Self::Bsp(BspHandler::new(bsp_id)),
            StorageProviderId::MainStorageProvider(msp_id) => Self::Msp(MspHandler::new(msp_id)),
        }
    }
}

/// Role of this node in a group of multiple instances of nodes running the same MSP/BSP.
///
/// - `Leader`: the only node allowed to submit extrinsics and manage nonces.
/// - `Follower`: keeps a local copy of Merkle Patricia Forests and file
///   chunks, but never submits extrinsics.
/// - `Standalone`: pending-tx DB is disabled; node behaves as a single-instance
///   deployment and does not persist pending transactions in the DB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MultiInstancesNodeRole {
    /// The only node allowed to submit extrinsics and manage nonces.
    ///
    /// This role means that the pending transactions DB is used to persist pending transactions
    /// and this node was the one to acquire the leadership advisory lock.
    Leader,
    /// Keeps a local copy of Merkle Patricia Forests and file chunks, but never submits extrinsics.
    ///
    /// This role means that the pending transactions DB is used to persist pending transactions
    /// and this node was not the one to acquire the leadership advisory lock. It keeps the Forest
    /// and file chunks stored to be able to take on the leadership role, should the `Leader` stop working.
    Follower,
    /// Pending-tx DB is disabled; node behaves as a single-instance deployment and does not persist
    /// pending transactions in the DB.
    Standalone,
}
