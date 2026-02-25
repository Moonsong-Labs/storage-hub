## Summary

Adds replication tracking to detect under-replication caused by BSP churn (insolvency/deletion). The `NewStorageRequest` pallet event now includes `bsps_required` and `msp_id`, the indexer computes a rolling `desired_replicas` target per file key, and the MSP backend API exposes both `desiredReplicas` and `currentReplication` so that dApps can prompt users to submit healing storage requests.

## What Changed

- **Pallet event extended**: Added `bsps_required: ReplicationTargetType<T>` and `msp_id: Option<ProviderIdFor<T>>` to the existing `NewStorageRequest` event (codec index 8) in `pallets/file-system/src/lib.rs`.
- **Event emission updated**: `do_request_storage` in `pallets/file-system/src/utils.rs` now passes `bsps_required` and `msp_id` when depositing the event.
- **Indexer migration**: New migration (`2026-02-24-000001_add_replication_tracking`) adds `bsps_required` and `desired_replicas` columns to the `file` table and changes the `bsp_file.bsp_id` foreign key to `ON DELETE CASCADE` so BSP deletion automatically cleans up associations.
- **Indexer handler logic**: `client/indexer-service/src/handler.rs` extracts the new event fields and computes `desired_replicas` using `max(prev_desired, current_bsp_count + bsps_required)` for user-initiated SRs (`msp_id.is_some()`), and carries forward the previous value unchanged for system-initiated SRs (`msp_id.is_none()`).
- **Indexer DB model**: `File` struct and `File::create()` accept the two new columns; two new query helpers added (`count_bsp_associations_by_file_key`, `get_max_desired_replicas_by_file_key`).
- **Backend repository layer**: `StorageOperations` trait, PostgreSQL implementation, and mock implementation extended with `get_desired_replicas_for_file_key` and `count_bsp_associations_for_file_key`.
- **MSP backend API**: `FileInfo` now includes `desiredReplicas` and `currentReplication` fields; `MspService::get_file_info` queries them at response time.
- **Client pattern matches**: BSP handler and utils in `client/blockchain-service` updated with `..` to tolerate the new event fields.
- **Import style cleanup**: Merged split `std` imports in `msp.rs`, `client.rs`, and `handler.rs` per project conventions.
- **Tests updated**: Two pallet-level assertions in `pallets/file-system/src/tests.rs` now include the new event fields. All 210 pallet tests pass.

## ⚠️ Breaking Changes ⚠️

- **Short description**: The `NewStorageRequest` event (codec index 8) SCALE encoding has changed — two new fields (`bsps_required`, `msp_id`) are appended, and a new DB migration must be applied.

- **Who is affected**
  - `🟣 [Runtime maintainers]` The `NewStorageRequest` event layout changed; any external decoder or indexer parsing this event must be updated to expect the two new trailing fields.
  - `🔵 [Node/client integrators]` The indexer DB migration `2026-02-24-000001_add_replication_tracking` must be run before starting the updated node. The `bsp_file` FK is changed to `ON DELETE CASCADE`.
  - `🟢 [MSP operators]` The `/file-info` API response now includes `desiredReplicas` (i32) and `currentReplication` (i64). Frontend consumers should handle these new fields.

- **Suggested code changes**

  1) **Database migration** — run diesel migrations before starting the node:

  ```bash
  diesel migration run --migration-dir client/indexer-db/migrations/
  ```

  The migration adds two columns and alters a foreign key:

  ```sql
  ALTER TABLE file ADD COLUMN bsps_required INTEGER NOT NULL DEFAULT 0;
  ALTER TABLE file ADD COLUMN desired_replicas INTEGER NOT NULL DEFAULT 0;

  ALTER TABLE bsp_file DROP CONSTRAINT bsp_file_bsp_id_fkey;
  ALTER TABLE bsp_file ADD CONSTRAINT bsp_file_bsp_id_fkey
      FOREIGN KEY (bsp_id) REFERENCES bsp(id) ON DELETE CASCADE;
  ```

  2) **External event decoders** — update any code that decodes `pallet_file_system::Event::NewStorageRequest` to include the two new trailing fields:

  ```rust
  pallet_file_system::Event::NewStorageRequest {
      who,
      file_key,
      bucket_id,
      location,
      fingerprint,
      size,
      peer_ids,
      expires_at,
      bsps_required,  // NEW
      msp_id,         // NEW
  } => { /* ... */ }
  ```

  Alternatively, use `..` to ignore the new fields if they are not needed:

  ```rust
  pallet_file_system::Event::NewStorageRequest {
      who, file_key, bucket_id, location, fingerprint, size, peer_ids, expires_at, ..
  } => { /* ... */ }
  ```
