use log::warn;
use std::{
    cmp::{min, Ordering},
    collections::{BTreeMap, BTreeSet},
    future::Future,
    pin::Pin,
    time::Duration,
};

use codec::{Decode, Encode};
use frame_system::DispatchEventInfo;
use sc_client_api::BlockImportNotification;
use sp_blockchain::{HashAndNumber, TreeRoute};
use sp_core::H256;
use sp_runtime::{
    traits::{Header, NumberFor},
    AccountId32, DispatchError, SaturatedConversion,
};

use shc_common::types::{
    BackupStorageProviderId, BlockNumber, BucketId, CustomChallenge, HasherOutT,
    MainStorageProviderId, ProofsDealerProviderId, RandomnessOutput, RejectedStorageRequestReason,
    StorageData, StorageHubEventsVec, StorageProofsMerkleTrieLayout, StorageProviderId,
};

use crate::{events, handler::LOG_TARGET};

/// A struct that holds the information to submit a storage proof.
///
/// This struct is used as an item in the `pending_submit_proof_requests` queue.
#[derive(Debug, Clone, Encode, Decode)]
pub struct SubmitProofRequest {
    pub provider_id: ProofsDealerProviderId,
    pub tick: BlockNumber,
    pub seed: RandomnessOutput,
    pub forest_challenges: Vec<H256>,
    pub checkpoint_challenges: Vec<CustomChallenge>,
}

impl SubmitProofRequest {
    pub fn new(
        provider_id: ProofsDealerProviderId,
        tick: BlockNumber,
        seed: RandomnessOutput,
        forest_challenges: Vec<H256>,
        checkpoint_challenges: Vec<CustomChallenge>,
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

impl Ord for SubmitProofRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        self.tick.cmp(&other.tick)
    }
}

impl PartialOrd for SubmitProofRequest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Two `SubmitProofRequest`s are considered equal if they have the same `tick` and `provider_id`.
// This helps to identify and remove duplicate requests from the queue.
impl PartialEq for SubmitProofRequest {
    fn eq(&self, other: &Self) -> bool {
        self.tick == other.tick && self.provider_id == other.provider_id
    }
}

impl Eq for SubmitProofRequest {}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ConfirmStoringRequest {
    pub file_key: H256,
    pub try_count: u32,
}

impl ConfirmStoringRequest {
    pub fn new(file_key: H256) -> Self {
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
pub struct RespondStorageRequest {
    pub file_key: H256,
    pub response: MspRespondStorageRequest,
    pub try_count: u32,
}

impl RespondStorageRequest {
    pub fn new(file_key: H256, response: MspRespondStorageRequest) -> Self {
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
pub struct StopStoringForInsolventUserRequest {
    pub user: AccountId32,
}

impl StopStoringForInsolventUserRequest {
    pub fn new(user: AccountId32) -> Self {
        Self { user }
    }
}

/// A struct that holds the information to delete a file from storage.
///
/// This struct is used as an item in the `pending_file_deletion_requests` queue.
#[derive(Debug, Clone, Encode, Decode)]
pub struct FileDeletionRequest {
    pub user: AccountId32,
    pub file_key: H256,
    pub file_size: StorageData,
    pub bucket_id: BucketId,
    pub msp_id: ProofsDealerProviderId,
    pub proof_of_inclusion: bool,
    pub try_count: u32,
}

impl FileDeletionRequest {
    pub fn new(
        user: AccountId32,
        file_key: H256,
        file_size: StorageData,
        bucket_id: BucketId,
        msp_id: ProofsDealerProviderId,
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

impl From<events::FileDeletionRequest> for FileDeletionRequest {
    fn from(event: events::FileDeletionRequest) -> Self {
        Self::new(
            event.user,
            event.file_key.into(),
            event.file_size,
            event.bucket_id,
            event.msp_id,
            event.proof_of_inclusion,
        )
    }
}

/// Extrinsic struct.
///
/// This struct represents an extrinsic in the blockchain.
#[derive(Debug, Clone)]
pub struct Extrinsic {
    /// Extrinsic hash.
    pub hash: H256,
    /// Block hash.
    pub block_hash: H256,
    /// Events vector.
    pub events: StorageHubEventsVec,
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

/// Type alias for the extrinsic hash.
pub type ExtrinsicHash = H256;

/// Type alias for the tip.
pub type Tip = pallet_transaction_payment::ChargeTransactionPayment<storage_hub_runtime::Runtime>;

/// Options for [`send_extrinsic`](crate::BlockchainService::send_extrinsic).
///
/// You can safely use [`SendExtrinsicOptions::default`] to create a new instance of `SendExtrinsicOptions`.
#[derive(Debug)]
pub struct SendExtrinsicOptions {
    /// Tip to add to the transaction to incentivize the collator to include the transaction in a block.
    tip: Tip,
    /// Optionally override the nonce to use when sending the transaction.
    nonce: Option<u32>,
    /// Maximum time to wait for a response before assuming the extrinsic submission has failed.
    timeout: Duration,
}

impl SendExtrinsicOptions {
    pub fn new(timeout: Duration) -> Self {
        Self {
            tip: Tip::from(0),
            nonce: None,
            timeout,
        }
    }

    pub fn with_tip(mut self, tip: u128) -> Self {
        self.tip = Tip::from(tip);
        self
    }

    pub fn with_nonce(mut self, nonce: Option<u32>) -> Self {
        self.nonce = nonce;
        self
    }

    pub fn tip(&self) -> Tip {
        self.tip.clone()
    }

    pub fn nonce(&self) -> Option<u32> {
        self.nonce
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

impl Default for SendExtrinsicOptions {
    fn default() -> Self {
        Self {
            tip: Tip::from(0),
            nonce: None,
            timeout: Duration::from_secs(60),
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
    pub max_tip: f64,
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
    pub fn new(max_retries: u32, max_tip: f64, base_multiplier: f64) -> Self {
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
    pub fn with_max_tip(mut self, max_tip: f64) -> Self {
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

    /// Sets [`Self::should_retry`] to retry only if the extrinsic times out.
    ///
    /// This means that the extrinsic will not be sent again if, for example, it
    /// is included in a block but it fails.
    ///
    /// See [`WatchTransactionError`] for other possible errors.
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
    pub fn compute_tip(&self, retry_count: u32) -> f64 {
        // Ensure the retry_count is within the bounds of max_retries
        let retry_count = min(retry_count, self.max_retries);

        // Calculate the geometric progression factor for this retry_count
        let factor = (self
            .base_multiplier
            .powf(retry_count as f64 / self.max_retries as f64)
            - 1.0)
            / (self.base_multiplier - 1.0);

        // Final tip formula for each retry, scaled to max_tip
        self.max_tip * factor
    }
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            max_retries: 5,
            max_tip: 0.0,
            base_multiplier: 2.0,
            should_retry: None,
        }
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum WatchTransactionError {
    #[error("Timeout waiting for transaction to be included in a block")]
    Timeout,
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

/// Minimum block information needed to register what is the current best block
/// and detect reorgs.
#[derive(Debug, Clone, Encode, Decode, Default, Copy)]
pub struct MinimalBlockInfo {
    pub number: BlockNumber,
    pub hash: H256,
}

impl<Block> From<&BlockImportNotification<Block>> for MinimalBlockInfo
where
    Block: cumulus_primitives_core::BlockT<Hash = H256>,
{
    fn from(notification: &BlockImportNotification<Block>) -> Self {
        Self {
            number: (*notification.header.number()).saturated_into(),
            hash: notification.hash,
        }
    }
}

impl<Block> From<BlockImportNotification<Block>> for MinimalBlockInfo
where
    Block: cumulus_primitives_core::BlockT<Hash = H256>,
{
    fn from(notification: BlockImportNotification<Block>) -> Self {
        Self {
            number: (*notification.header.number()).saturated_into(),
            hash: notification.hash,
        }
    }
}

impl<Block> Into<HashAndNumber<Block>> for MinimalBlockInfo
where
    Block: cumulus_primitives_core::BlockT<Hash = H256>,
    NumberFor<Block>: From<BlockNumber>,
{
    fn into(self) -> HashAndNumber<Block> {
        HashAndNumber {
            hash: self.hash,
            number: self.number.into(),
        }
    }
}

impl<Block> From<HashAndNumber<Block>> for MinimalBlockInfo
where
    Block: cumulus_primitives_core::BlockT<Hash = H256>,
    NumberFor<Block>: Into<BlockNumber>,
{
    fn from(hash_and_number: HashAndNumber<Block>) -> Self {
        Self {
            number: hash_and_number.number.into(),
            hash: hash_and_number.hash,
        }
    }
}

/// When a new block is imported, the block is checked to see if it is one of the members
/// of this enum.
pub enum NewBlockNotificationKind<Block>
where
    Block: cumulus_primitives_core::BlockT<Hash = H256>,
{
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
        last_best_block_processed: MinimalBlockInfo,
        new_best_block: MinimalBlockInfo,
        tree_route: TreeRoute<Block>,
    },
    /// The block belongs to a fork that is not currently the best fork.
    NewNonBestBlock(MinimalBlockInfo),
    /// This block causes a reorg, i.e. it is the new best block, but the previous best block
    /// is not the parent of this one.
    ///
    /// The old best block (from the now non-best fork) is provided, as well as the new best block.
    /// The [`TreeRoute`] between the two (both included) is also provided, where `old_best_block`
    /// is the first element in the `tree_route`, and `new_best_block` is the last element.
    Reorg {
        old_best_block: MinimalBlockInfo,
        new_best_block: MinimalBlockInfo,
        tree_route: TreeRoute<Block>,
    },
}

/// The information needed to register a Forest Storage snapshot.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
pub struct ForestStorageSnapshotInfo {
    /// The block number at which the Forest Storage snapshot was taken.
    ///
    /// i.e. the block number at which the Forest Storage changed from this snapshot
    /// version due to adding or removing files.
    pub block_number: BlockNumber,
    /// The Forest Storage snapshot hash.
    ///
    /// This is to uniquely identify the Forest Storage snapshot, as there could be
    /// snapshots at the same block number, but in different forks.
    pub block_hash: H256,
    /// The Forest Storage root when the snapshot was taken.
    ///
    /// This is used to identify the Forest Storage snapshot and retrieve it.
    pub forest_root: HasherOutT<StorageProofsMerkleTrieLayout>,
}

impl PartialOrd for ForestStorageSnapshotInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Implements the `Ord` trait for `ForestStorageSnapshotInfo`.
///
/// This allows for a BTreeSet to be used to store Forest Storage snapshots.
impl Ord for ForestStorageSnapshotInfo {
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

/// A struct that holds the information to handle a BSP.
///
/// This struct implements all the needed logic to manage BSP specific functionality.
#[derive(Debug)]
pub struct BspHandler {
    /// The BSP ID.
    pub(crate) bsp_id: BackupStorageProviderId,
    /// Pending submit proof requests. Note: this is not kept in the persistent state because of
    /// various edge cases when restarting the node.
    pub(crate) pending_submit_proof_requests: BTreeSet<SubmitProofRequest>,
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
    pub(crate) forest_root_snapshots: BTreeSet<ForestStorageSnapshotInfo>,
}

impl BspHandler {
    pub fn new(bsp_id: BackupStorageProviderId) -> Self {
        Self {
            bsp_id,
            pending_submit_proof_requests: BTreeSet::new(),
            forest_root_write_lock: None,
            forest_root_snapshots: BTreeSet::new(),
        }
    }
}
/// A struct that holds the information to handle an MSP.
///
/// This struct implements all the needed logic to manage MSP specific functionality.
#[derive(Debug)]
pub struct MspHandler {
    /// The MSP ID.
    pub(crate) msp_id: MainStorageProviderId,
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
    pub(crate) forest_root_snapshots: BTreeMap<BucketId, BTreeSet<ForestStorageSnapshotInfo>>,
}

impl MspHandler {
    pub fn new(msp_id: MainStorageProviderId) -> Self {
        Self {
            msp_id,
            forest_root_write_lock: None,
            forest_root_snapshots: BTreeMap::new(),
        }
    }
}

/// An enum that represents the managed provider, either a BSP or an MSP.
///
/// The enum variants hold the handler for the managed provider (see [`BspHandler`] and [`MspHandler`]).
#[derive(Debug)]
pub enum ManagedProvider {
    Bsp(BspHandler),
    Msp(MspHandler),
}

impl ManagedProvider {
    pub fn new(provider_id: StorageProviderId) -> Self {
        match provider_id {
            StorageProviderId::BackupStorageProvider(bsp_id) => Self::Bsp(BspHandler::new(bsp_id)),
            StorageProviderId::MainStorageProvider(msp_id) => Self::Msp(MspHandler::new(msp_id)),
        }
    }
}
