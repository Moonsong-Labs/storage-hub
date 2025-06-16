use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

use codec::{Decode, Encode};
use sc_network::Multiaddr;
use shc_actors_derive::{ActorEvent, ActorEventBus};
use sp_core::H256;
use sp_runtime::AccountId32;

use shc_common::{
    traits::StorageEnableRuntimeConfig,
    types::{
        Balance, BlockNumber, BucketId, CustomChallenge, FileKey, FileLocation, Fingerprint,
        ForestRoot, KeyProofs, PeerIds, ProofsDealerProviderId, ProviderId, RandomnessOutput,
        StorageData, TrieMutation, ValuePropId,
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
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct NewChallengeSeed<Runtime: StorageEnableRuntimeConfig> {
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
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct MultipleNewChallengeSeeds<Runtime: StorageEnableRuntimeConfig> {
    pub provider_id: ProofsDealerProviderId<Runtime>,
    pub seeds: Vec<(BlockNumber<Runtime>, RandomnessOutput<Runtime>)>,
}

/// New storage request event.
///
/// This event is emitted when a new storage request is created on-chain.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct NewStorageRequest<Runtime: StorageEnableRuntimeConfig> {
    /// Account ID of the requester.
    pub who: AccountId32,
    /// Computed file key for the file.
    pub file_key: FileKey,
    /// Bucket ID of the file.
    pub bucket_id: BucketId<Runtime>,
    /// Location of the file (as a file path).
    pub location: FileLocation<Runtime>,
    /// Fingerprint of the file (root hash of the merkelized file).
    pub fingerprint: Fingerprint,
    /// Size of the file.
    pub size: StorageData<Runtime>,
    /// libp2p peer IDs from where the user would send the file.
    pub user_peer_ids: PeerIds<Runtime>,
    /// Block number at which the storage request will expire if not fulfilled.
    pub expires_at: BlockNumber<Runtime>,
}

unsafe impl<Runtime: StorageEnableRuntimeConfig> Send for NewStorageRequest<Runtime> {}

/// MSP stopped storing bucket event.
///
/// This event is emitted when an MSP stops storing a bucket.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct FinalisedMspStoppedStoringBucket<Runtime: StorageEnableRuntimeConfig> {
    /// MSP ID who stopped storing the bucket.
    pub msp_id: ProofsDealerProviderId<Runtime>,
    /// Account ID owner of the bucket.
    pub owner: AccountId32,
    pub bucket_id: BucketId<Runtime>,
}

/// Accepted BSP volunteer event.
///
/// This event is emitted when a BSP volunteer is accepted to store a file.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct AcceptedBspVolunteer<Runtime: StorageEnableRuntimeConfig> {
    pub bsp_id: H256,
    pub bucket_id: BucketId<Runtime>,
    pub location: FileLocation<Runtime>,
    pub fingerprint: Fingerprint,
    pub multiaddresses: Vec<Multiaddr>,
    pub owner: AccountId32,
    pub size: StorageData<Runtime>,
}

unsafe impl<Runtime: StorageEnableRuntimeConfig> Send for AcceptedBspVolunteer<Runtime> {}

#[derive(Debug, Clone, Encode, Decode)]
pub enum ForestWriteLockTaskData<Runtime: StorageEnableRuntimeConfig> {
    SubmitProofRequest(ProcessSubmitProofRequestData<Runtime>),
    ConfirmStoringRequest(ProcessConfirmStoringRequestData),
    MspRespondStorageRequest(ProcessMspRespondStoringRequestData),
    StopStoringForInsolventUserRequest(ProcessStopStoringForInsolventUserRequestData),
    FileDeletionRequest(ProcessFileDeletionRequestData<Runtime>),
}

impl<Runtime: StorageEnableRuntimeConfig> From<ProcessSubmitProofRequestData<Runtime>>
    for ForestWriteLockTaskData<Runtime>
{
    fn from(data: ProcessSubmitProofRequestData<Runtime>) -> Self {
        Self::SubmitProofRequest(data)
    }
}

impl<Runtime: StorageEnableRuntimeConfig> From<ProcessConfirmStoringRequestData>
    for ForestWriteLockTaskData<Runtime>
{
    fn from(data: ProcessConfirmStoringRequestData) -> Self {
        Self::ConfirmStoringRequest(data)
    }
}

impl<Runtime: StorageEnableRuntimeConfig> From<ProcessMspRespondStoringRequestData>
    for ForestWriteLockTaskData<Runtime>
{
    fn from(data: ProcessMspRespondStoringRequestData) -> Self {
        Self::MspRespondStorageRequest(data)
    }
}

impl<Runtime: StorageEnableRuntimeConfig> From<ProcessStopStoringForInsolventUserRequestData>
    for ForestWriteLockTaskData<Runtime>
{
    fn from(data: ProcessStopStoringForInsolventUserRequestData) -> Self {
        Self::StopStoringForInsolventUserRequest(data)
    }
}

impl<Runtime: StorageEnableRuntimeConfig> From<ProcessFileDeletionRequestData<Runtime>>
    for ForestWriteLockTaskData<Runtime>
{
    fn from(data: ProcessFileDeletionRequestData<Runtime>) -> Self {
        Self::FileDeletionRequest(data)
    }
}

/// Data required to build a proof to submit to the runtime.
#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessSubmitProofRequestData<Runtime: StorageEnableRuntimeConfig> {
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
    pub forest_challenges: Vec<H256>,
    /// The checkpoint challenges that the proof to generate has to respond to.
    pub checkpoint_challenges: Vec<CustomChallenge<Runtime>>,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct ProcessSubmitProofRequest<Runtime: StorageEnableRuntimeConfig> {
    pub data: ProcessSubmitProofRequestData<Runtime>,
    pub forest_root_write_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessConfirmStoringRequestData {
    pub confirm_storing_requests: Vec<ConfirmStoringRequest>,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct ProcessConfirmStoringRequest<Runtime: StorageEnableRuntimeConfig> {
    pub data: ProcessConfirmStoringRequestData,
    pub forest_root_write_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    pub _phantom: core::marker::PhantomData<Runtime>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessMspRespondStoringRequestData {
    pub respond_storing_requests: Vec<RespondStorageRequest>,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct ProcessMspRespondStoringRequest<Runtime: StorageEnableRuntimeConfig> {
    pub data: ProcessMspRespondStoringRequestData,
    pub forest_root_write_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    pub _phantom: core::marker::PhantomData<Runtime>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessStopStoringForInsolventUserRequestData {
    pub who: AccountId32,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct ProcessStopStoringForInsolventUserRequest<Runtime: StorageEnableRuntimeConfig> {
    pub data: ProcessStopStoringForInsolventUserRequestData,
    pub forest_root_write_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    pub _phantom: core::marker::PhantomData<Runtime>,
}

/// Slashable Provider event.
///
/// This event is emitted when a provider is marked as slashable by the runtime.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct SlashableProvider<Runtime: StorageEnableRuntimeConfig> {
    pub provider: ProofsDealerProviderId<Runtime>,
    pub next_challenge_deadline: BlockNumber<Runtime>,
}

/// Mutations applied event in a finalised block.
///
/// This event is emitted when a finalised block is received by the Blockchain service,
/// in which there is a `MutationsApplied` event for one of the providers that this node is tracking.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct FinalisedTrieRemoveMutationsApplied<Runtime: StorageEnableRuntimeConfig> {
    pub provider_id: ProofsDealerProviderId<Runtime>,
    pub mutations: Vec<(ForestRoot<Runtime>, TrieMutation)>,
    pub new_root: H256,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct ProofAccepted<Runtime: StorageEnableRuntimeConfig> {
    pub provider_id: ProofsDealerProviderId<Runtime>,
    pub proofs: KeyProofs<Runtime>,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct LastChargeableInfoUpdated<Runtime: StorageEnableRuntimeConfig> {
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
pub struct UserWithoutFunds<Runtime: StorageEnableRuntimeConfig> {
    pub who: AccountId32,
    pub _phantom: core::marker::PhantomData<Runtime>,
}

/// Provider stopped storing for insolvent user event.
///
/// This event is emitted when a provider has stopped storing a file for an insolvent user.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct SpStopStoringInsolventUser<Runtime: StorageEnableRuntimeConfig> {
    pub sp_id: ProofsDealerProviderId<Runtime>,
    pub file_key: FileKey,
    pub owner: AccountId32,
    pub location: FileLocation<Runtime>,
    pub new_root: H256,
}

unsafe impl<Runtime: StorageEnableRuntimeConfig> Send for SpStopStoringInsolventUser<Runtime> {}

/// A MSP stopped storing a bucket for an insolvent user event was finalised.
///
/// This event is emitted when the relay chain block to which a block in which a MSP stopped storing a bucket
/// for an insolvent user event is anchored has been finalised.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct FinalisedMspStopStoringBucketInsolventUser<Runtime: StorageEnableRuntimeConfig> {
    pub msp_id: ProofsDealerProviderId<Runtime>,
    pub bucket_id: BucketId<Runtime>,
}

/// A user has requested to move one of its bucket to a new MSP.
///
/// This event is emitted so the BSP can allow the new MSP to download the files from the bucket.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct MoveBucketRequested<Runtime: StorageEnableRuntimeConfig> {
    pub bucket_id: BucketId<Runtime>,
    pub new_msp_id: ProviderId<Runtime>,
}

/// A user has requested to move one of its buckets to a new MSP which matches a currently managed MSP.
///
/// This event is emitted so the MSP can verify if it can download all files of the bucket from BSPs,
/// respond to the user accepting the request, download the bucket's files and insert the bucket into their forest.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct MoveBucketRequestedForMsp<Runtime: StorageEnableRuntimeConfig> {
    pub bucket_id: BucketId<Runtime>,
    pub value_prop_id: ValuePropId<Runtime>,
}

/// The new MSP that the user chose to store a bucket has rejected the move request.
///
/// This event is emitted so the BSPs can stop allowing the new MSP to download the files
/// from the bucket.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct MoveBucketRejected<Runtime: StorageEnableRuntimeConfig> {
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
pub struct MoveBucketAccepted<Runtime: StorageEnableRuntimeConfig> {
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
pub struct MoveBucketExpired<Runtime: StorageEnableRuntimeConfig> {
    pub bucket_id: BucketId<Runtime>,
}

/// BSP stopped storing a specific file.
///
/// This event is emitted when a BSP confirm stop storing a file.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct BspConfirmStoppedStoring<Runtime: StorageEnableRuntimeConfig> {
    pub bsp_id: H256,
    pub file_key: FileKey,
    pub new_root: H256,
    pub _phantom: core::marker::PhantomData<Runtime>,
}

/// Delete file event in a finalised block.
///
/// This event is emitted when a finalised block is received by the Blockchain service,
/// in which there is a `BspConfirmStoppedStoring` event for one of the providers that this node is tracking.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct FinalisedBspConfirmStoppedStoring<Runtime: StorageEnableRuntimeConfig> {
    pub bsp_id: H256,
    pub file_key: FileKey,
    pub new_root: H256,
    pub _phantom: core::marker::PhantomData<Runtime>,
}

/// Notify period event.
///
/// This event is emitted when a X amount of block has passed. It is configured at the start of the service.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct NotifyPeriod<Runtime: StorageEnableRuntimeConfig> {
    pub _phantom: core::marker::PhantomData<Runtime>,
}

/// File deletion request event.
#[derive(Debug, Clone, Encode, Decode, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct FileDeletionRequest<Runtime: StorageEnableRuntimeConfig> {
    /// Account ID of the user that requested the file deletion.
    pub user: AccountId32,
    /// File key that was requested to be deleted.
    pub file_key: FileKey,
    /// File size of the file that was requested to be deleted.
    pub file_size: StorageData<Runtime>,
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
pub struct ProcessFileDeletionRequestData<Runtime: StorageEnableRuntimeConfig> {
    pub file_deletion_requests: Vec<FileDeletionRequestType<Runtime>>,
}

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct ProcessFileDeletionRequest<Runtime: StorageEnableRuntimeConfig> {
    pub data: ProcessFileDeletionRequestData<Runtime>,
    pub forest_root_write_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

/// Finalised proof submitted by an MSP for a pending file deletion request event.
///
/// Fields are identical
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct FinalisedProofSubmittedForPendingFileDeletionRequest<Runtime: StorageEnableRuntimeConfig>
{
    /// Account ID of the user that requested the file deletion.
    pub user: AccountId32,
    /// File key that was requested to be deleted.
    pub file_key: FileKey,
    /// File size of the file that was requested to be deleted.
    pub file_size: StorageData<Runtime>,
    /// Bucket ID in which the file key belongs to.
    pub bucket_id: BucketId<Runtime>,
    /// The MSP ID that provided the proof of inclusion for a pending file deletion request.
    pub msp_id: ProofsDealerProviderId<Runtime>,
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
pub struct StartMovedBucketDownload<Runtime: StorageEnableRuntimeConfig> {
    pub bucket_id: BucketId<Runtime>,
    pub value_prop_id: ValuePropId<Runtime>,
}

/// Event emitted when a bucket is moved away from the current MSP to a new MSP.
/// This event is emitted by the Blockchain Service when it processes a MoveBucketAccepted event
/// on-chain, in a finalised block, and the current node is the old MSP that is losing the bucket.
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct FinalisedBucketMovedAway<Runtime: StorageEnableRuntimeConfig> {
    pub bucket_id: BucketId<Runtime>,
    pub old_msp_id: ProviderId<Runtime>,
    pub new_msp_id: ProviderId<Runtime>,
}

/// The event bus provider for the BlockchainService actor.
///
/// It holds the event buses for the different events that the BlockchainService actor
/// can emit.
#[ActorEventBus("blockchain_service")]
pub struct BlockchainServiceEventBusProvider;
