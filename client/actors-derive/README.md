# StorageHub Actors Derive

This crate provides procedural macros to reduce boilerplate code in the StorageHub actors framework.

## Features

- `ActorEvent` derive macro: Implements `EventBusMessage` for event structs and registers them with a specific actor.
- `ActorEventBus` attribute macro: Generates the event bus provider struct and implements all the required methods and traits.

## Usage

First, add the dependency to your crate by including it in your Cargo.toml:

```toml
[dependencies]
shc-actors-derive = { workspace = true }
```

### 1. Defining Event Messages

Import the macros directly and use the `ActorEvent` derive macro:

```rust
use shc_actors_derive::ActorEvent;

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct NewChallengeSeed {
    pub provider_id: String,
    pub tick: u32,
    pub seed: Vec<u8>,
}
```

This will:
- Implement `EventBusMessage` for the struct
- Register the event with the specified actor ID (`blockchain_service` in this example)

### 2. Creating Event Bus Providers

Use the `ActorEventBus` attribute macro:

```rust
use shc_actors_derive::ActorEventBus;

#[ActorEventBus("blockchain_service")]
pub struct BlockchainServiceEventBusProvider;
```

This will automatically:
- Add fields for each event bus registered with the specified actor ID
- Implement the `new()` method to initialize all event buses
- Implement `ProvidesEventBus<T>` for each event type

## Refactoring Example

### Before

```rust
#[derive(Clone)]
pub struct NewChallengeSeed {
    pub provider_id: String,
    pub tick: u32,
    pub seed: Vec<u8>,
}

impl EventBusMessage for NewChallengeSeed {}

#[derive(Clone, Default)]
pub struct BlockchainServiceEventBusProvider {
    new_challenge_seed_event_bus: EventBus<NewChallengeSeed>,
    // Many more fields...
}

impl BlockchainServiceEventBusProvider {
    pub fn new() -> Self {
        Self {
            new_challenge_seed_event_bus: EventBus::new(),
            // Many more initializations...
        }
    }
}

impl ProvidesEventBus<NewChallengeSeed> for BlockchainServiceEventBusProvider {
    fn event_bus(&self) -> &EventBus<NewChallengeSeed> {
        &self.new_challenge_seed_event_bus
    }
}
// Many more implementations...
```

### After

```rust
use shc_actors_derive::{ActorEvent, ActorEventBus};

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct NewChallengeSeed {
    pub provider_id: String,
    pub tick: u32,
    pub seed: Vec<u8>,
}

#[ActorEventBus("blockchain_service")]
pub struct BlockchainServiceEventBusProvider;
```

## How It Works

1. The `ActorEvent` derive macro registers each event type with an actor ID in a global registry.
2. The `ActorEventBus` attribute macro looks up all the event types registered for the specified actor ID and generates the required code.

This approach greatly reduces boilerplate code while maintaining type safety and performance.

## Limitations

- All event types must be defined and processed before the `ActorEventBus` macro is used.
- The macro relies on a global state to keep track of registered events, which may cause issues in certain build scenarios.
