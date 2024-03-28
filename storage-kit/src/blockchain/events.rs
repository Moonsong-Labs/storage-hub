use crate::{EventBus, EventBusMessage, ProvidesEventBus};

#[derive(Clone, Default)]
pub struct BlockchainEventBusProvider {
    challenge_request_event_bus: EventBus<ChallengeRequest>,
}

impl BlockchainEventBusProvider {
    pub fn new() -> Self {
        Self {
            challenge_request_event_bus: EventBus::new(),
        }
    }
}

impl ProvidesEventBus<ChallengeRequest> for BlockchainEventBusProvider {
    fn event_bus(&self) -> &EventBus<ChallengeRequest> {
        &self.challenge_request_event_bus
    }
}

#[derive(Debug, Clone)]
pub struct ChallengeRequest {
    pub challenge: String,
}

impl EventBusMessage for ChallengeRequest {}
