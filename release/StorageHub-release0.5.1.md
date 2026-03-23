# StorageHub v0.5.1

## Summary

StorageHub v0.5.1 is a patch release that fixes a missing `storage_proof_size::HostFunctions` registration for solochain builds. Without this fix, the solochain executor would panic when Frontier's EVM runner calls `get_proof_size()`, since the host function was not included in the solochain `HostFunctions` type tuple.

## Components

- Client code: v0.5.1
- Pallets code: v0.5.1
- Runtime code: v0.5.1 (spec_name/spec_version: parachain `sh-parachain-runtime`/1, solochain-evm `sh-solochain-evm`/1)
- SH Backend Docker image: v0.5.1 (image: `moonsonglabs/storage-hub-msp-backend:v0.5.1`)
- SH SDK (npm): v0.7.3 (`@storagehub-sdk/core`, `@storagehub-sdk/msp-client`)
- types-bundle/api-augment (npm): `@storagehub/types-bundle` v0.5.1, `@storagehub/api-augment` v0.5.1

## Changes since last tag

Base: `v0.5.0` (`5c5d2521`)

- Highlights:
  - **Solochain host function fix**: Added `cumulus_primitives_proof_size_hostfunction::storage_proof_size::HostFunctions` to the solochain `HostFunctions` type tuple in `client/common/src/types.rs`, for both the standard and `runtime-benchmarks` feature-gated variants. This is required because Frontier's EVM runner (`stable2503`) unconditionally calls `get_proof_size()` internally, and the host function must be registered even for solochains where `ProofSizeExt` is not active ([PR #713](https://github.com/Moonsong-Labs/storage-hub/pull/713)).

- Full diff: https://github.com/Moonsong-Labs/storage-hub/compare/v0.5.0...v0.5.1
- PRs included:
  - #713 fix: added missing storage_proof_size::HostFunctions for solochain

## Migrations

### RocksDB (File Storage)

- Changes: None.
- Action required: None.

### RocksDB (Forest Storage)

- Changes: None.
- Action required: None.

### RocksDB (State store)

- Changes: None.
- Action required: None.

### Indexer DB (Postgres)

- Migrations: None.

## Runtime

- Upgrades (spec_version): No change.
- Migrations: None.
- Constants changed: None.
- Scripts to run: None.

## Client

- Behaviour changes: The solochain executor now correctly registers the `storage_proof_size` host function, preventing potential panics when Frontier's EVM runner calls `get_proof_size()`.
- Initialisation changes: None.

## Backend

- Behaviour changes: None.
- Initialisation changes: None.

## SDK

- Behaviour changes: None.
- Initialisation changes: None.

## Versions

- Polkadot SDK: `stable2503`
- Rust: 1.90

## Compatibility

- SH Backend v0.5.1 → Compatible with pallets/client v0.5.1 and v0.5.0 (no runtime or storage changes).
- SDK v0.7.3 → Compatible with backend/client/pallets v0.5.1 and v0.5.0.

## Upgrade Guide

None. Upgrading from v0.5.0 should be seamless. The single PR included in this release is labelled `not-breaking` and does not introduce breaking changes to public APIs, runtime storage layouts, or configuration surfaces.
