use sp_core::H256;
use storage_hub_infra::event_bus::{EventBus, EventBusMessage, ProvidesEventBus};

#[derive(Debug, Clone)]
struct NewChallenge {
    who: String,
    key_challenged: H256,
}

impl EventBusMessage for NewChallenge {}

#[derive(Clone, Debug, Default)]
pub struct ForestServiceEventBustProvider {
    challenge_request_event_bus: EventBus<NewChallenge>,
}

impl ForestServiceEventBustProvider {
    pub fn new() -> Self {
        Self {
            challenge_request_event_bus: EventBus::new(),
        }
    }
}

impl ProvidesEventBus<NewChallenge> for ForestServiceEventBustProvider {
    fn event_bus(&self) -> &EventBus<NewChallenge> {
        &self.challenge_request_event_bus
    }
}
