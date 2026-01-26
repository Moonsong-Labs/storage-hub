# StorageHub v0.3.4

## Summary

StorageHub v0.3.4 is a **patch release focused on SDK gas handling, backend session cleanup, and client slashing optimisation**. It introduces automatic EIP-1559 gas limit computation in the SDK (adapting to network congestion while still allowing manual overrides), improves download session robustness in the backend by using RAII-style guards for automatic cleanup on failure or disconnection, and prevents the client from repeatedly submitting no-op slash extrinsics for providers that have zero capacity/stake.

## Components

- Client code: v0.3.4
- Pallets code: v0.3.4
- Runtime code: v0.3.4 (spec_name/spec_version: parachain `sh-parachain-runtime`/1, solochain-evm `sh-solochain-evm`/1)
- SH Backend Docker image: v0.3.4 (image: ghcr.io/<org>/storage-hub-msp-backend:v0.3.4)
- SH SDK (npm): v0.4.4 (`@storagehub-sdk/core`, `@storagehub-sdk/msp-client`)
- types-bundle/api-augment (npm): `@storagehub/types-bundle` v0.3.2, `@storagehub/api-augment` v0.3.2

## Changes since last tag

Base: 57d2a195d58d39e0d6e38a927ec312dd0f640522

- Highlights:

  - **SDK adaptive gas limit**: SDK now automatically computes EIP-1559 gas limits by querying the base fee from the latest block and setting a higher limit to handle periods of high network traffic; users can still override these values by providing an `EvmWriteOptions` object when calling precompile methods ([PR #655](https://github.com/Moonsong-Labs/storage-hub/pull/655)).
  - **Backend download session robustness**: download sessions now use a guard that automatically removes the session from active downloads when dropped, ensuring cleanup on failure, disconnection, or RPC response errors—matching the existing pattern for upload sessions ([PR #656](https://github.com/Moonsong-Labs/storage-hub/pull/656)).
  - **Skip slashing zero-capacity providers**: the client now queries a provider's capacity before attempting to slash them, preventing repeated submission of no-op slash extrinsics for providers with 0 capacity/stake ([PR #661](https://github.com/Moonsong-Labs/storage-hub/pull/661)).

- Full diff: https://github.com/Moonsong-Labs/storage-hub/compare/57d2a195d58d39e0d6e38a927ec312dd0f640522...92747d5122472be1d4e072b439589b9b64a224b7
- PRs included:
  - #661 fix: 🩹 Skip slashing storage provider with zero capacity
  - #656 fix(backend): 🐛 drop download sessions when failing or disconnecting
  - #655 feat: ✨ SDK adaptative gas limit

## Migrations

### RocksDB (File Storage)

- Changes:
  - No new schema changes in this release.
- Action required:
  - None.

### RocksDB (Forest Storage)

- Changes:
  - No new schema changes in this release.
- Action required:
  - None.

### RocksDB (State store)

- Changes:
  - No new schema changes in this release.
- Action required:
  - None.

### Indexer DB (Postgres)

- Migrations:
  - No new Postgres migrations in this release.
- How to apply: The indexer service runs migrations automatically on startup. Alternatively run `diesel migration run`.

## ⚠️ Breaking Changes ⚠️

None. All PRs included in this release are labelled `not-breaking` and do not introduce breaking changes to public APIs, runtime storage layouts, or configuration surfaces.

## Runtime

- Upgrades (spec_version): parachain and solochain-evm remain at spec_version 1.
- Behaviour changes:
  - None in this release range.
- Migrations: None (runtime storage layout unchanged in this release range).
- Constants changed: None requiring operator action.
- Scripts to run: None.

## Client

- Behaviour changes:
  - **Skip slashing zero-capacity providers**: before attempting to slash a storage provider, the client now queries the provider's capacity; providers with 0 capacity/stake are skipped, preventing repeated submission of no-op slash extrinsics ([PR #661](https://github.com/Moonsong-Labs/storage-hub/pull/661)).
- Initialisation / configuration changes:
  - None.

## Backend

- Behaviour changes:
  - **Download session cleanup robustness**: each download session now has a guard that automatically removes the session from the active downloads map when dropped; this ensures proper cleanup regardless of how the download task exits (success, failure, RPC errors, or disconnection), matching the existing pattern used for upload sessions ([PR #656](https://github.com/Moonsong-Labs/storage-hub/pull/656)).
- Initialisation / configuration changes:
  - None.

## SDK

- Behaviour changes:
  - **Automatic EIP-1559 gas limit computation**: the SDK now retrieves the base fee from the latest block and sets a higher gas limit to handle periods of high network traffic automatically; users can still override these values by providing an `EvmWriteOptions` object when calling any method from the precompiles ([PR #655](https://github.com/Moonsong-Labs/storage-hub/pull/655)).
- Initialisation changes:
  - None (SDK is now at v0.4.4).

## Versions

- Polkadot SDK: polkadot-stable2412-6
- Rust: 1.91 (from rust-toolchain.toml)

## Compatibility

- SH Backend v0.3.4 → compatible with pallets/runtime v0.3.4 and client v0.3.4 (all built from this release range).
- SDK v0.4.4 → compatible with backend v0.3.4, client v0.3.4, and pallets/runtime v0.3.4.
- types-bundle v0.3.2 + api-augment v0.3.2 → compatible with this runtime release's metadata; regenerate if you run custom runtimes.

## Upgrade Guide

### General upgrade notes

- No database migrations are required in this release; upgrading from v0.3.3 should be seamless.
- The SDK gas limit changes are automatic and require no code changes; if you were manually setting gas limits, your existing `EvmWriteOptions` overrides will continue to work.

### Breaking PRs

None. Upgrading from the previous release should be seamless. All PRs included in this release are labelled `not-breaking` and do not introduce breaking changes to public APIs, runtime storage layouts, or configuration surfaces.
