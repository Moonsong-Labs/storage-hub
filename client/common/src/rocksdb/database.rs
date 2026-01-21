//! # RocksDB Database Opening
//!
//! This module provides functions for opening RocksDB databases with optional
//! migration support.

use log::debug;
use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use std::collections::HashSet;
use std::path::Path;
use thiserror::Error;

use super::migrations::{MigrationError, MigrationRunner, SCHEMA_VERSION_CF};

const LOG_TARGET: &str = "rocksdb-migrations";

/// Top-level errors that can occur when opening or operating on a database.
///
/// This is the primary error type returned by [`open_db`] and [`open_db_with_migrations`].
#[derive(Debug, Error)]
pub enum DatabaseError {
    /// An error from RocksDB itself.
    #[error("RocksDB error: {0}")]
    RocksDb(#[from] rocksdb::Error),

    /// An I/O error (e.g., creating directories, file operations).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// An error during schema migration.
    #[error("Migration error: {0}")]
    Migration(#[from] MigrationError),
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

/// Opens a RocksDB database without migrations.
///
/// Use this for stores that don't need migration support.
///
/// # Schema Version Behavior
///
/// This function creates the `__schema_version__` column family and writes version 0
/// for fresh databases. Since migrations must start at version 1, this ensures:
///
/// - The database format is consistent with migration-enabled databases
/// - If you later switch to [`open_db_with_migrations`], all migrations will run
///
/// # Downgrade Protection
///
/// If the database was previously opened with [`open_db_with_migrations`] and has
/// a schema version > 0, this function returns [`MigrationError::CannotDowngrade`].
/// This prevents accidentally opening a migrated database without migration support,
/// which could lead to data corruption or undefined behavior.
///
/// # Arguments
///
/// * `opts` - RocksDB options for opening the database
/// * `path` - Path to the database directory
/// * `current_schema_cfs` - The column families defined in the current schema
pub fn open_db(
    opts: &Options,
    path: &str,
    current_schema_cfs: &[&str],
) -> Result<DB, DatabaseError> {
    // Validate that schema version CF is not in current schema
    if current_schema_cfs.contains(&SCHEMA_VERSION_CF) {
        return Err(MigrationError::InvalidColumnFamilyConfig(format!(
            "Column family '{}' is reserved for internal use by the migration system",
            SCHEMA_VERSION_CF
        ))
        .into());
    }

    let db = open_db_internal(opts, path, current_schema_cfs)?;

    // Only initialize schema version for fresh databases.
    // If a version > 0 exists, the database was previously opened with migrations,
    // and using open_db() (without migrations) would be a downgrade.
    match MigrationRunner::read_schema_version(&db)? {
        None => {
            // Fresh DB - initialize schema version to 0
            MigrationRunner::write_schema_version(&db, 0)?;
        }
        Some(0) => {
            // Already at version 0 - this is expected for open_db()
        }
        Some(current_version) => {
            // DB was previously opened with migrations - open_db() is not appropriate
            return Err(MigrationError::CannotDowngrade {
                current: current_version,
                target: 0,
            }
            .into());
        }
    }

    Ok(db)
}

/// Opens a RocksDB database with automatic migration support.
///
/// This function:
/// 1. Validates that `current_schema_cfs` doesn't include reserved or deprecated CF names
/// 2. Checks if the database exists (via CURRENT file presence)
/// 3. Lists existing column families in the database (or uses empty list for new DBs)
/// 4. Merges existing CFs with the current schema's CFs
/// 5. Opens the database with all necessary CFs
/// 6. Runs any pending migrations to drop deprecated CFs
///
/// # Arguments
///
/// * `opts` - RocksDB options for opening the database
/// * `path` - Path to the database directory
/// * `current_schema_cfs` - The column families defined in the current schema (without deprecated CFs)
/// * `migrations` - The store-specific migrations
pub fn open_db_with_migrations(
    opts: &Options,
    path: &str,
    current_schema_cfs: &[&str],
    migrations: impl Into<MigrationRunner>,
) -> Result<DB, MigrationError> {
    let runner = migrations.into();

    // Validate current schema CFs before proceeding
    validate_current_schema_cfs(current_schema_cfs, &runner)?;

    // Open the database
    let mut db = open_db_internal(opts, path, current_schema_cfs)?;

    // Run pending migrations
    runner.run_pending(&mut db)?;

    Ok(db)
}

/// Validates that the current schema column families do not include any reserved or deprecated names.
///
/// This function checks that:
/// 1. `current_schema_cfs` does not contain the reserved `__schema_version__` CF
/// 2. `current_schema_cfs` does not contain any deprecated CF names from the provided migrations
///
/// # Errors
///
/// Returns `MigrationError::InvalidColumnFamilyConfig` if validation fails.
fn validate_current_schema_cfs(
    current_schema_cfs: &[&str],
    runner: &MigrationRunner,
) -> Result<(), MigrationError> {
    // Check for reserved schema version CF
    if current_schema_cfs.contains(&SCHEMA_VERSION_CF) {
        return Err(MigrationError::InvalidColumnFamilyConfig(format!(
            "Column family '{}' is reserved for internal use by the migration system and must not be included in current_schema_cfs",
            SCHEMA_VERSION_CF
        )));
    }

    // Check for deprecated CF names (permanently reserved and cannot be reused)
    let all_deprecated = runner.all_deprecated_column_families();
    for cf_name in current_schema_cfs {
        if all_deprecated.contains(cf_name) {
            return Err(MigrationError::InvalidColumnFamilyConfig(format!(
                "Column family '{}' was deprecated in a previous migration and cannot be reused. \
                 Deprecated CF names are permanently reserved to prevent data confusion. \
                 Use a different name (e.g., '{}_v2') for new functionality.",
                cf_name, cf_name
            )));
        }
    }

    Ok(())
}

/// Checks if a directory contains RocksDB artifacts (SST files, MANIFEST, etc.)
///
/// This is used to detect potentially corrupted databases where the directory
/// contains RocksDB files but is missing the CURRENT file.
fn has_rocksdb_artifacts(path: &Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Check for common RocksDB file patterns
            if name_str.ends_with(".sst")
                || name_str.ends_with(".log")
                || name_str.starts_with("MANIFEST")
                || name_str.starts_with("OPTIONS")
                || name_str == "IDENTITY"
            {
                return true;
            }
        }
    }
    false
}

/// Internal function that handles the core database opening logic.
///
/// This is shared by both `open_db` and `open_db_with_migrations`.
fn open_db_internal(
    opts: &Options,
    path: &str,
    current_schema_cfs: &[&str],
) -> Result<DB, MigrationError> {
    // Check if this is an existing database by looking for the CURRENT file.
    // RocksDB always creates a CURRENT file that points to the current manifest.
    // If CURRENT exists, this is an existing database and list_cf must succeed.
    // If CURRENT doesn't exist, check for corruption (artifacts without CURRENT).
    let db_path = Path::new(path);
    let current_file = db_path.join("CURRENT");

    let existing_cfs = if current_file.exists() {
        // CURRENT file exists - this is an existing database
        // Any error from list_cf is a real error (corruption, permissions, etc.)
        debug!(target: LOG_TARGET, "CURRENT file found, listing existing column families");
        DB::list_cf(opts, path)?
    } else if db_path.exists() && has_rocksdb_artifacts(db_path) {
        // Directory exists with RocksDB artifacts but no CURRENT file - likely corruption
        return Err(MigrationError::InvalidColumnFamilyConfig(format!(
            "Database directory '{}' contains RocksDB artifacts but no CURRENT file. \
             This may indicate corruption or incomplete initialization. \
             Remove the directory contents to start fresh, or restore from backup.",
            path
        )));
    } else {
        // No CURRENT file and no artifacts - this is a new database
        debug!(target: LOG_TARGET, "No CURRENT file found, treating as new database");
        vec![]
    };

    // Merge existing CFs with current schema CFs
    let all_cfs = merge_column_families(&existing_cfs, current_schema_cfs);

    debug!(target: LOG_TARGET, "Opening database with column families: {:?}", all_cfs);

    // Create column family descriptors
    let cf_descriptors: Vec<ColumnFamilyDescriptor> = all_cfs
        .iter()
        .map(|name| ColumnFamilyDescriptor::new(name.clone(), Options::default()))
        .collect();

    // Open the database with all column families
    let db = DB::open_cf_descriptors(opts, path, cf_descriptors)?;

    Ok(db)
}

/// A helper function to create DB options with common settings for migration-compatible databases.
pub fn default_db_options() -> Options {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);
    opts
}
