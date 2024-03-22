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

#### Creating Local Docker Image (Required)

_In `test/` directory:_

1. Run `bun docker:registry:start` to start a docker registry
2. Run `bun docker:build:sh-node` to build docker image locally
3. Run `bun docker:registry:push` to push new docker image to local registry

#### Zombienet

Easiest way to get the latest zombienet runner binary is via their [Releases](https://github.com/paritytech/zombienet/releases) page. For example:

```sh
wget https://github.com/paritytech/zombienet/releases/download/v1.3.95/zombienet-linux-x64
chmod +x zombienet-linux-x64
```

> [!IMPORTANT]  
> If using a Mac use the `macos` binary

### Running Tests

```sh
bun test
```

### Running Zombienet manually

Now that (finally) all the pieces are together, let's start a StorageHub parachain connected to a rococco relaychain.

`<zombienet-bin-name> spawn <config_path>`

For example:

```sh
./zombienet-linux-x64 spawn configs/simple.toml
```

From here you should see in the terminal, the different nodes being spun up. When the network is fully launched, you should see something like this:

![success](../resources/zombieSuccess.png)

From here you can interact via the websockets exposed in the direct links, in the example above we have:

- Alice (relay): `35005`
- Bob (relay): `37613`
- Collator (storage-hub): `45615`
