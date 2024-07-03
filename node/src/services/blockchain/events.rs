use sc_network::Multiaddr;
use shc_actors_framework::event_bus::{EventBus, EventBusMessage, ProvidesEventBus};
use shc_common::types::{
    BlockNumber, BucketId, FileKey, FileLocation, Fingerprint, PeerIds, ProviderId, RandomSeed,
    StorageData,
};
use sp_core::H256;
use sp_runtime::AccountId32;

/// New random challenge emitted by the StorageHub runtime.
///
/// This event is emitted when there's a new random challenge seed that affects this
/// BSP. In other words, it only pays attention to the random seeds in the challenge
/// period of this BSP.
#[derive(Clone)]
pub struct NewChallengeSeed {
    pub provider_id: ProviderId,
    pub tick: BlockNumber,
    pub seed: RandomSeed,
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

// TODO: use proper types
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

// TODO: use proper types
#[derive(Debug, Clone)]
pub struct StorageRequestRevoked {
    pub location: String,
}

impl EventBusMessage for StorageRequestRevoked {}

#[derive(Clone, Default)]
pub struct BlockchainServiceEventBusProvider {
    new_challenge_seed_event_bus: EventBus<NewChallengeSeed>,
    new_storage_request_event_bus: EventBus<NewStorageRequest>,
    accepted_bsp_volunteer_event_bus: EventBus<AcceptedBspVolunteer>,
    storage_request_revoked_event_bus: EventBus<StorageRequestRevoked>,
}

impl BlockchainServiceEventBusProvider {
    pub fn new() -> Self {
        Self {
            new_challenge_seed_event_bus: EventBus::new(),
            new_storage_request_event_bus: EventBus::new(),
            accepted_bsp_volunteer_event_bus: EventBus::new(),
            storage_request_revoked_event_bus: EventBus::new(),
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

impl ProvidesEventBus<StorageRequestRevoked> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<StorageRequestRevoked> {
        &self.storage_request_revoked_event_bus
    }
}
