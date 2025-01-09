use codec::{Decode, Encode};
use sc_network::Multiaddr;
use shc_actors_framework::event_bus::{EventBus, EventBusMessage, ProvidesEventBus};
use shc_common::types::{
    Balance, BlockNumber, BucketId, FileKey, FileLocation, Fingerprint, ForestRoot, KeyProofs,
    PeerIds, ProofsDealerProviderId, ProviderId, RandomnessOutput, StorageData, TrieMutation,
    TrieRemoveMutation,
};
use sp_core::H256;
use sp_runtime::AccountId32;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

use crate::types::{ConfirmStoringRequest, RespondStorageRequest};

// TODO: Add the events from the `pallet-cr-randomness` here to process them in the BlockchainService.

/// New random challenge emitted by the StorageHub runtime.
///
/// This event is emitted when there's a new random challenge seed that affects this
/// BSP. In other words, it only pays attention to the random seeds in the challenge
/// period of this BSP.
#[derive(Debug, Clone, Encode, Decode)]
pub struct NewChallengeSeed {
    pub provider_id: ProofsDealerProviderId,
    pub tick: BlockNumber,
    pub seed: RandomnessOutput,
}

impl EventBusMessage for NewChallengeSeed {}

/// Multiple new challenge seeds that have to be responded in order.
///
/// This event is emitted when catching up to proof submissions, and there are multiple
/// new challenge seeds that have to be responded in order.
/// The `seeds` vector is expected to be sorted in ascending order, where the first element
/// is the seed that should be responded to first, and the last element is the seed that
/// should be responded to last.
#[derive(Debug, Clone, Encode, Decode)]
pub struct MultipleNewChallengeSeeds {
    pub provider_id: ProofsDealerProviderId,
    pub seeds: Vec<(BlockNumber, RandomnessOutput)>,
}

impl EventBusMessage for MultipleNewChallengeSeeds {}

/// New storage request event.
///
/// This event is emitted when a new storage request is created on-chain.
#[derive(Debug, Clone)]
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

impl EventBusMessage for NewStorageRequest {}

/// MSP stopped storing bucket event.
///
/// This event is emitted when an MSP stops storing a bucket.
#[derive(Debug, Clone)]
pub struct FinalisedMspStoppedStoringBucket {
    /// MSP ID who stopped storing the bucket.
    pub msp_id: ProofsDealerProviderId,
    /// Account ID owner of the bucket.
    pub owner: AccountId32,
    pub bucket_id: BucketId,
}

impl EventBusMessage for FinalisedMspStoppedStoringBucket {}

/// Accepted BSP volunteer event.
///
/// This event is emitted when a BSP volunteer is accepted to store a file.
#[derive(Debug, Clone)]
pub struct AcceptedBspVolunteer {
    pub bsp_id: H256,
    pub bucket_id: BucketId,
    pub location: FileLocation,
    pub fingerprint: Fingerprint,
    pub multiaddresses: Vec<Multiaddr>,
    pub owner: AccountId32,
    pub size: StorageData,
}

impl EventBusMessage for AcceptedBspVolunteer {}

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
    pub provider_id: ProofsDealerProviderId,
    pub tick: BlockNumber,
    pub seed: RandomnessOutput,
    pub forest_challenges: Vec<H256>,
    pub checkpoint_challenges: Vec<(H256, Option<TrieRemoveMutation>)>,
}

#[derive(Debug, Clone)]
pub struct ProcessSubmitProofRequest {
    pub data: ProcessSubmitProofRequestData,
    pub forest_root_write_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl EventBusMessage for ProcessSubmitProofRequest {}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessConfirmStoringRequestData {
    pub confirm_storing_requests: Vec<ConfirmStoringRequest>,
}

#[derive(Debug, Clone)]
pub struct ProcessConfirmStoringRequest {
    pub data: ProcessConfirmStoringRequestData,
    pub forest_root_write_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl EventBusMessage for ProcessConfirmStoringRequest {}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessMspRespondStoringRequestData {
    pub respond_storing_requests: Vec<RespondStorageRequest>,
}

#[derive(Debug, Clone)]
pub struct ProcessMspRespondStoringRequest {
    pub data: ProcessMspRespondStoringRequestData,
    pub forest_root_write_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl EventBusMessage for ProcessMspRespondStoringRequest {}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ProcessStopStoringForInsolventUserRequestData {
    pub who: AccountId32,
}

#[derive(Debug, Clone)]
pub struct ProcessStopStoringForInsolventUserRequest {
    pub data: ProcessStopStoringForInsolventUserRequestData,
    pub forest_root_write_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl EventBusMessage for ProcessStopStoringForInsolventUserRequest {}

/// Slashable Provider event.
///
/// This event is emitted when a provider is marked as slashable by the runtime.
#[derive(Debug, Clone)]
pub struct SlashableProvider {
    pub provider: ProofsDealerProviderId,
    pub next_challenge_deadline: BlockNumber,
}

impl EventBusMessage for SlashableProvider {}

/// Mutations applied event in a finalised block.
///
/// This event is emitted when a finalised block is received by the Blockchain service,
/// in which there is a `MutationsApplied` event for one of the providers that this node is tracking.
#[derive(Debug, Clone)]
pub struct FinalisedTrieRemoveMutationsApplied {
    pub provider_id: ProofsDealerProviderId,
    pub mutations: Vec<(ForestRoot, TrieMutation)>,
    pub new_root: H256,
}

impl EventBusMessage for FinalisedTrieRemoveMutationsApplied {}

#[derive(Debug, Clone)]
pub struct ProofAccepted {
    pub provider_id: ProofsDealerProviderId,
    pub proofs: KeyProofs,
}

impl EventBusMessage for ProofAccepted {}

#[derive(Debug, Clone)]
pub struct LastChargeableInfoUpdated {
    pub provider_id: ProofsDealerProviderId,
    pub last_chargeable_tick: BlockNumber,
    pub last_chargeable_price_index: Balance,
}

impl EventBusMessage for LastChargeableInfoUpdated {}

/// User without funds event.
///
/// This event is emitted when a User has been determined as insolvent by the Payment Streams pallet for
/// being unable to pay for their payment streams for a prolonged period of time.
#[derive(Debug, Clone)]
pub struct UserWithoutFunds {
    pub who: AccountId32,
}
impl EventBusMessage for UserWithoutFunds {}

/// Provider stopped storing for insolvent user event.
///
/// This event is emitted when a provider has stopped storing a file for an insolvent user.
#[derive(Debug, Clone)]
pub struct SpStopStoringInsolventUser {
    pub sp_id: ProofsDealerProviderId,
    pub file_key: FileKey,
    pub owner: AccountId32,
    pub location: FileLocation,
    pub new_root: H256,
}
impl EventBusMessage for SpStopStoringInsolventUser {}

#[derive(Debug, Clone)]
pub struct MoveBucketRequested {
    pub bucket_id: BucketId,
    pub new_msp_id: ProviderId,
}
impl EventBusMessage for MoveBucketRequested {}

#[derive(Debug, Clone)]
pub struct MoveBucketRequestedForNewMsp {
    pub bucket_id: BucketId,
}
impl EventBusMessage for MoveBucketRequestedForNewMsp {}

#[derive(Debug, Clone)]
pub struct MoveBucketRejected {
    pub bucket_id: BucketId,
    pub msp_id: ProviderId,
}
impl EventBusMessage for MoveBucketRejected {}

#[derive(Debug, Clone)]
pub struct MoveBucketAccepted {
    pub bucket_id: BucketId,
    pub msp_id: ProviderId,
}
impl EventBusMessage for MoveBucketAccepted {}

#[derive(Debug, Clone)]
pub struct MoveBucketExpired {
    pub bucket_id: BucketId,
    pub msp_id: ProviderId,
}
impl EventBusMessage for MoveBucketExpired {}
/// BSP stopped storing a specific file.
///
/// This event is emitted when a BSP confirm stop storing a file.
#[derive(Debug, Clone)]
pub struct BspConfirmStoppedStoring {
    pub bsp_id: H256,
    pub file_key: FileKey,
    pub new_root: H256,
}
impl EventBusMessage for BspConfirmStoppedStoring {}

/// Delete file event in a finalised block.
///
/// This event is emitted when a finalised block is received by the Blockchain service,
/// in which there is a `BspConfirmStoppedStoring` event for one of the providers that this node is tracking.
#[derive(Debug, Clone)]
pub struct FinalisedBspConfirmStoppedStoring {
    pub bsp_id: H256,
    pub file_key: FileKey,
    pub new_root: H256,
}

impl EventBusMessage for FinalisedBspConfirmStoppedStoring {}

/// Notify period event.
///
/// This event is emitted when a X amount of block has passed. It is configured at the start of the service.
#[derive(Debug, Clone)]
pub struct NotifyPeriod {}

impl EventBusMessage for NotifyPeriod {}

/// The event bus provider for the BlockchainService actor.
///
/// It holds the event buses for the different events that the BlockchainService actor
/// can emit.
#[derive(Clone, Default)]
pub struct BlockchainServiceEventBusProvider {
    new_challenge_seed_event_bus: EventBus<NewChallengeSeed>,
    multiple_new_challenge_seeds_event_bus: EventBus<MultipleNewChallengeSeeds>,
    new_storage_request_event_bus: EventBus<NewStorageRequest>,
    accepted_bsp_volunteer_event_bus: EventBus<AcceptedBspVolunteer>,
    process_submit_proof_request_event_bus: EventBus<ProcessSubmitProofRequest>,
    process_confirm_storage_request_event_bus: EventBus<ProcessConfirmStoringRequest>,
    process_msp_respond_storing_request_event_bus: EventBus<ProcessMspRespondStoringRequest>,
    process_stop_storing_for_insolvent_user_request_event_bus:
        EventBus<ProcessStopStoringForInsolventUserRequest>,
    slashable_provider_event_bus: EventBus<SlashableProvider>,
    finalised_mutations_applied_event_bus: EventBus<FinalisedTrieRemoveMutationsApplied>,
    proof_accepted_event_bus: EventBus<ProofAccepted>,
    last_chargeable_info_updated_event_bus: EventBus<LastChargeableInfoUpdated>,
    user_without_funds_event_bus: EventBus<UserWithoutFunds>,
    sp_stop_storing_insolvent_user_event_bus: EventBus<SpStopStoringInsolventUser>,
    finalised_msp_stopped_storing_bucket_event_bus: EventBus<FinalisedMspStoppedStoringBucket>,
    move_bucket_requested_event_bus: EventBus<MoveBucketRequested>,
    move_bucket_rejected_event_bus: EventBus<MoveBucketRejected>,
    move_bucket_accepted_event_bus: EventBus<MoveBucketAccepted>,
    move_bucket_expired_event_bus: EventBus<MoveBucketExpired>,
    move_bucket_requested_for_new_msp_event_bus: EventBus<MoveBucketRequestedForNewMsp>,
    bsp_stop_storing_event_bus: EventBus<BspConfirmStoppedStoring>,
    finalised_bsp_stop_storing_event_bus: EventBus<FinalisedBspConfirmStoppedStoring>,
    notify_period_event_bus: EventBus<NotifyPeriod>,
}

impl BlockchainServiceEventBusProvider {
    pub fn new() -> Self {
        Self {
            new_challenge_seed_event_bus: EventBus::new(),
            multiple_new_challenge_seeds_event_bus: EventBus::new(),
            new_storage_request_event_bus: EventBus::new(),
            accepted_bsp_volunteer_event_bus: EventBus::new(),
            process_submit_proof_request_event_bus: EventBus::new(),
            process_confirm_storage_request_event_bus: EventBus::new(),
            process_msp_respond_storing_request_event_bus: EventBus::new(),
            process_stop_storing_for_insolvent_user_request_event_bus: EventBus::new(),
            slashable_provider_event_bus: EventBus::new(),
            finalised_mutations_applied_event_bus: EventBus::new(),
            proof_accepted_event_bus: EventBus::new(),
            last_chargeable_info_updated_event_bus: EventBus::new(),
            user_without_funds_event_bus: EventBus::new(),
            sp_stop_storing_insolvent_user_event_bus: EventBus::new(),
            finalised_msp_stopped_storing_bucket_event_bus: EventBus::new(),
            move_bucket_requested_event_bus: EventBus::new(),
            move_bucket_rejected_event_bus: EventBus::new(),
            move_bucket_accepted_event_bus: EventBus::new(),
            move_bucket_expired_event_bus: EventBus::new(),
            move_bucket_requested_for_new_msp_event_bus: EventBus::new(),
            bsp_stop_storing_event_bus: EventBus::new(),
            finalised_bsp_stop_storing_event_bus: EventBus::new(),
            notify_period_event_bus: EventBus::new(),
        }
    }
}

impl ProvidesEventBus<NewChallengeSeed> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<NewChallengeSeed> {
        &self.new_challenge_seed_event_bus
    }
}

impl ProvidesEventBus<MultipleNewChallengeSeeds> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<MultipleNewChallengeSeeds> {
        &self.multiple_new_challenge_seeds_event_bus
    }
}

impl ProvidesEventBus<NewStorageRequest> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<NewStorageRequest> {
        &self.new_storage_request_event_bus
    }
}

impl ProvidesEventBus<AcceptedBspVolunteer> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<AcceptedBspVolunteer> {
        &self.accepted_bsp_volunteer_event_bus
    }
}

impl ProvidesEventBus<ProcessSubmitProofRequest> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<ProcessSubmitProofRequest> {
        &self.process_submit_proof_request_event_bus
    }
}

impl ProvidesEventBus<ProcessConfirmStoringRequest> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<ProcessConfirmStoringRequest> {
        &self.process_confirm_storage_request_event_bus
    }
}

impl ProvidesEventBus<ProcessMspRespondStoringRequest> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<ProcessMspRespondStoringRequest> {
        &self.process_msp_respond_storing_request_event_bus
    }
}

impl ProvidesEventBus<ProcessStopStoringForInsolventUserRequest>
    for BlockchainServiceEventBusProvider
{
    fn event_bus(&self) -> &EventBus<ProcessStopStoringForInsolventUserRequest> {
        &self.process_stop_storing_for_insolvent_user_request_event_bus
    }
}

impl ProvidesEventBus<SlashableProvider> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<SlashableProvider> {
        &self.slashable_provider_event_bus
    }
}

impl ProvidesEventBus<FinalisedTrieRemoveMutationsApplied> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<FinalisedTrieRemoveMutationsApplied> {
        &self.finalised_mutations_applied_event_bus
    }
}

impl ProvidesEventBus<ProofAccepted> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<ProofAccepted> {
        &self.proof_accepted_event_bus
    }
}

impl ProvidesEventBus<LastChargeableInfoUpdated> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<LastChargeableInfoUpdated> {
        &self.last_chargeable_info_updated_event_bus
    }
}

impl ProvidesEventBus<UserWithoutFunds> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<UserWithoutFunds> {
        &self.user_without_funds_event_bus
    }
}

impl ProvidesEventBus<SpStopStoringInsolventUser> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<SpStopStoringInsolventUser> {
        &self.sp_stop_storing_insolvent_user_event_bus
    }
}

impl ProvidesEventBus<FinalisedMspStoppedStoringBucket> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<FinalisedMspStoppedStoringBucket> {
        &self.finalised_msp_stopped_storing_bucket_event_bus
    }
}

impl ProvidesEventBus<MoveBucketRequested> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<MoveBucketRequested> {
        &self.move_bucket_requested_event_bus
    }
}

impl ProvidesEventBus<MoveBucketRejected> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<MoveBucketRejected> {
        &self.move_bucket_rejected_event_bus
    }
}

impl ProvidesEventBus<MoveBucketAccepted> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<MoveBucketAccepted> {
        &self.move_bucket_accepted_event_bus
    }
}

impl ProvidesEventBus<MoveBucketExpired> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<MoveBucketExpired> {
        &self.move_bucket_expired_event_bus
    }
}

impl ProvidesEventBus<MoveBucketRequestedForNewMsp> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<MoveBucketRequestedForNewMsp> {
        &self.move_bucket_requested_for_new_msp_event_bus
    }
}

impl ProvidesEventBus<BspConfirmStoppedStoring> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<BspConfirmStoppedStoring> {
        &self.bsp_stop_storing_event_bus
    }
}

impl ProvidesEventBus<FinalisedBspConfirmStoppedStoring> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<FinalisedBspConfirmStoppedStoring> {
        &self.finalised_bsp_stop_storing_event_bus
    }
}

impl ProvidesEventBus<NotifyPeriod> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<NotifyPeriod> {
        &self.notify_period_event_bus
    }
}
