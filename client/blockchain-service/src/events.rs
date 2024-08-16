use sc_network::Multiaddr;
use shc_actors_framework::event_bus::{EventBus, EventBusMessage, ProvidesEventBus};
use shc_common::types::{
    BlockNumber, BucketId, FileKey, FileLocation, Fingerprint, ForestRoot, PeerIds, ProviderId,
    RandomnessOutput, StorageData, TrieRemoveMutation,
};
use sp_core::H256;
use sp_runtime::AccountId32;

/// New random challenge emitted by the StorageHub runtime.
///
/// This event is emitted when there's a new random challenge seed that affects this
/// BSP. In other words, it only pays attention to the random seeds in the challenge
/// period of this BSP.
#[derive(Debug, Clone)]
pub struct NewChallengeSeed {
    pub provider_id: ProviderId,
    pub tick: BlockNumber,
    pub seed: RandomnessOutput,
}

impl EventBusMessage for NewChallengeSeed {}

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
}

impl EventBusMessage for NewStorageRequest {}

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

/// BSP confirmed storing event.
///
/// This event is emitted when a BSP confirms storing a file and the Runtime updates it's Forest
/// trie root.
#[derive(Debug, Clone)]
pub struct BspConfirmedStoring {
    pub bsp_id: H256,
    pub file_keys: Vec<FileKey>,
    pub new_root: H256,
}

impl EventBusMessage for BspConfirmedStoring {}

/// Slashable Provider event.
///
/// This event is emitted when a provider is marked as slashable by the runtime.
#[derive(Debug, Clone)]
pub struct SlashableProvider {
    pub provider: ProviderId,
    pub next_challenge_deadline: BlockNumber,
}

impl EventBusMessage for SlashableProvider {}

/// Mutations applied event in a finalised block.
///
/// This event is emitted when a finalised block is received by the Blockchain service,
/// in which there is a `MutationsApplied` event for one of the providers that this node is tracking.
#[derive(Debug, Clone)]
pub struct FinalisedMutationsApplied {
    pub provider_id: ProviderId,
    pub mutations: Vec<(ForestRoot, TrieRemoveMutation)>,
    pub new_root: H256,
}

impl EventBusMessage for FinalisedMutationsApplied {}

/// The event bus provider for the BlockchainService actor.
///
/// It holds the event buses for the different events that the BlockchainService actor
/// can emit.
#[derive(Clone, Default)]
pub struct BlockchainServiceEventBusProvider {
    new_challenge_seed_event_bus: EventBus<NewChallengeSeed>,
    new_storage_request_event_bus: EventBus<NewStorageRequest>,
    accepted_bsp_volunteer_event_bus: EventBus<AcceptedBspVolunteer>,
    bsp_confirmed_storing_event_bus: EventBus<BspConfirmedStoring>,
    slashable_provider_event_bus: EventBus<SlashableProvider>,
    finalised_mutations_applied_event_bus: EventBus<FinalisedMutationsApplied>,
}

impl BlockchainServiceEventBusProvider {
    pub fn new() -> Self {
        Self {
            new_challenge_seed_event_bus: EventBus::new(),
            new_storage_request_event_bus: EventBus::new(),
            accepted_bsp_volunteer_event_bus: EventBus::new(),
            bsp_confirmed_storing_event_bus: EventBus::new(),
            slashable_provider_event_bus: EventBus::new(),
            finalised_mutations_applied_event_bus: EventBus::new(),
        }
    }
}

impl ProvidesEventBus<NewChallengeSeed> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<NewChallengeSeed> {
        &self.new_challenge_seed_event_bus
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

impl ProvidesEventBus<BspConfirmedStoring> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<BspConfirmedStoring> {
        &self.bsp_confirmed_storing_event_bus
    }
}

impl ProvidesEventBus<SlashableProvider> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<SlashableProvider> {
        &self.slashable_provider_event_bus
    }
}

impl ProvidesEventBus<FinalisedMutationsApplied> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<FinalisedMutationsApplied> {
        &self.finalised_mutations_applied_event_bus
    }
}
