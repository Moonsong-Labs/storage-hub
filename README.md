# Storage Hub

> [!IMPORTANT]
> 🏗️ This repo is very much work in progress!

## Overview

StorageHub is a storage optimized parachain that is designed to work with other Polkadot & Kusama parachains. It focuses on storing data in an efficient and decentralized way, while allowing that storage to be accessed, used, and managed by other parachains. It will be possible for users to directly interact with the storage on the chain, but StorageHub also seeks to natively interoperate with existing parachains via XCM.

### Layout

This repo contains all aspects relating to StorageHub, including clients, the runtime, tools and test apparatus. It is organized:

```sh
.
├── Cargo.lock
├── Cargo.toml
├── Containerfile
├── LICENSE
├── README.md
├── biome.json
├── bsp                    <---- backup storage provider application
├── bun.lockb
├── msp                    <---- main storage provider application
├── node                   <---- storage-hub substrate client side module
├── package.json
├── pallets                <---- storage-hub pallets
├── runtime                <---- storage-hub runtime
├── storage-kit            <---- storage provider library kit
├── test                   <---- TypeScript module
├── tmp.txt
├── tsconfig.json
└── zombienet
```

## Installation

## Usage

## Testing

Please see the testing[README.md](test/README.md) for a full description.

## StorageHub Runtime

A Cumulus based runtime implementation for StorageHub.

For a more detail explanation on the StorageHub runtime, please refer to the [StorageHub design document](https://github.com/Moonsong-Labs/storage-hub-design-proposal/blob/main/techincal_design/runtimeBreakdown.md) presented for the grant application of this project.

### Running StorageHub with Zombienet

In the [zombienet directory](./zombienet) you'll find a [config.toml](./zombienet/config.toml) that configures Zombienet to run a Rococo-like relay chain with 4 validators, as well as one collator for StorageHub's parachain.

To run this network, you'll need to:

1. Compile the StorageHub runtime with `cargo build --release`.
2. Get Polkadot's binaries:
   1. If you're running on a Linux machine, you can download the binaries from the [Releases](https://github.com/paritytech/polkadot-sdk/releases/) page. The current version at the moment of writing is `v1.5.0`, but the latest version should work as well. You should download the `polkadot`, `polkadot-execute-worker` and `polkadot-prepare-worker` binaries.
   2. If you're running on a Mac, you'll need to compile the binaries yourself. You can do so by cloning the [Polkadot repository](https://github.com/paritytech/polkadot-sdk) and running `cargo build --release` in the root directory. You'll find the binaries in the `target/release` directory.
3. Move the binaries to the `zombienet/bin` directory and make them executable with `chmod +x ./zombienet/bin/*`.
4. Download the corresponding `zombienet` binary for your platform from [Zombienet Releases](https://github.com/paritytech/zombienet/releases). and move it to the `zombienet/bin` directory. Make it executable with `chmod +x ./zombienet/bin/zombienet`.
5. From the root folder of the project, run `./zombienet/bin/zombienet-{your-platform} spawn ./zombienet/config.toml -p native`
