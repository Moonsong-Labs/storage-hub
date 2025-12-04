//! # RocksDB Column Family Migration System
//!
//! This module provides a versioned migration system for managing RocksDB column family
//! lifecycle, specifically for handling deprecated column families that need to be removed.
//!
//! ## Background
//!
//! RocksDB requires ALL existing column families to be opened when opening a database in
//! read-write mode. If you try to open a database without specifying all existing column
//! families, `DB::Open` returns `InvalidArgument`. This creates a challenge when deprecating
//! column families - you must still open them before you can drop them.
//!
//! ## Solution
//!
//! This migration system:
//! 1. Uses `DB::list_cf()` to discover all existing column families in the database
//! 2. Opens the database with ALL existing CFs plus any new CFs from the current schema
//! 3. Runs versioned migrations to drop deprecated column families using `drop_cf()`
//! 4. Tracks which migrations have been applied using a schema version stored in the database
//!
//! ## Usage
//!
//! ### Adding New Migrations
//!
//! When deprecating column families in a new release:
//!
//! 1. Create a new version module (e.g., `v2.rs`) with a migration struct
//! 2. Implement the [`Migration`] trait for your struct
//! 3. Register the migration in [`MigrationRunner::all_migrations()`]
//!
//! ```ignore
//! // migrations/v2.rs
//! pub struct V2Migration;
//!
//! impl Migration for V2Migration {
//!     const VERSION: u32 = 2;
//!
//!     fn deprecated_column_families() -> &'static [&'static str] {
//!         &["old_cf_name_1", "old_cf_name_2"]
//!     }
//!
//!     fn description() -> &'static str {
//!         "Remove legacy storage request column families"
//!     }
//! }
//! ```
//!
//! ### Opening a Database with Migrations
//!
//! ```ignore
//! use shc_common::migrations::{MigrationRunner, open_db_with_migrations};
//!
//! // Define your current column families (without deprecated ones)
//! const CURRENT_CFS: &[&str] = &["cf1", "cf2", "cf3"];
//!
//! let db = open_db_with_migrations(&db_opts, &path, CURRENT_CFS)?;
//! ```

pub mod v1;

use log::{debug, info};
use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use std::collections::HashSet;
use thiserror::Error;

/// The name of the column family used to store the schema version.
/// This is a reserved name and should not be used for application data.
pub const SCHEMA_VERSION_CF: &str = "__schema_version__";

/// The key used to store the current schema version within the schema version CF.
const SCHEMA_VERSION_KEY: &[u8] = b"version";

/// Errors that can occur during migration operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MigrationError {
    #[error("RocksDB error: {0}")]
    RocksDb(#[from] rocksdb::Error),

    #[error("Failed to parse schema version: {0}")]
    VersionParse(String),

    #[error("Migration {version} failed: {reason}")]
    MigrationFailed { version: u32, reason: String },

    #[error("Cannot downgrade schema version from {current} to {target}")]
    CannotDowngrade { current: u32, target: u32 },
}

/// A trait representing a database schema migration.
///
/// Each migration has a version number and specifies which column families
/// should be dropped when the migration is applied.
///
/// Migrations are applied in order of their version numbers, and each migration
/// only runs once (tracked by the schema version stored in the database).
pub trait Migration: Send + Sync {
    /// The version number of this migration.
    /// Must be unique and migrations are applied in ascending version order.
    const VERSION: u32;

    /// Returns the names of column families that should be dropped by this migration.
    ///
    /// These column families will be removed from the database when this migration
    /// is applied. The migration system will:
    /// 1. First open the database with these CFs (discovered via `list_cf`)
    /// 2. Then drop them using `drop_cf()`
    fn deprecated_column_families() -> &'static [&'static str];

    /// A human-readable description of what this migration does.
    fn description() -> &'static str;
}

/// A descriptor for a migration that can be stored in collections.
///
/// This struct captures the static information from a [`Migration`] implementation
/// for use at runtime without requiring the original type.
pub struct MigrationDescriptor {
    version: u32,
    deprecated_cfs: &'static [&'static str],
    description: &'static str,
}

impl MigrationDescriptor {
    /// Create a new migration descriptor from a type implementing [`Migration`].
    pub fn new<M: Migration>() -> Self {
        Self {
            version: M::VERSION,
            deprecated_cfs: M::deprecated_column_families(),
            description: M::description(),
        }
    }

    /// Returns the version number of this migration.
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Returns the column families deprecated by this migration.
    pub fn deprecated_cfs(&self) -> &'static [&'static str] {
        self.deprecated_cfs
    }

    /// Returns a human-readable description of this migration.
    pub fn description(&self) -> &'static str {
        self.description
    }
}

/// The migration runner handles discovering existing column families,
/// opening the database with all necessary CFs, and running pending migrations.
pub struct MigrationRunner;

impl MigrationRunner {
    /// Returns all registered migrations in version order.
    ///
    /// When adding new migrations, register them here by adding a new
    /// `MigrationDescriptor::new::<YourMigration>()` to the vector.
    pub fn all_migrations() -> Vec<MigrationDescriptor> {
        let mut migrations = vec![MigrationDescriptor::new::<v1::V1Migration>()];

        // Sort by version to ensure correct order
        migrations.sort_by_key(|m| m.version());
        migrations
    }

    /// Get the latest migration version.
    pub fn latest_version() -> u32 {
        Self::all_migrations()
            .last()
            .map(|m| m.version())
            .unwrap_or(0)
    }

    /// Get all column families that have been deprecated across all migrations.
    pub fn all_deprecated_cfs() -> HashSet<&'static str> {
        Self::all_migrations()
            .iter()
            .flat_map(|m| m.deprecated_cfs().iter().copied())
            .collect()
    }

    /// Get deprecated column families for migrations up to and including the given version.
    pub fn deprecated_cfs_up_to_version(version: u32) -> HashSet<&'static str> {
        Self::all_migrations()
            .iter()
            .filter(|m| m.version() <= version)
            .flat_map(|m| m.deprecated_cfs().iter().copied())
            .collect()
    }

    /// Read the current schema version from the database.
    ///
    /// Returns `Ok(0)` if no version has been set (fresh database).
    pub fn read_schema_version(db: &DB) -> Result<u32, MigrationError> {
        let cf_handle = match db.cf_handle(SCHEMA_VERSION_CF) {
            Some(cf) => cf,
            None => {
                // Schema version CF doesn't exist yet - this is a fresh database
                return Ok(0);
            }
        };

        match db.get_cf(&cf_handle, SCHEMA_VERSION_KEY)? {
            Some(bytes) => {
                let bytes_slice: &[u8] = bytes.as_ref();
                let version_bytes: [u8; 4] = bytes_slice.try_into().map_err(|_| {
                    MigrationError::VersionParse(format!(
                        "Invalid version bytes length: {}",
                        bytes_slice.len()
                    ))
                })?;
                Ok(u32::from_le_bytes(version_bytes))
            }
            None => Ok(0),
        }
    }

    /// Write the schema version to the database.
    pub fn write_schema_version(db: &DB, version: u32) -> Result<(), MigrationError> {
        let cf_handle = db.cf_handle(SCHEMA_VERSION_CF).ok_or_else(|| {
            MigrationError::VersionParse(format!(
                "Schema version column family '{}' not found",
                SCHEMA_VERSION_CF
            ))
        })?;
        db.put_cf(&cf_handle, SCHEMA_VERSION_KEY, version.to_le_bytes())?;
        Ok(())
    }

    /// Run all pending migrations on the database.
    ///
    /// This will:
    /// 1. Read the current schema version
    /// 2. Find migrations with version > current
    /// 3. For each pending migration, drop the deprecated column families
    /// 4. Update the schema version after each successful migration
    pub fn run_pending(db: &mut DB) -> Result<u32, MigrationError> {
        let current_version = Self::read_schema_version(db)?;
        let migrations = Self::all_migrations();

        let pending: Vec<_> = migrations
            .iter()
            .filter(|m| m.version() > current_version)
            .collect();

        if pending.is_empty() {
            debug!(
                "No pending migrations. Current schema version: {}",
                current_version
            );
            return Ok(current_version);
        }

        info!(
            "Running {} pending migration(s) from version {} to {}",
            pending.len(),
            current_version,
            pending.last().map(|m| m.version()).unwrap_or(current_version)
        );

        let mut applied_version = current_version;

        for migration in pending {
            info!(
                "Applying migration v{}: {}",
                migration.version(),
                migration.description()
            );

            // Drop deprecated column families
            for cf_name in migration.deprecated_cfs() {
                if db.cf_handle(cf_name).is_some() {
                    info!("  Dropping column family: {}", cf_name);
                    db.drop_cf(cf_name)
                        .map_err(|e| MigrationError::MigrationFailed {
                            version: migration.version(),
                            reason: format!("Failed to drop column family '{}': {}", cf_name, e),
                        })?;
                } else {
                    debug!("Column family '{}' does not exist, skipping", cf_name);
                }
            }

            // Update schema version
            Self::write_schema_version(db, migration.version())?;
            applied_version = migration.version();

            info!("Migration v{} completed successfully", migration.version());
        }

        info!(
            "All migrations completed. Schema version: {}",
            applied_version
        );
        Ok(applied_version)
    }
}

/// Merges existing column families from the database with the current schema's column families.
///
/// This function ensures that:
/// 1. All existing CFs in the database are included (required by RocksDB)
/// 2. All CFs from the current schema are included
/// 3. The schema version CF is always included
///
/// The result is a set of CF names that can be used to open the database.
pub fn merge_column_families<'a>(
    existing_cfs: &[String],
    current_schema_cfs: &[&'a str],
) -> HashSet<String> {
    let mut all_cfs: HashSet<String> = existing_cfs.iter().cloned().collect();

    // Add current schema CFs
    for cf in current_schema_cfs {
        all_cfs.insert(cf.to_string());
    }

    // Always include schema version CF
    all_cfs.insert(SCHEMA_VERSION_CF.to_string());

    // Note: We intentionally do NOT remove deprecated CFs here.
    // They will be removed by the migration runner after the DB is opened.

    all_cfs
}

/// Opens a RocksDB database with automatic migration support.
///
/// This function:
/// 1. Lists existing column families in the database (or uses defaults for new DBs)
/// 2. Merges existing CFs with the current schema's CFs
/// 3. Opens the database with all necessary CFs
/// 4. Runs any pending migrations to drop deprecated CFs
///
/// # Arguments
///
/// * `opts` - RocksDB options for opening the database
/// * `path` - Path to the database directory
/// * `current_schema_cfs` - The column families defined in the current schema (without deprecated CFs)
///
/// # Returns
///
/// The opened database after running all pending migrations.
pub fn open_db_with_migrations(
    opts: &Options,
    path: &str,
    current_schema_cfs: &[&str],
) -> Result<DB, MigrationError> {
    // Try to list existing column families
    let existing_cfs = match DB::list_cf(opts, path) {
        Ok(cfs) => {
            debug!("Found existing column families: {:?}", cfs);
            cfs
        }
        Err(e) => {
            // This typically means the database doesn't exist yet
            debug!(
                "Could not list column families (likely new database): {}",
                e
            );
            vec!["default".to_string()]
        }
    };

    // Merge existing CFs with current schema CFs
    let all_cfs = merge_column_families(&existing_cfs, current_schema_cfs);

    debug!("Opening database with column families: {:?}", all_cfs);

    // Create column family descriptors
    let cf_descriptors: Vec<ColumnFamilyDescriptor> = all_cfs
        .iter()
        .map(|name| ColumnFamilyDescriptor::new(name.clone(), Options::default()))
        .collect();

    // Open the database with all column families
    let mut db = DB::open_cf_descriptors(opts, path, cf_descriptors)?;

    // Run pending migrations
    MigrationRunner::run_pending(&mut db)?;

    Ok(db)
}

/// A helper function to create DB options with common settings for migration-compatible databases.
pub fn default_db_options() -> Options {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);
    opts
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_merge_column_families_empty() {
        let existing: Vec<String> = vec![];
        let current: Vec<&str> = vec!["cf1", "cf2"];

        let merged = merge_column_families(&existing, &current);

        assert!(merged.contains("cf1"));
        assert!(merged.contains("cf2"));
        assert!(merged.contains(SCHEMA_VERSION_CF));
    }

    #[test]
    fn test_merge_column_families_with_existing() {
        let existing = vec![
            "default".to_string(),
            "old_cf".to_string(),
            "cf1".to_string(),
        ];
        let current = vec!["cf1", "cf2", "cf3"];

        let merged = merge_column_families(&existing, &current);

        // Should contain all existing CFs
        assert!(merged.contains("default"));
        assert!(merged.contains("old_cf"));

        // Should contain all current schema CFs
        assert!(merged.contains("cf1"));
        assert!(merged.contains("cf2"));
        assert!(merged.contains("cf3"));

        // Should contain schema version CF
        assert!(merged.contains(SCHEMA_VERSION_CF));
    }

    #[test]
    fn test_migration_runner_latest_version() {
        let latest = MigrationRunner::latest_version();
        assert!(latest >= 1, "Should have at least v1 migration");
    }

    #[test]
    fn test_migration_runner_all_deprecated_cfs() {
        let deprecated = MigrationRunner::all_deprecated_cfs();

        // V1 migration deprecates MSP respond storage request CFs
        assert!(deprecated.contains("pending_msp_respond_storage_request"));
        assert!(deprecated.contains("pending_msp_respond_storage_request_left_index"));
        assert!(deprecated.contains("pending_msp_respond_storage_request_right_index"));
    }

    #[test]
    fn test_open_fresh_database() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let opts = default_db_options();
        let current_cfs = vec!["test_cf"];

        let db = open_db_with_migrations(&opts, path, &current_cfs).unwrap();

        // Check that the database was created with the expected CFs
        assert!(db.cf_handle("test_cf").is_some());
        assert!(db.cf_handle(SCHEMA_VERSION_CF).is_some());

        // Check that schema version was set
        let version = MigrationRunner::read_schema_version(&db).unwrap();
        assert_eq!(version, MigrationRunner::latest_version());
    }

    #[test]
    fn test_migration_drops_deprecated_cfs() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // First, create a database with some "old" column families
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let old_cfs = vec![
                ColumnFamilyDescriptor::new("default", Options::default()),
                ColumnFamilyDescriptor::new(
                    "pending_msp_respond_storage_request",
                    Options::default(),
                ),
                ColumnFamilyDescriptor::new(
                    "pending_msp_respond_storage_request_left_index",
                    Options::default(),
                ),
                ColumnFamilyDescriptor::new("current_cf", Options::default()),
            ];

            let _db = DB::open_cf_descriptors(&opts, path, old_cfs).unwrap();
            // Database is dropped here
        }

        // Now open with migrations - deprecated CFs should be dropped
        let opts = default_db_options();
        let current_cfs = vec!["current_cf"];

        let db = open_db_with_migrations(&opts, path, &current_cfs).unwrap();

        // Current CF should still exist
        assert!(db.cf_handle("current_cf").is_some());

        // Deprecated CFs should be gone
        assert!(db
            .cf_handle("pending_msp_respond_storage_request")
            .is_none());
        assert!(db
            .cf_handle("pending_msp_respond_storage_request_left_index")
            .is_none());
    }

    #[test]
    fn test_migrations_are_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let opts = default_db_options();
        let current_cfs = vec!["test_cf"];

        // Open and run migrations first time
        {
            let db = open_db_with_migrations(&opts, path, &current_cfs).unwrap();
            let version = MigrationRunner::read_schema_version(&db).unwrap();
            assert_eq!(version, MigrationRunner::latest_version());
        }

        // Open again - migrations should not run again
        {
            let db = open_db_with_migrations(&opts, path, &current_cfs).unwrap();
            let version = MigrationRunner::read_schema_version(&db).unwrap();
            assert_eq!(version, MigrationRunner::latest_version());
        }
    }

    #[test]
    fn test_migration_preserves_data_in_active_cfs() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let test_key = b"test_key";
        let test_value = b"important_data_that_must_survive";

        // Create a database simulating "old" state with data
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let cfs = vec![
                ColumnFamilyDescriptor::new("default", Options::default()),
                ColumnFamilyDescriptor::new("active_cf", Options::default()),
                // Deprecated CF that will be removed
                ColumnFamilyDescriptor::new(
                    "pending_msp_respond_storage_request",
                    Options::default(),
                ),
            ];

            let db = DB::open_cf_descriptors(&opts, path, cfs).unwrap();

            // Write data to active CF
            let active_cf = db.cf_handle("active_cf").unwrap();
            db.put_cf(&active_cf, test_key, test_value).unwrap();

            // Verify data was written
            let read_value = db.get_cf(&active_cf, test_key).unwrap().unwrap();
            assert_eq!(&read_value[..], test_value);
        }

        // Now open with migrations
        let opts = default_db_options();
        let current_cfs = vec!["active_cf"];

        let db = open_db_with_migrations(&opts, path, &current_cfs).unwrap();

        // Verify data survived the migration
        let active_cf = db.cf_handle("active_cf").unwrap();
        let read_value = db.get_cf(&active_cf, test_key).unwrap();
        assert!(read_value.is_some(), "Data should survive migration");
        assert_eq!(
            &read_value.unwrap()[..],
            test_value,
            "Data should be unchanged after migration"
        );

        // Deprecated CF should be gone
        assert!(db
            .cf_handle("pending_msp_respond_storage_request")
            .is_none());
    }

    #[test]
    fn test_full_v1_migration_scenario() {
        // This test simulates a real upgrade scenario with all the column families
        // that existed before V1 migration
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // All 3 deprecated CFs from V1 migration
        let deprecated_cfs = v1::V1Migration::deprecated_column_families();
        assert_eq!(deprecated_cfs.len(), 3, "V1 should deprecate exactly 3 CFs");

        // Simulate the old database with all CFs (current + deprecated)
        let current_cfs = vec![
            "last_processed_block_number",
            "pending_confirm_storing_request",
            "pending_confirm_storing_request_left_index",
            "pending_confirm_storing_request_right_index",
            "pending_stop_storing_for_insolvent_user_request",
            "pending_stop_storing_for_insolvent_user_request_left_index",
            "pending_stop_storing_for_insolvent_user_request_right_index",
            "pending_file_deletion_request",
            "pending_file_deletion_request_left_index",
            "pending_file_deletion_request_right_index",
        ];

        // Create old database with both current and deprecated CFs
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let mut all_cfs: Vec<ColumnFamilyDescriptor> = current_cfs
                .iter()
                .map(|name| ColumnFamilyDescriptor::new(*name, Options::default()))
                .collect();

            // Add deprecated CFs
            for cf_name in deprecated_cfs {
                all_cfs.push(ColumnFamilyDescriptor::new(*cf_name, Options::default()));
            }

            // Add default CF
            all_cfs.push(ColumnFamilyDescriptor::new("default", Options::default()));

            let db = DB::open_cf_descriptors(&opts, path, all_cfs).unwrap();

            // Write some data to current CFs
            let cf = db.cf_handle("last_processed_block_number").unwrap();
            db.put_cf(&cf, b"block", b"12345").unwrap();

            // Write some data to deprecated CFs (to ensure they had data)
            let deprecated_cf = db.cf_handle("pending_msp_respond_storage_request").unwrap();
            db.put_cf(&deprecated_cf, b"old_key", b"old_value").unwrap();
        }

        // Now open with migration system
        let opts = default_db_options();
        let db = open_db_with_migrations(&opts, path, &current_cfs).unwrap();

        // Verify all current CFs exist
        for cf_name in &current_cfs {
            assert!(
                db.cf_handle(cf_name).is_some(),
                "Current CF '{}' should exist after migration",
                cf_name
            );
        }

        // Verify all deprecated CFs are gone
        for cf_name in deprecated_cfs {
            assert!(
                db.cf_handle(cf_name).is_none(),
                "Deprecated CF '{}' should be removed after migration",
                cf_name
            );
        }

        // Verify data in current CFs survived
        let cf = db.cf_handle("last_processed_block_number").unwrap();
        let value = db.get_cf(&cf, b"block").unwrap();
        assert!(value.is_some(), "Data should survive migration");
        assert_eq!(&value.unwrap()[..], b"12345");

        // Verify schema version is correct
        let version = MigrationRunner::read_schema_version(&db).unwrap();
        assert_eq!(version, 1, "Schema version should be 1 after V1 migration");
    }

    #[test]
    fn test_database_works_normally_after_migration() {
        // Test that the database is fully functional after migration
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // Create old database with deprecated CFs
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let cfs = vec![
                ColumnFamilyDescriptor::new("default", Options::default()),
                ColumnFamilyDescriptor::new("my_cf", Options::default()),
                ColumnFamilyDescriptor::new(
                    "pending_msp_respond_storage_request",
                    Options::default(),
                ),
            ];

            let _db = DB::open_cf_descriptors(&opts, path, cfs).unwrap();
        }

        // Open with migrations
        let opts = default_db_options();
        let current_cfs = vec!["my_cf"];
        let db = open_db_with_migrations(&opts, path, &current_cfs).unwrap();

        // Test that we can still do normal operations
        let cf = db.cf_handle("my_cf").unwrap();

        // Write
        db.put_cf(&cf, b"key1", b"value1").unwrap();
        db.put_cf(&cf, b"key2", b"value2").unwrap();

        // Read
        assert_eq!(&db.get_cf(&cf, b"key1").unwrap().unwrap()[..], b"value1");

        // Delete
        db.delete_cf(&cf, b"key1").unwrap();
        assert!(db.get_cf(&cf, b"key1").unwrap().is_none());

        // Iterate
        let iter = db.iterator_cf(&cf, rocksdb::IteratorMode::Start);
        let count = iter.count();
        assert_eq!(count, 1, "Should have 1 remaining key");

        // Close and reopen to verify persistence
        drop(db);

        let db = open_db_with_migrations(&opts, path, &current_cfs).unwrap();
        let cf = db.cf_handle("my_cf").unwrap();
        assert_eq!(&db.get_cf(&cf, b"key2").unwrap().unwrap()[..], b"value2");
    }

    #[test]
    fn test_migration_with_no_deprecated_cfs_present() {
        // Test the case where someone already has a clean database without deprecated CFs
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // Create a database that never had deprecated CFs
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let cfs = vec![
                ColumnFamilyDescriptor::new("default", Options::default()),
                ColumnFamilyDescriptor::new("clean_cf", Options::default()),
            ];

            let db = DB::open_cf_descriptors(&opts, path, cfs).unwrap();
            let cf = db.cf_handle("clean_cf").unwrap();
            db.put_cf(&cf, b"key", b"value").unwrap();
        }

        // Open with migrations - should work fine even without deprecated CFs
        let opts = default_db_options();
        let current_cfs = vec!["clean_cf"];
        let db = open_db_with_migrations(&opts, path, &current_cfs).unwrap();

        // Data should be preserved
        let cf = db.cf_handle("clean_cf").unwrap();
        assert_eq!(&db.get_cf(&cf, b"key").unwrap().unwrap()[..], b"value");

        // Version should still be set
        let version = MigrationRunner::read_schema_version(&db).unwrap();
        assert_eq!(version, MigrationRunner::latest_version());
    }
}
