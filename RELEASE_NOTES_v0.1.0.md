# StorageHub v0.1.0

## Summary

Initial public minor release of StorageHub as a reusable library for Substrate projects, including pallets, runtime, client, SDK, and backend image.

## Components

- Client code: v0.1.0
- Pallets code: v0.1.0
- Runtime code: v0.1.0 (spec_name/spec_version: parachain 1, solochain-evm 1)
- SH Backend Docker image: v0.1.0 (image: ghcr.io/Moonsong-Labs/storage-hub-msp-backend:v0.1.0)
- SH SDK (npm): v0.1.0
- types-bundle: v0.2.7
- api-augment: v0.2.8

## Changes since last tag

Base commit: 05d269a26d11c1ed8a6d917b3e08ff3b5d3d4b22

- Highlights:
  - fix: Make storage path consistent for snapshots of RocksDB (#540)
  - refactor: Redirect RPC download stream to user in download file endpoint (#526)
  - fix: Open bucket RocksDB forests on startup in MSP (#534)
  - fix: Error out of task if BSP distributing is already registered by other task (#532)
  - feat: report file status (#529)
  - fix: index the StorageRequestRejected event and update the file accordingly (#533)
  - feat: batch db queries for files marked for deletion (#528)
  - fix: backend logging (#530)
  - fix(backend): dynamic cost per tick as integer (#524)
  - Make MspClient immutable (#525)
  - Feat delete file and bucket (#492)
  - fix: RocksDB MSP fixes when receiving files (#520)
  - Filesystem contract address mandatory (#523)
  - refactor(indexer): use IncompleteStorageRequest as canonical event (#527)
  - feat: add standalone indexer service integration test (#512)
  - fix: make MSP delete files from its file storage after forest storage delete finalisation (#522)
  - feat: index request file deletion user signatures (#521)
  - fix: Improve structure and naming of RocksDB path when opening DB (#518)
  - feat: add backend logs (#519)
  - feat(fisherman): fisherman processes incomplete storage requests after syncing (#499)
  - fix: remove unneeded filesystem precompile address from contract (#515)
  - feat: unify authentication process (#503)
  - refactor: make backend RPC and MSP connections initialization retry indefinitely (#510)
  - feat: Abstract signed message adaptation to be configurable in runtime config (#514)
  - feat(file-system): batch file deletion support (#506)
- Full diff: https://github.com/Moonsong-Labs/storage-hub/compare/05d269a26d11c1ed8a6d917b3e08ff3b5d3d4b22...c81ce2f58132124bd152efe092218f3790ecf0c7

## Migrations

### RocksDB (File Storage)

- Changes:
  - New refcount column to track number of files using a given fingerprint.
  - Prefixing existing file keys with bucket id, to allow efficient deletion of all files in a bucket.
  - Path consistency and naming improvements in RocksDB directories
  - MSP opens bucket forests on startup
- Action required: For a running chain, would require migration to new schema with new refcount column and prefixing existing file keys with bucket id.

### RocksDB (Forest Storage)

- Changes:
  -
- Action required: No schema-level migrations detected.

### RocksDB (State store)

- Changes: None that require migration.
- Action required: None.

### Indexer DB (Postgres)

- Migrations present (applied on indexer startup):
  - 2024-09-20-035333_create_service_state
  - 2024-09-26-132546_create_multiaddress
  - 2024-09-26-145832_create_bsp
  - 2024-09-27-112918_create_msp
  - 2024-09-27-125722_add_blockchain_id_for_msp_and_bsp
  - 2024-09-27-152605_create_bucket
  - 2024-10-01-112655_create_paymentstream
  - 2024-10-07-133907_create_file
  - 2024-10-07-133908_create_peer_id
  - 2024-11-15-160045_track_merkle_roots
  - 2025-07-18-085225_create_msp_file_association
  - 2025-07-18-104055_create_bsp_file_association
  - 2025-09-17-081751_add_payment_stream_types
  - 2025-10-08-000001_rename_service_state_last_processed_block_to_last_indexed_finalized_block
  - 2025-10-22-142857_add_deletion_signature_to_file
  - 2025-10-24-163538_add_indexes_for_batch_deletion_queries
- How to apply: The indexer service runs migrations automatically on startup. Alternatively run `diesel migration run`.

## Runtime

- Upgrades (spec_version): parachain and solochain-evm remain at spec_version 1.
- Migrations: No runtime storage migrations detected.
- Constants changed: None requiring operator action.
- Scripts to run: None.

## Client

- Behaviour changes:
  - File deletion flows and fisherman handling of incomplete storage requests improved.
  - Batch DB queries for deletions; report file status; handle StorageRequestRejected.
- Initialisation changes:
  - MSP opens RocksDB forests on startup.

## Backend

- Behaviour changes:
  - Unified authentication process; backend logs added.
  - Download endpoint streams directly to user.
- Initialisation changes:
  - RPC/MSP connections retry indefinitely at startup.

## Versions

- Polkadot SDK: polkadot-stable2412-6
- Rust: 1.87 (from rust-toolchain.toml)

## Compatibility

- SH Backend v0.1.0 → compatible with pallets/runtime v0.1.0 and client v0.1.0.
- SDK v0.1.0 → compatible with backend v0.1.0, client v0.1.0, and pallets/runtime v0.1.0.

## Upgrade Guide

- Ensure indexer service is restarted to apply migrations automatically.
- No manual RocksDB migrations are provided for this release. But to keep a chain running, they should be implemented. Otherwise, restarting the chain from genesis is required.
