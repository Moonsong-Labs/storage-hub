use sp_core::H256;
use sp_runtime::AccountId32;
use storage_hub_infra::event_bus::{EventBus, EventBusMessage, ProvidesEventBus};

type StorageData = pallet_file_system::types::StorageData<storage_hub_runtime::Runtime>;
type FileLocation = pallet_file_system::types::FileLocation<storage_hub_runtime::Runtime>;
type PeerIds = pallet_file_system::types::PeerIds<storage_hub_runtime::Runtime>;

// TODO: use proper types
#[derive(Clone)]
pub struct ChallengeRequest {
    pub location: String,
}

impl EventBusMessage for ChallengeRequest {}

/// New storage request event.
///
/// This event is emitted when a new storage request is created on-chain.
#[derive(Debug, Clone)]
pub struct NewStorageRequest {
    /// Account ID of the requester.
    pub who: AccountId32,
    /// Location of the file (as a file path).
    pub location: FileLocation,
    /// Fingerprint of the file (root hash of the merklised file).
    pub fingerprint: H256,
    /// Size of the file.
    pub size: StorageData,
    /// lib2p peer IDs from where the user would send the file.
    pub user_peer_ids: PeerIds,
}

impl EventBusMessage for NewStorageRequest {}

// TODO: use proper types
#[derive(Debug, Clone)]
pub struct AcceptedBspVolunteer {
    pub who: String,
    pub location: String,
    pub fingerprint: String,
    pub multiaddresses: Vec<String>,
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
    challenge_request_event_bus: EventBus<ChallengeRequest>,
    new_storage_request_event_bus: EventBus<NewStorageRequest>,
    accepted_bsp_volunteer_event_bus: EventBus<AcceptedBspVolunteer>,
    storage_request_revoked_event_bus: EventBus<StorageRequestRevoked>,
}

impl BlockchainServiceEventBusProvider {
    pub fn new() -> Self {
        Self {
            challenge_request_event_bus: EventBus::new(),
            new_storage_request_event_bus: EventBus::new(),
            accepted_bsp_volunteer_event_bus: EventBus::new(),
            storage_request_revoked_event_bus: EventBus::new(),
        }
    }
}

impl ProvidesEventBus<ChallengeRequest> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<ChallengeRequest> {
        &self.challenge_request_event_bus
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
