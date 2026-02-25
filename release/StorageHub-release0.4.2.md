# StorageHub v0.4.2

## Summary

StorageHub v0.4.2 focuses on **MSP file recovery from BSPs**, **further Backend→MSP upload performance gains**, **fisherman deletion robustness**, and **build toolchain compatibility**. Highlights include a new MSP task that detects missing or incomplete files after bucket root verification and recovers them from BSP peers via a new trusted-MSP authorisation flow, deeper batched-write optimisation for the trusted upload path with a configurable batch size, adaptive tip escalation for fisherman deletion extrinsics to recover from forest-root conflicts, a correctness fix preventing accidental RocksDB creation when opening forests from disk, and a Rust toolchain downgrade to 1.90 in preparation for the upcoming polkadot-sdk stable2503 upgrade.

## Components

- Client code: v0.4.2
- Pallets code: v0.4.2
- Runtime code: v0.4.2 (spec_name/spec_version: parachain `sh-parachain-runtime`/1, solochain-evm `sh-solochain-evm`/1)
- SH Backend Docker image: v0.4.2 (image: moonsonglabs/storage-hub-msp-backend:v0.4.2)
- SH SDK (npm): v0.4.6 (`@storagehub-sdk/core`, `@storagehub-sdk/msp-client`)
- types-bundle/api-augment (npm): `@storagehub/types-bundle` v0.4.2, `@storagehub/api-augment` v0.4.2

## Changes since last tag

Base: b5d6eb2ffa153d97e079d1fda382773b466f4702

- Highlights:
  - **MSP file recovery from BSPs**: MSPs can now detect missing or incomplete files after bucket root verification and recover them from BSP peers. A new `CheckBucketFileStorage` event triggers `MspCheckBucketFileStorageTask`, which enumerates bucket files via a new `get_all_files()` forest-storage method, queries the indexer for candidate BSP peers, and reuses the existing download manager to retrieve missing data. BSPs gain a new `--trusted-msps` CLI/config flag to allow-list MSP on-chain IDs authorised to request downloads, with runtime-based validation that the requesting MSP is the current bucket MSP ([PR #651](https://github.com/Moonsong-Labs/storage-hub/pull/651)).
  - **Backend→MSP upload performance**: introduces `write_chunks_batched` with a configurable batch target size (`--trusted-file-transfer-batch-size-bytes`, default 2 MB) and RocksDB batched write commits that flush trie changes, partial root, and chunk count in a single transaction. Benchmarks show approximately 5.7–6.7× higher throughput compared to per-chunk writes ([PR #690](https://github.com/Moonsong-Labs/storage-hub/pull/690)).
  - **Fisherman deletion tip escalation**: adds adaptive per-target tip escalation to fisherman deletion extrinsics to recover from forest-root conflicts when a deletion and a BSP confirm target the same root in the same block. Tips escalate geometrically on failure (0 → ~129 → ~293 → 500) using the same `RetryStrategy::compute_tip` progression as BSP confirms, and reset to 0 on success ([PR #693](https://github.com/Moonsong-Labs/storage-hub/pull/693)).
  - **Forest open correctness**: `open_forest_from_disk` no longer accidentally creates a new RocksDB database when the forest does not exist on disk; it now calls a dedicated `open_db` function instead of `create_db`, matching the assumed semantics at all call sites ([PR #694](https://github.com/Moonsong-Labs/storage-hub/pull/694)).
  - **Rust toolchain downgrade to 1.90**: downgraded from 1.91 to 1.90 for compatibility with the upcoming polkadot-sdk stable2503 upgrade. Rust 1.91 introduced a breaking change in target-spec parsing (`target-pointer-width` must be an integer, not a string) that `polkavm-linker` does not yet comply with, breaking `pallet-revive-fixtures` compilation ([PR #695](https://github.com/Moonsong-Labs/storage-hub/pull/695)).

- Full diff: https://github.com/Moonsong-Labs/storage-hub/compare/b5d6eb2ffa153d97e079d1fda382773b466f4702...dbc8d424b1c0d4eb47b1ac6e4ebb05601efdad12
- PRs included:
  - #695 build: ⬇️ Downgrade Rust toolchain version from `1.91` to `1.90`
  - #694 fix: 🐛 Open db instead of creating when opening forest from disk
  - #693 feat: ✨ Add incremental tip to fisherman deletion extrinsics
  - #692 feat: ✨ Fisherman logging improvements
  - #691 docs: 📝 Update release docs process
  - #690 feat: ⚡ improve Backend → MSP upload
  - #688 feat: ✨ Add useful prompts for PR descriptions and releases
  - #651 feat: ✨ Add task to get missing files from buckets

## Migrations

### RocksDB (File Storage)

- Changes:
  - No new schema changes in this release.
- Action required:
  - None.

### RocksDB (Forest Storage)

- Changes:
  - `open_forest_from_disk` now uses `open_db` instead of `create_db`, so attempting to open a non-existent forest will return an error rather than silently creating an empty database ([PR #694](https://github.com/Moonsong-Labs/storage-hub/pull/694)). This is a behaviour correction, not a schema change.
- Action required:
  - None. Existing forests on disk are unaffected.

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

- [PR #651](https://github.com/Moonsong-Labs/storage-hub/pull/651): adds BSP trusted-MSP authorisation for MSP-initiated file recovery downloads. BSP operators must set `--trusted-msps=<comma-separated MSP IDs>` to allow specific MSPs to request downloads. Node/client integrators that mirror the StorageHub `node/` code must wire the new `trusted_msps` CLI/config field and update `with_file_transfer` call sites to pass the additional `client` and `trusted_msps` parameters.
- [PR #690](https://github.com/Moonsong-Labs/storage-hub/pull/690): adds a new `batch_target_bytes` field to the trusted file transfer server `Config` and `Context` structs, and a new MSP provider CLI/config option `--trusted-file-transfer-batch-size-bytes` (default 2 MB). Node/client integrators must update provider option structs and trusted file transfer `Config` construction.
- [PR #695](https://github.com/Moonsong-Labs/storage-hub/pull/695): downgrades the Rust toolchain from 1.91 to 1.90 for polkadot-sdk stable2503 compatibility. Downstream projects integrating StorageHub pallets and/or client must switch to Rust 1.90 and apply any resulting `cargo fmt`/`cargo clippy` changes.

## Runtime

- Upgrades (spec_version): parachain and solochain-evm remain at spec_version 1.
- Migrations: none in this release.
- Behaviour changes:
  - No runtime logic/API changes were introduced between this base/head range.
- Constants changed: None requiring operator action.
- Scripts to run: None.

## Client

- Behaviour changes:
  - **MSP file recovery from BSPs**: after bucket root verification, MSPs now detect missing or incomplete files in local file storage and attempt to recover them from BSP peers. The new `MspCheckBucketFileStorageTask` enumerates bucket files via `get_all_files()`, queries the indexer for candidate BSPs holding the files, and reuses the existing download manager to retrieve missing data. Recovery outcomes are logged as `recovered`, `failed`, or `panicked` ([PR #651](https://github.com/Moonsong-Labs/storage-hub/pull/651)).
  - **BSP trusted-MSP download authorisation**: BSPs can now allow-list MSP on-chain IDs via `--trusted-msps` (comma-separated hex IDs). When an incoming download request originates from a trusted MSP that is the current bucket MSP (verified via runtime), the BSP permits the download. Non-trusted or non-bucket-MSP requests are rejected ([PR #651](https://github.com/Moonsong-Labs/storage-hub/pull/651)).
  - **Backend→MSP upload batched writes**: the trusted file transfer server now buffers incoming chunks and flushes them in configurable batches (`batch_target_bytes`, default 2 MB), committing trie changes, partial root, and chunk count in a single RocksDB transaction. This reduces per-chunk mutation and allocation overhead ([PR #690](https://github.com/Moonsong-Labs/storage-hub/pull/690)).
  - **Fisherman deletion tip escalation**: fisherman deletion extrinsics now use adaptive per-target tip escalation to recover from forest-root conflicts. Tips escalate geometrically on failure (0 → ~129 → ~293 → 500, capped at 3 retries) and reset to 0 on success ([PR #693](https://github.com/Moonsong-Labs/storage-hub/pull/693)).
  - **Forest open correctness**: `open_forest_from_disk` no longer calls `create_db`; it uses a dedicated `open_db` function, preventing accidental creation of empty RocksDB databases when a forest does not exist on disk ([PR #694](https://github.com/Moonsong-Labs/storage-hub/pull/694)).
- Initialisation / configuration changes:
  - **BSP trusted-MSP allow-list**:
    - New CLI option `--trusted-msps` (comma-separated hex MSP on-chain IDs).
    - Only valid when running as a BSP provider ([PR #651](https://github.com/Moonsong-Labs/storage-hub/pull/651)).
  - **Trusted upload batch sizing**:
    - New CLI option `--trusted-file-transfer-batch-size-bytes` (default: `2097152` / 2 MB).
    - New provider config field `trusted_file_transfer_batch_size_bytes: Option<u64>` ([PR #690](https://github.com/Moonsong-Labs/storage-hub/pull/690)).
  - **Rust toolchain**: downgraded from 1.91 to 1.90 ([PR #695](https://github.com/Moonsong-Labs/storage-hub/pull/695)).

## Backend

- Behaviour changes:
  - **Upload ingestion optimisation**: backend trusted upload processing now uses `write_chunks_batched` to buffer and flush chunk batches in a single RocksDB transaction, reducing per-chunk overhead and significantly accelerating trie build time for large uploads. The batch target size is configurable via `--trusted-file-transfer-batch-size-bytes` (default 2 MB) ([PR #690](https://github.com/Moonsong-Labs/storage-hub/pull/690)).
- Initialisation / configuration changes:
  - New provider config/CLI surface for `--trusted-file-transfer-batch-size-bytes` (see Client section) ([PR #690](https://github.com/Moonsong-Labs/storage-hub/pull/690)).

## SDK

- Behaviour changes:
  - No SDK changes in this release.
- Initialisation changes:
  - SDK npm packages remain at **v0.4.6** (`@storagehub-sdk/core`, `@storagehub-sdk/msp-client`).

## Versions

- Polkadot SDK: polkadot-stable2412-6
- Rust: 1.90 (from rust-toolchain.toml; downgraded from 1.91 in [PR #695](https://github.com/Moonsong-Labs/storage-hub/pull/695))

## Compatibility

- SH Backend v0.4.2 → compatible with pallets/runtime v0.4.2 and client v0.4.2 (all built from this release).
- SDK v0.4.6 → compatible with backend v0.4.2, client v0.4.2, and pallets/runtime v0.4.2.
- types-bundle v0.4.2 + api-augment v0.4.2 → compatible with this runtime release's metadata; regenerate if you run custom runtimes.

## Upgrade Guide

### General upgrade notes

- Apply standard service restarts and migration startup flows (no new DB schema migrations are introduced in this release).
- If you run **BSP nodes** and want to enable MSP-initiated file recovery, configure `--trusted-msps` with the on-chain IDs of MSPs you trust.
- If you run **MSP nodes** with the trusted file transfer server, the new `--trusted-file-transfer-batch-size-bytes` option defaults to 2 MB and should work well out of the box; tune if you have specific memory/throughput constraints.
- If you maintain **downstream projects** integrating StorageHub, switch to Rust 1.90 and re-run `cargo fmt`/`cargo clippy` to address any formatting or lint changes.

### Breaking PRs

- [PR #651](https://github.com/Moonsong-Labs/storage-hub/pull/651) – MSP file recovery + BSP trusted-MSP authorisation
  - **Short description**:
    - Added BSP CLI/config option `--trusted-msps` to allow downloads from trusted MSP on-chain IDs. The `with_file_transfer` builder method now requires additional `client` and `trusted_msps` parameters.
  - **Who is affected**:
    - `🟠 [BSP operators]` BSP node operators on chains enabling MSP-initiated recovery/download flows must set `--trusted-msps=<comma-separated MSP IDs>` (or equivalent config).
    - `🔵 [Node/client integrators]` Maintainers of chains/networks using the StorageHub Client must wire the new `trusted_msps` CLI/config field and update `with_file_transfer` call sites.
  - **Suggested code changes**:
    - Add `trusted_msps: Vec<H256>` to `ProviderConfigurations` (CLI) and `ProviderOptions` (config).
    - Validate that `--trusted-msps` is only used for BSP provider type.
    - Update `service.rs` to extract `trusted_msps` from `RoleOptions` and pass it (along with `client.clone()`) to `builder.with_file_transfer(...)`.
    - See the "Suggested code changes" section in [PR #651](https://github.com/Moonsong-Labs/storage-hub/pull/651) for the full migration snippets covering `cli.rs`, `command.rs`, and `service.rs`.

- [PR #690](https://github.com/Moonsong-Labs/storage-hub/pull/690) – Backend→MSP trusted upload batch sizing
  - **Short description**:
    - Adds a new `batch_target_bytes` field to the trusted file transfer server `Config` and `Context` structs, and a corresponding MSP provider CLI/config option `--trusted-file-transfer-batch-size-bytes` (default 2 MB).
  - **Who is affected**:
    - `🔵 [Node/client integrators]` Teams that mirror or vendor StorageHub `node/` code must wire the new provider CLI/config option and update trusted file transfer `Config` construction.
    - `🟢 [MSP operators]` MSP operators can optionally tune the batch size for trusted uploads (the default of 2 MB is appropriate for most workloads).
  - **Suggested code changes**:
    - Add `--trusted-file-transfer-batch-size-bytes` to your CLI struct and `trusted_file_transfer_batch_size_bytes: Option<u64>` to `ProviderOptions`.
    - Wire the field through CLI-to-options conversion and add it to `conflicts_with_all` for `provider_config_file`.
    - Update `service.rs` to compute `batch_target_bytes` from the provider option and pass it to the trusted file transfer `Config`.
    - See the "Suggested code changes" section in [PR #690](https://github.com/Moonsong-Labs/storage-hub/pull/690) for the full migration snippets.

- [PR #695](https://github.com/Moonsong-Labs/storage-hub/pull/695) – Rust toolchain downgrade to 1.90
  - **Short description**:
    - Downgrades the Rust toolchain from 1.91 to 1.90. This is required because Rust 1.91 introduced a breaking change in target-spec parsing (`target-pointer-width` must be an integer, not a string) that `polkavm-linker` does not yet comply with, breaking `pallet-revive-fixtures` compilation. This prepares for the polkadot-sdk stable2503 upgrade.
  - **Who is affected**:
    - `🔵 [Node/client integrators]` Downstream projects that integrate StorageHub pallets and/or client.
  - **Suggested code changes**:
    - Switch to Rust toolchain 1.90 and apply changes as `cargo fmt` and `cargo clippy` suggest.
