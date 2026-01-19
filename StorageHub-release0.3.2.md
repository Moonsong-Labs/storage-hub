# StorageHub v0.3.2

## Summary

StorageHub v0.3.2 is a **small patch release focused on MSP/Fisherman robustness**. It prevents MSPs from creating local forest instances during the post-sync sanity check when the bucket is missing locally (treating this as valid iff the on-chain root is the default/empty root), and ensures the Fisherman can always make forward progress by truncating batch deletions to the on-chain `MaxFileDeletionsPerExtrinsic` limit (using `BoundedVec` end-to-end).

## Components

- Client code: v0.3.0
- Pallets code: v0.3.0
- Runtime code: v0.3.0 (spec_name/spec_version: parachain `sh-parachain-runtime`/1, solochain-evm `sh-solochain-evm`/1)
- SH Backend Docker image: v0.3.0 (image: ghcr.io/<org>/storage-hub-msp-backend:v0.3.0)
- SH SDK (npm): v0.4.3 (`@storagehub-sdk/core`, `@storagehub-sdk/msp-client`)
- types-bundle/api-augment (npm): `@storagehub/types-bundle` v0.3.0, `@storagehub/api-augment` v0.3.1

## Changes since last tag

Base: ea8181cb71d15b62db766debb025927a40efbbcd

- Highlights:

  - **MSP post-sync sanity check no longer creates forests for missing buckets**: when a bucket forest is missing locally after initial sync, MSPs now treat it as valid *only if* the on-chain bucket root is the default/empty root (`DefaultMerkleRoot::<Runtime>::get()`); otherwise it is reported as **CRITICAL** (bucket is not empty but local forest is missing) ([PR #652](https://github.com/Moonsong-Labs/storage-hub/pull/652)).
  - **Fisherman batch deletions always make progress**: the Fisherman now truncates “files to delete” batches to the on-chain `MaxFileDeletionsPerExtrinsic` constant before building extrinsics, avoiding a stuck loop where oversized batches repeatedly fail. The processing pipeline now uses `BoundedVec::truncate_from(...)` (and defensive warnings if later stages still truncate) ([PR #654](https://github.com/Moonsong-Labs/storage-hub/pull/654)).

- Full diff: https://github.com/Moonsong-Labs/storage-hub/compare/ea8181cb71d15b62db766debb025927a40efbbcd...d27e41fb8805c33f63875ef470ebb8c0edc25887
- PRs included:
  - #654 fix: Fisherman truncate files to delete based on `MaxFileDeletionsPerExtrinsic` runtime constant
  - #652 fix: :adhesive_bandage: Avoid creating forests locally when not found in initial check after sync

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

None. Upgrading from the previous release should be seamless. All PRs included in this release are labelled `not-breaking` and do not introduce breaking changes to public APIs, runtime storage layouts, or configuration surfaces.

## Runtime

- Upgrades (spec_version): parachain and solochain-evm remain at spec_version 1.
- Behaviour changes: None in this release range.
- Migrations: None.
- Constants changed: None requiring operator action.
- Scripts to run: None.

## Client

- Behaviour changes:
  - **MSP post-sync root verification**:
    - If a local bucket forest exists after sync, its local root must match the on-chain root; mismatches are reported as **CRITICAL**.
    - If a local bucket forest does **not** exist, the on-chain root must be the default/empty root (`DefaultMerkleRoot::<Runtime>::get()`); otherwise it is reported as **CRITICAL** (bucket is not empty but local forest is missing) ([PR #652](https://github.com/Moonsong-Labs/storage-hub/pull/652)).
  - **Fisherman batch deletions**:
    - The Fisherman truncates the number of files processed for deletion per cycle to the on-chain `MaxFileDeletionsPerExtrinsic` limit, ensuring extrinsic submission never fails purely due to an oversized batch and that remaining deletions are processed in subsequent cycles.
    - The extrinsics `delete_files` and `delete_files_for_incomplete_storage_request` now receive `BoundedVec` inputs constructed via `BoundedVec::truncate_from(...)` to avoid conversion errors ([PR #654](https://github.com/Moonsong-Labs/storage-hub/pull/654)).
- Initialisation / configuration changes:
  - None.

## Backend

- Behaviour changes: None in this release range.
- Initialisation / configuration changes: None.

## SDK

- Behaviour changes: None in this release range.
- Initialisation changes: None (SDK remains at v0.4.3).

## Versions

- Polkadot SDK: polkadot-stable2412-6
- Rust: 1.91 (from rust-toolchain.toml)

## Compatibility

- SH Backend v0.3.0 → compatible with pallets/runtime v0.3.0 and client v0.3.0 (all built from this release range).
- SDK v0.4.3 → compatible with backend v0.3.0, client v0.3.0, and pallets/runtime v0.3.0.
- types-bundle v0.3.0 + api-augment v0.3.1 → compatible with this runtime release’s metadata; regenerate if you run custom runtimes.

## Upgrade Guide

None. Upgrading from the previous release should be seamless. All PRs included in this release are labelled `not-breaking` and do not introduce breaking changes to public APIs, runtime storage layouts, or configuration surfaces.

