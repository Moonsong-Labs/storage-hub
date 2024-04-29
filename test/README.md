# StorageHub Testing

## Types

### Runtime Tests

### Integration Tests

### End-To-End Tests

## Local Usage

### Pre-requisites

#### Bun

[Bun](https://bun.sh) is used in this project as the Javascript runtime to execute tests in. To install it you can follow the official instructions at: [https://bun.sh/docs/installation](https://bun.sh/docs/installation)

The quickest way is via their script: `curl -fsSL https://bun.sh/install | bash`

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
bun docker:build
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

In `/test` run: `bun install` to install zombienet

### Running Standard Tests

```sh
bun test
```

### Running ZombieNet Tests

```sh
bun zombie:test:native
```

### Spawning ZombieNet Native

> [!TIP]  
> Polkadot binaries are required to run a zombienet network.
> For Linux you can run the script: `bun scripts/downloadPolkadot.ts <version>`
> For macOS you will have to [compile from source](https://github.com/paritytech/polkadot-sdk/tree/master/polkadot#build-from-source).

To launch a non-ephemeral ZombieNetwork by executing the following in: `/test` directory:

```sh
bun install
bun zombie:run:native
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
bun zombie:run:native
```

In another terminal window in `/test`:

```sh
bun scalegen
bun typegen
```

This will update the scale files, and create type interfaces from them into the `/typegen` directory.
These generated descriptors are used throughout the tests to interact with relay and StorageHub chain.
