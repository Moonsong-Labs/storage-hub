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
//! ### Defining Store-Specific Migrations
//!
//! Each store defines its own migrations. When deprecating column families in a new release:
//!
//! 1. Create a migrations module in your store's crate (e.g., `migrations.rs`)
//! 2. Implement the [`Migration`] trait for your migration struct
//! 3. Create a function that returns a vector of migrations for your store
//!
//! ```ignore
//! // my_store/src/migrations.rs
//! use shc_common::migrations::Migration;
//!
//! pub struct MyStoreV1Migration;
//!
//! impl Migration for MyStoreV1Migration {
//!     fn version(&self) -> u32 { 1 }
//!
//!     fn deprecated_column_families(&self) -> &'static [&'static str] {
//!         &["old_cf_name_1", "old_cf_name_2"]
//!     }
//!
//!     fn description(&self) -> &'static str {
//!         "Remove legacy storage request column families"
//!     }
//! }
//!
//! pub fn my_store_migrations() -> Vec<Box<dyn Migration>> {
//!     vec![Box::new(MyStoreV1Migration)]
//! }
//! ```
//!
//! ### Opening a Database with Migrations
//!
//! ```ignore
//! use shc_common::typed_store::TypedRocksDB;
//!
//! // Define your current column families (without deprecated ones)
//! const CURRENT_CFS: &[&str] = &["cf1", "cf2", "cf3"];
//!
//! // With migrations
//! let db = TypedRocksDB::open_with_migrations(&path, &CURRENT_CFS, my_store_migrations())?;
//!
//! // Without migrations
//! let db = TypedRocksDB::open(&path, &CURRENT_CFS)?;
//! ```
use log::{debug, info};
use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use std::collections::HashSet;
use std::path::Path;
use thiserror::Error;

#[cfg(test)]
mod tests;

/// The name of the column family used to store the schema version.
/// This is a reserved name and should not be used for application data.
pub const SCHEMA_VERSION_CF: &str = "__schema_version__";

/// The key used to store the current schema version within the schema version CF.
const SCHEMA_VERSION_KEY: &[u8] = b"version";

/// Errors that can occur during migration operations.
#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("RocksDB error: {0}")]
    RocksDb(#[from] rocksdb::Error),

    #[error("Failed to parse schema version: {0}")]
    VersionParse(String),

    #[error("Migration {version} failed: {reason}")]
    MigrationFailed { version: u32, reason: String },

    #[error("Cannot downgrade schema version from {current} to {target}")]
    CannotDowngrade { current: u32, target: u32 },

    #[error("Invalid column family configuration: {0}")]
    InvalidColumnFamilyConfig(String),
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
    fn version(&self) -> u32;

    /// Returns the names of column families that should be dropped by this migration.
    ///
    /// These column families will be removed from the database when this migration
    /// is applied. The migration system will:
    /// 1. First open the database with these CFs (discovered via `list_cf`)
    /// 2. Then drop them using `drop_cf()`
    ///
    /// # Important: Deprecated names are permanently reserved
    ///
    /// Once a column family name is listed here, it can **never** be reused in a future
    /// schema version. The migration system enforces this by:
    /// - Rejecting any `current_schema_cfs` that include a deprecated CF name
    /// - Always dropping deprecated CFs during the cleanup pass (even if schema version is current)
    ///
    /// This constraint exists for safety:
    /// - **Prevents data confusion**: Old data could be misinterpreted if the name is reused
    /// - **Ensures clean separation**: No ambiguity about whether data is old or new
    ///
    /// If you need similar functionality in the future, use a new name (e.g., `my_cf_v2`).
    fn deprecated_column_families(&self) -> &'static [&'static str];

    /// A human-readable description of what this migration does.
    fn description(&self) -> &'static str;
}

/// The migration runner handles discovering existing column families,
/// opening the database with all necessary CFs, and running pending migrations.
///
/// Migrations are automatically sorted by version.
///
/// ```ignore
/// let runner = MigrationRunner::from(my_store_migrations());
/// ```
pub struct MigrationRunner {
    migrations: Vec<Box<dyn Migration>>,
}

/// Create a MigrationRunner from a vector of migrations.
impl From<Vec<Box<dyn Migration>>> for MigrationRunner {
    fn from(mut migrations: Vec<Box<dyn Migration>>) -> Self {
        // Sort migrations by version to ensure correct execution order
        migrations.sort_by_key(|m| m.version());
        Self { migrations }
    }
}

impl MigrationRunner {
    /// Create a new MigrationRunner with the given migrations.
    ///
    /// Migrations are automatically sorted by version to ensure correct execution order.
    pub fn new(mut migrations: Vec<Box<dyn Migration>>) -> Self {
        // Sort migrations by version to ensure correct execution order
        migrations.sort_by_key(|m| m.version());
        Self { migrations }
    }

    /// Get the latest migration version.
    pub fn latest_version(&self) -> u32 {
        self.migrations.last().map(|m| m.version()).unwrap_or(0)
    }

    /// Returns a set of all deprecated column family names across all migrations.
    ///
    /// This aggregates deprecated CF names from **all** migrations, regardless of version.
    /// Once a name appears in any migration's `deprecated_column_families()`, it is
    /// **permanently reserved** and cannot be reused in future schema versions.
    ///
    /// This is used to:
    /// 1. Validate that `current_schema_cfs` don't accidentally include deprecated CF names
    /// 2. Ensure deprecated CFs are always cleaned up, even if schema version is already latest
    ///
    /// See [`Migration::deprecated_column_families()`] for more details on why deprecated
    /// names can never be reused.
    pub fn all_deprecated_column_families(&self) -> HashSet<&'static str> {
        self.migrations
            .iter()
            .flat_map(|m| m.deprecated_column_families().iter().copied())
            .collect()
    }

    /// Validates the migration order and returns any issues found.
    ///
    /// This function checks for:
    /// 1. Duplicate version numbers
    /// 2. Version numbers starting from 1 (if migrations are not empty)
    /// 3. No gaps in version sequence
    ///
    /// Returns `Ok(())` if all validations pass, or an error describing the issue.
    pub fn validate_order(&self) -> Result<(), MigrationError> {
        if self.migrations.is_empty() {
            return Ok(());
        }

        // Check for duplicates
        let mut seen_versions = HashSet::new();
        for migration in &self.migrations {
            if !seen_versions.insert(migration.version()) {
                return Err(MigrationError::MigrationFailed {
                    version: migration.version(),
                    reason: format!("Duplicate migration version: {}", migration.version()),
                });
            }
        }

        // Check that versions start from 1
        if self.migrations.first().map(|m| m.version()) != Some(1) {
            return Err(MigrationError::MigrationFailed {
                version: 0,
                reason: "Migrations must start from version 1".to_string(),
            });
        }

        // Check for gaps in version sequence
        for (i, migration) in self.migrations.iter().enumerate() {
            let expected_version = (i + 1) as u32;
            if migration.version() != expected_version {
                return Err(MigrationError::MigrationFailed {
                    version: migration.version(),
                    reason: format!(
                        "Gap in migration sequence: expected version {}, found {}",
                        expected_version,
                        migration.version()
                    ),
                });
            }
        }

        Ok(())
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
    /// 1. Validate the migration configuration (no duplicates, no gaps, starts from 1)
    /// 2. Read the current schema version
    /// 3. Check for downgrade attempts (current version > latest known version)
    /// 4. Cleanup pass: drop any straggler deprecated CFs from already-applied migrations
    ///    (handles partial migration failures, manual tampering, etc.)
    /// 5. For each pending migration (version > current):
    ///    - Drop that migration's deprecated column families
    ///    - Update the schema version
    ///
    /// # Errors
    ///
    /// Returns `MigrationError::MigrationFailed` if the migration configuration is invalid
    /// (e.g., duplicate versions, gaps in sequence, or not starting from version 1).
    ///
    /// Returns `MigrationError::CannotDowngrade` if the database was created with a newer
    /// version of the software than the current code supports. This prevents data corruption
    /// from running older code against a newer database schema.
    pub fn run_pending(&self, db: &mut DB) -> Result<u32, MigrationError> {
        // Validate migration configuration before running
        self.validate_order()?;

        let current_version = Self::read_schema_version(db)?;
        let latest_version = self.latest_version();

        // Check for downgrade attempt
        if current_version > latest_version {
            return Err(MigrationError::CannotDowngrade {
                current: current_version,
                target: latest_version,
            });
        }

        // Drop straggler deprecated CFs from already-applied migrations.
        //
        // Note:
        //
        // RocksDB does not support batching multiple `drop_cf()` calls into a single
        // atomic transaction. Each `DropColumnFamily` is an independent operation that
        // immediately records a drop record in the MANIFEST file. There is no API to
        // roll back a drop or to commit multiple drops atomically.
        //
        // This means partial migrations are possible (e.g., crash after dropping CF1
        // but before dropping CF2). We handle this by making the cleanup **idempotent**:
        // - We only clean up CFs from migrations that have already been applied (version <= current)
        // - We check `cf_handle().is_some()` before each drop (already-dropped CFs are skipped)
        // - This runs on every startup to catch stragglers from crashes or tampering
        //
        // This guards against edge cases like:
        // - Partial migration failures (crash mid-migration)
        // - Manual DB tampering that recreated deprecated CFs
        // - Schema version already at latest but deprecated CFs still present
        //
        // See: https://github.com/facebook/rocksdb/wiki/column-families
        for migration in self
            .migrations
            .iter()
            .filter(|m| m.version() <= current_version)
        {
            for cf_name in migration.deprecated_column_families() {
                if db.cf_handle(cf_name).is_some() {
                    info!(
                        "Cleanup pass (v{}): dropping straggler column family '{}'",
                        migration.version(),
                        cf_name
                    );
                    db.drop_cf(cf_name)
                        .map_err(|e| MigrationError::MigrationFailed {
                            version: migration.version(),
                            reason: format!(
                                "Failed to drop straggler column family '{}': {}",
                                cf_name, e
                            ),
                        })?;
                }
            }
        }

        // Apply migrations that haven't been applied yet
        let pending: Vec<_> = self
            .migrations
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
            pending
                .last()
                .map(|m| m.version())
                .unwrap_or(current_version)
        );

        let mut applied_version = current_version;

        for migration in pending {
            info!(
                "Applying migration v{}: {}",
                migration.version(),
                migration.description()
            );

            // Drop this migration's deprecated column families
            for cf_name in migration.deprecated_column_families() {
                if db.cf_handle(cf_name).is_some() {
                    info!("  Dropping column family: {}", cf_name);
                    db.drop_cf(cf_name)
                        .map_err(|e| MigrationError::MigrationFailed {
                            version: migration.version(),
                            reason: format!("Failed to drop column family '{}': {}", cf_name, e),
                        })?;
                } else {
                    debug!("  Column family '{}' does not exist, skipping", cf_name);
                }
            }

            // Update schema version after this migration completes
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

/// Opens a RocksDB database without migrations.
///
/// Use this for stores that don't need migration support.
///
/// # Schema Version Behavior
///
/// This function creates the `__schema_version__` column family and writes version 0.
/// Since migrations must start at version 1, this ensures:
///
/// - The database format is consistent with migration-enabled databases
/// - If you later switch to [`open_db_with_migrations`], all migrations will run
///
/// # Arguments
///
/// * `opts` - RocksDB options for opening the database
/// * `path` - Path to the database directory
/// * `current_schema_cfs` - The column families defined in the current schema
///
/// # Returns
///
/// The opened database.
pub fn open_db(
    opts: &Options,
    path: &str,
    current_schema_cfs: &[&str],
) -> Result<DB, MigrationError> {
    // Validate that schema version CF is not in current schema
    if current_schema_cfs.contains(&SCHEMA_VERSION_CF) {
        return Err(MigrationError::InvalidColumnFamilyConfig(format!(
            "Column family '{}' is reserved for internal use by the migration system",
            SCHEMA_VERSION_CF
        )));
    }

    let db = open_db_internal(opts, path, current_schema_cfs)?;

    // Write version 0 for consistency (migrations must start at version 1)
    MigrationRunner::write_schema_version(&db, 0)?;

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
///
/// # Returns
///
/// The opened database after running all pending migrations.
///
/// # Errors
///
/// Returns `MigrationError::InvalidColumnFamilyConfig` if `current_schema_cfs` contains
/// reserved or deprecated CF names.
///
/// Returns `MigrationError::RocksDb` if the database exists but cannot be read
/// (e.g., corruption, permission issues, etc.).
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
    // If CURRENT doesn't exist, this is a new database.
    let db_path = Path::new(path);
    let current_file = db_path.join("CURRENT");

    let existing_cfs = if current_file.exists() {
        // CURRENT file exists - this is an existing database
        // Any error from list_cf is a real error (corruption, permissions, etc.)
        debug!("CURRENT file found, listing existing column families");
        DB::list_cf(opts, path)?
    } else {
        // No CURRENT file - this is a new database
        debug!("No CURRENT file found, treating as new database");
        vec![]
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
