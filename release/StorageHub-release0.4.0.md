# StorageHub v0.4.0

## Summary

StorageHub v0.4.0 focuses on **improving provider operability and performance under load**, with major work on **BSP confirm-storing correctness**, **MSP forest scalability**, **Fisherman deletion throughput**, and **runtime/client decoding stability**. Highlights include new runtime APIs to support safer BSP confirmation flows, an LRU-backed “open forests” cache to cap file descriptor usage for MSPs managing many buckets, `/stats` endpoint hardening and new stats fields, and a new Prometheus/Grafana telemetry surface for the client.

## Components

- Client code: v0.4.0
- Pallets code: v0.4.0
- Runtime code: v0.4.0 (spec_name/spec_version: parachain `sh-parachain-runtime`/1, solochain-evm `sh-solochain-evm`/1)
- SH Backend Docker image: v0.4.0 (image: moonsonglabs/storage-hub-msp-backend:v0.4.0)
- SH SDK (npm): v0.4.5 (`@storagehub-sdk/core`, `@storagehub-sdk/msp-client`)
- types-bundle/api-augment (npm): `@storagehub/types-bundle` v0.4.0, `@storagehub/api-augment` v0.4.0

## Changes since last tag

Base: 017118ee91d23f401c45e7abfaa8ac82c8137de8

- Highlights:
  - **BSP confirm flow correctness and performance**: retry confirm-storing on transient/proof errors, avoid filtering races around volunteer extrinsics, and introduce new runtime APIs `query_pending_bsp_confirm_storage_requests` and `get_max_batch_confirm_storage_requests` to pre-filter and batch confirmations safely ([PR #624](https://github.com/Moonsong-Labs/storage-hub/pull/624), [PR #670](https://github.com/Moonsong-Labs/storage-hub/pull/670)).
  - **MSP scalability for many buckets**: forests are now lazy-loaded and kept open via a bounded LRU cache, with a new `max_open_forests` setting (`--max-open-forests`) to cap simultaneous open forests and prevent running out of memory/file descriptors ([PR #645](https://github.com/Moonsong-Labs/storage-hub/pull/645)).
  - **`/stats` hardening and richer stats**: add caching to reduce runtime API load, introduce a new Payment Streams runtime API `get_number_of_active_users_of_provider`, and add a new `/stats` response field `files_amount` representing the number of files currently stored by an MSP ([PR #650](https://github.com/Moonsong-Labs/storage-hub/pull/650), [PR #672](https://github.com/Moonsong-Labs/storage-hub/pull/672)).
  - **Fisherman deletion throughput and reliability**: remove N+1 query patterns in deletion pre-processing, cap batch deletions using the on-chain `MaxFileDeletionsPerExtrinsic` constant, add `.watch_for_success` to submitted extrinsics, and introduce new Fisherman strategy/config options (filtering/ordering/TTL, cooldown-based scheduling) ([PR #647](https://github.com/Moonsong-Labs/storage-hub/pull/647), [PR #654](https://github.com/Moonsong-Labs/storage-hub/pull/654), [PR #675](https://github.com/Moonsong-Labs/storage-hub/pull/675), [PR #667](https://github.com/Moonsong-Labs/storage-hub/pull/667), [PR #676](https://github.com/Moonsong-Labs/storage-hub/pull/676)).
  - **Runtime/client decoding guardrails**: explicitly pin SCALE codec indices for events and errors across StorageHub pallets and document encoding stability rules to prevent client-side decode failures across runtime upgrades ([PR #658](https://github.com/Moonsong-Labs/storage-hub/pull/658)).
  - **Telemetry and monitoring**: add Prometheus metrics instrumentation (commands, event handlers, block processing, file transfers, resource usage) plus Docker/Grafana dashboards for provider roles; `StorageHubBuilder::new()` now accepts an `Option<&Registry>` to enable/disable metrics ([PR #594](https://github.com/Moonsong-Labs/storage-hub/pull/594)).
  - **Dynamic-rate price updater correctness**: fix NANOUNIT scaling in exponent-factor derivation, and update exponent-factor types to match `Price`/`Balance` (with corrected default values) ([PR #674](https://github.com/Moonsong-Labs/storage-hub/pull/674)).

- Full diff: https://github.com/Moonsong-Labs/storage-hub/compare/017118ee91d23f401c45e7abfaa8ac82c8137de8...325c93b684224d3b93024fa0f912e175fe2380ae
- PRs included:
  - #679 chore: 🔖 Update versions to 0.4.0
  - #677 fix: 🥹 set content type header
  - #676 perf: ⚡ Optimise Fisherman deletion cycle to avoid idle time
  - #675 feat: ✨ Watch for success in fisherman file deletion tasks
  - #674 fix: 🐛 fix units on the exponents of our dynamic-rate price per tick calculator
  - #672 feat(backend): ✨ add the amount of files the MSP is storing to `/stats`
  - #670 feat: ✨ optimize BSP confirm flow
  - #669 fix: 🐛 Allow updating payment streams when provider is insolvent
  - #667 feat(fisherman): ✨ Add fisherman file deletion filtering and ordering configurations
  - #666 feat: ✨ Add BSP and MSP batch size cli options responding to storage requests
  - #662 fix: 🐛 Account for replication target in `batchStorageRequests` testing helper
  - #661 fix: 🥹 Skip slashing storage provider with zero capacity
  - #660 fix: 🥹 types-bundle Readme
  - #659 perf: ⚡️ Replace forest write lock Channel with Semaphore
  - #658 refactor: ♻️ Ensure event and error processing backward and forward compatibility
  - #657 fix: 🐛 Allow updating payment stream for insolvent provider
  - #656 fix(backend): 🐛 drop download sessions when failing or disconnecting
  - #655 feat: ✨ SDK adaptative gas limit
  - #654 fix: Fisherman truncate files to delete based on `MaxFileDeletionsPerExtrinsic` runtime constant
  - #652 fix: 🩹 Avoid creating forests locally when not found in initial check after sync
  - #650 feat: ✨ Add caching to `/stats` endpoint
  - #648 refactor(backend): ⚡ make it so the health endpoint of the backend does not query the provider ID
  - #647 perf: ⚡️ Optimize fisherman batch deletion queries
  - #645 fix: 🐛 make it so we don't open all forests and instead cache a subset of them
  - #644 build: ⬆ Upgrade to Rust 1.91
  - #641 monitor: update fisherman service logs
  - #640 fix: 🐛 Process finality notifications only after processing the corresponding block import
  - #639 fix: 🐛 make chain null for compatibility with reown's social login
  - #624 feat(bsp): ✨ Add retry logic for confirm storing proof errors
  - #620 test: 🧪 Add integration tests for `is_in_bucket` consistency and deletion
  - #594 feat: 📺 Add telemetry metrics instrumentation

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
  - No mandatory migrations. The Blockchain Service persists additional state to ensure finality notifications are only processed after their corresponding block-import processing completes (new “write-on-first-run” behaviour) ([PR #640](https://github.com/Moonsong-Labs/storage-hub/pull/640)).
- Action required:
  - None.

### Indexer DB (Postgres)

- Migrations:
  - No new migrations in this release.
- How to apply: The indexer service runs migrations automatically on startup. Alternatively: `diesel migration run`.

## ⚠️ Breaking Changes ⚠️

- [PR #594](https://github.com/Moonsong-Labs/storage-hub/pull/594): `StorageHubBuilder::new()` now accepts an additional `Option<&Registry>` parameter (from `substrate_prometheus_endpoint`) to enable/disable Prometheus telemetry; node integrations must update constructor calls if they wire the client manually.
- [PR #624](https://github.com/Moonsong-Labs/storage-hub/pull/624): adds a new File System runtime API `query_pending_bsp_confirm_storage_requests`; custom runtimes using StorageHub pallets must implement it.
- [PR #645](https://github.com/Moonsong-Labs/storage-hub/pull/645): forests are now lazy-loaded and kept open via a bounded LRU cache; introduces a new `max_open_forests` setting (`--max-open-forests`) that operators/integrators can tune to cap worst-case file descriptor usage.
- [PR #650](https://github.com/Moonsong-Labs/storage-hub/pull/650): adds a new Payment Streams runtime API `get_number_of_active_users_of_provider`; custom runtimes using StorageHub pallets must implement it.
- [PR #666](https://github.com/Moonsong-Labs/storage-hub/pull/666): adds new MSP/BSP CLI options `--bsp-confirm-file-batch-size` and `--msp-respond-storage-batch-size` (default 20) for batching confirms/responses.
- [PR #667](https://github.com/Moonsong-Labs/storage-hub/pull/667): adds new Fisherman strategy CLI options `--fisherman-filtering`, `--fisherman-ordering`, and `--fisherman-ttl-threshold-seconds`.
- [PR #670](https://github.com/Moonsong-Labs/storage-hub/pull/670): adds a new File System runtime API `get_max_batch_confirm_storage_requests`; custom runtimes using StorageHub pallets must implement it.
- [PR #674](https://github.com/Moonsong-Labs/storage-hub/pull/674): price-updater exponent-factor types change from `u32` to the same type as `Price`/`Balance`; custom runtimes must update types and dynamic-parameter values accordingly.
- [PR #676](https://github.com/Moonsong-Labs/storage-hub/pull/676): Fisherman batch-deletion scheduling becomes event-driven and adds new CLI options (`--fisherman-batch-cooldown-seconds`, `--fisherman-consecutive-no-work-batches-threshold`); default `--fisherman-batch-interval-seconds` is now 30s (was 60s).

## Runtime

- Upgrades (spec_version): parachain and solochain-evm remain at spec_version 1.
- Migrations: none in this release.
- Behaviour changes:
  - **BSP confirm-storing filtering**: new File System runtime API `query_pending_bsp_confirm_storage_requests` to filter file keys to those that still require BSP confirmation ([PR #624](https://github.com/Moonsong-Labs/storage-hub/pull/624)).
  - **BSP confirm-storing batching**: new File System runtime API `get_max_batch_confirm_storage_requests` to expose the maximum confirm batch size ([PR #670](https://github.com/Moonsong-Labs/storage-hub/pull/670)).
  - **`/stats` user-count optimisation**: new Payment Streams runtime API `get_number_of_active_users_of_provider` used by backend stats collection ([PR #650](https://github.com/Moonsong-Labs/storage-hub/pull/650)).
  - **Payment streams and insolvent providers**: allow updating payment streams even when a provider is insolvent (unblocking file deletion flows that require stream updates) ([PR #657](https://github.com/Moonsong-Labs/storage-hub/pull/657), [PR #669](https://github.com/Moonsong-Labs/storage-hub/pull/669)).
  - **SCALE encoding stability**: event/error variants across StorageHub pallets now have explicit `#[codec(index = N)]` indices, and stability rules are documented to prevent decode breakages across runtime upgrades ([PR #658](https://github.com/Moonsong-Labs/storage-hub/pull/658)).
  - **Dynamic-rate pricing correctness**: exponent factors are now scaled correctly (NANOUNIT-aware), and exponent-factor dynamic-parameter types now match `Price`/`Balance` with corrected defaults (e.g. `UpperExponentFactor = 8_777_389`, `LowerExponentFactor = 114_318`) ([PR #674](https://github.com/Moonsong-Labs/storage-hub/pull/674)).
- Constants changed: None requiring operator action beyond the above runtime API/parameter changes.
- Scripts to run: None.

## Client

- Behaviour changes:
  - **Prometheus telemetry**: adds comprehensive metrics instrumentation (commands, event handlers, block import/finality processing, file transfers, and resource usage) plus dashboards for BSP/MSP/Fisherman roles ([PR #594](https://github.com/Moonsong-Labs/storage-hub/pull/594)).
  - **Finality correctness under lag**: finality notifications are queued and only processed once the corresponding block import has been fully processed, avoiding undefined behaviour when block import lags behind finality ([PR #640](https://github.com/Moonsong-Labs/storage-hub/pull/640)).
  - **MSP forest scalability**: keep only a bounded subset of forests open via LRU, rather than opening all forests for all buckets; reduces memory/FD pressure for MSPs with many buckets ([PR #645](https://github.com/Moonsong-Labs/storage-hub/pull/645)).
  - **Forest write-lock simplification**: replace the forest-root write-lock channel with a semaphore + RAII guard to reliably reassign locks even on task errors/panics and reduce lock-handling boilerplate ([PR #659](https://github.com/Moonsong-Labs/storage-hub/pull/659)).
  - **BSP confirm-storing robustness**: retry confirm-storing when proof errors occur due to concurrent forest mutations (e.g. deletions), and fix races where confirms were filtered out before the volunteer extrinsic was finalised on-chain ([PR #624](https://github.com/Moonsong-Labs/storage-hub/pull/624)).
  - **BSP confirm-storing performance**: optimise confirm queue handling and add runtime-max batching support via `get_max_batch_confirm_storage_requests` ([PR #670](https://github.com/Moonsong-Labs/storage-hub/pull/670)).
  - **Fisherman deletion throughput**:
    - Avoid N+1 metadata queries when building ephemeral tries for batch deletions by fetching required metadata columns in a single batch query ([PR #647](https://github.com/Moonsong-Labs/storage-hub/pull/647)).
    - Truncate deletion targets using the on-chain `MaxFileDeletionsPerExtrinsic` constant and use `BoundedVec` throughout to avoid “batch size exceeds” failures that would stall deletions ([PR #654](https://github.com/Moonsong-Labs/storage-hub/pull/654)).
    - Use `.watch_for_success` for Fisherman-submitted extrinsics so tasks succeed/fail based on the on-chain outcome (with improved logs) ([PR #675](https://github.com/Moonsong-Labs/storage-hub/pull/675)).
    - Add configurable deletion strategies (filtering/ordering/TTL threshold) and an event-driven scheduler with cooldown/backoff to reduce idle time ([PR #667](https://github.com/Moonsong-Labs/storage-hub/pull/667), [PR #676](https://github.com/Moonsong-Labs/storage-hub/pull/676)).
  - **Slashing hygiene**: skip slashing providers with zero capacity/stake to avoid repeatedly submitting no-op slash extrinsics ([PR #661](https://github.com/Moonsong-Labs/storage-hub/pull/661)).
  - **Initial sync behaviour**: after initial sync, missing local forests are no longer created during the “check all buckets” step; forests are created lazily at first storage request or first applied mutation ([PR #652](https://github.com/Moonsong-Labs/storage-hub/pull/652)).
- Initialisation / configuration changes:
  - **Telemetry enablement**: `StorageHubBuilder::new()` now accepts `Option<&Registry>` (`substrate_prometheus_endpoint::Registry`) to enable Prometheus metrics ([PR #594](https://github.com/Moonsong-Labs/storage-hub/pull/594)).
  - **MSP forest cache sizing**:
    - New provider config key `max_open_forests` and CLI option `--max-open-forests` (default 512) to cap the number of simultaneously open forests ([PR #645](https://github.com/Moonsong-Labs/storage-hub/pull/645)).
  - **Batch sizing**:
    - New CLI options `--bsp-confirm-file-batch-size` and `--msp-respond-storage-batch-size` (defaults: 20) ([PR #666](https://github.com/Moonsong-Labs/storage-hub/pull/666)).
  - **Fisherman strategy options**:
    - `--fisherman-filtering` (`none` default, `ttl`)
    - `--fisherman-ordering` (`chronological` default, `randomized`)
    - `--fisherman-ttl-threshold-seconds` (required when `--fisherman-filtering=ttl`) ([PR #667](https://github.com/Moonsong-Labs/storage-hub/pull/667)).
  - **Fisherman scheduling options**:
    - `--fisherman-batch-cooldown-seconds`
    - `--fisherman-consecutive-no-work-batches-threshold`
    - Default `--fisherman-batch-interval-seconds` is now 30s ([PR #676](https://github.com/Moonsong-Labs/storage-hub/pull/676)).

## Backend

- Behaviour changes:
  - **`/stats` caching**: adds a simple cache to the `/stats` endpoint to prevent it being spammed (it calls an expensive runtime API) ([PR #650](https://github.com/Moonsong-Labs/storage-hub/pull/650)).
  - **Richer `/stats`**: adds a new `files_amount` field to `/stats`, implemented via indexer-DB counting rather than file-storage scanning or per-bucket enumeration ([PR #672](https://github.com/Moonsong-Labs/storage-hub/pull/672)).
  - **Health endpoint load reduction**: backend health endpoint no longer queries provider ID, reducing RPC calls and connected-node log noise ([PR #648](https://github.com/Moonsong-Labs/storage-hub/pull/648)).
  - **Download-session robustness**: download sessions are now cleaned up via a guard dropped on task exit (including RPC errors/disconnects), preventing stale “active downloads” entries ([PR #656](https://github.com/Moonsong-Labs/storage-hub/pull/656)).
  - **Content type handling**: infer `Content-Type` from file extension where possible (fallback `application/octet-stream`) to fix preview issues; includes common mappings (PDF, images, SVG, JSON, MP4/MP3, etc.) ([PR #677](https://github.com/Moonsong-Labs/storage-hub/pull/677)).
- Initialisation / configuration changes:
  - None required for v0.4.0 beyond standard upgrades.

## SDK

- Behaviour changes:
  - **EVM write compatibility**: set chain ID to null during write-contract transactions to improve compatibility with Reown social login flows ([PR #639](https://github.com/Moonsong-Labs/storage-hub/pull/639)).
  - **Adaptive EIP-1559 gas limits**: automatically compute gas limits using the latest-block base fee, while still allowing overrides via `EvmWriteOptions` on precompile methods ([PR #655](https://github.com/Moonsong-Labs/storage-hub/pull/655)).
- Initialisation changes:
  - Upgrade to `@storagehub-sdk/core` and `@storagehub-sdk/msp-client` **v0.4.5**.

## Versions

- Polkadot SDK: polkadot-stable2412-6
- Rust: 1.91 (from rust-toolchain.toml)

## Compatibility

- SH Backend v0.4.0 → compatible with pallets/runtime v0.4.0 and client v0.4.0 (all built from this release).
- SDK v0.4.5 → compatible with backend v0.4.0, client v0.4.0, and pallets/runtime v0.4.0.
- types-bundle v0.4.0 + api-augment v0.4.0 → compatible with this runtime release’s metadata; regenerate if you run custom runtimes.

## Upgrade Guide

### General upgrade notes

- Apply any database migrations as part of the upgrade (none are introduced in this release, but migrations will still run on startup as normal).
- If you run **custom runtimes** using StorageHub pallets, ensure you implement the newly introduced runtime APIs and update the dynamic-parameter types described below before deploying a runtime upgrade.
- If you run **MSP nodes at scale**, review and, if needed, tune `--max-open-forests` to keep file descriptor usage within OS limits.
- If you run **Fisherman nodes**, note that deletion scheduling is now more responsive by default (interval default 30s) and new configuration flags are available to tune load/throughput.

### Breaking PRs

- [PR #594](https://github.com/Moonsong-Labs/storage-hub/pull/594) – Client telemetry (`StorageHubBuilder::new` signature)
  - **Short description**:
    - The `StorageHubBuilder::new()` constructor now takes an additional `Option<&Registry>` parameter (`Registry` from `substrate_prometheus_endpoint`) to configure Prometheus metrics. If `Some(registry)` is provided, metrics are enabled; if `None`, metrics are disabled (no-op).
  - **Who is affected**:
    - Node implementations integrating the StorageHub client via `StorageHubBuilder`.
  - **Suggested code changes**:
    - Update `StorageHubBuilder::new(...)` call sites to pass an `Option<&Registry>` (and wire a Prometheus registry if you want metrics enabled). See [PR #594](https://github.com/Moonsong-Labs/storage-hub/pull/594) for the concrete diff.

- [PR #624](https://github.com/Moonsong-Labs/storage-hub/pull/624) – BSP confirm-storing retry + new File System runtime API
  - **Short description**:
    - Added a new runtime API `query_pending_bsp_confirm_storage_requests` to the `FileSystemApi` to filter file keys to only those that still require BSP confirmation (BSP volunteered but not yet confirmed storing).
  - **Who is affected**:
    - Custom runtimes using StorageHub pallets (must implement the new runtime API method).
  - **Suggested code changes**:
    - No changes are required for nodes running with the pre-configured Parachain and Solochain runtimes.
    - Implement the runtime API for custom runtimes as shown in [PR #624](https://github.com/Moonsong-Labs/storage-hub/pull/624) (example `query_pending_bsp_confirm_storage_requests` wiring into `FileSystem::query_pending_bsp_confirm_storage_requests`).

- [PR #645](https://github.com/Moonsong-Labs/storage-hub/pull/645) – Forest LRU caching / `max_open_forests`
  - **Short description**:
    - Forest storage instances are now lazy-loaded and kept open via a bounded LRU cache. A new `max_open_forests` setting controls the maximum number of simultaneously open forests (default: 512).
  - **Who is affected**:
    - Operators running MSPs (new config key available; safe default exists).
    - Projects integrating the StorageHub client/node that need to wire the new CLI/config surface.
  - **Suggested code changes**:
    - Add `--max-open-forests` / `max_open_forests` to provider configuration wiring and pass it into storage-layer setup. See [PR #645](https://github.com/Moonsong-Labs/storage-hub/pull/645) for the exact CLI/config snippets and wiring.

- [PR #650](https://github.com/Moonsong-Labs/storage-hub/pull/650) – `/stats` caching + new Payment Streams runtime API
  - **Short description**:
    - Added a new Payment Streams runtime API `get_number_of_active_users_of_provider`.
  - **Who is affected**:
    - Runtime managers of runtimes that use StorageHub (the new runtime API must be implemented).
  - **Suggested code changes**:
    - Implement the runtime API for custom runtimes as shown in [PR #650](https://github.com/Moonsong-Labs/storage-hub/pull/650) (wiring `PaymentStreams::get_number_of_active_users_of_provider` into the runtime API impl).

- [PR #666](https://github.com/Moonsong-Labs/storage-hub/pull/666) – MSP/BSP batch sizing CLI options
  - **Short description**:
    - Adds `--bsp-confirm-file-batch-size` (confirming) and `--msp-respond-storage-batch-size` (accept/reject) to control how many requests are batched into a single extrinsic (defaults: 20).
  - **Who is affected**:
    - MSP and BSP node operators.
  - **Suggested code changes**:
    - Ensure the new CLI options are wired into node configuration if you maintain custom node integrations. See [PR #666](https://github.com/Moonsong-Labs/storage-hub/pull/666) for the exact option definitions and config wiring.

- [PR #667](https://github.com/Moonsong-Labs/storage-hub/pull/667) – Fisherman deletion filtering/ordering configuration
  - **Short description**:
    - Adds Fisherman CLI options for pending-deletion filtering and ordering strategies:
      - `--fisherman-filtering` (`none` default, `ttl`)
      - `--fisherman-ordering` (`chronological` default, `randomized`)
      - `--fisherman-ttl-threshold-seconds` (required when filtering is `ttl`)
  - **Who is affected**:
    - Fisherman node operators.
  - **Suggested code changes**:
    - Ensure the new CLI options are wired into node configuration if you maintain custom node integrations. See [PR #667](https://github.com/Moonsong-Labs/storage-hub/pull/667) for the exact option definitions and how they map to indexer-db enums.

- [PR #670](https://github.com/Moonsong-Labs/storage-hub/pull/670) – BSP confirm flow optimisation + new File System runtime API
  - **Short description**:
    - Adds a new File System runtime API `get_max_batch_confirm_storage_requests`.
  - **Who is affected**:
    - Runtime managers of runtimes that use StorageHub (the new runtime API must be implemented).
  - **Suggested code changes**:
    - Implement the runtime API for custom runtimes as shown in [PR #670](https://github.com/Moonsong-Labs/storage-hub/pull/670) (wiring `FileSystem::get_max_batch_confirm_storage_requests` into the runtime API impl).

- [PR #674](https://github.com/Moonsong-Labs/storage-hub/pull/674) – Dynamic-rate price updater exponent-factor types/values
  - **Short description**:
    - The exponent types of the price updater have changed: they are no longer `u32` and now use the same type as `Price`, because exponents must be scaled by the unit used for prices.
  - **Who is affected**:
    - Runtime managers of runtimes that use StorageHub (dynamic-parameter types and defaults must be updated).
  - **Suggested code changes**:
    - Update dynamic parameters to use `Balance`/`Price` types and set corrected values (e.g. `UpperExponentFactor: Balance = 8_777_389`, `LowerExponentFactor: Balance = 114_318`). See [PR #674](https://github.com/Moonsong-Labs/storage-hub/pull/674) for the full snippet.

- [PR #676](https://github.com/Moonsong-Labs/storage-hub/pull/676) – Fisherman deletion scheduler cooldown/backoff
  - **Short description**:
    - `FishermanOptions` adds `batch_cooldown_seconds` (TOML: `batch_cooldown_seconds`, CLI: `--fisherman-batch-cooldown-seconds`), and batch-deletion scheduling is now event-driven (permit-drop notifications) with a more responsive default interval (30s).
  - **Who is affected**:
    - Fisherman node operators (behavioural changes and new tuning options).
    - Projects integrating the StorageHub client/node that need to wire the new CLI/config surface.
  - **Suggested code changes**:
    - Add and wire the new CLI flags (`--fisherman-batch-cooldown-seconds`, `--fisherman-consecutive-no-work-batches-threshold`) and update the default interval as needed. See [PR #676](https://github.com/Moonsong-Labs/storage-hub/pull/676) for the exact option definitions and wiring snippets.
