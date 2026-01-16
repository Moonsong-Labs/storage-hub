# StorageHub Actors Derive

This crate provides procedural macros to reduce boilerplate code in the StorageHub actors framework.

## Features

- `actor` attribute macro: Implements `EventBusMessage` for event structs, auto-derives `Debug` and `Clone`, and registers them with a specific actor. Optionally injects a `forest_root_write_lock` field and implements `ForestRootWriteAccess`.
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

Use the `actor` attribute macro:

```rust
use shc_actors_derive::actor;

#[actor(actor = "blockchain_service")]
pub struct NewChallengeSeed {
    pub provider_id: String,
    pub tick: u32,
    pub seed: Vec<u8>,
}
```

This will:
- Automatically derive `Debug` and `Clone` for the struct
- Implement `EventBusMessage` for the struct
- Register the event with the specified actor ID (`blockchain_service` in this example)

For events that need additional derives (like `Encode`, `Decode`), add them before the macro:

```rust
use codec::{Encode, Decode};
use shc_actors_derive::actor;

#[derive(Encode, Decode)]
#[actor(actor = "blockchain_service")]
pub struct MultipleNewChallengeSeeds {
    pub provider_id: String,
    pub seeds: Vec<(u32, Vec<u8>)>,
}
```

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
use shc_actors_derive::{actor, ActorEventBus};

#[actor(actor = "blockchain_service")]
pub struct NewChallengeSeed {
    pub provider_id: String,
    pub tick: u32,
    pub seed: Vec<u8>,
}

#[ActorEventBus("blockchain_service")]
pub struct BlockchainServiceEventBusProvider;
```

# StorageHub Actors Command Macros

This crate provides procedural macros to simplify actor command boilerplate code in the StorageHub actors framework.

## Features

- `actor_command` attribute macro: Automatically enhances command enums with callbacks and generates the Interface trait.
- `command` attribute macro: Specifies behavior for individual command variants.

## Usage

### Basic Command Definition

```rust
#[actor_command(
    service = FileTransferService,
    default_mode = "SyncAwait",
    default_error_type = RequestError
)]
pub enum FileTransferServiceCommand {
    #[command(mode = "AsyncAwait", success_type = (Vec<u8>, ProtocolName), error_type = RequestFailure)]
    UploadRequest {
        peer_id: PeerId,
        file_key: FileKey,
        file_key_proof: FileKeyProof,
        bucket_id: Option<BucketId>,
    },
    
    UploadResponse {
        request_id: UploadRequestId,
        file_complete: bool,
    },
    
    // Other commands...
}
```

### Command Modes

The macro supports three command modes:

1. **FireAndForget**: No response is expected
2. **SyncAwait**: Wait for a direct response from the actor
3. **AsyncAwait**: Wait for an asynchronous response (e.g., from a network operation)

### Extension Traits

You can define extension traits in addition to the automatically generated Interface trait:

```rust
#[async_trait::async_trait]
pub trait FileTransferServiceInterfaceExt {
    fn parse_remote_upload_data_response(
        &self,
        data: Vec<u8>,
    ) -> Result<schema::v1::provider::RemoteUploadDataResponse, RequestError>;

    async fn extract_peer_ids_and_register_known_addresses(
        &self,
        multiaddresses: Vec<Multiaddr>,
    ) -> Vec<PeerId>;
}

#[async_trait::async_trait]
impl FileTransferServiceInterfaceExt for ActorHandle<FileTransferService> {
    // Implementations...
}
```

## Attribute Parameters

### `actor_command` Parameters

- `service`: (Required) The service type that processes these commands
- `default_mode`: (Optional) Default command mode, one of: "FireAndForget", "SyncAwait", "AsyncAwait"
- `default_error_type`: (Optional) Default error type for command responses
- `default_inner_channel_type`: (Optional) Default channel type for AsyncAwait mode

### `command` Parameters

- `mode`: (Optional) Override the default command mode
- `success_type`: (Optional) The success type returned in the Result
- `error_type`: (Optional) Override the default error type
- `inner_channel_type`: (Optional) Override the default channel type for AsyncAwait mode

## Generated Code

The macro automatically:

1. Adds a `callback` field to each command variant based on the mode
2. Generates a trait with a method for each command
3. Implements the trait for ActorHandle<ServiceType>

This eliminates boilerplate code and ensures consistent error handling.

## Real World Example

Here's an example from the StorageHub codebase:

```rust
#[actor_command(
    service = BlockchainService<FSH: ForestStorageHandler + Clone + Send + Sync + 'static>,
    default_mode = "SyncAwait",
    default_inner_channel_type = tokio::sync::oneshot::Receiver,
)]
pub enum BlockchainServiceCommand {
    #[command(success_type = SubmittedTransaction)]
    SendExtrinsic {
        call: sh_parachain_runtime::RuntimeCall,
        options: SendExtrinsicOptions,
    },
    #[command(success_type = Extrinsic)]
    GetExtrinsicFromBlock {
        block_hash: H256,
        extrinsic_hash: H256,
    },
    #[command(mode = "AsyncAwait", inner_channel_type = tokio::sync::oneshot::Receiver)]
    WaitForBlock {
        block_number: BlockNumber,
    },
    // ... more commands
}
```

This generates a trait `BlockchainServiceCommandInterface` with methods like `send_extrinsic`, `get_extrinsic_from_block`, etc., which can be called on an `ActorHandle<BlockchainService>`.
