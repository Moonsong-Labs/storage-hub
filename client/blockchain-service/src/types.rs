use std::cmp::Ordering;

use codec::{Decode, Encode};
use frame_support::dispatch::DispatchInfo;
use frame_system::EventRecord;
use shc_common::types::{
    BlockNumber, ProviderId, RandomnessOutput, RejectedStorageRequestReason, TrieRemoveMutation,
};
use sp_core::H256;
use sp_runtime::DispatchError;

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

#[derive(Debug, Clone, Encode, Decode)]
pub enum MspResponse {
    Accept,
    Reject(RejectedStorageRequestReason),
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct RespondStorageRequest {
    pub file_key: H256,
    pub response: MspResponse,
    pub try_count: u32,
}

impl RespondStorageRequest {
    pub fn new(file_key: H256, response: MspResponse) -> Self {
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
