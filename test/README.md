# StorageHub Testing

## Types

### Runtime Tests

> [!NOTE]  
> TODO Add description here of what this test suite does and what it intends to cover

### Integration Tests

> [!NOTE]  
> TODO Add description here of what this test suite does and what it intends to cover

### End-To-End Tests

> [!NOTE]  
> TODO Add description here of what this test suite does and what it intends to cover

```shell
# in the /test directory
pnpm i
pnpm zombie:run:full:native
```f

Wait for zombie network to start, and then:

```sh
pnpm zombie:setup:native
pnpm zombie:test suites/zombie
```

## Local Usage

### Pre-requisites

#### pnpm

[pnpm](https://pnpm.io/) is used in this project as the Javascript package manager to install dependencies. To install it you can follow the official instructions at: [https://pnpm.io/installation](https://pnpm.io/installation)

The quickest way is via their script: `curl -fsSL https://get.pnpm.io/install.sh | sh -`

#### Kubernetes

> [!IMPORTANT]  
> Currently storage-hub on k8 is having issues due to how we are generating chain specs, you can skip directly to [Spawning ZombieNet Native](#spawning-zombienet-native)

For simplicity, we can use minikube to be a local [kubernetes](https://kubernetes.io/) cluster.

Visit their [docs](https://minikube.sigs.k8s.io/docs/) for a guide on GettingStarted, but once installed can be started with:

```sh
minikube start
```

#### Creating Local Docker Image

_In `test/` directory:_

Run:

```sh
pnpm docker:build
```

to create a local Docker image `storage-hub:local`.

#### Running Local built via Docker

```sh
docker compose -f docker/local-node-compose.yml up -d
```

#### Running Latest built via Docker

```sh
docker compose -f docker/latest-node-compose.yml up -d
```

#### Zombienet

> [!NOTE]  
> Please ensure the rust project is built first e.g. `cargo build --release`

In `/test` run: `pnpm install` to install zombienet

### Running Standard Tests

```sh
pnpm test
```

### Running ZombieNet Tests

```sh
pnpm zombie:test:native
```

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

### Generating new Type Interfaces

This repo uses Parity's [polkadot-api](https://github.com/polkadot-api/polkadot-api) AKA PAPI.
To generate new type interfaces run the following in `/test`:

```sh
pnpm zombie:run:native
```

In another terminal window in `/test`:

```sh
pnpm scalegen
pnpm typegen
```

This will update the scale files, and create type interfaces from them into the `/typegen` directory.
These generated descriptors are used throughout the tests to interact with relay and StorageHub chain.

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
