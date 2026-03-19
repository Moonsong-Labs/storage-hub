# StorageHub v0.5.0

## Summary

StorageHub v0.5.0 focuses on **runtime and operator upgrade readiness**, **BSP/MSP operational automation**, **upload-path performance**, and **SDK ergonomics/security**. Highlights include the move to **polkadot-sdk/frontier `stable2503`** (with the corresponding Rust 1.90 toolchain requirement), new runtime and client support for **BSP stop-storing automation**, **replication tracking** exposed through the backend and SDK, a major speed-up for both the **backend file-trie build path** and the **Backend -> MSP trusted upload path**, and a richer SDK with **streaming encryption**, **public download links**, **sticky sessions**, and updated upload/pagination contracts.

## Components

- Client code: v0.5.0
- Pallets code: v0.5.0
- Runtime code: v0.5.0 (spec_name/spec_version: parachain `sh-parachain-runtime`/1, solochain-evm `sh-solochain-evm`/1)
- SH Backend Docker image: v0.5.0 (image: `moonsonglabs/storage-hub-msp-backend:v0.5.0`)
- SH SDK (npm): v0.7.3 (`@storagehub-sdk/core`, `@storagehub-sdk/msp-client`)
- types-bundle/api-augment (npm): `@storagehub/types-bundle` v0.5.0, `@storagehub/api-augment` v0.5.0

## Changes since last tag

Base: `ea2611cb3b47e448fa2812082e130c697b66277a`

- Highlights:
  - **Polkadot SDK / Frontier upgrade groundwork**: StorageHub now targets `polkadot-sdk` and Frontier `stable2503`, wraps the transaction extension pipeline with `StorageWeightReclaim`, adds the required EVM proof-recording changes for `solochain-evm`, and standardises on Rust 1.90 for compatibility ([PR #671](https://github.com/Moonsong-Labs/storage-hub/pull/671), [PR #695](https://github.com/Moonsong-Labs/storage-hub/pull/695)).
  - **BSP operational automation**: a new `bspStopStoringFile` RPC, on-chain/runtime support for pending stop-storing request synchronisation, and a BSP confirm queue that survives restarts by re-syncing from chain state make the stop-storing flow much easier to operate safely ([PR #663](https://github.com/Moonsong-Labs/storage-hub/pull/663)).
  - **MSP recovery and replication visibility**: MSPs can now recover missing files from BSPs after forest validation, and the stack tracks per-file desired/current replication via runtime events, indexer DB migration, backend file-info responses, and SDK types ([PR #651](https://github.com/Moonsong-Labs/storage-hub/pull/651), [PR #699](https://github.com/Moonsong-Labs/storage-hub/pull/699)).
  - **File-system/runtime scaling improvements**: `StorageRequestBsps` moves from a `StorageDoubleMap` to a bounded `StorageMap`, reducing PoV/weight pressure and introducing the new `MaxBspVolunteers` and `MaxMspRespondFileKeys` runtime constants ([PR #689](https://github.com/Moonsong-Labs/storage-hub/pull/689)).
  - **Much faster upload paths**: the backend now batches trie insertion while reading client uploads, and the trusted Backend -> MSP upload path now batches RocksDB/trie writes with an operator-tunable target size, significantly improving throughput ([PR #683](https://github.com/Moonsong-Labs/storage-hub/pull/683), [PR #690](https://github.com/Moonsong-Labs/storage-hub/pull/690), [PR #700](https://github.com/Moonsong-Labs/storage-hub/pull/700)).
  - **SDK usability and security improvements**: v0.7.3 adds streaming encryption/decryption, public-file download-link generation, sticky-session support for Node.js, bucket pagination metadata, and a stricter MSP upload API that now requires a precomputed fingerprint and `0x`-normalised identifiers ([PR #673](https://github.com/Moonsong-Labs/storage-hub/pull/673), [PR #685](https://github.com/Moonsong-Labs/storage-hub/pull/685), [PR #678](https://github.com/Moonsong-Labs/storage-hub/pull/678), [PR #698](https://github.com/Moonsong-Labs/storage-hub/pull/698), [PR #703](https://github.com/Moonsong-Labs/storage-hub/pull/703)).
  - **Backend operability improvements**: the MSP backend now exposes a dedicated `GET /node-health` endpoint for node-level health signals, triggers lazy bucket-healing on active access, and surfaces replication metadata in file-info responses ([PR #686](https://github.com/Moonsong-Labs/storage-hub/pull/686), [PR #699](https://github.com/Moonsong-Labs/storage-hub/pull/699), [PR #702](https://github.com/Moonsong-Labs/storage-hub/pull/702)).

- Full diff: https://github.com/Moonsong-Labs/storage-hub/compare/ea2611cb3b47e448fa2812082e130c697b66277a...e8903e8451fe755d5c442747ae61a6f4aca62070
- PRs included:
  - #683 feat: ⚡ improve backend upload
  - #682 chore: downgrade "Re-queuing file key" log from info to debug
  - #685 feat: ✨ SDK download link for public files
  - #680 fix: 🩹 suppress spurious provider ID warning for fisherman role
  - #684 feat: ✨ make extrinsic mortality configurable and fix tx manager issues
  - #688 feat: ✨ Add useful prompts for PR descriptions and releases
  - #691 docs: 📝 Update release docs process
  - #692 feat: ✨ Fisherman logging improvements
  - #651 feat: ✨ Add task to get missing files from buckets
  - #693 feat: ✨ Add incremental tip to fisherman deletion extrinsics
  - #694 fix: 🐛 Open db instead of creating when opening forest from disk
  - #695 build: ⬇️ Downgrade Rust toolchain version from `1.91` to `1.90`
  - #690 feat: ⚡ improve Backend --> MSP upload
  - #698 feat: ✨ SDK upload file improvements
  - #663 feat: ✨ add a file deletion RPC for BSPs to stop storing a file
  - #700 fix: 🐛 Repeated fingerprint upload bug
  - #686 feat: add GET /node-health endpoint for MSP node operational health
  - #678 feat: ✨ SDK - Bucket list pagination
  - #681 feat: ✨ Delete forest from disk on bucket removal
  - #705 style: 🎨 Add `taplo` formatting for `.toml` files
  - #611 feat: 🧪 Polenta dynamic integration test network topology
  - #707 refactor: ♻️ Migrate from a `pnpm` workspace to a `bun` project
  - #708 ci: 💚 Add `NPM_CONFIG_TOKEN` which is expected by `bun publish`
  - #671 build: ⬆️ Upgrade `polkadot-sdk` deps to `stable2503`
  - #699 feat: ✨ Add replication tracking for BSP churn detection
  - #673 feat: 🔐 SDK encryption
  - #702 fix: ⚡ lazy file storage healing
  - #703 feat: ✨ SDK sticky sessions
  - #706 feat: ✨ Make delete intention message human readable
  - #689 perf: refactor `StorageRequestBsps` storage layout to reduce weight consumption
  - #711 fix: 🐛 CI missing node binary
  - #710 refactor: ♻️ Move solochain-evm runtime APIs implementation into their own file
  - #712 build: 🔖 Upgrade versions for minor release

## Migrations

### RocksDB (File Storage)

- Changes:
  - No new on-disk schema migration in this release.
  - File completeness checks now treat the fingerprint as the source of truth, while `chunks_count` remains progress-tracking metadata; trusted-upload ingestion also writes chunks in batches instead of one-by-one ([PR #690](https://github.com/Moonsong-Labs/storage-hub/pull/690), [PR #700](https://github.com/Moonsong-Labs/storage-hub/pull/700)).
- Action required:
  - None.

### RocksDB (Forest Storage)

- Changes:
  - No new schema migration in this release.
  - Forests opened from disk no longer implicitly create a RocksDB database, and bucket removal now deletes the corresponding forest from disk when applicable ([PR #694](https://github.com/Moonsong-Labs/storage-hub/pull/694), [PR #681](https://github.com/Moonsong-Labs/storage-hub/pull/681)).
- Action required:
  - None.

### RocksDB (State store)

- Changes:
  - No new schema migration in this release.
  - The client adds new queue-management and cleanup behaviour for pending stop-storing requests and stale pending transactions, but these do not require manual database intervention ([PR #663](https://github.com/Moonsong-Labs/storage-hub/pull/663), [PR #684](https://github.com/Moonsong-Labs/storage-hub/pull/684)).
- Action required:
  - None.

### Indexer DB (Postgres)

- Migrations:
  - New migration `2026-02-24-000001_add_replication_tracking` adds `bsps_required` and `desired_replicas` to the `file` table, backfills `desired_replicas` from existing BSP associations, and changes the `bsp_file.bsp_id` foreign key to `ON DELETE CASCADE` so BSP deletions clean up associations automatically ([PR #699](https://github.com/Moonsong-Labs/storage-hub/pull/699)).
- How to apply:
  - The indexer service runs migrations automatically on start-up. Alternatively: `diesel migration run`.

## ⚠️ Breaking Changes ⚠️

- [PR #684](https://github.com/Moonsong-Labs/storage-hub/pull/684): adds configurable extrinsic mortality to provider and fisherman blockchain-service configuration (`extrinsic_mortality`, `--extrinsic-mortality`, `--fisherman-extrinsic-mortality`). Provider operators, fisherman operators, and custom node/client integrations should review the new CLI/TOML surface and wire it through if they mirror StorageHub node code.
- [PR #651](https://github.com/Moonsong-Labs/storage-hub/pull/651): adds BSP-side trusted MSP allow-listing for MSP-initiated recovery downloads via `--trusted-msps`. BSP operators enabling recovery/download flows and downstream node integrations must configure the new allow-list.
- [PR #695](https://github.com/Moonsong-Labs/storage-hub/pull/695): Rust toolchain support moves from `1.91` to `1.90`. Downstream runtimes and node/client integrations should align their toolchain before rebuilding, formatting, or running clippy.
- [PR #690](https://github.com/Moonsong-Labs/storage-hub/pull/690): trusted file transfer server config now requires a `batch_target_bytes` field, and the node/provider config path gains `--trusted-file-transfer-batch-size-bytes` / `trusted_file_transfer_batch_size_bytes`. Custom node integrations and any code constructing the trusted file-transfer config/context must update.
- [PR #698](https://github.com/Moonsong-Labs/storage-hub/pull/698): SDK `files.uploadFile(...)` now requires a precomputed fingerprint and strict `0x` string values for `bucketId`, `fileKey`, and `owner`. SDK consumers and wrappers around MSP upload helpers must update call sites.
- [PR #663](https://github.com/Moonsong-Labs/storage-hub/pull/663): adds three new File System runtime APIs for BSP stop-storing, changes `FileSystemApi` generics, updates builder/RPC wiring, and introduces `check_stop_storing_requests_period`. Custom runtimes and nodes mirroring StorageHub service/RPC/CLI code must update accordingly.
- [PR #678](https://github.com/Moonsong-Labs/storage-hub/pull/678): bucket pagination contracts now include `totalBuckets` in backend and SDK responses, and backend internals move from plain `Vec<Bucket>` contracts to `BucketsPage<T>`. Direct backend consumers and SDK pagination consumers must adjust response handling.
- [PR #671](https://github.com/Moonsong-Labs/storage-hub/pull/671): StorageHub now targets `polkadot-sdk` / Frontier `stable2503`, which requires downstream runtimes to adopt `StorageWeightReclaim`, EVM nodes to enable proof recording consistently, and custom code to align with upstream SDK changes.
- [PR #699](https://github.com/Moonsong-Labs/storage-hub/pull/699): the `NewStorageRequest` event gains trailing fields `bsps_required` and `msp_id`, changing its SCALE encoding, while backend/SDK file-info responses gain replication metadata (`desiredReplicas`, `currentReplication`). Operators should upgrade runtime and client/indexer together, and SDK consumers should handle the new fields.
- [PR #689](https://github.com/Moonsong-Labs/storage-hub/pull/689): `StorageRequestBsps` moves from `StorageDoubleMap` to `StorageMap<..., BoundedBTreeMap<...>>`, and `pallet_file_system::Config` now requires `MaxBspVolunteers` and `MaxMspRespondFileKeys`. Custom runtime implementations and mocks must add the new constants and account for the storage-layout change.

## Runtime

- Upgrades (spec_version): parachain and solochain-evm remain at spec_version 1.
- Migrations: none in this release.
- Behaviour changes:
  - **Polkadot SDK / Frontier alignment**: StorageHub now targets `stable2503`, replaces deprecated `sp_std` usage with `alloc` / `core`, updates genesis config presets to `build_struct_json_patch!`, and adopts related upstream API changes ([PR #671](https://github.com/Moonsong-Labs/storage-hub/pull/671)).
  - **Transaction extension pipeline**: runtime transaction extensions are now wrapped in `cumulus_pallet_weight_reclaim::StorageWeightReclaim`, improving PoV accounting and reclaiming unused storage weight ([PR #671](https://github.com/Moonsong-Labs/storage-hub/pull/671)).
  - **BSP stop-storing support**: the File System runtime API now exposes `query_min_wait_for_stop_storing`, `has_pending_stop_storing_request`, and `pending_stop_storing_requests_by_bsp` to support automated BSP stop-storing flows ([PR #663](https://github.com/Moonsong-Labs/storage-hub/pull/663)).
  - **Replication metadata in events**: `NewStorageRequest` now emits `bsps_required` and `msp_id`, enabling downstream replication tracking across the indexer, backend, and SDK ([PR #699](https://github.com/Moonsong-Labs/storage-hub/pull/699)).
  - **Storage-request volunteer layout refactor**: `StorageRequestBsps` is now a bounded `StorageMap` rather than a `StorageDoubleMap`, reducing write-heavy PoV costs and bounding volunteer state explicitly ([PR #689](https://github.com/Moonsong-Labs/storage-hub/pull/689)).
  - **Delete-intention signing UX**: the message signed for delete intention flows is now human-readable rather than opaque ASCII bytes, improving wallet-signing clarity ([PR #706](https://github.com/Moonsong-Labs/storage-hub/pull/706)).
- Constants changed:
  - `pallet_file_system::Config` now requires `MaxBspVolunteers` and `MaxMspRespondFileKeys`; recommended defaults from the PR are `ConstU32<1000>` and `ConstU32<10>` respectively, with `MaxBspVolunteers >= MaxReplicationTarget` ([PR #689](https://github.com/Moonsong-Labs/storage-hub/pull/689)).
- Scripts to run:
  - None.

## Client

- Behaviour changes:
  - **Extrinsic lifecycle robustness**: provider/fisherman extrinsics can now use configurable mortality, transaction subscriptions handle non-terminal closure as dropped transactions, and stale expired pending transactions are cleaned up automatically to prevent nonce deadlocks ([PR #684](https://github.com/Moonsong-Labs/storage-hub/pull/684)).
  - **BSP stop-storing automation**: adds `bspStopStoringFile`, queue-based request/confirm handling, periodic/on-start-up sync against on-chain pending requests, and a helper RPC `getAllStoredFileKeys` for BSP-operated sign-off flows ([PR #663](https://github.com/Moonsong-Labs/storage-hub/pull/663)).
  - **MSP missing-file recovery**: after bucket-root verification, MSPs can detect missing or incomplete file storage and recover files from BSPs using existing download machinery ([PR #651](https://github.com/Moonsong-Labs/storage-hub/pull/651)).
  - **Lazy file-storage healing**: MSP file-storage healing is now deferred until a bucket becomes active (new storage request, mutation, or backend-triggered download path), instead of scanning every bucket on start-up ([PR #702](https://github.com/Moonsong-Labs/storage-hub/pull/702)).
  - **Upload and file-transfer resilience**: fisherman deletion extrinsics now escalate tips on retries for conflicting roots, and the chunk uploader has follow-on compatibility fixes for `litep2p` error behaviour introduced by the SDK upgrade ([PR #693](https://github.com/Moonsong-Labs/storage-hub/pull/693), [PR #671](https://github.com/Moonsong-Labs/storage-hub/pull/671)).
  - **Forest/file-store correctness**:
    - Opening an on-disk forest no longer accidentally creates a new RocksDB instance ([PR #694](https://github.com/Moonsong-Labs/storage-hub/pull/694)).
    - Bucket removal now deletes the associated forest from disk where applicable ([PR #681](https://github.com/Moonsong-Labs/storage-hub/pull/681)).
    - Re-uploading a file with the same fingerprint but a different file key no longer fails completeness checks ([PR #700](https://github.com/Moonsong-Labs/storage-hub/pull/700)).
  - **Role-aware logging**: fishermen no longer emit the misleading “no Provider ID linked” warning on every block import ([PR #680](https://github.com/Moonsong-Labs/storage-hub/pull/680)).
  - **Networking/runtime-service fixes for `stable2503`**: dev-mode GRANDPA notification keepalive and increased retry tolerance for request/refused errors accommodate `litep2p` behavioural differences introduced by the SDK upgrade ([PR #671](https://github.com/Moonsong-Labs/storage-hub/pull/671)).
- Initialisation / configuration changes:
  - **Provider blockchain-service config**:
    - New TOML field `[provider.blockchain_service].extrinsic_mortality`
    - New CLI flag `--extrinsic-mortality`
  - **Fisherman blockchain-service config**:
    - New TOML section `[fisherman.blockchain_service]`
    - New CLI flag `--fisherman-extrinsic-mortality`
  - **BSP recovery/download authorisation**:
    - New CLI/config option `--trusted-msps` to allow MSP-initiated recovery/downloads from selected MSP on-chain IDs ([PR #651](https://github.com/Moonsong-Labs/storage-hub/pull/651)).
  - **BSP stop-storing queue sync**:
    - New blockchain-service option `check_stop_storing_requests_period`
    - New CLI flag `--check-stop-storing-requests-period` (default: 600 blocks) ([PR #663](https://github.com/Moonsong-Labs/storage-hub/pull/663)).

## Backend

- Behaviour changes:
  - **Faster client -> backend ingestion**: the backend now buffers chunks and inserts them into the file trie in batches while reading uploads, cutting the “read + build trie” phase dramatically for large files ([PR #683](https://github.com/Moonsong-Labs/storage-hub/pull/683)).
  - **Faster Backend -> MSP trusted uploads**: trusted uploads now use batched RocksDB/trie writes and commit metadata in a single transaction; benchmark data in the PR showed roughly 5.7x-6.7x higher throughput for the batched path ([PR #690](https://github.com/Moonsong-Labs/storage-hub/pull/690)).
  - **Repeated-fingerprint uploads**: MSP upload validation now treats fingerprint equality as the completeness source of truth, allowing repeated uploads of the same content under different file keys ([PR #700](https://github.com/Moonsong-Labs/storage-hub/pull/700)).
  - **Bucket pagination API**: `GET /buckets` now supports `page` / `limit` pagination and returns a `totalBuckets` count alongside the bucket list ([PR #678](https://github.com/Moonsong-Labs/storage-hub/pull/678)).
  - **File info now exposes replication state**: `GET /file/{file_key}` now includes `desiredReplicas` and `currentReplication`, sourced from the indexer DB replication-tracking changes ([PR #699](https://github.com/Moonsong-Labs/storage-hub/pull/699)).
  - **MSP node operational health**: new `GET /node-health` endpoint reports indexer freshness, storage-request acceptance, and transaction-nonce liveness as `healthy | degraded | unhealthy | unknown`, separate from the existing infrastructure-focused `/health` endpoint ([PR #686](https://github.com/Moonsong-Labs/storage-hub/pull/686)).
  - **Active-access healing hook**: the backend can trigger bucket file-storage healing when a file download indicates that a bucket is active ([PR #702](https://github.com/Moonsong-Labs/storage-hub/pull/702)).
- Initialisation / configuration changes:
  - **Trusted upload batch sizing**:
    - New provider CLI flag `--trusted-file-transfer-batch-size-bytes`
    - New config field `trusted_file_transfer_batch_size_bytes`
    - Default: `2097152` (2 MiB) ([PR #690](https://github.com/Moonsong-Labs/storage-hub/pull/690)).
  - **Node-health thresholds**:
    - New `NodeHealthConfig` thresholds configure stale-indexer detection, request-acceptance windows, and nonce-liveness thresholds for `GET /node-health` ([PR #686](https://github.com/Moonsong-Labs/storage-hub/pull/686)).

## SDK

- Behaviour changes:
  - **Streaming encryption / decryption**: `@storagehub-sdk/core` adds `encryptFile()` / `decryptFile()` using chunked `chacha20-poly1305`, a plaintext self-describing header, deterministic CBOR header encoding, random per-file salt, and password/signature-based key derivation flows ([PR #673](https://github.com/Moonsong-Labs/storage-hub/pull/673)).
  - **Public-file download links**: `@storagehub-sdk/msp-client` can now generate download links for public files after verifying that the target file exists and is in `Ready` state ([PR #685](https://github.com/Moonsong-Labs/storage-hub/pull/685)).
  - **Upload API tightening**: `files.uploadFile(...)` now requires a precomputed fingerprint and strict `0x`-prefixed values for `bucketId`, `fileKey`, and `owner` ([PR #698](https://github.com/Moonsong-Labs/storage-hub/pull/698)).
  - **Bucket pagination metadata**: `listBucketsByPage()` now returns `buckets`, `page`, `limit`, and `totalBuckets`; `listBuckets()` remains the convenience first-page wrapper ([PR #678](https://github.com/Moonsong-Labs/storage-hub/pull/678)).
  - **Sticky sessions in Node.js**: `MspClient.connect()` can now opt into cookie persistence (`enableCookies=true`) so SIWE/authenticated flows behave more like browser sessions in Node.js scripts ([PR #703](https://github.com/Moonsong-Labs/storage-hub/pull/703)).
  - **Replication metadata in file info**: `getFileInfo()` now surfaces `desiredReplicas` and `currentReplication` when available ([PR #699](https://github.com/Moonsong-Labs/storage-hub/pull/699)).
  - **Delete-intention signing UX**: the message presented to wallets for delete-intention signing is now human-readable ([PR #706](https://github.com/Moonsong-Labs/storage-hub/pull/706)).
- Initialisation changes:
  - Upgrade to `@storagehub-sdk/core` and `@storagehub-sdk/msp-client` **v0.7.3**.
  - For sticky sessions in Node.js, enable cookie persistence when connecting the MSP client (`enableCookies=true`) ([PR #703](https://github.com/Moonsong-Labs/storage-hub/pull/703)).

## Versions

- Polkadot SDK: `stable2503`
- Rust: `1.90` (from `rust-toolchain.toml`)

## Compatibility

- SH Backend v0.5.0 -> compatible with pallets/runtime v0.5.0 and client v0.5.0 (all built from this release).
- SDK v0.7.3 -> compatible with backend v0.5.0, client v0.5.0, and pallets/runtime v0.5.0.
- types-bundle v0.5.0 + api-augment v0.5.0 -> compatible with this runtime release's metadata; regenerate if you run custom runtimes.
- Downstream custom runtimes / EVM-enabled nodes should align to `polkadot-sdk` / Frontier `stable2503` and Rust 1.90 before upgrading StorageHub crates.

## Upgrade Guide

### General upgrade notes

- Apply database migrations as part of the upgrade. This release introduces a new indexer DB migration for replication tracking; the indexer runs it automatically on start-up, or you can run `diesel migration run` manually.
- If you operate **custom runtimes** or **custom nodes**, align your `polkadot-sdk` / Frontier dependencies to `stable2503` and switch your Rust toolchain to `1.90` before building.
- If you operate **MSP/BSP/indexer nodes**, coordinate the runtime/client upgrade around [PR #699](https://github.com/Moonsong-Labs/storage-hub/pull/699): once the runtime emits the expanded `NewStorageRequest` event, outdated clients/indexers will fail to decode it.
- Review the new CLI/TOML surfaces introduced in this release, especially `extrinsic_mortality`, `trusted-msps`, `check_stop_storing_requests_period`, and `trusted_file_transfer_batch_size_bytes`.
- If you consume backend or SDK pagination/file-info responses, update consumers for `totalBuckets`, `desiredReplicas`, and `currentReplication`.

### Breaking PRs

- [PR #684](https://github.com/Moonsong-Labs/storage-hub/pull/684) - Configurable extrinsic mortality and fisherman blockchain-service config
  - **Short description**:
    - There's a new config parameter `extrinsic_mortality` that has a sane default but can be configured when spinning up a new client. Fisherman runners can now also configure their Blockchain Service under the `[fisherman.blockchain_service]` TOML section, with the corresponding `--fisherman-extrinsic-mortality` CLI flag.
  - **Who is affected**:
    - `🟢 [MSP operators]` Provider client runners should review the new `extrinsic_mortality` setting for their node configuration.
    - `🟠 [BSP operators]` Provider client runners should review the new `extrinsic_mortality` setting for their node configuration.
    - `🟡 [Fisherman operators]` Fisherman runners can now configure their Blockchain Service separately via `[fisherman.blockchain_service]` / `--fisherman-extrinsic-mortality`.
    - `🔵 [Node/client integrators]` Projects integrating the StorageHub Client into their node must plumb the new provider/fisherman blockchain-service config fields through their CLI and TOML wiring.
  - **Suggested code changes**:
    - Add `extrinsic_mortality` under `[provider.blockchain_service]`, or set it via `--extrinsic-mortality <blocks>`.
    - Add `[fisherman.blockchain_service].extrinsic_mortality`, or set it via `--fisherman-extrinsic-mortality <blocks>`.
    - If you mirror StorageHub node code, forward the new fields into `BlockchainServiceOptions`. See the “Suggested code changes” section in [PR #684](https://github.com/Moonsong-Labs/storage-hub/pull/684) for the exact `cli.rs` / `service.rs` snippets.

- [PR #651](https://github.com/Moonsong-Labs/storage-hub/pull/651) - Trusted MSP allow-list for BSP recovery/download flows
  - **Short description**:
    - Added BSP CLI/config option `--trusted-msps` to allow downloads from trusted MSP on-chain IDs during MSP-initiated recovery/download flows.
  - **Who is affected**:
    - `🔵 [Node/client integrators]` Maintainers of chains/networks using the StorageHub Client must add the new BSP CLI/config surface if they vendor StorageHub node code.
    - `🟠 [BSP operators]` BSP node operators enabling MSP-initiated recovery/download flows must set `--trusted-msps=<comma-separated MSP IDs>` (or the equivalent config value).
  - **Suggested code changes**:
    - Add the `--trusted-msps` CLI/config option to BSP provider configuration and wire it into the file-transfer service.
    - Validate that the option is only usable when running as a BSP provider.
    - See the “Suggested code changes” section in [PR #651](https://github.com/Moonsong-Labs/storage-hub/pull/651) for the exact `cli.rs`, `command.rs`, and `service.rs` changes.

- [PR #695](https://github.com/Moonsong-Labs/storage-hub/pull/695) - Rust toolchain 1.90
  - **Short description**:
    - Downgrading Rust toolchain version from `1.91` to `1.90`.
  - **Who is affected**:
    - `🟣 [Runtime maintainers]` Downstream projects that integrate StorageHub pallets must build and test against Rust 1.90.
    - `🔵 [Node/client integrators]` Downstream projects that integrate StorageHub client/node code must build and test against Rust 1.90.
  - **Suggested code changes**:
    - Switch your Rust toolchain to `1.90` before building, formatting, or running clippy. This release expects the `stable2503`-compatible toolchain version.

- [PR #690](https://github.com/Moonsong-Labs/storage-hub/pull/690) - Trusted upload batch sizing
  - **Short description**:
    - This PR adds a new `batch_target_bytes` field to the trusted file transfer server `Config` and `Context` structs, and a corresponding MSP trusted upload ingestion batch-size parameter in the node configuration path:
      - CLI flag: `--trusted-file-transfer-batch-size-bytes`
      - Provider option/config field: `trusted_file_transfer_batch_size_bytes`
      - Default value: `2097152` (2 MiB)
  - **Who is affected**:
    - `🔵 [Node/client integrators]` Teams that mirror or vendor StorageHub `node/` code, or construct trusted file-transfer `Config` / `Context` structs directly, must update their config and service wiring.
    - `🟢 [MSP operators]` MSP operators using the trusted upload ingestion path may want to review and tune the new batch-size setting.
  - **Suggested code changes**:
    - Add `--trusted-file-transfer-batch-size-bytes` to your provider CLI/config surface and wire it into `ProviderOptions`.
    - Update trusted file-transfer server construction to set `batch_target_bytes`.
    - Add the TOML field `trusted_file_transfer_batch_size_bytes = 2097152` under `[provider]` if you maintain custom configuration files.
    - See [PR #690](https://github.com/Moonsong-Labs/storage-hub/pull/690) for the exact snippets.

- [PR #698](https://github.com/Moonsong-Labs/storage-hub/pull/698) - MSP SDK upload API changes
  - **Short description**:
    - `uploadFile(...)` now requires a precomputed fingerprint parameter, and it now enforces `0x` hex string types for `bucketId`, `fileKey`, and `owner`.
  - **Who is affected**:
    - `🔴 [SDK users]` Teams that call `@storagehub-sdk/msp-client` `files.uploadFile(...)` directly, or have wrappers that currently pass non-`0x` values, must update their upload flow.
  - **Suggested code changes**:
    - Compute the fingerprint before calling `uploadFile(...)` and pass it explicitly.
    - Normalise `bucketId`, `fileKey`, and `owner` to strict `0x`-prefixed strings at your boundaries.
    - See the updated signature and sample code in [PR #698](https://github.com/Moonsong-Labs/storage-hub/pull/698).

- [PR #663](https://github.com/Moonsong-Labs/storage-hub/pull/663) - BSP stop-storing runtime APIs and builder/RPC refactor
  - **Short description**:
    - There are three new runtime APIs under the File System pallet: `query_min_wait_for_stop_storing`, `has_pending_stop_storing_request`, and `pending_stop_storing_requests_by_bsp`. `FileSystemApi` now has an additional generic parameter (`PendingStopStoringRequest`), `BlockchainServiceOptions` has a new `check_stop_storing_requests_period` field, and `init_sh_builder` / `finish_sh_builder_and_run_tasks` changed to move blockchain-service initialisation and RPC-handler setup.
  - **Who is affected**:
    - `🟣 [Runtime maintainers]` Runtime managers of runtimes that use StorageHub must implement the new File System runtime APIs.
    - `🔵 [Node/client integrators]` Node managers that maintain their own copies of StorageHub `service.rs`, `rpc.rs`, or `cli.rs` must update builder, RPC, trait bounds, and CLI/config wiring.
  - **Suggested code changes**:
    - Implement the new File System runtime APIs in your runtime.
    - Update `init_sh_builder` and `finish_sh_builder_and_run_tasks` to match the new blockchain-service lifecycle and `set_blockchain_rpc_handlers` flow.
    - Add `Clone` bounds where required on RPC generics, and wire `check_stop_storing_requests_period` into `BlockchainServiceOptions`.
    - See the detailed migration snippets in [PR #663](https://github.com/Moonsong-Labs/storage-hub/pull/663) for the exact signatures and wiring.

- [PR #678](https://github.com/Moonsong-Labs/storage-hub/pull/678) - Bucket pagination response changes
  - **Short description**:
    - This PR introduces breaking API contract changes for bucket list pagination:
      - SDK paginated bucket lists return `totalBuckets: number`
      - Backend bucket-list responses now explicitly include `totalBuckets`
      - Internal backend data-access contracts change from plain `Vec<Bucket>` to `BucketsPage<T>`
  - **Who is affected**:
    - `🔴 [SDK users]` Teams consuming `@storagehub-sdk/msp-client` bucket-pagination APIs must handle `totalBuckets` in paginated responses.
    - `🔵 [Node/client integrators]` Teams consuming backend `/buckets` JSON directly, or mirroring backend internals, must account for `totalBuckets` and `BucketsPage<T>`.
  - **Suggested code changes**:
    - Use `totalBuckets` together with `page` and `limit` to derive pagination state.
    - If you consume backend JSON directly, parse `totalBuckets` from the response and update any response types accordingly.
    - See [PR #678](https://github.com/Moonsong-Labs/storage-hub/pull/678) for the exact response examples.

- [PR #671](https://github.com/Moonsong-Labs/storage-hub/pull/671) - `polkadot-sdk` / Frontier `stable2503`
  - **Short description**:
    - StorageHub now targets `polkadot-sdk` / Frontier `stable2503`. Downstream runtimes must wrap `SignedExtra` / `TxExtension` with `cumulus_pallet_weight_reclaim::StorageWeightReclaim` and implement `cumulus_pallet_weight_reclaim::Config`. EVM-enabled nodes must also ensure proof recording is enabled both during block building and block import to avoid digest mismatches.
  - **Who is affected**:
    - `🟣 [Runtime maintainers]` Downstream runtimes that integrate StorageHub pallets and define their own transaction-extension pipeline must adopt `StorageWeightReclaim` and other inherited SDK changes.
    - `🔵 [Node/client integrators]` Downstream node/client implementations, especially EVM-enabled ones, must align dependencies to `stable2503` and update proposer/client initialisation for proof recording.
  - **Suggested code changes**:
    - Wrap your transaction-extension tuple in `cumulus_pallet_weight_reclaim::StorageWeightReclaim` and implement `cumulus_pallet_weight_reclaim::Config`.
    - For Frontier EVM nodes, switch to `ProposerFactory::with_proof_recording` and `sc_service::new_full_parts_record_import(..., true)`.
    - Replace `sp_std::*` imports with `alloc::*` / `core::*`, migrate genesis presets to `build_struct_json_patch!`, and align your `polkadot-sdk` / Frontier versions to `stable2503`.
    - See [PR #671](https://github.com/Moonsong-Labs/storage-hub/pull/671) for the full inherited migration details.

- [PR #699](https://github.com/Moonsong-Labs/storage-hub/pull/699) - Replication tracking and `NewStorageRequest` event shape
  - **Short description**:
    - The `NewStorageRequest` event gains two new trailing fields (`bsps_required`, `msp_id`), changing its SCALE encoding. The indexer now tracks desired/current replication, and the backend / SDK expose that information as `desiredReplicas` and `currentReplication`.
  - **Who is affected**:
    - `🟢 [MSP operators]` Once the runtime upgrade is done, upgrade the client immediately to avoid failures when decoding `NewStorageRequest` events.
    - `🟠 [BSP operators]` Once the runtime upgrade is done, upgrade the client immediately to avoid failures when decoding `NewStorageRequest` events.
    - `🟤 [Indexer operators]` Once the runtime upgrade is done, upgrade the indexer/client immediately to avoid failures when decoding `NewStorageRequest` events.
    - `🔴 [SDK users]` `getFileInfo()` now additionally returns `desiredReplicas` and `currentReplication`.
  - **Suggested code changes**:
    - Update any consumers of `getFileInfo()` to handle `desiredReplicas` and `currentReplication`.
    - For operators, make sure runtime and client/indexer upgrades are coordinated so the expanded event encoding is decoded by matching client code.

- [PR #689](https://github.com/Moonsong-Labs/storage-hub/pull/689) - `StorageRequestBsps` layout and new runtime constants
  - **Short description**:
    - `StorageRequestBsps` changed from `StorageDoubleMap<FileKey, BspId, StorageRequestBspsMetadata>` to `StorageMap<FileKey, BoundedBTreeMap<BspId, bool, MaxBspVolunteers>>`. `pallet_file_system::Config` now also requires `MaxBspVolunteers` and `MaxMspRespondFileKeys`, which bound BSP volunteer maps and MSP multi-file responses respectively.
  - **Who is affected**:
    - `🟣 [Runtime maintainers]` All runtime implementations of `pallet_file_system::Config`, along with any downstream pallets/mocks that implement it, must add the new constants and account for the storage-layout change.
  - **Suggested code changes**:
    - Add both constants to your runtime config. Recommended defaults from the PR are:
      - `type MaxBspVolunteers = ConstU32<1000>;`
      - `type MaxMspRespondFileKeys = ConstU32<10>;`
    - Ensure `MaxBspVolunteers >= MaxReplicationTarget`.
