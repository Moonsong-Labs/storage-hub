# Storage Hub

> [!IMPORTANT]
> 🏗️ This repo is very much work in progress!

---

## Overview

StorageHub is a storage-optimized parachain codebase designed to work with other Polkadot & Kusama parachains. It focuses on storing data efficiently and decentralizing access, while also enabling data to be accessed, used, and managed by other parachains. It will be possible for users to directly interact with the storage on the chain, but StorageHub also seeks to natively interoperate with existing parachains via XCM.

### Layout

This repo contains various components related to StorageHub, including clients, runtime, tools and test apparatus.
It is organized:

```sh
.
├── .github                <---- GitHub Actions and related files
├── client                 <---- storage-hub substrate client-side modules
├── node                   <---- storage-hub node implementation
├── pallets                <---- storage-hub pallets
├── primitives             <---- storage-hub primitives
├── resources
├── runtime                <---- storage-hub runtime
├── support                <---- Traits and implementations used by storage-hub
├── test                   <---- Testing module for storage-hub, including Zombienet and TypeScript tests
├── .gitignore
├── biome.json
├── bun.lockb
├── Cargo.lock
├── Cargo.toml
├── Containerfile
├── LICENSE
├── package.json
├── README.md
├── rust-toolchain
└── tsconfig.json
```

## Component Description

### StorageHub Runtime

A Cumulus-based runtime implementation for StorageHub, designed to enhance storage capabilities within the Polkadot network. For a more detailed explanation, please refer to the [StorageHub design document](https://github.com/Moonsong-Labs/storage-hub-design-proposal/blob/main/techincal_design/runtimeBreakdown.md) presented for the grant application of this project.

### Main Storage Provider

Main Storage Providers (MSPs) are responsible for offering data retrieval services with unique value propositions in an open market. They ensure high-quality data accessibility and are directly selected by users, enabling competitive service offerings that cater to specific user needs.

### Backup Storage Provider

Backup Storage Providers (BSPs) enhance the reliability and unstoppability of data storage. They operate in a decentralized network to provide redundancy and data backup services, ensuring that user data remains accessible even if primary providers face issues.

## Usage

### Running StorageHub Chain with Zombienet

Full instructions can be found [here](test/README.md#local-usage).

### Testing

For details on how to conduct tests on StorageHub, please see the testing [README.md](test/README.md).
