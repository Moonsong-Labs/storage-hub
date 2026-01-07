# StorageHub v0.3.0

## Summary

StorageHub v0.3.0 focuses on **making MSP operations more robust at scale**, **hardening the indexer around file deletion / redundancy edge cases**, and **improving operability for runtime upgrades**. Highlights include a **trusted file transfer server** for backend uploads (moving away from RPC/proof-heavy uploads), **new file lifecycle states** (`revoked`/`rejected`) that propagate through backend + SDK, a **governance-controlled pause switch** for user-facing File System operations, and multiple fixes to prevent indexer stalls and MSP lag on large bucket counts.

## Components

- Client code: v0.3.0
- Pallets code: v0.3.0
- Runtime code: v0.3.0 (spec_name/spec_version: parachain `sh-parachain-runtime`/1, solochain-evm `sh-solochain-evm`/1)
- SH Backend Docker image: v0.3.0 (image: ghcr.io/<org>/storage-hub-msp-backend:v0.3.0)
- SH SDK (npm): v0.4.2 (`@storagehub-sdk/core`, `@storagehub-sdk/msp-client`)
- types-bundle/api-augment (npm): `@storagehub/types-bundle` v0.3.0, `@storagehub/api-augment` v0.3.1

## Changes since last tag

Base: d9a283293a2612a1a567ab5b6848e84e4ea0a858

- Highlights:

  - **Trusted file transfer for backend uploads**: MSP nodes can now spawn a **trusted HTTP upload server**, used by the backend to stream files without per-chunk proofs. This introduces new backend and node configuration fields and should be firewalled from the public internet ([PR #595](https://github.com/Moonsong-Labs/storage-hub/pull/595)).
  - **Indexer/runtime/backend/SDK file deletion hardening**: the indexer no longer deletes file records while there are open storage requests; the runtime blocks new storage requests/deletions in inconsistent states; backend + SDK handle two new file statuses (`revoked`, `rejected`) ([PR #596](https://github.com/Moonsong-Labs/storage-hub/pull/596)).
  - **Runtime operability**: File System user operations can now be paused granularly via a new storage bitmask and a Root-only extrinsic `set_user_operation_pause_flags` (and a new `UserOperationPaused` error) ([PR #632](https://github.com/Moonsong-Labs/storage-hub/pull/632)).
  - **Storage request correctness for “redundancy-only” flows**: prevent fisherman from incorrectly deleting files from buckets when users add redundancy, including introducing `MspStorageRequestStatus`/`msp_status` and new cleanup/event semantics ([PR #600](https://github.com/Moonsong-Labs/storage-hub/pull/600)).
  - **Required runtime migration for open storage requests**: adds a File System pallet migration to safely migrate `StorageRequestMetadata` to the new `msp_status` layout on upgrade ([PR #628](https://github.com/Moonsong-Labs/storage-hub/pull/628)).
  - **SIWX support (CAIP‑122)**: backend + SDK now support SIWX, with automatic domain extraction from URIs and an updated `MspClient` auth/session-provider story ([PR #592](https://github.com/Moonsong-Labs/storage-hub/pull/592)).
  - **Indexer DB consistency and repair**: new migrations to normalise `is_in_bucket` across sibling file records and to recalculate bucket stats (`file_count`, `total_size`) from linked file records ([PR #598](https://github.com/Moonsong-Labs/storage-hub/pull/598), [PR #619](https://github.com/Moonsong-Labs/storage-hub/pull/619)).
  - **MSP scalability**: remove an expensive “query all buckets” call from block import processing (fixing MSP lag with large bucket counts) and fix a race that could emit duplicate `NewStorageRequest` events ([PR #636](https://github.com/Moonsong-Labs/storage-hub/pull/636), [PR #638](https://github.com/Moonsong-Labs/storage-hub/pull/638)).

- Full diff: https://github.com/Moonsong-Labs/storage-hub/compare/d9a283293a2612a1a567ab5b6848e84e4ea0a858...017118ee91d23f401c45e7abfaa8ac82c8137de8
- PRs included:
  - #638 fix: 🚑 MSP race condition storage request emission
  - #637 fix: 🔊 Improve some printing of hex strings in logs
  - #636 fix: 🚑 Avoid iterating through all buckets at every event in block import notification
  - #635 fix: 🐛 Configure network before requesting accounts in wallet connection
  - #634 fix: 🩹 change `MaxMultiAddressSize` to match DataHaven's
  - #633 feat: check SDK version in the CI
  - #632 feat: ✨ Make user operations in File System pallet pausable
  - #631 feat: 🔈 Improve logs for Blockchain Service processing of notifications
  - #630 test: 🧪 Add test used to reproduce deadlock in Blockchain Service
  - #629 refactor: 🔒 Reduce file storage locks critical zone
  - #628 feat: ✨ add migration for `msp` field in file system's `StorageRequestMetadata`
  - #627 feat: ✨ Add script to cleanup files from MSP forest based on json
  - #626 feat: ✨ Script to bump versions and CI to check in release
  - #625 ci: 💚 Fix SDK CI Next JS workspace error
  - #623 refactor: 🔒 Narrow scope of usage of read and write locks to forest and file storage
  - #621 bench: update `delete_files_bucket` and `delete_files_bsp` benchmarks for worst case scenario
  - #619 fix: migration calculating calculating bucket stats (file_count, size) using files associated to bucket
  - #617 feat: ✨ mutations initial sync handlers
  - #616 test: 🔊 Improve integration test logs
  - #615 fix: 🐛 Create bucket forest on apply Forest mutation if it doesn't exist
  - #614 feat(file-transfer): use `TryConnect` for peer requests
  - #613 fix: 🐛 correctly emit mutations in events
  - #612 revert: ⏮️ revert back to using metamask recommended version from dappwright
  - #610 fix: 🐛 file record deduplication on deletion
  - #609 fix(e2e): use MetaMask 12.23.1 for dappwright compatibility
  - #608 build: ⬆️ Upgrade to Rust 1.90 and fix warnings
  - #607 feat: 🔊 Add verbose failure logs to indexing
  - #604 fix: 🚑 Query bucket of storage request by onchain ID
  - #603 fix: 🐛 general indexer improvements
  - #602 feat: 🎨 BSP stop storing improvements
  - #601 feat: 🎨 move bucket flow improvements
  - #600 fix: 🐛 multiple fixes to avoid the fisherman incorrectly deleting a file from a bucket
  - #599 style: 🎨 clean up file system pallet
  - #598 fix: 🚑 make it so `is_in_bucket` is consistent across same file key records
  - #596 fix: 🚑 indexer handling of file deletion
  - #595 feat: ✨ Implement a trusted file transfer for backend uploads
  - #593 fix: 🚑 Handle multiple file records for same `file_key`
  - #592 feat: SIWX
  - #591 feat: 🔊 Add success logs to all tasks and RPCs
  - #590 ci: 📝 Enforce breaking changes doc structure
  - #589 fix: 🚑 Avoid cleaning up tx store with genesis block
  - #585 test: persist backend logs from a tests run
  - #581 feat(shc-common): feature gate parachain dependencies
  - #571 feat(msp): MSP retries responding storage requests
  - #567 feat(backend): /stats endpoint
  - #543 feat: 🪟 Windows script compatibility

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
  - No mandatory migrations. Note that the Blockchain Service now also persists a block hash alongside the last processed block number (lazy “write-on-first-run” behaviour; see [PR #617](https://github.com/Moonsong-Labs/storage-hub/pull/617)).
- Action required:
  - None.

### Indexer DB (Postgres)

- Migrations:
  - `2025-12-05-191030_normalize_is_in_bucket_across_file_records` ([PR #598](https://github.com/Moonsong-Labs/storage-hub/pull/598))
  - `2025-12-15-153000_fix_bucket_stats` ([PR #619](https://github.com/Moonsong-Labs/storage-hub/pull/619))
- How to apply: The indexer service runs migrations automatically on startup. Alternatively run `diesel migration run`.

## ⚠️ Breaking Changes ⚠️

- [PR #571](https://github.com/Moonsong-Labs/storage-hub/pull/571): Storage Enable client trait changes (`StorageEnableRuntime::RuntimeError` associated type; `StorageEnableErrors::Other` now stores a `String`). Downstream runtimes implementing `StorageEnableRuntime` must update their trait impl and error conversion.
- [PR #581](https://github.com/Moonsong-Labs/storage-hub/pull/581): `ParachainClient` type alias renamed to `StorageHubClient`, and Cumulus host functions are now behind an explicit `shc-common` `parachain` feature. Downstream consumers must rename the type and set features appropriately.
- [PR #590](https://github.com/Moonsong-Labs/storage-hub/pull/590): CI enforces a breaking-changes documentation structure for PRs. This affects contributors (not runtime operators).
- [PR #592](https://github.com/Moonsong-Labs/storage-hub/pull/592): SIWX support changes SIWE/SIWX APIs: backend extracts domain from URI (remove `domain` field), and SDK `MspClient` session provider is now optional.
- [PR #595](https://github.com/Moonsong-Labs/storage-hub/pull/595): Backend uploads now use the trusted file transfer server by default and require new backend/node configuration (`trusted_file_transfer_server_url`, `use_legacy_upload_method`, `trusted_file_transfer_server{,_host,_port}`).
- [PR #596](https://github.com/Moonsong-Labs/storage-hub/pull/596): File lifecycle adds new statuses (`revoked`, `rejected`) surfaced by backend (`FileStatus`) and SDK; direct consumers of `/buckets/{bucket_id}/info/{file_key}` and SDK file info must handle them.
- [PR #598](https://github.com/Moonsong-Labs/storage-hub/pull/598): New Indexer DB migration is required to repair and normalise `is_in_bucket` across sibling file records.
- [PR #599](https://github.com/Moonsong-Labs/storage-hub/pull/599): File System pallet event order changed; deployments must upgrade in `runtime upgrade -> client upgrade` order (or as close to simultaneous as possible) to avoid SCALE decoding issues.
- [PR #600](https://github.com/Moonsong-Labs/storage-hub/pull/600): `StorageRequestMetadata` gains `msp_status` (and new events), requiring runtime upgrade + metadata update. A safe upgrade path is described in the Upgrade Guide.
- [PR #617](https://github.com/Moonsong-Labs/storage-hub/pull/617): Removes two client config parameters and adds a new Storage Providers runtime API `query_bucket_root`; node runners and runtime integrators must update configuration and runtime APIs.
- [PR #619](https://github.com/Moonsong-Labs/storage-hub/pull/619): New Indexer DB migration recalculates bucket stats (`file_count`, `total_size`); infra maintainers must run migrations.
- [PR #628](https://github.com/Moonsong-Labs/storage-hub/pull/628): Adds a runtime storage migration to migrate open storage requests to the new `msp_status` layout; runtimes must wire migrations into the Executive for upgrade.
- [PR #632](https://github.com/Moonsong-Labs/storage-hub/pull/632): Adds pause flags storage + new Root-only extrinsic and error variants; clients that decode errors/events must regenerate/upgrade bindings.

## Runtime

- Upgrades (spec_version): parachain and solochain-evm remain at spec_version 1.
- Migrations:
  - **File System storage request migration**: adds `pallet_file_system::migrations::v1::MigrateV0ToV1` to migrate `StorageRequestMetadata` to the new `msp_status` layout ([PR #628](https://github.com/Moonsong-Labs/storage-hub/pull/628), related to [PR #600](https://github.com/Moonsong-Labs/storage-hub/pull/600)).
- Behaviour changes:
  - **Pausable user operations**: new storage `UserOperationPauseFlagsStorage`, Root-only extrinsic `set_user_operation_pause_flags`, and error `UserOperationPaused` for paused operations ([PR #632](https://github.com/Moonsong-Labs/storage-hub/pull/632)).
  - **Deletion and redundancy safeguards**: runtime blocks new storage requests and deletion requests in inconsistent states, and fixes multiple `IncompleteStorageRequest` edge cases ([PR #596](https://github.com/Moonsong-Labs/storage-hub/pull/596), [PR #600](https://github.com/Moonsong-Labs/storage-hub/pull/600), [PR #615](https://github.com/Moonsong-Labs/storage-hub/pull/615)).
  - **Events emitted with full mutation data**: remove-mutation events now include the key value (metadata), improving provider reorg/revert handling ([PR #613](https://github.com/Moonsong-Labs/storage-hub/pull/613)).
  - **BSP stop-storing improvements**: payment stream updates happen during stop-storing request, and stop-storing is blocked when there is an open `IncompleteStorageRequest` ([PR #602](https://github.com/Moonsong-Labs/storage-hub/pull/602)).
- Constants changed: None requiring operator action.
- Scripts to run: None.

## Client

- Behaviour changes:
  - **MSP block import performance**: avoids querying all buckets for every event during block import processing, preventing MSP lag on chains with large bucket counts ([PR #636](https://github.com/Moonsong-Labs/storage-hub/pull/636)).
  - **MSP storage request robustness**:
    - Retry MSP responses for storage requests that fail due to proof invalidations by re-queueing and re-emitting `NewStorageRequest` when safe ([PR #571](https://github.com/Moonsong-Labs/storage-hub/pull/571)).
  - **Sync correctness**: providers now process blocks during initial sync and include new persistence of the last processed block hash (to reduce reorg edge cases) ([PR #617](https://github.com/Moonsong-Labs/storage-hub/pull/617)).
  - **File transfer connectivity**: MSP now uses `TryConnect` for `UploadRequest`/`DownloadRequest` peer requests, improving stability when distributing to many BSPs ([PR #614](https://github.com/Moonsong-Labs/storage-hub/pull/614)).
  - **Indexer and fisherman robustness**:
    - Better handling of multiple file records per `file_key` across deletion and step updates ([PR #593](https://github.com/Moonsong-Labs/storage-hub/pull/593), [PR #610](https://github.com/Moonsong-Labs/storage-hub/pull/610)).
    - Improved logging for success/failure paths across tasks and indexing ([PR #591](https://github.com/Moonsong-Labs/storage-hub/pull/591), [PR #607](https://github.com/Moonsong-Labs/storage-hub/pull/607)).
    - Move bucket flow improvements to correctly reset MSP-file associations ([PR #601](https://github.com/Moonsong-Labs/storage-hub/pull/601)).
  - **Trusted file transfer server**: nodes can spawn a trusted HTTP upload server (to be consumed by the backend) ([PR #595](https://github.com/Moonsong-Labs/storage-hub/pull/595)).
- Initialisation / configuration changes:
  - **Trusted file transfer server** (MSP nodes): add the following to MSP node configuration (for example in `configs/msp_config.toml`):
    - `trusted_file_transfer_server = true`
    - `trusted_file_transfer_server_host = "127.0.0.1"`
    - `trusted_file_transfer_server_port = 7070`
  - **Removed client config parameters**: remove `max_blocks_behind_to_catch_up_root_changes` and `sync_mode_min_blocks_behind` from client configuration ([PR #617](https://github.com/Moonsong-Labs/storage-hub/pull/617)).

## Backend

- Behaviour changes:
  - **Trusted uploads**: backend uploads are now sent to the MSP trusted file transfer server by default, replacing the prior RPC/proof-heavy upload mechanism. This requires new MSP backend configuration and firewalling the trusted server to backend-only access ([PR #595](https://github.com/Moonsong-Labs/storage-hub/pull/595)).
  - **SIWX support**: backend supports CAIP‑122 (SIWX) and domain is extracted from URI, affecting `/auth/nonce` and `/auth/message` request payloads ([PR #592](https://github.com/Moonsong-Labs/storage-hub/pull/592)).
  - **Indexer status surface**: backend models now include additional file statuses (`revoked`, `rejected`) returned by `/buckets/{bucket_id}/info/{file_key}` and related APIs ([PR #596](https://github.com/Moonsong-Labs/storage-hub/pull/596)).
  - **`/stats` endpoint correctness**: backend now reads chain storage via RPC and runtime APIs to return actual values, and the backend depends on runtime types for storage-key encoding and decoding ([PR #567](https://github.com/Moonsong-Labs/storage-hub/pull/567)).
  - **Compatibility fix**: `MaxMultiAddressSize` updated to match DataHaven so SCALE decoding works as expected ([PR #634](https://github.com/Moonsong-Labs/storage-hub/pull/634)).
- Initialisation / configuration changes:
  - **Backend MSP upload config** (see `configs/backend_config.toml`):
    - `msp.trusted_file_transfer_server_url` (e.g. `http://localhost:7070`)
    - `msp.use_legacy_upload_method` (set to `true` to keep the legacy RPC upload path)

## SDK

- Behaviour changes:
  - **SIWX (CAIP‑122) support**: authentication can use SIWX; domain extraction is automatic from URIs, and the `sessionProvider` argument is now optional when creating `MspClient` instances ([PR #592](https://github.com/Moonsong-Labs/storage-hub/pull/592)).
  - **New file statuses**: SDK surfaces `revoked` and `rejected` in file info/file list APIs (propagated from backend) ([PR #596](https://github.com/Moonsong-Labs/storage-hub/pull/596)).
  - **E2E tooling compatibility**: dappwright / MetaMask version adjustments for test stability ([PR #612](https://github.com/Moonsong-Labs/storage-hub/pull/612), [PR #609](https://github.com/Moonsong-Labs/storage-hub/pull/609)).
- Initialisation changes:
  - Upgrade to `@storagehub-sdk/core` and `@storagehub-sdk/msp-client` **v0.4.2**.

## Versions

- Polkadot SDK: polkadot-stable2412-6
- Rust: 1.90 (from rust-toolchain.toml)

## Compatibility

- SH Backend v0.3.0 → compatible with pallets/runtime v0.3.0 and client v0.3.0 (all built from this release).
- SDK v0.4.2 → compatible with backend v0.3.0, client v0.3.0, and pallets/runtime v0.3.0.
- types-bundle v0.3.0 + api-augment v0.3.1 → compatible with this runtime release’s metadata; regenerate if you run custom runtimes.

## Upgrade Guide

### General upgrade notes

- Apply Indexer DB migrations as part of the upgrade (they will run on startup, or you can run `diesel migration run`).
- IMPORTANT❗️ Upgrading to this version requires a runtime upgrade, that will include event layout changes (notably [PR #599](https://github.com/Moonsong-Labs/storage-hub/pull/599)), aim for `runtime upgrade -> client/indexer upgrade` as close to simultaneous as your deployment tooling allows. It is expected that while the client and the runtime do not match the version, the client might fail to process some blocks due to decoding issues.

### Breaking PRs

- [PR #571](https://github.com/Moonsong-Labs/storage-hub/pull/571) – Storage Enable trait changes

  - **Short description**:
    - Added a new `RuntimeError` associated type to the `StorageEnableRuntime` trait in `client/common/src/traits.rs`.
    - Changed `StorageEnableErrors::Other` variant from `Other(sp_runtime::ModuleError)` to `Other(String)`.
  - **Who is affected**:
    - Any downstream runtime that implements the `StorageEnableRuntime` trait (e.g., custom runtimes using the StorageHub client).
  - **Suggested code changes**:
    - Add `type RuntimeError = crate::RuntimeError;` to your `StorageEnableRuntime` implementation and implement `Into<StorageEnableErrors>` for your runtime error as shown in the PR. See [PR #571](https://github.com/Moonsong-Labs/storage-hub/pull/571) for the full example implementation and rationale.

- [PR #581](https://github.com/Moonsong-Labs/storage-hub/pull/581) – Feature gate parachain dependencies / rename client types

  - **Short description**:
    - `ParachainClient` has been replaced by `StorageHubClient`.
    - Cumulus host functions are now behind an explicit `parachain` Cargo feature in `shc-common`.
  - **Who is affected**:
    - Substrate projects using StorageHub client crates that refer to `ParachainClient` or relied on Cumulus crates being pulled in transitively.
  - **Suggested code changes**:
    - Rename `ParachainClient` → `StorageHubClient`.
    - If targeting a parachain, enable `shc-common` with `features = ["std", "parachain"]`. If targeting a solochain, keep `features = ["std"]`. See [PR #581](https://github.com/Moonsong-Labs/storage-hub/pull/581) for the exact snippets.

- [PR #590](https://github.com/Moonsong-Labs/storage-hub/pull/590) – Breaking changes doc structure enforcement (contributors)

  - **Short description**:
    - CI enforces a breaking changes documentation structure for PRs.
  - **Who is affected**:
    - Contributors opening PRs to this repository.
  - **Suggested code changes**:
    - No runtime/operator action required. Follow the PR template/CI guidance when authoring PR descriptions.

- [PR #592](https://github.com/Moonsong-Labs/storage-hub/pull/592) – SIWX

  - **Short description**:
    - Domain extraction is now automatic from URIs, and session providers are optional when creating `MspClient` instances.
  - **Who is affected**:
    - Backend API consumers calling `/auth/nonce` or `/auth/message` that previously provided a `domain` field.
    - SDK users calling `MspClient.auth.SIWE()` or `MspClient.auth.SIWX()` with a `domain` parameter.
    - SDK users creating `MspClient` instances with a required `sessionProvider` parameter.
  - **Suggested code changes**:
    - Backend: remove `domain` from `NonceRequest`; backend extracts domain from `uri`.
    - SDK: remove the `domain` parameter from `SIWE`/`SIWX` calls, and treat `sessionProvider` as optional at `MspClient.connect(...)`. See [PR #592](https://github.com/Moonsong-Labs/storage-hub/pull/592) for examples.

- [PR #595](https://github.com/Moonsong-Labs/storage-hub/pull/595) – Trusted file transfer for backend uploads

  - **Short description**:
    - Added `msp_trusted_file_transfer_server_url` and `msp_use_legacy_upload_method` to backend config.
    - Added node options: `trusted_file_transfer_server`, `trusted_file_transfer_server_host`, `trusted_file_transfer_server_port`.
  - **Who is affected**:
    - MSP providers who expose a backend endpoint.
  - **Suggested code changes**:
    - Backend: set `msp.trusted_file_transfer_server_url` and `msp.use_legacy_upload_method = false` (or set to `true` to keep the legacy path).
    - Node: enable and configure the trusted server and ensure firewall rules only allow backend servers to reach it. See [PR #595](https://github.com/Moonsong-Labs/storage-hub/pull/595) for config snippets and the detailed warnings.

- [PR #596](https://github.com/Moonsong-Labs/storage-hub/pull/596) – New file statuses and deletion safeguards

  - **Short description**:
    - Adds two new file `Step`s in the indexer DB: `revoked` and `rejected`, surfaced as backend `FileStatus` and propagated through the SDK.
  - **Who is affected**:
    - Backend API consumers calling `/buckets/{bucket_id}/info/{file_key}`.
    - SDK users calling `MspClient.files.getFileInfo(...)` or `MspClient.buckets.getFiles(...)`.
  - **Suggested code changes**:
    - Handle `revoked` and `rejected` in your status logic. See [PR #596](https://github.com/Moonsong-Labs/storage-hub/pull/596) for concrete before/after examples.

- [PR #598](https://github.com/Moonsong-Labs/storage-hub/pull/598) – Indexer DB migration: normalise `is_in_bucket`

  - **Short description**:
    - There’s a new indexer DB migration that must be executed for all existing DBs.
  - **Who is affected**:
    - Indexer node runners (you must run migrations).
  - **Suggested code changes**:
    - No code changes required; apply DB migrations for the release.

- [PR #599](https://github.com/Moonsong-Labs/storage-hub/pull/599) – File System pallet event order change

  - **Short description**:
    - File System pallet event order changed; if clients/indexers are decoding events, mismatched runtime/client versions can lead to decoding failures or incorrect DB writes.
  - **Who is affected**:
    - Infra maintainers upgrading runtimes and clients/indexers.
  - **Suggested code changes**:
    - Prefer `runtime upgrade -> client upgrade` order (or near-simultaneous upgrades). If you upgrade the client first, it may get stuck processing old event layouts; if you upgrade runtime first, old clients can misinterpret events. Plan an upgrade window accordingly.

- [PR #600](https://github.com/Moonsong-Labs/storage-hub/pull/600) – `msp_status` and new events for storage request cleanup

  - **Short description**:
    - `StorageRequestMetadata` has a new `msp_status` field of type `MspStorageRequestStatus` and new events (including `IncompleteStorageRequestCleanedUp`), requiring runtime upgrade + metadata update.
  - **Who is affected**:
    - Runtime managers and node runners upgrading StorageHub runtimes and associated clients (metadata/event decoding must match).
  - **Suggested code changes**:
    - Upgrade runtime and client/indexer together; ensure no storage requests are open if you are not including the migration from [PR #628](https://github.com/Moonsong-Labs/storage-hub/pull/628). With v0.3.0, prefer including and executing the runtime migration described in #628.

- [PR #617](https://github.com/Moonsong-Labs/storage-hub/pull/617) – Sync handlers, config removals, new runtime API

  - **Short description**:
    - Removed client config parameters `max_blocks_behind_to_catch_up_root_changes` and `sync_mode_min_blocks_behind`.
    - Added Storage Providers runtime API `query_bucket_root`.
    - Blockchain Service persists last processed block number + hash (`LastProcessedBlock`), with a lazy migration from the older `LastProcessedBlockNumber` column.
  - **Who is affected**:
    - Client/node runners that need to update their config files.
    - Runtimes that integrate StorageHub (must expose the new runtime API).
  - **Suggested code changes**:
    - Remove the two config keys from config files.
    - Add the `query_bucket_root` runtime API to your runtime as described in [PR #617](https://github.com/Moonsong-Labs/storage-hub/pull/617) (including the `QueryBucketRootError` import and the `H256` return type wiring).

- [PR #619](https://github.com/Moonsong-Labs/storage-hub/pull/619) – Indexer DB migration: fix bucket stats

  - **Short description**:
    - Adds an indexer-db migration to recalculate `file_count` and `total_size` bucket fields based on linked file records.
  - **Who is affected**:
    - Infra maintainers running indexers (must run migrations).
  - **Suggested code changes**:
    - No code changes required; apply DB migrations for the release.

- [PR #628](https://github.com/Moonsong-Labs/storage-hub/pull/628) – Runtime storage migration for `msp_status`

  - **Short description**:
    - `StorageRequestMetadata` has a new `msp_status` field requiring a runtime migration of all open storage requests.
  - **Who is affected**:
    - Runtime managers upgrading runtimes that use StorageHub.
  - **Suggested code changes**:
    - Wire migrations into the runtime Executive, e.g. `pub type Migrations = (pallet_file_system::migrations::v1::MigrateV0ToV1<Runtime>,);` and include it in `frame_executive::Executive<..., Migrations>`. See [PR #628](https://github.com/Moonsong-Labs/storage-hub/pull/628) for the full snippet.

- [PR #632](https://github.com/Moonsong-Labs/storage-hub/pull/632) – Pausable user operations
  - **Short description**:
    - Introduces storage `UserOperationPauseFlagsStorage`, Root-only extrinsic `set_user_operation_pause_flags`, and a new `UserOperationPaused` error for paused operations.
  - **Who is affected**:
    - Runtime managers upgrading runtimes that use StorageHub.
    - Client/applications that decode runtime errors/events (enums changed).
  - **Suggested code changes**:
    - No code changes beyond runtime/client upgrade, but regenerate/upgrade any bindings or decoders that depend on runtime metadata.
