use codec::{Decode, Encode};
use sc_network::Multiaddr;
use shc_actors_derive::{ActorEvent, ActorEventBus};
use sp_core::H256;
use sp_runtime::AccountId32;

use shc_common::types::{
    Balance, BlockNumber, BucketId, CustomChallenge, FileKey, FileLocation, Fingerprint,
    ForestRoot, KeyProofs, PeerIds, ProofsDealerProviderId, ProviderId, RandomnessOutput,
    StorageData, TrieMutation, ValuePropId,
};

use crate::{
    lock_manager::ForestRootWriteTicket,
    types::{
        ConfirmStoringRequest, FileDeletionRequest as FileDeletionRequestType,
        RespondStorageRequest,
    },
};

// TODO: Add the events from the `pallet-cr-randomness` here to process them in the BlockchainService.

/// New random challenge emitted by the StorageHub runtime.
///
/// This event is emitted when there's a new random challenge seed that affects this
/// BSP. In other words, it only pays attention to the random seeds in the challenge
/// period of this BSP.
#[derive(Debug, Clone, Encode, Decode, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct NewChallengeSeed {
    pub provider_id: ProofsDealerProviderId,
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
#[derive(Debug, Clone, Encode, Decode, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct MultipleNewChallengeSeeds {
    pub provider_id: ProofsDealerProviderId,
    pub seeds: Vec<(BlockNumber, RandomnessOutput)>,
}

/// New storage request event.
///
/// This event is emitted when a new storage request is created on-chain.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
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
    /// Block number at which the storage request will expire if not fulfilled.
    pub expires_at: BlockNumber,
}

/// MSP stopped storing bucket event.
///
/// This event is emitted when an MSP stops storing a bucket.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct FinalisedMspStoppedStoringBucket {
    /// MSP ID who stopped storing the bucket.
    pub msp_id: ProofsDealerProviderId,
    /// Account ID owner of the bucket.
    pub owner: AccountId32,
    pub bucket_id: BucketId,
}

/// Accepted BSP volunteer event.
///
/// This event is emitted when a BSP volunteer is accepted to store a file.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
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
    FileDeletionRequest(ProcessFileDeletionRequestData),
}

impl ForestWriteLockTaskData {
    /// Returns the priority value for this task type
    pub fn priority(&self) -> crate::lock_manager::PriorityValue {
        match self {
            Self::SubmitProofRequest(_) => crate::handler::Priorities::SUBMIT_PROOF,
            Self::ConfirmStoringRequest(_) => crate::handler::Priorities::CONFIRM_STORING,
            Self::FileDeletionRequest(_) => crate::handler::Priorities::FILE_DELETION,
            Self::MspRespondStorageRequest(_) => crate::handler::Priorities::RESPOND_STORAGE,
            Self::StopStoringForInsolventUserRequest(_) => crate::handler::Priorities::STOP_STORING,
        }
    }
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

impl From<ProcessFileDeletionRequestData> for ForestWriteLockTaskData {
    fn from(data: ProcessFileDeletionRequestData) -> Self {
        Self::FileDeletionRequest(data)
    }
}

/// Data required to build a proof to submit to the runtime.
#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessSubmitProofRequestData {
    /// The Provider ID of the BSP that is submitting the proof.
    pub provider_id: ProofsDealerProviderId,
    /// The tick for which the proof is being built.
    ///
    /// This tick should be the tick where [`Self::seed`] was generated.
    pub tick: BlockNumber,
    /// The seed that was used to generate the challenges for this proof.
    pub seed: RandomnessOutput,
    /// All the Forest challenges that the proof to generate has to respond to.
    ///
    /// This includes the [`Self::checkpoint_challenges`].
    pub forest_challenges: Vec<H256>,
    /// The checkpoint challenges that the proof to generate has to respond to.
    pub checkpoint_challenges: Vec<CustomChallenge>,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct ProcessSubmitProofRequest {
    pub data: ProcessSubmitProofRequestData,
    pub ticket: ForestRootWriteTicket,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessConfirmStoringRequestData {
    pub confirm_storing_requests: Vec<ConfirmStoringRequest>,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct ProcessConfirmStoringRequest {
    pub data: ProcessConfirmStoringRequestData,
    pub ticket: ForestRootWriteTicket,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessMspRespondStoringRequestData {
    pub respond_storing_requests: Vec<RespondStorageRequest>,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct ProcessMspRespondStoringRequest {
    pub data: ProcessMspRespondStoringRequestData,
    pub ticket: ForestRootWriteTicket,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessStopStoringForInsolventUserRequestData {
    pub who: AccountId32,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct ProcessStopStoringForInsolventUserRequest {
    pub data: ProcessStopStoringForInsolventUserRequestData,
    pub ticket: ForestRootWriteTicket,
}

/// Slashable Provider event.
///
/// This event is emitted when a provider is marked as slashable by the runtime.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct SlashableProvider {
    pub provider: ProofsDealerProviderId,
    pub next_challenge_deadline: BlockNumber,
}

/// Mutations applied event in a finalised block.
///
/// This event is emitted when a finalised block is received by the Blockchain service,
/// in which there is a `MutationsApplied` event for one of the providers that this node is tracking.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct FinalisedTrieRemoveMutationsApplied {
    pub provider_id: ProofsDealerProviderId,
    pub mutations: Vec<(ForestRoot, TrieMutation)>,
    pub new_root: H256,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct ProofAccepted {
    pub provider_id: ProofsDealerProviderId,
    pub proofs: KeyProofs,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct LastChargeableInfoUpdated {
    pub provider_id: ProofsDealerProviderId,
    pub last_chargeable_tick: BlockNumber,
    pub last_chargeable_price_index: Balance,
}

/// User without funds event.
///
/// This event is emitted when a User has been determined as insolvent by the Payment Streams pallet for
/// being unable to pay for their payment streams for a prolonged period of time.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct UserWithoutFunds {
    pub who: AccountId32,
}

/// Provider stopped storing for insolvent user event.
///
/// This event is emitted when a provider has stopped storing a file for an insolvent user.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct SpStopStoringInsolventUser {
    pub sp_id: ProofsDealerProviderId,
    pub file_key: FileKey,
    pub owner: AccountId32,
    pub location: FileLocation,
    pub new_root: H256,
}

/// A MSP stopped storing a bucket for an insolvent user event was finalised.
///
/// This event is emitted when the relay chain block to which a block in which a MSP stopped storing a bucket
/// for an insolvent user event is anchored has been finalised.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct FinalisedMspStopStoringBucketInsolventUser {
    pub msp_id: ProofsDealerProviderId,
    pub bucket_id: BucketId,
}

/// A user has requested to move one of its bucket to a new MSP.
///
/// This event is emitted so the BSP can allow the new MSP to download the files from the bucket.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct MoveBucketRequested {
    pub bucket_id: BucketId,
    pub new_msp_id: ProviderId,
}

/// A user has requested to move one of its buckets to a new MSP which matches a currently managed MSP.
///
/// This event is emitted so the MSP can verify if it can download all files of the bucket from BSPs,
/// respond to the user accepting the request, download the bucket's files and insert the bucket into their forest.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct MoveBucketRequestedForMsp {
    pub bucket_id: BucketId,
    pub value_prop_id: ValuePropId,
}

/// The new MSP that the user chose to store a bucket has rejected the move request.
///
/// This event is emitted so the BSPs can stop allowing the new MSP to download the files
/// from the bucket.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct MoveBucketRejected {
    pub bucket_id: BucketId,
    pub old_msp_id: Option<ProviderId>,
    pub new_msp_id: ProviderId,
}

/// The new MSP that the user chose to store a bucket has accepted the move request.
///
/// This event is emitted so the BSPs know that the new MSP is allowed to download the files
/// from the bucket.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct MoveBucketAccepted {
    pub bucket_id: BucketId,
    pub old_msp_id: Option<ProviderId>,
    pub new_msp_id: ProviderId,
    pub value_prop_id: ValuePropId,
}

/// The move bucket request has expired without a response from the new MSP.
///
/// This event is emitted so the BSPs can stop allowing the new MSP to download the files
/// from the bucket.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct MoveBucketExpired {
    pub bucket_id: BucketId,
}

/// BSP stopped storing a specific file.
///
/// This event is emitted when a BSP confirm stop storing a file.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct BspConfirmStoppedStoring {
    pub bsp_id: H256,
    pub file_key: FileKey,
    pub new_root: H256,
}

/// Delete file event in a finalised block.
///
/// This event is emitted when a finalised block is received by the Blockchain service,
/// in which there is a `BspConfirmStoppedStoring` event for one of the providers that this node is tracking.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct FinalisedBspConfirmStoppedStoring {
    pub bsp_id: H256,
    pub file_key: FileKey,
    pub new_root: H256,
}

/// Notify period event.
///
/// This event is emitted when a X amount of block has passed. It is configured at the start of the service.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct NotifyPeriod {}

/// File deletion request event.
#[derive(Debug, Clone, Encode, Decode, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct FileDeletionRequest {
    /// Account ID of the user that requested the file deletion.
    pub user: AccountId32,
    /// File key that was requested to be deleted.
    pub file_key: FileKey,
    /// File size of the file that was requested to be deleted.
    pub file_size: StorageData,
    /// Bucket ID in which the file key belongs to.
    pub bucket_id: BucketId,
    /// The MSP ID that provided the proof of inclusion for a pending file deletion request.
    pub msp_id: ProofsDealerProviderId,
    /// Whether a proof of inclusion was provided by the user.
    ///
    /// This means that the file key requested to be deleted was included in the user's submitted inclusion forest proof.
    /// The key would have been
    pub proof_of_inclusion: bool,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessFileDeletionRequestData {
    pub file_deletion_requests: Vec<FileDeletionRequestType>,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct ProcessFileDeletionRequest {
    pub data: ProcessFileDeletionRequestData,
    pub ticket: ForestRootWriteTicket,
}

/// Finalised proof submitted by an MSP for a pending file deletion request event.
///
/// Fields are identical
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct FinalisedProofSubmittedForPendingFileDeletionRequest {
    /// Account ID of the user that requested the file deletion.
    pub user: AccountId32,
    /// File key that was requested to be deleted.
    pub file_key: FileKey,
    /// File size of the file that was requested to be deleted.
    pub file_size: StorageData,
    /// Bucket ID in which the file key belongs to.
    pub bucket_id: BucketId,
    /// The MSP ID that provided the proof of inclusion for a pending file deletion request.
    pub msp_id: ProofsDealerProviderId,
    /// Whether a proof of inclusion was provided by the MSP.
    ///
    /// This means that the file key requested to be deleted was responded to by the MSP with an inclusion forest proof,
    /// which would have deleted the file key from the bucket's forest.
    pub proof_of_inclusion: bool,
}

/// Event emitted when a bucket move is confirmed on-chain and the download process should start.
/// This event is emitted by the blockchain service when it receives a MoveBucketAccepted event
/// and the current node is the new MSP.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct StartMovedBucketDownload {
    pub bucket_id: BucketId,
    pub value_prop_id: ValuePropId,
}

/// Event emitted when a bucket is moved away from the current MSP to a new MSP.
/// This event is emitted by the Blockchain Service when it processes a MoveBucketAccepted event
/// on-chain, in a finalised block, and the current node is the old MSP that is losing the bucket.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct FinalisedBucketMovedAway {
    pub bucket_id: BucketId,
    pub old_msp_id: ProviderId,
    pub new_msp_id: ProviderId,
}

/// The event bus provider for the BlockchainService actor.
///
/// It holds the event buses for the different events that the BlockchainService actor
/// can emit.
#[ActorEventBus("blockchain_service")]
pub struct BlockchainServiceEventBusProvider;
