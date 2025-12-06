## Summary

Adds a versioned migration framework for managing RocksDB column family schema changes. This enables safe removal of deprecated column families when upgrading existing databases, ensuring backward compatibility with older node installations.

### Design

The migration system handles a key RocksDB constraint: you cannot open a database without specifying all existing column families. This means deprecated CFs must be discovered and included when opening, then dropped via migration.

**Migration Flow:**
1. `DB::list_cf()` discovers all existing column families (including deprecated ones)
2. Database opens with union of existing + current CFs
3. `MigrationRunner` checks schema version and runs pending migrations
4. Deprecated CFs are dropped and schema version is updated

Schema version is tracked in a dedicated `__schema_version__` column family. Databases without this CF (pre-migration) are treated as version 0.

### Notable changes

#### Migration Framework (`client/common/src/migrations/`)

- `Migration` trait defines versioned migrations with deprecated CF lists
- `MigrationDescriptor` captures migration metadata for runtime use
- `MigrationRunner` handles version tracking and migration execution
- `MigrationError` enum for structured error handling
- Comprehensive test suite covering fresh databases, existing databases with deprecated CFs, idempotency, and data preservation

#### V1 Migration

Drops deprecated MSP respond storage request column families:
- `pending_msp_respond_storage_request`
- `pending_msp_respond_storage_request_left_index`
- `pending_msp_respond_storage_request_right_index`

These were replaced with in-memory queueing in `MspHandler`.

#### Unified Database Opening

- `TypedRocksDB::open()` now handles migrations automatically
- Removed manual `DB::open_cf_descriptors()` calls from all stores
- Consistent `CURRENT_COLUMN_FAMILIES` pattern across stores

**Stores updated:**
- `BlockchainServiceStateStore`
- `DownloadStateStore`
- `BspPeerManagerStore`

### Adding New Migrations

```rust
// 1. Create client/common/src/migrations/v2.rs
pub struct V2Migration;

impl Migration for V2Migration {
    const VERSION: u32 = 2;

    fn deprecated_column_families() -> &'static [&'static str] {
        &["old_cf_to_remove"]
    }

    fn description() -> &'static str {
        "Remove old_cf_to_remove column family"
    }
}

// 2. Register in MigrationRunner::all_migrations()
vec![
    MigrationDescriptor::new::<v1::V1Migration>(),
    MigrationDescriptor::new::<v2::V2Migration>(),
]
```
