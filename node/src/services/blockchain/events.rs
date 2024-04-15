use sp_core::H256;
use sp_runtime::{AccountId32, BoundedVec};
use storage_hub_infra::event_bus::{EventBus, EventBusMessage, ProvidesEventBus};

type StorageData = pallet_file_system::types::StorageData<storage_hub_runtime::Runtime>;
type FileLocation = pallet_file_system::types::FileLocation<storage_hub_runtime::Runtime>;
type MultiAddress = pallet_file_system::types::MultiAddress<storage_hub_runtime::Runtime>;
type MaxDataServerMultiAddresses =
    <storage_hub_runtime::Runtime as pallet_file_system::Config>::MaxDataServerMultiAddresses;

// TODO: use proper types
#[derive(Debug, Clone)]
pub struct ChallengeRequest {
    pub location: String,
}

impl EventBusMessage for ChallengeRequest {}

// TODO: use proper types
#[derive(Debug, Clone)]
pub struct NewStorageRequest {
    pub who: AccountId32,
    pub location: FileLocation,
    pub fingerprint: H256,
    pub size: StorageData,
    pub multiaddresses: BoundedVec<MultiAddress, MaxDataServerMultiAddresses>,
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

#[derive(Clone, Debug, Default)]
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
