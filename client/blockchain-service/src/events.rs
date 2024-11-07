use codec::{Decode, Encode};
use sc_network::Multiaddr;
use shc_actors_framework::event_bus::{EventBusMessage, ProvidesEventBus};
use shc_common::types::{
    Balance, BlockNumber, BucketId, FileKey, FileLocation, Fingerprint, ForestRoot, KeyProofs,
    PeerIds, ProviderId, RandomnessOutput, StorageData, TrieRemoveMutation,
};
use sp_core::H256;
use sp_runtime::AccountId32;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

use crate::types::{ConfirmStoringRequest, RespondStorageRequest};

/// New random challenge emitted by the StorageHub runtime.
///
/// This event is emitted when there's a new random challenge seed that affects this
/// BSP. In other words, it only pays attention to the random seeds in the challenge
/// period of this BSP.
#[derive(Debug, Clone, Encode, Decode, EventBusMessage)]
pub struct NewChallengeSeed {
    pub provider_id: ProviderId,
    pub tick: BlockNumber,
    pub seed: RandomnessOutput,
}

/// Multiple new challenge seeds that have to be responded in order.
///
/// This event is emitted when catching up to proof submissions, and there are multiple
/// new challenge seeds that have to be responded in order.
/// The `seeds` vector is expected to be sorted in ascending order, where the first element
/// is the seed that should be responded to first, and the last element is the seed that
/// should be responded to last.
#[derive(Debug, Clone, Encode, Decode, EventBusMessage)]
pub struct MultipleNewChallengeSeeds {
    pub provider_id: ProviderId,
    pub seeds: Vec<(BlockNumber, RandomnessOutput)>,
}

/// New storage request event.
///
/// This event is emitted when a new storage request is created on-chain.
#[derive(Debug, Clone, EventBusMessage)]
pub struct NewStorageRequest {
    /// Account ID of the requester.
    pub who: AccountId32,
    /// Computed file key for the file.
    pub file_key: FileKey,
    /// Bucket ID of the file.
    pub bucket_id: BucketId,
    /// Location of the file (as a file path).
    pub location: FileLocation,
    /// Fingerprint of the file (root hash of the merkelized file).
    pub fingerprint: Fingerprint,
    /// Size of the file.
    pub size: StorageData,
    /// libp2p peer IDs from where the user would send the file.
    pub user_peer_ids: PeerIds,
}

/// Accepted BSP volunteer event.
///
/// This event is emitted when a BSP volunteer is accepted to store a file.
#[derive(Debug, Clone, EventBusMessage)]
pub struct AcceptedBspVolunteer {
    pub bsp_id: H256,
    pub bucket_id: BucketId,
    pub location: FileLocation,
    pub fingerprint: Fingerprint,
    pub multiaddresses: Vec<Multiaddr>,
    pub owner: AccountId32,
    pub size: StorageData,
}

#[derive(Debug, Clone, Encode, Decode)]
pub enum ForestWriteLockTaskData {
    SubmitProofRequest(ProcessSubmitProofRequestData),
    ConfirmStoringRequest(ProcessConfirmStoringRequestData),
    MspRespondStorageRequest(ProcessMspRespondStoringRequestData),
    StopStoringForInsolventUserRequest(ProcessStopStoringForInsolventUserRequestData),
}

impl From<ProcessSubmitProofRequestData> for ForestWriteLockTaskData {
    fn from(data: ProcessSubmitProofRequestData) -> Self {
        Self::SubmitProofRequest(data)
    }
}

impl From<ProcessConfirmStoringRequestData> for ForestWriteLockTaskData {
    fn from(data: ProcessConfirmStoringRequestData) -> Self {
        Self::ConfirmStoringRequest(data)
    }
}

impl From<ProcessMspRespondStoringRequestData> for ForestWriteLockTaskData {
    fn from(data: ProcessMspRespondStoringRequestData) -> Self {
        Self::MspRespondStorageRequest(data)
    }
}

impl From<ProcessStopStoringForInsolventUserRequestData> for ForestWriteLockTaskData {
    fn from(data: ProcessStopStoringForInsolventUserRequestData) -> Self {
        Self::StopStoringForInsolventUserRequest(data)
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessSubmitProofRequestData {
    pub provider_id: ProviderId,
    pub tick: BlockNumber,
    pub seed: RandomnessOutput,
    pub forest_challenges: Vec<H256>,
    pub checkpoint_challenges: Vec<(H256, Option<TrieRemoveMutation>)>,
}

#[derive(Debug, Clone, EventBusMessage)]
pub struct ProcessSubmitProofRequest {
    pub data: ProcessSubmitProofRequestData,
    pub forest_root_write_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessConfirmStoringRequestData {
    pub confirm_storing_requests: Vec<ConfirmStoringRequest>,
}

#[derive(Debug, Clone, EventBusMessage)]
pub struct ProcessConfirmStoringRequest {
    pub data: ProcessConfirmStoringRequestData,
    pub forest_root_write_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessMspRespondStoringRequestData {
    pub respond_storing_requests: Vec<RespondStorageRequest>,
}

#[derive(Debug, Clone, EventBusMessage)]
pub struct ProcessMspRespondStoringRequest {
    pub data: ProcessMspRespondStoringRequestData,
    pub forest_root_write_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessStopStoringForInsolventUserRequestData {
    pub who: AccountId32,
}

#[derive(Debug, Clone, EventBusMessage)]
pub struct ProcessStopStoringForInsolventUserRequest {
    pub data: ProcessStopStoringForInsolventUserRequestData,
    pub forest_root_write_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

/// Slashable Provider event.
///
/// This event is emitted when a provider is marked as slashable by the runtime.
#[derive(Debug, Clone, EventBusMessage)]
pub struct SlashableProvider {
    pub provider: ProviderId,
    pub next_challenge_deadline: BlockNumber,
}

/// Mutations applied event in a finalised block.
///
/// This event is emitted when a finalised block is received by the Blockchain service,
/// in which there is a `MutationsApplied` event for one of the providers that this node is tracking.
#[derive(Debug, Clone, EventBusMessage)]
pub struct FinalisedTrieRemoveMutationsApplied {
    pub provider_id: ProviderId,
    pub mutations: Vec<(ForestRoot, TrieRemoveMutation)>,
    pub new_root: H256,
}

#[derive(Debug, Clone, EventBusMessage)]
pub struct ProofAccepted {
    pub provider_id: ProviderId,
    pub proofs: KeyProofs,
}

#[derive(Debug, Clone, EventBusMessage)]
pub struct LastChargeableInfoUpdated {
    pub provider_id: ProviderId,
    pub last_chargeable_tick: BlockNumber,
    pub last_chargeable_price_index: Balance,
}

/// User without funds event.
///
/// This event is emitted when a User has been determined as insolvent by the Payment Streams pallet for
/// being unable to pay for their payment streams for a prolonged period of time.
#[derive(Debug, Clone, EventBusMessage)]
pub struct UserWithoutFunds {
    pub who: AccountId32,
}

/// Provider stopped storing for insolvent user event.
///
/// This event is emitted when a provider has stopped storing a file for an insolvent user.
#[derive(Debug, Clone, EventBusMessage)]
pub struct SpStopStoringInsolventUser {
    pub sp_id: ProviderId,
    pub file_key: FileKey,
    pub owner: AccountId32,
    pub location: FileLocation,
    pub new_root: H256,
}

shc_actors_framework::define_event_bus!(
    BlockchainServiceEventBusProvider,
    NewChallengeSeed,
    MultipleNewChallengeSeeds,
    NewStorageRequest,
    AcceptedBspVolunteer,
    ProcessSubmitProofRequest,
    ProcessConfirmStoringRequest,
    ProcessMspRespondStoringRequest,
    ProcessStopStoringForInsolventUserRequest,
    SlashableProvider,
    FinalisedTrieRemoveMutationsApplied,
    ProofAccepted,
    LastChargeableInfoUpdated,
    UserWithoutFunds,
    SpStopStoringInsolventUser
);
