# StorageHub Testing

## Pre-requisites

### Bun

[Bun](https://bun.sh/) is used in this project as the JavaScript package manager and runtime for workspace scripts. Install it by following the official instructions at: [https://bun.sh/docs/installation](https://bun.sh/docs/installation)

## Docker Setup

> [!IMPORTANT]
> This is required for `DEV` & `BSPNET` modes.

### 1. Build Node

#### Linux (Node)

```sh
cargo build --release
```

#### macOS (Node)

> [!IMPORTANT]
> If you are running this on a Mac, `zig` is a pre-requisite for crossbuilding the node. See the [Zig installation guide](https://ziglang.org/learn/getting-started/).

```sh
bun install
bun run crossbuild:mac
```

### 2. Build Docker Image

```sh
bun run docker:build
```

### 3. Build Backend (required for Backend & Solochain-EVM tests)

#### Linux (Backend)

```sh
cargo build --release -p sh-msp-backend
```

#### macOS (Backend)

> [!IMPORTANT]
> If you are running this on a Mac, `zig` is a pre-requisite for crossbuilding the backend. See the [Zig installation guide](https://ziglang.org/learn/getting-started/).

```sh
bun install
bun run crossbuild:mac:backend
```

### 4. Build Backend Docker Image

```sh
bun run docker:build:backend
```

## Testing Types

### BSPNet

This is a small network running in `dev` mode, with manual sealing on blocks, between a BSP & a User node. This is used to test the merklisation of files, and their retrieval.

```sh
bun run test:bspnet
```

### Dev Node Test

The `storage-hub` node is run in a Docker container in dev mode, so that it can be isolated and parallelized across multiple threads & runners. The purpose of this suite is verify the functionality of both the RPC and the runtime.

> [!IMPORTANT]
> Provider functionality is not covered here, only how the system chain behaves.

```sh
bun run test:node
```

### End-To-End Tests

> [!NOTE]
> Please ensure the Rust project is built first, e.g., `cargo build --release`.
> This is required as currently we only support native binaries.

In `/test` run: `bun install` to install ZombieNet

#### 1. Run Network

```shell
# In the /test directory
bun install
bun run zombie:run:full:native
```

Wait for ZombieNet network to start, and then:

#### 2. Run Setup & Tests

```shell
bun run typegen
bun run zombie:setup:native
bun run test:full
```

### Backend Integration Tests

> [!IMPORTANT]
> Requires both images: node (`bun run docker:build`) and backend (`bun run docker:build:backend`). On macOS, build the backend via `bun run crossbuild:mac:backend`; on Linux, `cargo build --release -p sh-msp-backend`. The backend Docker build uses `docker/storage-hub-msp-backend.Dockerfile`.

```sh
# In the /test directory
bun install
bun run test:backend
```

Runs a local full network with indexer and the backend, then executes backend tests.

### Solochain EVM Integration Tests

> [!IMPORTANT]
> Requires both images: node (`bun run docker:build`) and backend (`bun run docker:build:backend`). On macOS, build the backend via `bun run crossbuild:mac:backend`; on Linux, `cargo build --release -p sh-msp-backend`. The backend Docker build uses `docker/storage-hub-msp-backend.Dockerfile`.

```sh
# In the /test directory
bun install
bun run test:solochain-evm
```

Launches Solochain EVM runtime with indexer and backend enabled and runs SDK precompile tests.

### ZombieNet

This is the networking testing suite for topology and network stability. It is a suite of tests that run on a network of nodes, and is used to verify the network's stability and the nodes' ability to communicate with each other.

```sh
bun run zombie:test:native
```

## Launching Networks

### Spawning Local DevNode

- Native launch: `../target/release/storage-hub --dev`
- Docker launch (local): `bun run docker:start` / `bun run docker:stop`
- Docker launch (latest): `bun run docker:start:latest` / `bun run docker:stop:latest`

### Spawning BSPNet

```sh
bun run docker:start:bspnet
```

This will start a BSPNet network with a BSP and a User node. As part of the setup it will force onboard a MSP and BSP, and then upload a file from user node.

### Spawning Solochain EVM (initialised fullnet)

```sh
bun run docker:start:solochain-evm:initialised
```

Starts a full network on Solochain EVM runtime with indexer and backend, pre-initialised for demos.

> [!NOTE]
> The BSP id is chosen to be the fingerprint of a file that is uploaded by the user node. This is done to "game the system" to ensure that the BSP is guaranteed to be selected to store the file.

### Spawning NoisyNet

- Docker launch (local): `bun run docker:start:noisynet` / `bun run docker:stop:noisynet`

### Spawning ZombieNet Native

> [!TIP]
> Polkadot binaries are required to run a ZombieNet network.
> For Linux you can run the script: `bun run scripts/downloadPolkadot.ts <version>`
> For macOS you will have to [compile from source](https://github.com/paritytech/polkadot-sdk/tree/master/polkadot#build-from-source).

To launch a non-ephemeral ZombieNet network by executing the following in: `/test` directory:

```sh
bun install
bun run zombie:run:native
```

From here you should see in the terminal, the different nodes being spun up. When the network is fully launched, you should see something like this:

![success](../resources/zombieSuccess.png)

From here you can interact via the websockets exposed in the direct links, in the example above we have:

- Alice (relay): `35005`
- Bob (relay): `37613`
- Collator (storage-hub): `45615`

## Generating new Type Interfaces

This repo uses polkadot{.js} [TS Type Generation](https://polkadot.js.org/docs/api/examples/promise/typegeni) AKA `api-augment`.
To generate new type interfaces run the following in `/test`:

```sh
bun run typegen
```

> [!TIP]  
> Like with other commands, this assumes you have built a node binary and Docker image before executing this activity.

## Misc

### Why do we use Docker so much?

![docker](../resources/docker.jpg)
