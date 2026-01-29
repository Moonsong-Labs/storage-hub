# StorageHub v0.3.5

## Summary

StorageHub v0.3.5 is a **patch release focused on client configurability for storage providers**. It introduces new CLI options for controlling how the Fisherman service queries and processes pending file deletions (with filtering by TTL and randomised ordering to reduce cross-node collisions), as well as configurable batch sizes for BSP confirm-storing and MSP respond-storage extrinsics.

## Components

- Client code: v0.3.5
- Pallets code: v0.3.5
- Runtime code: v0.3.5 (spec_name/spec_version: parachain `sh-parachain-runtime`/1, solochain-evm `sh-solochain-evm`/1)
- SH Backend Docker image: v0.3.5 (image: ghcr.io/<org>/storage-hub-msp-backend:v0.3.5)
- SH SDK (npm): v0.3.4 (`@storagehub-sdk/core`, `@storagehub-sdk/msp-client`)
- types-bundle/api-augment (npm): `@storagehub/types-bundle` v0.3.2, `@storagehub/api-augment` v0.3.2

## Changes since last tag

Base: 5a7e487d7c9f06c946cccbf8c8c5bd88cf08b89b

- Highlights:

  - **Fisherman filtering and ordering configurations**: the Fisherman service now supports configurable strategies for querying pending file deletions; operators can filter files by TTL (skipping files older than a threshold) and order them randomly to reduce the probability of collisions between multiple Fisherman nodes deleting files belonging to the same Bucket or BSP ([PR #667](https://github.com/Moonsong-Labs/storage-hub/pull/667)).
  - **BSP and MSP batch size CLI options**: new CLI options allow operators to configure how many storage requests are batched into a single extrinsic call for both BSP confirm-storing (`--bsp-confirm-file-batch-size`) and MSP respond-storage (`--msp-respond-storage-batch-size`) operations, with a default of 20 for both ([PR #666](https://github.com/Moonsong-Labs/storage-hub/pull/666)).

- Full diff: https://github.com/Moonsong-Labs/storage-hub/compare/5a7e487d7c9f06c946cccbf8c8c5bd88cf08b89b...e7b1f0aa95a9839e595a5b2fa9af0bc0fb44bf9b
- PRs included:
  - #667 feat(fisherman): Add fisherman file deletion filtering and ordering configurations
  - #666 feat: Add BSP and MSP batch size cli options responding to storage requests

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

- [PR #667](https://github.com/Moonsong-Labs/storage-hub/pull/667): Fisherman CLI options have been added to support specifying filtering and ordering strategies for pending file deletions (`--fisherman-filtering`, `--fisherman-ordering`, `--fisherman-ttl-threshold-seconds`); Fisherman node operators should review the new options.
- [PR #666](https://github.com/Moonsong-Labs/storage-hub/pull/666): MSP and BSP CLI options have been added to support specifying a specific batch response and confirm size (`--bsp-confirm-file-batch-size`, `--msp-respond-storage-batch-size`); MSP and BSP node operators should review the new options.

## Runtime

- Upgrades (spec_version): parachain and solochain-evm remain at spec_version 1.
- Behaviour changes:
  - None in this release range.
- Migrations: None (runtime storage layout unchanged in this release range).
- Constants changed: None requiring operator action.
- Scripts to run: None.

## Client

- Behaviour changes:
  - **Fisherman filtering and ordering**: the Fisherman service now supports configurable strategies for querying pending file deletions via new CLI options:
    - `--fisherman-filtering`: filtering strategy (`none` (default), `ttl`)
    - `--fisherman-ordering`: ordering strategy (`chronological` (default), `randomized`)
    - `--fisherman-ttl-threshold-seconds`: TTL threshold in seconds (required when `--fisherman-filtering=ttl`)

    The `ttl` filtering strategy skips files that have been pending deletion for longer than the specified threshold. The `randomized` ordering strategy uses PostgreSQL's `RANDOM()` function to reduce collisions between multiple Fisherman nodes ([PR #667](https://github.com/Moonsong-Labs/storage-hub/pull/667)).
  - **Configurable batch sizes for storage requests**: new CLI options allow operators to control how many storage requests are batched into a single extrinsic:
    - `--bsp-confirm-file-batch-size`: maximum number of BSP confirm-storing requests to batch (default: 20)
    - `--msp-respond-storage-batch-size`: maximum number of MSP respond-storage requests to batch (default: 20)

    ([PR #666](https://github.com/Moonsong-Labs/storage-hub/pull/666)).
- Initialisation / configuration changes:
  - New CLI options added as described above. Existing configurations will continue to work with the default values.

## Backend

- Behaviour changes:
  - None in this release range.
- Initialisation / configuration changes:
  - None.

## SDK

- Behaviour changes:
  - None in this release range (SDK remains at v0.3.4).
- Initialisation changes:
  - None.

## Versions

- Polkadot SDK: polkadot-stable2412-6
- Rust: 1.91 (from rust-toolchain.toml)

## Compatibility

- SH Backend v0.3.5 → compatible with pallets/runtime v0.3.5 and client v0.3.5 (all built from this release range).
- SDK v0.3.4 → compatible with backend v0.3.5, client v0.3.5, and pallets/runtime v0.3.5.
- types-bundle v0.3.2 + api-augment v0.3.2 → compatible with this runtime release's metadata; regenerate if you run custom runtimes.

## Upgrade Guide

### General upgrade notes

- No database migrations are required in this release; upgrading from v0.3.4 should be straightforward.
- New CLI options have been added but all have sensible defaults, so existing startup scripts will continue to work without modification.

### Breaking PRs

- [PR #667](https://github.com/Moonsong-Labs/storage-hub/pull/667) – Fisherman filtering and ordering configurations

  - **Short description**:

    Fisherman CLI options have been added to support specifying filtering and ordering strategies for pending file deletions:

    - `--fisherman-filtering`: The filtering strategy [`none` (default), `ttl`]
    - `--fisherman-ordering`: The ordering strategy [`chronological` (default), `randomized`]
    - `--fisherman-ttl-threshold-seconds`: TTL for a file to be ignored for deletion in seconds

  - **Who is affected**:

    - Fisherman node operators.

  - **Suggested code changes**:

    - Add the CLI options:

      ```rust
      /// Filtering strategy for pending deletions.
      #[arg(
          long,
          value_enum,
          default_value = "none",
          help_heading = "Fisherman Strategy Options"
      )]
      pub fisherman_filtering: FishermanFiltering,

      /// Ordering strategy for pending deletions.
      #[arg(
          long,
          value_enum,
          default_value = "chronological",
          help_heading = "Fisherman Strategy Options"
      )]
      pub fisherman_ordering: FishermanOrdering,

      /// TTL threshold in seconds for pending deletions.
      /// Files that have been pending deletion for longer than this threshold are skipped.
      /// Required when --fisherman-filtering=ttl.
      #[arg(
          long,
          value_parser = clap::value_parser!(u64).range(1..),
          required_if_eq("fisherman_filtering", "ttl"),
          help_heading = "Fisherman Strategy Options"
      )]
      pub fisherman_ttl_threshold_seconds: Option<u64>,
      ```

    - Add to `fisherman_options`:

      ```rust
      // Convert CLI enums to indexer-db enums
      let filtering = match self.fisherman_filtering {
          FishermanFiltering::None => FileFiltering::None,
          FishermanFiltering::Ttl => FileFiltering::Ttl {
              threshold_seconds: self
                  .fisherman_ttl_threshold_seconds
                  .expect("Required when filtering=ttl"),
          },
      };

      let ordering = match self.fisherman_ordering {
          FishermanOrdering::Chronological => FileOrdering::Chronological,
          FishermanOrdering::Randomized => FileOrdering::Randomized,
      };

      Some(FishermanOptions {
          database_url: self
              .fisherman_database_url
              .clone()
              .expect("Fisherman database URL is required"),
          batch_interval_seconds: self.fisherman_batch_interval_seconds,
          batch_deletion_limit: self.fisherman_batch_deletion_limit,
          maintenance_mode,
          filtering, // new config field
          ordering, // new config field
      })
      ```

- [PR #666](https://github.com/Moonsong-Labs/storage-hub/pull/666) – BSP and MSP batch size CLI options

  - **Short description**:

    MSP and BSP CLI options have been added to support specifying a specific batch response and confirm size for MSP and BSP nodes.

    - `--bsp-confirm-file-batch-size`: How many storage requests to respond to (confirming) in a single extrinsic call
    - `--msp-respond-storage-batch-size`: How many storage requests to respond to (accepting or rejecting) in a single extrinsic call

  - **Who is affected**:

    - MSP and BSP node operators.

  - **Suggested code changes**:

    - Add the CLI options:

      ```rust
      /// Maximum number of MSP respond storage requests to batch together
      #[arg(
          long,
          value_name = "COUNT",
          help_heading = "Blockchain Service Options",
          default_value = "20"
      )]
      pub msp_respond_storage_batch_size: Option<u32>,

      /// Maximum number of BSP confirm storing requests to batch together
      #[arg(
          long,
          value_name = "COUNT",
          help_heading = "Blockchain Service Options",
          default_value = "20"
      )]
      pub bsp_confirm_file_batch_size: Option<u32>,
      ```

    - Add to `provider_options` for blockchain service building:

      ```rust
      if let Some(bsp_confirm_file_batch_size) = self.bsp_confirm_file_batch_size {
          bs_options.bsp_confirm_file_batch_size = Some(bsp_confirm_file_batch_size);
          bs_changed = true;
      }

      if let Some(msp_respond_storage_batch_size) = self.msp_respond_storage_batch_size {
          bs_options.msp_respond_storage_batch_size = Some(msp_respond_storage_batch_size);
          bs_changed = true;
      }
      ```
