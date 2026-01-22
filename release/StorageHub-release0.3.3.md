# StorageHub v0.3.3

## Summary

StorageHub v0.3.3 is a **patch release focused on BSP confirmation robustness and client block-processing correctness**. It introduces a new File System runtime API to let BSPs pre-filter confirm-storing requests (and adds retry logic for transient proof failures caused by concurrent forest modifications), fixes a race where finality notifications could be processed before their corresponding block import (by queueing finality until import processing completes), and removes an unnecessary restriction that prevented users/fishermen from updating payment streams when a provider is marked insolvent.

## Components

- Client code: v0.3.3
- Pallets code: v0.3.3
- Runtime code: v0.3.3 (spec_name/spec_version: parachain `sh-parachain-runtime`/1, solochain-evm `sh-solochain-evm`/1)
- SH Backend Docker image: v0.3.3 (image: ghcr.io/<org>/storage-hub-msp-backend:v0.3.3)
- SH SDK (npm): v0.4.3 (`@storagehub-sdk/core`, `@storagehub-sdk/msp-client`)
- types-bundle/api-augment (npm): `@storagehub/types-bundle` v0.3.1, `@storagehub/api-augment` v0.3.2

## Changes since last tag

Base: d27e41fb8805c33f63875ef470ebb8c0edc25887

- Highlights:

  - **BSP confirm storing retries + pre-filtering via runtime API**: BSP upload now pre-filters “file keys to confirm” using a new runtime API (`query_pending_bsp_confirm_storage_requests`) before generating proofs, and retries confirmation when proof verification fails due to concurrent forest changes (handling `ForestProofVerificationFailed` / `FailedToApplyDelta` by re-queuing) ([PR #624](https://github.com/Moonsong-Labs/storage-hub/pull/624)).
  - **Finality/import ordering fix (no more undefined behaviour)**: finality notifications are queued if they arrive before their corresponding block import has been fully processed, and drained after import processing catches up; this avoids processing finality for blocks that have not yet completed import-side effects ([PR #640](https://github.com/Moonsong-Labs/storage-hub/pull/640)).
  - **Payment streams update allowed for insolvent providers**: removing the “provider must be solvent” pre-check for updates lets users/fishermen update payment streams even if a BSP/MSP is marked insolvent (unblocking file deletion flows) ([PR #657](https://github.com/Moonsong-Labs/storage-hub/pull/657)).

- Full diff: https://github.com/Moonsong-Labs/storage-hub/compare/d27e41fb8805c33f63875ef470ebb8c0edc25887...c1c32684cf374fdd70b4f3a25f17ce0df15fd853
- PRs included:
  - #657 fix: :bug: Allow updating payment stream for insolvent provider
  - #640 fix: :bug: Process finality notifications only after processing the corresponding block import
  - #624 feat(bsp): ✨ Add retry logic for confirm storing proof errors

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
  - Blockchain service state store now persists full “last processed block” info (number + hash) via a new RocksDB column family `last_processed_block` (keeping the old `last_processed_block_number` CF for backward compatibility).
  - Finality processing state is persisted via `last_finalised_block` to avoid redundant finality re-processing after restart ([PR #640](https://github.com/Moonsong-Labs/storage-hub/pull/640)).
- Action required:
  - None. The blockchain service state store creates missing column families automatically on startup.

### Indexer DB (Postgres)

- Migrations:
  - No new Postgres migrations in this release.
- How to apply: The indexer service runs migrations automatically on startup. Alternatively run `diesel migration run`.

## ⚠️ Breaking Changes ⚠️

- [PR #624](https://github.com/Moonsong-Labs/storage-hub/pull/624): Adds a new File System runtime API method `query_pending_bsp_confirm_storage_requests` under `FileSystemApi`; downstream/custom runtimes integrating StorageHub must implement this API.

## Runtime

- Upgrades (spec_version): parachain and solochain-evm remain at spec_version 1.
- Behaviour changes:
  - **File System runtime API for BSP confirmations**: adds `query_pending_bsp_confirm_storage_requests` to `pallet_file_system_runtime_api::FileSystemApi`, allowing BSPs to filter a list of file keys to only those that still require confirm-storing (excluding already-confirmed, non-volunteer, or non-existent storage requests) ([PR #624](https://github.com/Moonsong-Labs/storage-hub/pull/624)).
  - **Payment Streams: allow updates even if provider is insolvent**: removes the “provider must be solvent” pre-check from `update_fixed_rate_payment_stream` and `update_dynamic_rate_payment_stream`, so user-driven updates are not blocked purely because the provider is marked insolvent ([PR #657](https://github.com/Moonsong-Labs/storage-hub/pull/657)).
- Migrations: None (runtime storage layout unchanged in this release range).
- Constants changed: None requiring operator action.
- Scripts to run: None.

## Client

- Behaviour changes:
  - **BSP confirm storing robustness**:
    - Pre-filters storage requests using the new runtime API before proof generation (`query_pending_bsp_confirm_storage_requests`).
    - Retries confirm-storing when proof verification fails due to concurrent forest modifications, re-queuing on `ForestProofVerificationFailed` and `FailedToApplyDelta`.
    - Tracks pending volunteer transactions so confirm-storing requests are not incorrectly filtered out if the volunteer tx is not yet finalised on-chain; such requests are re-queued instead of discarded (fixes flaky integration tests) ([PR #624](https://github.com/Moonsong-Labs/storage-hub/pull/624)).
  - **Block processing correctness**: queues finality notifications that arrive “too early” (before import processing has completed for that block) and drains them after each block import, ensuring finality is only processed once the corresponding block import side-effects have been fully applied ([PR #640](https://github.com/Moonsong-Labs/storage-hub/pull/640)).
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

- SH Backend v0.3.3 → compatible with pallets/runtime v0.3.3 and client v0.3.3 (all built from this release range).
- SDK v0.4.3 → compatible with backend v0.3.3, client v0.3.3, and pallets/runtime v0.3.3.
- types-bundle v0.3.1 + api-augment v0.3.2 → compatible with this runtime release’s metadata; regenerate if you run custom runtimes.

## Upgrade Guide

### General upgrade notes

- The blockchain service state store has a small on-disk schema extension (new RocksDB column families) but it is applied automatically on startup; no manual operator action is required ([PR #640](https://github.com/Moonsong-Labs/storage-hub/pull/640)).
- If you are integrating StorageHub into a downstream runtime, ensure you update runtime APIs and regenerate bindings as needed before rolling out clients/backends.

### Breaking PRs

- [PR #624](https://github.com/Moonsong-Labs/storage-hub/pull/624) – New File System runtime API for BSP confirm-storing pre-filtering

  - **Short description**:

    Added a new runtime API `query_pending_bsp_confirm_storage_requests` to the `FileSystemApi`. This API allows BSPs to filter a list of file keys to only those that still require confirmation (i.e., where the BSP has volunteered but not yet confirmed storing).

  - **Who is affected**:

    - Custom runtimes using StorageHub pallets: Must implement the new runtime API method in their FileSystemApi implementation.

  - **Suggested code changes**:

    - No changes are required for nodes running with the pre-configured Parachain and Solochain runtimes.
    - Implement the runtime API for custom runtime:

      ```rust
      impl pallet_file_system_runtime_api::FileSystemApi<Block, ...> for Runtime {
          // ... existing methods ...

          fn query_pending_bsp_confirm_storage_requests(
              bsp_id: BackupStorageProviderId,
              file_keys: Vec<FileKey>,
          ) -> Vec<FileKey> {
              FileSystem::query_pending_bsp_confirm_storage_requests(&bsp_id, file_keys)
          }
      }
      ```

