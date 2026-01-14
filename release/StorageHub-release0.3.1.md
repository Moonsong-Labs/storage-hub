# StorageHub v0.3.1

## Summary

StorageHub v0.3.1 is a **patch release focused on operability and scale**: it adds first-class **Prometheus/Grafana telemetry** for client services, introduces a **bounded LRU cache for MSP forest instances** to prevent running out of memory / file descriptors on large bucket counts, and hardens MSP backend endpoints by **reducing RPC load** and **caching `/stats`**. It also includes a small SDK compatibility fix for Reown social login and updates the toolchain to Rust 1.91.

## Components

- Client code: v0.3.0
- Pallets code: v0.3.0
- Runtime code: v0.3.0 (spec_name/spec_version: parachain `sh-parachain-runtime`/1, solochain-evm `sh-solochain-evm`/1)
- SH Backend Docker image: v0.3.0 (image: ghcr.io/<org>/storage-hub-msp-backend:v0.3.0)
- SH SDK (npm): v0.4.3 (`@storagehub-sdk/core`, `@storagehub-sdk/msp-client`)
- types-bundle/api-augment (npm): `@storagehub/types-bundle` v0.3.0, `@storagehub/api-augment` v0.3.1

## Changes since last tag

Base: 017118ee91d23f401c45e7abfaa8ac82c8137de8

- Highlights:

  - **Prometheus metrics for client services (MSP/BSP/Fisherman)**: adds `shc-telemetry` and a comprehensive metrics catalogue for command processing, event handlers, block processing, file transfer, and system resources. Includes Docker Prometheus + Grafana scaffolding and dashboards for provider roles ([PR #594](https://github.com/Moonsong-Labs/storage-hub/pull/594)).
  - **MSP scale fix: bound open forest instances via LRU**: MSP forests are now lazy-loaded and retained through a bounded cache, preventing runaway memory/FD usage on large bucket counts. Adds a new `max_open_forests` configuration surface (default **512**) ([PR #645](https://github.com/Moonsong-Labs/storage-hub/pull/645)).
  - **Cheaper and safer `/stats`**: backend `/stats` is cached to avoid being spammed, and the runtime API it depends on is simplified via a new Payment Streams runtime API `get_number_of_active_users_of_provider` ([PR #650](https://github.com/Moonsong-Labs/storage-hub/pull/650)).
  - **Backend load reduction**: backend health checks no longer query provider ID, reducing RPC churn and log noise ([PR #648](https://github.com/Moonsong-Labs/storage-hub/pull/648)).

- Full diff: https://github.com/Moonsong-Labs/storage-hub/compare/017118ee91d23f401c45e7abfaa8ac82c8137de8...e135f65ad54d5c500259d0f6d3b6c34ea66dc2a4
- PRs included:
  - #650 feat: ✨ Add caching to `/stats` endpoint
  - #648 refactor(backend): :zap: make it so the health endpoint of the backend does not query the provider ID
  - #647 perf: ⚡️ Optimize fisherman batch deletion queries
  - #645 fix: :bug: make it so we don't open all forests and instead cache a subset of them
  - #644 build: ⬆ Upgrade to Rust 1.91
  - #641 monitor: update fisherman service logs
  - #639 fix: 🐛 make chain null for compatibility with reown's social login
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
  - No new schema changes in this release.
- Action required:
  - None.

### Indexer DB (Postgres)

- Migrations:
  - No new Postgres migrations in this release.
- How to apply: The indexer service runs migrations automatically on startup. Alternatively run `diesel migration run`.

## ⚠️ Breaking Changes ⚠️

- [PR #594](https://github.com/Moonsong-Labs/storage-hub/pull/594): `StorageHubBuilder::new()` now requires an `Option<&Registry>` (from `substrate_prometheus_endpoint`) to configure Prometheus metrics; node implementations using `StorageHubBuilder` must update the builder call site.
- [PR #645](https://github.com/Moonsong-Labs/storage-hub/pull/645): MSP forest storage is now lazy-loaded and bounded by an LRU cache; operators and integrators may need to tune the new `max_open_forests` setting (and ensure OS FD limits match the chosen cap).
- [PR #650](https://github.com/Moonsong-Labs/storage-hub/pull/650): Payment Streams introduces a new runtime API `get_number_of_active_users_of_provider`; downstream runtimes integrating StorageHub must implement the new API.

## Runtime

- Upgrades (spec_version): parachain and solochain-evm remain at spec_version 1.
- Behaviour changes:
  - **Payment Streams runtime API change for `/stats`**: adds `get_number_of_active_users_of_provider` and simplifies stats collection by iterating payment streams by provider prefix and returning a count (rather than materialising full user lists) ([PR #650](https://github.com/Moonsong-Labs/storage-hub/pull/650)).
- Migrations: None.
- Constants changed: None requiring operator action.
- Scripts to run: None.

## Client

- Behaviour changes:
  - **Telemetry metrics instrumentation**: Prometheus counters/histograms/gauges for command processing and event handler lifecycle (including automatic labelling via actor macros), plus block-processing, file-transfer, and resource-usage metrics via a periodic `sysinfo` collector ([PR #594](https://github.com/Moonsong-Labs/storage-hub/pull/594)).
  - **Forest storage scalability**: forests are no longer all opened eagerly; forest instances are lazy-loaded and retained via a bounded LRU cache to cap FD/memory usage on large bucket counts ([PR #645](https://github.com/Moonsong-Labs/storage-hub/pull/645)).
  - **Fisherman batch deletions performance**: removes an N+1 pattern when building ephemeral tries for batch deletions by fetching required metadata in a single batch query (important for large file counts) ([PR #647](https://github.com/Moonsong-Labs/storage-hub/pull/647)).
  - **Operational logging**: fisherman logs improve hex visibility for file keys and clarify deletion targets/remaining time for batch intervals ([PR #641](https://github.com/Moonsong-Labs/storage-hub/pull/641)).
- Initialisation / configuration changes:
  - **New provider config key**: `max_open_forests` (default: **512**) controls the maximum number of simultaneously open forest storage instances (primarily relevant for MSPs) ([PR #645](https://github.com/Moonsong-Labs/storage-hub/pull/645)).
  - **Monitoring stack**: Prometheus/Grafana deployment templates and dashboards are provided under `docker/` for observing the new metrics ([PR #594](https://github.com/Moonsong-Labs/storage-hub/pull/594)).

## Backend

- Behaviour changes:
  - **`/stats` caching**: adds a simple cache layer to reduce RPC/runtime API load and mitigate endpoint spamming ([PR #650](https://github.com/Moonsong-Labs/storage-hub/pull/650)).
  - **Health endpoint RPC reduction**: backend health checks no longer query provider ID, reducing unnecessary RPC calls and log noise ([PR #648](https://github.com/Moonsong-Labs/storage-hub/pull/648)).
- Initialisation / configuration changes:
  - None required for existing deployments (cache behaviour is internal), but operators should monitor `/stats` freshness expectations after upgrade.

## SDK

- Behaviour changes:
  - **EVM write compatibility**: sets chain ID to `null` during write-contract transactions to improve compatibility with Reown’s social login flows ([PR #639](https://github.com/Moonsong-Labs/storage-hub/pull/639)).
- Initialisation changes:
  - Upgrade to `@storagehub-sdk/core` and `@storagehub-sdk/msp-client` **v0.4.3**.

## Versions

- Polkadot SDK: polkadot-stable2412-6
- Rust: 1.91 (from rust-toolchain.toml)

## Compatibility

- SH Backend v0.3.0 → compatible with pallets/runtime v0.3.0 and client v0.3.0 (all built from this release range).
- SDK v0.4.3 → compatible with backend v0.3.0, client v0.3.0, and pallets/runtime v0.3.0.
- types-bundle v0.3.0 + api-augment v0.3.1 → compatible with this runtime release’s metadata; regenerate if you run custom runtimes.

## Upgrade Guide

### General upgrade notes

- No DB migrations are introduced in this release range; apply migrations as usual if your deployment pipeline expects them (indexer runs them on startup, or run `diesel migration run`).
- If you are integrating StorageHub into a downstream runtime, ensure you update runtime APIs and regenerate bindings as needed before rolling out clients/backends.

### Breaking PRs

- [PR #594](https://github.com/Moonsong-Labs/storage-hub/pull/594) – Telemetry metrics instrumentation (builder API change)

  - **Short description**:
    - The `StorageHubBuilder::new()` constructor now takes an additional `Option<&Registry>` parameter (`Registry` from `substrate_prometheus_endpoint`) to configure Prometheus metrics. If `Some(registry)` is provided, metrics will be enabled for all services. If `None`, metrics will be disabled (no-op).
  - **Who is affected**:
    - Node implementations using `StorageHubBuilder`: you must update the `new()` call to include the prometheus registry parameter.
  - **Suggested code changes**:
    - Update the `StorageHubBuilder::new()` call site and wiring as per [PR #594](https://github.com/Moonsong-Labs/storage-hub/pull/594). The PR includes a full monitoring setup guide and the exact diff for the builder/registry plumbing.

- [PR #645](https://github.com/Moonsong-Labs/storage-hub/pull/645) – Bounded forest LRU cache (`max_open_forests`)

  - **Short description**:
    - Forest storage instances are now **lazy-loaded** and kept open via a **bounded LRU cache** to prevent running out of memory / file descriptors when an MSP manages many buckets. A new `max_open_forests` setting controls the maximum number of simultaneously open forests (default: **512**).
  - **Who is affected**:
    - **Projects using the StorageHub Client**: this change directly impacts runtime behaviour and file descriptor usage.
    - **Operators running via `config.toml`**: a new provider config key is available (`max_open_forests`) (safe default exists).
  - **Suggested code changes**:
    - If you maintain a downstream node wrapper, wire `--max-open-forests`/`max_open_forests` through CLI + config and pass it into storage-layer setup as per the examples in [PR #645](https://github.com/Moonsong-Labs/storage-hub/pull/645).
    - If you are only operating the upstream node, you may keep defaults; tune `max_open_forests` alongside OS `ulimit`/FD limits if you manage very large bucket counts.

- [PR #650](https://github.com/Moonsong-Labs/storage-hub/pull/650) – New Payment Streams runtime API for `/stats`

  - **Short description**:
    - There’s a new runtime API called `get_number_of_active_users_of_provider` under the Payment Streams pallet. Runtime managers of runtimes that use StorageHub will have to implement it.
  - **Who is affected**:
    - Runtime managers of runtimes that use StorageHub since the new runtime API has to be implemented.
  - **Suggested code changes**:
    - Implement the runtime API in your runtime. See [PR #650](https://github.com/Moonsong-Labs/storage-hub/pull/650) for the full snippet and context.
