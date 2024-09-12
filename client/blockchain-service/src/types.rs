use codec::{Decode, Encode};
use frame_support::dispatch::DispatchInfo;
use frame_system::EventRecord;
use shc_common::types::{BlockNumber, ProviderId, RandomnessOutput, TrieRemoveMutation};
use sp_core::H256;
use sp_runtime::DispatchError;
use std::{cmp::Ordering, future::Future, pin::Pin, time::Duration};

/// A struct that holds the information to submit a storage proof.
///
/// This struct is used as an item in the `pending_submit_proof_requests` queue.
#[derive(Debug, Clone, Encode, Decode)]
pub struct SubmitProofRequest {
    pub provider_id: ProviderId,
    pub tick: BlockNumber,
    pub seed: RandomnessOutput,
    pub forest_challenges: Vec<H256>,
    pub checkpoint_challenges: Vec<(H256, Option<TrieRemoveMutation>)>,
}

impl SubmitProofRequest {
    pub fn new(
        provider_id: ProviderId,
        tick: BlockNumber,
        seed: RandomnessOutput,
        forest_challenges: Vec<H256>,
        checkpoint_challenges: Vec<(H256, Option<TrieRemoveMutation>)>,
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

/// Type alias for the events vector.
///
/// The events vector is a storage element in the FRAME system pallet, which stores all the events that have occurred
/// in a block. This is syntactic sugar to make the code more readable.
pub type EventsVec = Vec<
    Box<
        EventRecord<
            <storage_hub_runtime::Runtime as frame_system::Config>::RuntimeEvent,
            <storage_hub_runtime::Runtime as frame_system::Config>::Hash,
        >,
    >,
>;

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
    pub events: EventsVec,
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
        dispatch_info: DispatchInfo,
    },
    /// Failure variant.
    ///
    /// This variant represents a failed extrinsic execution.
    Failure {
        /// Dispatch error.
        dispatch_error: DispatchError,
        /// Dispatch info.
        dispatch_info: DispatchInfo,
    },
}

/// Type alias for the extrinsic hash.
pub type ExtrinsicHash = H256;

/// Type alias for the tip.
pub type Tip = pallet_transaction_payment::ChargeTransactionPayment<storage_hub_runtime::Runtime>;

/// A struct which defines a submit extrinsic retry strategy.
pub struct RetryStrategy {
    /// Maximum number of retries after which the extrinsic submission will be considered failed.
    pub max_retries: u32,
    /// Maximum time to wait for a response before assuming the extrinsic submission has failed.
    pub timeout: Duration,
    /// Maximum tip to be paid for the extrinsic submission. The progression follows an exponential
    /// backoff strategy.
    pub max_tip: f64,
    /// Base multiplier for the tip calculation.
    /// This is a constant value that is used to calculate the tip multiplier.
    /// A higher value will make tips grow faster.
    pub base_multiplier: f64,
    /// An optional check function to determine if the extrinsic should be retried.
    pub should_retry: Option<Box<dyn Fn() -> Pin<Box<dyn Future<Output = bool> + Send>> + Send>>,
}

impl RetryStrategy {
    /// Creates a new `RetryStrategy` instance.
    pub fn new(max_retries: u32, timeout: Duration, max_tip: f64, base_multiplier: f64) -> Self {
        Self {
            max_retries,
            timeout,
            max_tip,
            base_multiplier,
            should_retry: None,
        }
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_max_tip(mut self, max_tip: f64) -> Self {
        self.max_tip = max_tip;
        self
    }

    pub fn with_base_multiplier(mut self, base_multiplier: f64) -> Self {
        self.base_multiplier = base_multiplier;
        self
    }

    pub fn with_should_retry(
        mut self,
        should_retry: Option<Box<dyn Fn() -> Pin<Box<dyn Future<Output = bool> + Send>> + Send>>,
    ) -> Self {
        self.should_retry = should_retry;
        self
    }

    /// Compute the exponential increase (multiplier) in tip at each retry.
    /// A higher multiplier will make tips grow exponentially faster.
    fn compute_tip_multiplier(&self) -> f64 {
        (self.base_multiplier.ln() / self.max_retries as f64).exp()
    }

    pub fn compute_tip(&self, retry_count: u32) -> f64 {
        let multiplier = self.compute_tip_multiplier();

        // Calculate the geometric progression factor for each retry
        let factor = (multiplier.powf(retry_count as f64) - 1.0)
            / (multiplier.powf(self.max_retries as f64) - 1.0);

        // Final tip formula for each retry, scaled to max_tip
        self.max_tip * factor
    }
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            max_retries: 5,
            timeout: Duration::from_secs(30),
            max_tip: 0.0,
            base_multiplier: 2.0,
            should_retry: None,
        }
    }
}
