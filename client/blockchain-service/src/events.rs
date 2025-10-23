use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

use codec::{Decode, Encode};
use sc_network::Multiaddr;
use shc_actors_derive::{ActorEvent, ActorEventBus};

use shc_common::{
    traits::StorageEnableRuntime,
    types::{
        BackupStorageProviderId, Balance, BlockNumber, BucketId, CustomChallenge, FileKey,
        FileLocation, Fingerprint, ForestRoot, KeyProofs, PeerIds, ProofsDealerProviderId,
        ProviderId, RandomnessOutput, StorageDataUnit, TickNumber, TrieMutation, ValuePropId,
    },
};

use crate::types::{
    ConfirmStoringRequest, FileDeletionRequest as FileDeletionRequestType, RespondStorageRequest,
};

// TODO: Add the events from the `pallet-cr-randomness` here to process them in the BlockchainService.

/// New random challenge emitted by the StorageHub runtime.
///
/// This event is emitted when there's a new random challenge seed that affects this
/// BSP. In other words, it only pays attention to the random seeds in the challenge
/// period of this BSP.
#[derive(Debug, Clone, Encode, Decode, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct NewChallengeSeed<Runtime: StorageEnableRuntime> {
    pub provider_id: ProofsDealerProviderId<Runtime>,
    pub tick: BlockNumber<Runtime>,
    pub seed: RandomnessOutput<Runtime>,
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
pub struct MultipleNewChallengeSeeds<Runtime: StorageEnableRuntime> {
    pub provider_id: ProofsDealerProviderId<Runtime>,
    pub seeds: Vec<(BlockNumber<Runtime>, RandomnessOutput<Runtime>)>,
}

/// New storage request event.
///
/// This event is emitted when a new storage request is created on-chain.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct NewStorageRequest<Runtime: StorageEnableRuntime> {
    /// Account ID of the requester.
    pub who: Runtime::AccountId,
    /// Computed file key for the file.
    pub file_key: FileKey,
    /// Bucket ID of the file.
    pub bucket_id: BucketId<Runtime>,
    /// Location of the file (as a file path).
    pub location: FileLocation<Runtime>,
    /// Fingerprint of the file (root hash of the merkelized file).
    pub fingerprint: Fingerprint,
    /// Size of the file.
    pub size: StorageDataUnit<Runtime>,
    /// libp2p peer IDs from where the user would send the file.
    pub user_peer_ids: PeerIds<Runtime>,
    /// Block number at which the storage request will expire if not fulfilled.
    pub expires_at: TickNumber<Runtime>,
}

/// MSP stopped storing bucket event.
///
/// This event is emitted when an MSP stops storing a bucket.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct FinalisedMspStoppedStoringBucket<Runtime: StorageEnableRuntime> {
    /// MSP ID who stopped storing the bucket.
    pub msp_id: ProofsDealerProviderId<Runtime>,
    /// Account ID owner of the bucket.
    pub owner: Runtime::AccountId,
    pub bucket_id: BucketId<Runtime>,
}

/// Accepted BSP volunteer event.
///
/// This event is emitted when a BSP volunteer is accepted to store a file.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct AcceptedBspVolunteer<Runtime: StorageEnableRuntime> {
    pub bsp_id: Runtime::Hash,
    pub bucket_id: BucketId<Runtime>,
    pub location: FileLocation<Runtime>,
    pub fingerprint: Fingerprint,
    pub multiaddresses: Vec<Multiaddr>,
    pub owner: Runtime::AccountId,
    pub size: StorageDataUnit<Runtime>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub enum ForestWriteLockTaskData<Runtime: StorageEnableRuntime> {
    SubmitProofRequest(ProcessSubmitProofRequestData<Runtime>),
    ConfirmStoringRequest(ProcessConfirmStoringRequestData<Runtime>),
    MspRespondStorageRequest(ProcessMspRespondStoringRequestData<Runtime>),
    StopStoringForInsolventUserRequest(ProcessStopStoringForInsolventUserRequestData<Runtime>),
    FileDeletionRequest(ProcessFileDeletionRequestData<Runtime>),
}

impl<Runtime: StorageEnableRuntime> From<ProcessSubmitProofRequestData<Runtime>>
    for ForestWriteLockTaskData<Runtime>
{
    fn from(data: ProcessSubmitProofRequestData<Runtime>) -> Self {
        Self::SubmitProofRequest(data)
    }
}

impl<Runtime: StorageEnableRuntime> From<ProcessConfirmStoringRequestData<Runtime>>
    for ForestWriteLockTaskData<Runtime>
{
    fn from(data: ProcessConfirmStoringRequestData<Runtime>) -> Self {
        Self::ConfirmStoringRequest(data)
    }
}

impl<Runtime: StorageEnableRuntime> From<ProcessMspRespondStoringRequestData<Runtime>>
    for ForestWriteLockTaskData<Runtime>
{
    fn from(data: ProcessMspRespondStoringRequestData<Runtime>) -> Self {
        Self::MspRespondStorageRequest(data)
    }
}

impl<Runtime: StorageEnableRuntime> From<ProcessStopStoringForInsolventUserRequestData<Runtime>>
    for ForestWriteLockTaskData<Runtime>
{
    fn from(data: ProcessStopStoringForInsolventUserRequestData<Runtime>) -> Self {
        Self::StopStoringForInsolventUserRequest(data)
    }
}

impl<Runtime: StorageEnableRuntime> From<ProcessFileDeletionRequestData<Runtime>>
    for ForestWriteLockTaskData<Runtime>
{
    fn from(data: ProcessFileDeletionRequestData<Runtime>) -> Self {
        Self::FileDeletionRequest(data)
    }
}

/// Data required to build a proof to submit to the runtime.
#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessSubmitProofRequestData<Runtime: StorageEnableRuntime> {
    /// The Provider ID of the BSP that is submitting the proof.
    pub provider_id: ProofsDealerProviderId<Runtime>,
    /// The tick for which the proof is being built.
    ///
    /// This tick should be the tick where [`Self::seed`] was generated.
    pub tick: BlockNumber<Runtime>,
    /// The seed that was used to generate the challenges for this proof.
    pub seed: RandomnessOutput<Runtime>,
    /// All the Forest challenges that the proof to generate has to respond to.
    ///
    /// This includes the [`Self::checkpoint_challenges`].
    pub forest_challenges: Vec<Runtime::Hash>,
    /// The checkpoint challenges that the proof to generate has to respond to.
    pub checkpoint_challenges: Vec<CustomChallenge<Runtime>>,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct ProcessSubmitProofRequest<Runtime: StorageEnableRuntime> {
    pub data: ProcessSubmitProofRequestData<Runtime>,
    pub forest_root_write_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessConfirmStoringRequestData<Runtime: StorageEnableRuntime> {
    pub confirm_storing_requests: Vec<ConfirmStoringRequest<Runtime>>,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct ProcessConfirmStoringRequest<Runtime: StorageEnableRuntime> {
    pub data: ProcessConfirmStoringRequestData<Runtime>,
    pub forest_root_write_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessMspRespondStoringRequestData<Runtime: StorageEnableRuntime> {
    pub respond_storing_requests: Vec<RespondStorageRequest<Runtime>>,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct ProcessMspRespondStoringRequest<Runtime: StorageEnableRuntime> {
    pub data: ProcessMspRespondStoringRequestData<Runtime>,
    pub forest_root_write_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessStopStoringForInsolventUserRequestData<Runtime: StorageEnableRuntime> {
    pub who: Runtime::AccountId,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct ProcessStopStoringForInsolventUserRequest<Runtime: StorageEnableRuntime> {
    pub data: ProcessStopStoringForInsolventUserRequestData<Runtime>,
    pub forest_root_write_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

/// Slashable Provider event.
///
/// This event is emitted when a provider is marked as slashable by the runtime.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct SlashableProvider<Runtime: StorageEnableRuntime> {
    pub provider: ProofsDealerProviderId<Runtime>,
    pub next_challenge_deadline: BlockNumber<Runtime>,
}

/// Mutations applied event in a finalised block, for a BSP.
///
/// This event is emitted when a finalised block is received by the Blockchain service,
/// in which there is a `MutationsAppliedForProvider` event for the BSP that this node is tracking.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct FinalisedTrieRemoveMutationsAppliedForBsp<Runtime: StorageEnableRuntime> {
    pub provider_id: ProofsDealerProviderId<Runtime>,
    pub mutations: Vec<(ForestRoot<Runtime>, TrieMutation)>,
    pub new_root: Runtime::Hash,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct ProofAccepted<Runtime: StorageEnableRuntime> {
    pub provider_id: ProofsDealerProviderId<Runtime>,
    pub proofs: KeyProofs<Runtime>,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct LastChargeableInfoUpdated<Runtime: StorageEnableRuntime> {
    pub provider_id: ProofsDealerProviderId<Runtime>,
    pub last_chargeable_tick: BlockNumber<Runtime>,
    pub last_chargeable_price_index: Balance<Runtime>,
}

/// User without funds event.
///
/// This event is emitted when a User has been determined as insolvent by the Payment Streams pallet for
/// being unable to pay for their payment streams for a prolonged period of time.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct UserWithoutFunds<Runtime: StorageEnableRuntime> {
    pub who: Runtime::AccountId,
}

/// Provider stopped storing for insolvent user event.
///
/// This event is emitted when a provider has stopped storing a file for an insolvent user.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct SpStopStoringInsolventUser<Runtime: StorageEnableRuntime> {
    pub sp_id: ProofsDealerProviderId<Runtime>,
    pub file_key: FileKey,
    pub owner: Runtime::AccountId,
    pub location: FileLocation<Runtime>,
    pub new_root: Runtime::Hash,
}

/// A MSP stopped storing a bucket for an insolvent user event was finalised.
///
/// This event is emitted when the relay chain block to which a block in which a MSP stopped storing a bucket
/// for an insolvent user event is anchored has been finalised.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct FinalisedMspStopStoringBucketInsolventUser<Runtime: StorageEnableRuntime> {
    pub msp_id: ProofsDealerProviderId<Runtime>,
    pub bucket_id: BucketId<Runtime>,
}

/// A user has requested to move one of its bucket to a new MSP.
///
/// This event is emitted so the BSP can allow the new MSP to download the files from the bucket.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct MoveBucketRequested<Runtime: StorageEnableRuntime> {
    pub bucket_id: BucketId<Runtime>,
    pub new_msp_id: ProviderId<Runtime>,
}

/// A user has requested to move one of its buckets to a new MSP which matches a currently managed MSP.
///
/// This event is emitted so the MSP can verify if it can download all files of the bucket from BSPs,
/// respond to the user accepting the request, download the bucket's files and insert the bucket into their forest.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct MoveBucketRequestedForMsp<Runtime: StorageEnableRuntime> {
    pub bucket_id: BucketId<Runtime>,
    pub value_prop_id: ValuePropId<Runtime>,
}

/// The new MSP that the user chose to store a bucket has rejected the move request.
///
/// This event is emitted so the BSPs can stop allowing the new MSP to download the files
/// from the bucket.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct MoveBucketRejected<Runtime: StorageEnableRuntime> {
    pub bucket_id: BucketId<Runtime>,
    pub old_msp_id: Option<ProviderId<Runtime>>,
    pub new_msp_id: ProviderId<Runtime>,
}

/// The new MSP that the user chose to store a bucket has accepted the move request.
///
/// This event is emitted so the BSPs know that the new MSP is allowed to download the files
/// from the bucket.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct MoveBucketAccepted<Runtime: StorageEnableRuntime> {
    pub bucket_id: BucketId<Runtime>,
    pub old_msp_id: Option<ProviderId<Runtime>>,
    pub new_msp_id: ProviderId<Runtime>,
    pub value_prop_id: ValuePropId<Runtime>,
}

/// The move bucket request has expired without a response from the new MSP.
///
/// This event is emitted so the BSPs can stop allowing the new MSP to download the files
/// from the bucket.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct MoveBucketExpired<Runtime: StorageEnableRuntime> {
    pub bucket_id: BucketId<Runtime>,
}

/// BSP stopped storing a specific file.
///
/// This event is emitted when a BSP confirm stop storing a file.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct BspConfirmStoppedStoring<Runtime: StorageEnableRuntime> {
    pub bsp_id: Runtime::Hash,
    pub file_key: FileKey,
    pub new_root: Runtime::Hash,
}

/// Delete file event in a finalised block, for a BSP.
///
/// This event is emitted when a finalised block is received by the Blockchain service,
/// in which there is a `BspConfirmStoppedStoring` event for one of the providers that this node is tracking.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct FinalisedBspConfirmStoppedStoring<Runtime: StorageEnableRuntime> {
    pub bsp_id: Runtime::Hash,
    pub file_key: FileKey,
    pub new_root: Runtime::Hash,
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
pub struct FileDeletionRequest<Runtime: StorageEnableRuntime> {
    /// Account ID of the user that requested the file deletion.
    pub user: Runtime::AccountId,
    /// File key that was requested to be deleted.
    pub file_key: FileKey,
    /// File size of the file that was requested to be deleted.
    pub file_size: StorageDataUnit<Runtime>,
    /// Bucket ID in which the file key belongs to.
    pub bucket_id: BucketId<Runtime>,
    /// The MSP ID that provided the proof of inclusion for a pending file deletion request.
    pub msp_id: ProofsDealerProviderId<Runtime>,
    /// Whether a proof of inclusion was provided by the user.
    ///
    /// This means that the file key requested to be deleted was included in the user's submitted inclusion forest proof.
    /// The key would have been
    pub proof_of_inclusion: bool,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessFileDeletionRequestData<Runtime: StorageEnableRuntime> {
    pub file_deletion_requests: Vec<FileDeletionRequestType<Runtime>>,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct ProcessFileDeletionRequest<Runtime: StorageEnableRuntime> {
    pub data: ProcessFileDeletionRequestData<Runtime>,
    pub forest_root_write_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

/// Event emitted when a bucket move is confirmed on-chain and the download process should start.
/// This event is emitted by the blockchain service when it receives a MoveBucketAccepted event
/// and the current node is the new MSP.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct StartMovedBucketDownload<Runtime: StorageEnableRuntime> {
    pub bucket_id: BucketId<Runtime>,
    pub value_prop_id: ValuePropId<Runtime>,
}

/// Event emitted when a bucket is moved away from the current MSP to a new MSP.
/// This event is emitted by the Blockchain Service when it processes a MoveBucketAccepted event
/// on-chain, in a finalised block, and the current node is the old MSP that is losing the bucket.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct FinalisedBucketMovedAway<Runtime: StorageEnableRuntime> {
    pub bucket_id: BucketId<Runtime>,
    pub old_msp_id: ProviderId<Runtime>,
    pub new_msp_id: ProviderId<Runtime>,
}

/// Event emitted when a file needs to be distributed to a BSP who volunteered to store it.
/// and the current node is an MSP configured to distribute files to BSPs.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct DistributeFileToBsp<Runtime: StorageEnableRuntime> {
    pub file_key: FileKey,
    pub bsp_id: BackupStorageProviderId<Runtime>,
}

/// The event bus provider for the BlockchainService actor.
///
/// It holds the event buses for the different events that the BlockchainService actor
/// can emit.
#[ActorEventBus("blockchain_service")]
pub struct BlockchainServiceEventBusProvider<Runtime: StorageEnableRuntime>;
