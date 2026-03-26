P# StorageHub

> [!IMPORTANT]
> 🏗️ This repo is very much work in progress!

---

## Overview

StorageHub is a storage-optimized parachain codebase, designed to work with other Polkadot & Kusama parachains. It focuses on storing data efficiently and in a decentralized manner, while also enabling data to be accessed, used, and managed by other parachains. It will be possible for users to directly interact with StorageHub and their files stored there, but StorageHub mainly seeks to natively interoperate with existing parachains via XCM.

### Layout

This repo contains various components related to StorageHub, including clients, runtime, tools and test apparatus.
It is organized:

```sh
.
├── .github                <---- GitHub Actions and related files
├── client                 <---- storage-hub substrate client-side modules
├── configs                <---- Example TOML configuration files for providers, fisherman and backend
├── docker                 <---- storage-hub docker files, scripts and keystore files
├── node                   <---- storage-hub node implementation
├── pallets                <---- storage-hub pallets
├── primitives             <---- storage-hub primitives
├── prompts                <---- storage-hub prompts for AI agents to do specific "repetitive" tasks
├── release                <---- storage-hub release utilities and release notes
├── resources
├── runtime                <---- storage-hub runtime
├── scripts                <---- storage-hub utility scripts for automating repetitive tasks
├── sdk                    <---- storage-hub SDK
├── support                <---- Traits and implementations used by storage-hub
├── test                   <---- Testing module for storage-hub, including Zombienet and TypeScript tests
├── .gitignore
├── biome.json
├── Cargo.lock
├── Cargo.toml
├── Containerfile
├── LICENSE
├── package.json
├── README.md
├── rust-toolchain
└── tsconfig.json
```

Sample configuration files live under `configs/`:

- `configs/msp_config.toml` – example configuration for an MSP node
- `configs/bsp_config.toml` – example configuration for a BSP node
- `configs/fisherman_config.toml` – example configuration for the fisherman service
- `configs/backend_config.toml` – example configuration for the MSP backend service

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

## FAQ

### I've just updated a RuntimeApi or RPC call, how do I update the fn signatures in polkadot{.js} API/App?

Navigate to the `/types-bundle` package and make your changes to `/src/rpc.ts` and `/src/runtime.ts`.

Any new Structs or ErrorEnums can be defined at `/src/types.ts` , using existing examples for guidance, and any new branded types as well.

After updating `types-bundle`, validate the change using the documented type generation flow.

First build the node binary from the repository root:

```sh
cargo build --release
```

#### Linux

Then run type generation:

```sh
bun run --cwd test typegen
```

#### macOS

If you are running locally on macOS, perform the full cross-build and Docker-backed type-generation flow from the `/test` directory:

```sh
bun install
bun run crossbuild:mac
bun run docker:build
bun run crossbuild:mac:backend
bun run docker:build:backend
bun run typegen
```

From there you should be able to see the new RPC in your tests for the `EnrichedBspApi` object.

> [!TIP]
> If the TS is still yelling at you with red squiggles, use command pallete to restart language server.
