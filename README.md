# Storage Hub

> [!IMPORTANT]
> 🏗️ This repo is very much work in progress!

---

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

## Component Description

### StorageHub Runtime

A Cumulus based runtime implementation for StorageHub.

For a more detail explanation on the StorageHub runtime, please refer to the [StorageHub design document](https://github.com/Moonsong-Labs/storage-hub-design-proposal/blob/main/techincal_design/runtimeBreakdown.md) presented for the grant application of this project.

### Main Storage Provider

> [!NOTE]  
> `TODO:` {Add Description}

### Backup Storage Provider

> [!NOTE]  
> `TODO:` {Add Description}

### File Uploader

> [!NOTE]  
> `TODO:` {Add Description}

## Usage

## Running StorageHub Chain with Zombienet

Full Instructions can be found: [here](test/README.md#local-usage).

## Testing

Please see the testing[README.md](test/README.md) for a full description.
