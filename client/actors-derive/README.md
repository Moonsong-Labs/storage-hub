# StorageHub Actors Derive

This crate provides procedural macros to reduce boilerplate code in the StorageHub actors framework.

## Features

- `ActorEvent` derive macro: Implements `EventBusMessage` for event structs and registers them with a specific actor.
- `ActorEventBus` attribute macro: Generates the event bus provider struct and implements all the required methods and traits.
- `subscribe_actor_event` macro: Simplifies event subscription code with named parameters for better readability.
- `subscribe_actor_event_map` macro: Simplifies subscribing multiple events to tasks with shared parameters and per-mapping overrides.

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

This will generate:
- A struct with appropriate event bus fields for all registered events
- Implementation of the `Default` trait
- Implementation of the `ProvidesEventBus` trait for each event type

### 3. Subscribing to Events

Use the `subscribe_actor_event` macro to simplify event subscription code:

```rust
use shc_actors_derive::subscribe_actor_event;

// Creating a new task instance and subscribing
subscribe_actor_event!(
    event: FinalisedBspConfirmStoppedStoring,
    task: BspDeleteFileTask,
    service: &self.blockchain,
    spawner: &self.task_spawner,
    context: self.clone(),
    critical: true,
);

// Using an existing task instance
let task = BspDeleteFileTask::new(self.clone());
subscribe_actor_event!(
    event: FinalisedBspConfirmStoppedStoring,
    task: task,
    service: &self.blockchain,
    spawner: &self.task_spawner,
    critical: true,
);
```

#### Parameters for `subscribe_actor_event`:

- `event`: The event type to subscribe to (required)
- `task`: Either a task type (if creating a new task) or a task instance (required)
- `service`: The service that provides the event bus (required)
- `spawner`: The task spawner for spawning the event handler (required)
- `context`: The context to create a new task (required when `task` is a type)
- `critical`: Whether the event is critical (optional, defaults to false)

#### Equivalent Code

The `subscribe_actor_event` macro expands to code equivalent to:

```rust
// When using a task type:
let task = BspDeleteFileTask::new(self.clone());
let event_bus_listener: EventBusListener<FinalisedBspConfirmStoppedStoring, _> =
    task.subscribe_to(&task_spawner, &service, true);
event_bus_listener.start();

// When using an existing task instance:
let event_bus_listener: EventBusListener<FinalisedBspConfirmStoppedStoring, _> =
    task.subscribe_to(&task_spawner, &service, true);
event_bus_listener.start();
```

### 4. Mapping Multiple Events to Tasks

Use the `subscribe_actor_event_map` macro to simplify subscribing multiple events to tasks with shared parameters:

```rust
use shc_actors_derive::subscribe_actor_event_map;

subscribe_actor_event_map!(
    service: &self.blockchain,
    spawner: &self.task_spawner,
    context: self.clone(),
    critical: true,
    [
        // Override critical for specific mapping
        NewStorageRequest => { task: MspUploadFileTask, critical: false },
        // Use default critical value
        ProcessMspRespondStoringRequest => MspUploadFileTask,
        FinalisedMspStoppedStoringBucket => MspDeleteBucketTask,
    ]
);
```

#### Parameters for `subscribe_actor_event_map`:

- `service`: The service that provides the event bus (required)
- `spawner`: The task spawner for spawning event handlers (required)
- `context`: The context to create new tasks (required)
- `critical`: Default critical value for all mappings (optional, defaults to false)
- An array of mappings, where each mapping is either:
  - `EventType => TaskType`: Uses default critical value
  - `EventType => { task: TaskType, critical: bool }`: Overrides critical value for this mapping

This macro is particularly useful when you need to subscribe multiple events to tasks with shared parameters, as it reduces boilerplate code and makes the relationships between events and tasks more explicit.

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
