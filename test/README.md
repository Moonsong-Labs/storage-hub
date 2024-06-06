# StorageHub Testing

## Pre-requisites

### pnpm

[pnpm](https://pnpm.io/) is used in this project as the Javascript package manager to install dependencies. To install it you can follow the official instructions at: [https://pnpm.io/installation](https://pnpm.io/installation)

The quickest way is via their script: `curl -fsSL https://get.pnpm.io/install.sh | sh -`

## Testing Types

### Dev Node Test

The `storage-hub` node is run in a docker container in dev mode, so that it can be isolated and parallelized across multiple threads & runners. The purpose of this suite is verify functionality of both the RPC and runtime.

> [!IMPORTANT]  
> Provider functionality is not covered here, only how the system chain behaves.

#### 1. Build Node

##### Linux

```sh
cargo build --release
```

##### MacOS

> [!IMPORTANT]  
> If you are running this on a Mac, `zig` is a pre-requisite for crossbuilding the node. Instructions to install can be found [here](https://ziglang.org/learn/getting-started/).

```sh
pnpm crossbuild:mac
```

#### 2. Build Docker Image

```sh
pnpm docker:build
```

#### 3. Run Test Suite

```sh
pnpm test:node
```

### End-To-End Tests

> [!NOTE]  
> Please ensure the rust project is built first e.g. `cargo build --release`. 
> This is required as currently we only support native binaries.

In `/test` run: `pnpm install` to install zombienet

#### 1. Run Network

```shell
# in the /test directory
pnpm i
pnpm zombie:run:full:native
```

Wait for zombie network to start, and then:


#### 2. Run Setup & Tests

```shell
pnpm update-types
pnpm zombie:setup:native
pnpm zombie:test suites/zombie
```

### ZombieNet 

This is the networking testing suite for topology and network stability. It is a suite of tests that run on a network of nodes, and is used to verify the network's stability and the nodes' ability to communicate with each other.

```sh
pnpm zombie:test:native
```

## Launching Networks

### Spawning Local DevNode

- Native launch: `../target/release/storage-hub --dev`
- Docker launch (local): `pnpm docker:start` / `pnpm docker:stop`
- Docker launch (latest): `pnpm docker:start:latest` / `pnpm docker:stop:latest`

### Spawning ZombieNet Native

> [!TIP]  
> Polkadot binaries are required to run a zombienet network.
> For Linux you can run the script: `pnpm tsx scripts/downloadPolkadot.ts <version>`
> For macOS you will have to [compile from source](https://github.com/paritytech/polkadot-sdk/tree/master/polkadot#build-from-source).

To launch a non-ephemeral ZombieNetwork by executing the following in: `/test` directory:

```sh
pnpm install
pnpm zombie:run:native
```

From here you should see in the terminal, the different nodes being spun up. When the network is fully launched, you should see something like this:

![success](../resources/zombieSuccess.png)

From here you can interact via the websockets exposed in the direct links, in the example above we have:

- Alice (relay): `35005`
- Bob (relay): `37613`
- Collator (storage-hub): `45615`

## Generating new Type Interfaces

This repo uses Parity's [polkadot-api](https://github.com/polkadot-api/polkadot-api) AKA PAPI.
To generate new type interfaces run the following in `/test`:

```sh
pnpm update-types
```

> [!IMPORTANT]  
> This requires that you have a built network bin at `target/release`. If you are on mac and have cross built onto x86, you will need to rebuild in native again (sorry, WIP).


## Troubleshooting

### Errors

#### Weird error for `Incompatible runtime entry`

Occasionally you might see this error when trying to use the Polkadot-API typed interfaces to interact with a storageHub chain.

```shell
Waiting a maximum of 60 seconds for Local Testnet chain to be ready...✅
916 | var getFakeSignature = () => fakeSignature;
917 | var createTxEntry = (pallet, name, assetChecksum, chainHead, broadcast, compatibilityHelper2) => {
918 |   const { isCompatible, compatibleRuntime$ } = compatibilityHelper2(
919 |     (ctx) => ctx.checksumBuilder.buildCall(pallet, name)
920 |   );
921 |   const checksumError = () => new Error(`Incompatible runtime entry Tx(${pallet}.${name})`);
                                                                                                ^
error: Incompatible runtime entry Tx(Sudo.sudo)
      at checksumError (/home/runner/work/storage-hub/storage-hub/node_modules/polkadot-api/dist/index.mjs:921:91)
      at /home/runner/work/storage-hub/storage-hub/node_modules/polkadot-api/dist/index.mjs:355:15
      at /home/runner/work/storage-hub/storage-hub/node_modules/rxjs/dist/cjs/internal/operators/map.js:10:29
      at /home/runner/work/storage-hub/storage-hub/node_modules/rxjs/dist/cjs/internal/operators/OperatorSubscriber.js:33:21
      at /home/runner/work/storage-hub/storage-hub/node_modules/rxjs/dist/cjs/internal/Subscriber.js:51:13
      at /home/runner/work/storage-hub/storage-hub/node_modules/rxjs/dist/cjs/internal/observable/combineLatest.js:51:29
      at /home/runner/work/storage-hub/storage-hub/node_modules/rxjs/dist/cjs/internal/operators/OperatorSubscriber.js:33:21
      at /home/runner/work/storage-hub/storage-hub/node_modules/rxjs/dist/cjs/internal/Subscriber.js:51:13
      at /home/runner/work/storage-hub/storage-hub/node_modules/rxjs/dist/cjs/internal/observable/innerFrom.js:120:17
```

This is caused by the decorated API referring to a different version of the wasm runtime it was expecting.

> [!TIP]  
> This can be fixed by running the following:
>
> 1. running a local network: `pnpm zombie:run:native`
> 2. in a separate terminal, generating new metadata blob: `pnpm scalegen`
> 3. generating new types bundle: `pnpm typegen`
