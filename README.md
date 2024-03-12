# Storage Hub

> [!IMPORTANT]
> 🏗️ This repo is very much work in progress!

## Overview

StorageHub is a storage optimized parachain that is designed to work with other Polkadot & Kusama parachains. It focuses on storing data in an efficient and decentralized way, while allowing that storage to be accessed, used, and managed by other parachains. It will be possible for users to directly interact with the storage on the chain, but StorageHub also seeks to natively interoperate with existing parachains via XCM.

### Layout

This repo contains all aspects relating to StorageHub, including clients, the runtime, tools and test apparatus. It is organized:

```sh
.
├── biome.json
├── bun.lockb
├── Cargo.lock
├── Cargo.toml             <---- Rust Workspace definition
├── client1                <---- Rust application project
│   ├── Cargo.toml
│   └── src
│       └── main.rs
├── client2
│   ├── Cargo.toml
│   └── src
│       └── main.rs
├── LICENSE
├── package.json
├── README.md
├── runtime                <---- Rust library project
│   ├── Cargo.toml
│   └── src
│       └── lib.rs
├── test                   <---- TypeScript module
│   ├── end2end
│   ├── integration
│   ├── package.json
│   ├── README.md
│   ├── runtime
│   │   └── sample.test.ts
│   └── tsconfig.json
└── tsconfig.json
```

## Installation

## Usage

## Testing

Please see the testing[README.md](test/README.md) for a full description.

## Links