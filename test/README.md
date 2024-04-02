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

### Spawning ZombieNet

> [!INFORMATION]
> Currently SH on k8 is having issue due to how we are generating chain specs, hence why we use native provider

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
