# StorageHub v0.4.3

## Summary

StorageHub v0.4.3 is a focused **bug-fix release** that resolves a **repeated fingerprint upload issue** affecting MSPs. When an MSP already stored a file and received a second upload request for a different file key sharing the same fingerprint, the deduplication logic incorrectly rejected the upload because no new chunks were inserted (resulting in `chunks_count` of 0). The fix makes the file fingerprint the authoritative indicator of file completeness in file storage, relegating chunk count to progress tracking only. An integration test covering the scenario has been added.

## Components

- Client code: v0.4.3
- Pallets code: v0.4.3
- Runtime code: v0.4.3 (spec_name/spec_version: parachain `sh-parachain-runtime`/1, solochain-evm `sh-solochain-evm`/1)
- SH Backend Docker image: v0.4.3 (image: moonsonglabs/storage-hub-msp-backend:v0.4.3)
- SH SDK (npm): v0.4.6 (`@storagehub-sdk/core`, `@storagehub-sdk/msp-client`)
- types-bundle/api-augment (npm): `@storagehub/types-bundle` v0.4.3, `@storagehub/api-augment` v0.4.3

## Changes since last tag

Base: 5b52af21ca6c60db96bb7c3fe7c069075e941614

- Highlights:
  - **Repeated fingerprint upload fix**: MSPs now correctly handle uploads for different file keys that share the same fingerprint. Previously, when an MSP already held a file and received a second upload for a different file key with an identical fingerprint, the deduplication logic in the file storage rejected the upload because no new chunks were inserted into the trie (`chunks_count` was 0), causing the MSP to consider the file incomplete or inconsistent. The fix makes the fingerprint the sole authority on whether a file is complete in file storage; the chunk count is now used only for progress tracking ([PR #700](https://github.com/Moonsong-Labs/storage-hub/pull/700)).

- Full diff: https://github.com/Moonsong-Labs/storage-hub/compare/5b52af21ca6c60db96bb7c3fe7c069075e941614...2a82a5ba58fe8e19053f5840ddd9a27ec88a5d20
- PRs included:
  - #700 Fix: 🐛 Repeated fingerprint upload bug

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
  - No mandatory migrations in this release.
- Action required:
  - None.

### Indexer DB (Postgres)

- Migrations:
  - No new migrations in this release.
- How to apply: The indexer service runs migrations automatically on startup. Alternatively: `diesel migration run`.

## ⚠️ Breaking Changes ⚠️

None. All PRs included in this release are labelled `not-breaking` and do not introduce breaking changes to public APIs, runtime storage layouts, or configuration surfaces.

## Runtime

- Upgrades (spec_version): parachain and solochain-evm remain at spec_version 1.
- Migrations: none in this release.
- Behaviour changes:
  - No runtime logic/API changes were introduced between this base/head range.
- Constants changed: None requiring operator action.
- Scripts to run: None.

## Client

- Behaviour changes:
  - **Repeated fingerprint upload fix**: the trusted file transfer server on MSPs no longer rejects uploads for file keys whose fingerprint already exists in file storage. Previously, when a deduplicated file (same fingerprint, different file key) was uploaded, the server observed `chunks_count == 0` (no new chunks inserted) and treated the file as incomplete or inconsistent, returning an error to the backend. Now, file completeness is determined exclusively by the stored fingerprint matching the expected fingerprint; `chunks_count` is used only for progress tracking. This ensures that two distinct file keys sharing the same underlying data can both be correctly registered by the MSP ([PR #700](https://github.com/Moonsong-Labs/storage-hub/pull/700)).
- Initialisation / configuration changes:
  - No new CLI options or configuration changes in this release.

## Backend

- Behaviour changes:
  - No backend-specific changes in this release. The upload fix is in the client-side trusted file transfer server that the backend communicates with (see Client section).
- Initialisation / configuration changes:
  - None.

## SDK

- Behaviour changes:
  - No SDK changes in this release.
- Initialisation changes:
  - SDK npm packages remain at **v0.4.6** (`@storagehub-sdk/core`, `@storagehub-sdk/msp-client`).

## Versions

- Polkadot SDK: polkadot-stable2412-6
- Rust: 1.90 (from rust-toolchain.toml)

## Compatibility

- SH Backend v0.4.3 → compatible with pallets/runtime v0.4.3 and client v0.4.3 (all built from this release).
- SDK v0.4.6 → compatible with backend v0.4.3, client v0.4.3, and pallets/runtime v0.4.3.
- types-bundle v0.4.3 + api-augment v0.4.3 → compatible with this runtime release's metadata; regenerate if you run custom runtimes.

## Upgrade Guide

None. Upgrading from the previous release should be seamless. All PRs included in this release are labelled `not-breaking` and do not introduce breaking changes to public APIs, runtime storage layouts, or configuration surfaces.
