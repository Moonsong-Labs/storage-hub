# StorageHub v0.4.1

## Summary

StorageHub v0.4.1 focuses on **transaction-lifecycle resilience and operator configurability** in the client, plus **major MSP upload throughput improvements** and **a new SDK helper for public-file download URLs**. Highlights include configurable extrinsic mortality for providers and fishermen, transaction-manager handling for non-terminal watcher disconnects and stale pending transactions, batched trie ingestion during backend uploads, and a new SDK `getDownloadUrl` method for public files in `ready` status.

## Components

- Client code: v0.4.1
- Pallets code: v0.4.1
- Runtime code: v0.4.1 (spec_name/spec_version: parachain `sh-parachain-runtime`/1, solochain-evm `sh-solochain-evm`/1)
- SH Backend Docker image: v0.4.1 (image: moonsonglabs/storage-hub-msp-backend:v0.4.1)
- SH SDK (npm): v0.4.6 (`@storagehub-sdk/core`, `@storagehub-sdk/msp-client`)
- types-bundle/api-augment (npm): `@storagehub/types-bundle` v0.4.1, `@storagehub/api-augment` v0.4.1

## Changes since last tag

Base: 325c93b684224d3b93024fa0f912e175fe2380ae

- Highlights:
  - **Configurable extrinsic mortality and tx-manager robustness**: introduce configurable `extrinsic_mortality` for providers (`--extrinsic-mortality`, `[provider.blockchain_service].extrinsic_mortality`) and fishermen (`--fisherman-extrinsic-mortality`, `[fisherman.blockchain_service].extrinsic_mortality`), improve handling of non-terminal transaction watcher disconnects, and clean stale pending transactions whose mortality has elapsed to avoid nonce deadlocks after deep reorg scenarios ([PR #684](https://github.com/Moonsong-Labs/storage-hub/pull/684)).
  - **MSP upload performance**: speed up file-ingestion trie construction by batching chunk inserts (`write_chunks_batched`) and reducing per-chunk mutator/allocation overhead in the backend upload flow; for large files this significantly reduces the "read + build trie" phase ([PR #683](https://github.com/Moonsong-Labs/storage-hub/pull/683)).
  - **SDK public-file download URL helper**: add `getDownloadUrl(bucketId, fileKey)` in the MSP client module, validating that files exist, are public (`isPublic`), and are in `ready` status before returning a canonical `/download/<fileKey>` URL ([PR #685](https://github.com/Moonsong-Labs/storage-hub/pull/685)).
  - **Cleaner fisherman logs for non-provider roles**: avoid syncing provider ID for fisherman role paths and suppress recurring "no Provider ID linked" warnings when this is expected ([PR #680](https://github.com/Moonsong-Labs/storage-hub/pull/680)).

- Full diff: https://github.com/Moonsong-Labs/storage-hub/compare/325c93b684224d3b93024fa0f912e175fe2380ae...72400309b7fdc659f60a2af486a43a6e94de0aec
- PRs included:
  - #685 feat: ✨ SDK download link for public files
  - #684 feat: ✨ make extrinsic mortality configurable and fix tx manager issues
  - #683 feat: ⚡ improve backend upload
  - #682 chore: downgrade "Re-queuing file key" log from info to debug
  - #680 fix: 🩹 suppress spurious provider ID warning for fisherman role

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
  - Transaction-manager cleanup now removes stale pending transactions whose extrinsic mortality has elapsed, improving recovery from dropped/reorged transactions ([PR #684](https://github.com/Moonsong-Labs/storage-hub/pull/684)).
- Action required:
  - None.

### Indexer DB (Postgres)

- Migrations:
  - No new migrations in this release.
- How to apply: The indexer service runs migrations automatically on startup. Alternatively: `diesel migration run`.

## ⚠️ Breaking Changes ⚠️

- [PR #684](https://github.com/Moonsong-Labs/storage-hub/pull/684): adds configurable `extrinsic_mortality` surfaces for provider and fisherman blockchain-service configuration/CLI wiring. Provider operators, fisherman operators, and node/client integrators should review and propagate the new options (`--extrinsic-mortality`, `--fisherman-extrinsic-mortality`, plus TOML keys) where they maintain custom configuration paths.

## Runtime

- Upgrades (spec_version): parachain and solochain-evm remain at spec_version 1.
- Migrations: none in this release.
- Behaviour changes:
  - No runtime logic/API changes were introduced between this base/head range.
- Constants changed: None requiring operator action.
- Scripts to run: None.

## Client

- Behaviour changes:
  - **Extrinsic lifecycle handling**:
    - Configurable extrinsic mortality for provider blockchain service (`extrinsic_mortality`) and fisherman blockchain service (`fisherman_extrinsic_mortality` CLI front-end, wired to blockchain-service `extrinsic_mortality`) ([PR #684](https://github.com/Moonsong-Labs/storage-hub/pull/684)).
    - Treat non-terminal transaction-watcher subscription closures as dropped transactions and clean stale pending transactions during cleanup, reducing stuck-nonce/deadlock risk after deep reorg or dropped-transaction paths ([PR #684](https://github.com/Moonsong-Labs/storage-hub/pull/684)).
  - **Role-aware provider-ID sync**: skip provider-ID synchronisation for non-provider roles (notably fishermen), suppressing expected-but-noisy warning logs ([PR #680](https://github.com/Moonsong-Labs/storage-hub/pull/680)).
  - **Upload path throughput (client/backend integration)**: backend upload ingestion now batches trie chunk writes and avoids per-chunk overhead in the file-manager/trie path used during uploads ([PR #683](https://github.com/Moonsong-Labs/storage-hub/pull/683)).
- Initialisation / configuration changes:
  - New provider CLI/config surface:
    - CLI: `--extrinsic-mortality`
    - TOML: `[provider.blockchain_service].extrinsic_mortality` (default 256) ([PR #684](https://github.com/Moonsong-Labs/storage-hub/pull/684)).
  - New fisherman CLI/config surface:
    - CLI: `--fisherman-extrinsic-mortality`
    - TOML: `[fisherman.blockchain_service].extrinsic_mortality` (default 256) ([PR #684](https://github.com/Moonsong-Labs/storage-hub/pull/684)).

## Backend

- Behaviour changes:
  - **Upload ingestion optimisation**: backend file-upload processing now buffers and flushes chunk batches into trie storage (`write_chunks_batched`) instead of per-chunk insertion, reducing CPU/allocation overhead and significantly accelerating trie build time for large uploads ([PR #683](https://github.com/Moonsong-Labs/storage-hub/pull/683)).
- Initialisation / configuration changes:
  - None required for v0.4.1 beyond standard upgrade steps.

## SDK

- Behaviour changes:
  - **Public-file download URL helper**: add `getDownloadUrl(bucketId, fileKey)` to `@storagehub-sdk/msp-client` files module. The method verifies file existence and enforces that `isPublic === true` and `status === "ready"` before returning a direct download URL; private files are not supported by this helper yet ([PR #685](https://github.com/Moonsong-Labs/storage-hub/pull/685)).
- Initialisation changes:
  - Upgrade to `@storagehub-sdk/core` and `@storagehub-sdk/msp-client` **v0.4.6**.

## Versions

- Polkadot SDK: polkadot-stable2412-6
- Rust: 1.91 (from rust-toolchain.toml)

## Compatibility

- SH Backend v0.4.1 -> compatible with pallets/runtime v0.4.1 and client v0.4.1 (all built from this release).
- SDK v0.4.6 -> compatible with backend v0.4.1, client v0.4.1, and pallets/runtime v0.4.1.
- types-bundle v0.4.1 + api-augment v0.4.1 -> compatible with this runtime release metadata.

## Upgrade Guide

### General upgrade notes

- Apply standard service restarts and migration startup flows (no new DB schema migrations are introduced in this release).
- If you run custom node/client integrations, wire the new extrinsic mortality configuration surfaces for provider and fisherman roles.
- If you run provider/fisherman operations, review mortality defaults and tune for your network conditions (reorg tolerance vs faster nonce recovery).

### Breaking PRs

- [PR #684](https://github.com/Moonsong-Labs/storage-hub/pull/684) - Configurable extrinsic mortality and blockchain-service wiring updates
  - **Short description**:
    - There's a new config parameter `extrinsic_mortality` that has a sane default but can be configured when spinning up a new client, so client runners should be aware of it. Also fisherman runners should be aware that they can now configure their Blockchain Service like providers can, under the `[fisherman.blockchain_service]` header of the TOML configuration file.
  - **Who is affected**:
    - 🟢 [MSP operators] StorageHub Provider client runners.
    - 🟠 [BSP operators] StorageHub Provider client runners.
    - 🔵 [Node/client integrators] Projects integrating the StorageHub Client into their node.
    - 🟡 [Fisherman operators] Fisherman runners.
  - **Suggested code changes**:
    - Providers should add `[provider.blockchain_service].extrinsic_mortality` to TOML configuration, or pass `--extrinsic-mortality <BLOCKS>`.
    - Fishermen should add `[fisherman.blockchain_service].extrinsic_mortality` to TOML configuration, or pass `--fisherman-extrinsic-mortality <BLOCKS>`.
    - Integrators should forward the new CLI/config fields into blockchain-service options for provider and fisherman builders. See the "Suggested code changes" section in [PR #684](https://github.com/Moonsong-Labs/storage-hub/pull/684) for the full migration snippets and wiring details.
