//! Tests for the migration framework.
//!
//! These tests verify the migration system including:
//! - Column family merging logic
//! - Migration runner functionality
//! - Error handling and validation
//! - Database operations with migrations
//! - Multi-version migration chains
//! - Downgrade prevention
//! - Store-specific migration scenarios

use super::*;
use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use tempfile::TempDir;

/// Tests for the `merge_column_families` function.
mod merge_column_families_tests {
    use super::*;

    #[test]
    fn empty_existing_cfs() {
        let existing: Vec<String> = vec![];
        let current: Vec<&str> = vec!["cf1", "cf2"];

        let merged = merge_column_families(&existing, &current);

        assert!(merged.contains("cf1"));
        assert!(merged.contains("cf2"));
        assert!(merged.contains(SCHEMA_VERSION_CF));
    }

    #[test]
    fn with_existing_cfs() {
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
}

/// Tests for `MigrationRunner` instance methods.
mod migration_runner_tests {
    use super::*;

    #[test]
    fn latest_version_with_empty_migrations_is_zero() {
        let runner = MigrationRunner::new(vec![]);
        let latest = runner.latest_version();
        assert_eq!(latest, 0, "Empty migrations should have version 0");
    }

    #[test]
    fn latest_version_returns_last_migration_version() {
        let migrations = test_migrations::all_test_migrations();
        let runner = MigrationRunner::from(migrations);
        let latest = runner.latest_version();
        assert_eq!(latest, 3, "Should return version of last migration");
    }

    #[test]
    fn validate_order_passes_for_valid_migrations() {
        let migrations = test_migrations::all_test_migrations();
        let runner = MigrationRunner::from(migrations);
        let result = runner.validate_order();
        assert!(result.is_ok(), "Valid migrations should pass validation");
    }

    #[test]
    fn validate_empty_migrations_passes() {
        let runner = MigrationRunner::new(vec![]);
        let result = runner.validate_order();
        assert!(result.is_ok(), "Empty migrations should pass validation");
    }

    #[test]
    fn from_vec_sorts_migrations_by_version() {
        // Create migrations in reverse order
        let migrations: Vec<Box<dyn Migration>> = vec![
            Box::new(test_migrations::TestV3Migration),
            Box::new(test_migrations::TestV1Migration),
            Box::new(test_migrations::TestV2Migration),
        ];

        let runner = MigrationRunner::from(migrations);

        assert!(
            runner.validate_order().is_ok(),
            "validate_order passing proves migrations were sorted"
        );
        assert_eq!(runner.latest_version(), 3);
    }
}

/// Tests for migration validation logic.
///
/// These tests verify that `MigrationRunner::validate_order()` correctly
/// detects invalid migration configurations.
mod validation_tests {
    use super::*;

    #[test]
    fn detects_duplicate_versions() {
        struct DuplicateV1A;
        impl Migration for DuplicateV1A {
            fn version(&self) -> u32 {
                1
            }
            fn deprecated_column_families(&self) -> &'static [&'static str] {
                &[]
            }
            fn description(&self) -> &'static str {
                "First V1"
            }
        }

        struct DuplicateV1B;
        impl Migration for DuplicateV1B {
            fn version(&self) -> u32 {
                1
            }
            fn deprecated_column_families(&self) -> &'static [&'static str] {
                &[]
            }
            fn description(&self) -> &'static str {
                "Second V1 (duplicate)"
            }
        }

        let migrations: Vec<Box<dyn Migration>> =
            vec![Box::new(DuplicateV1A), Box::new(DuplicateV1B)];
        let runner = MigrationRunner::from(migrations);

        let result = runner.validate_order();
        assert!(result.is_err());
        match result {
            Err(MigrationError::MigrationFailed { version, reason }) => {
                assert_eq!(version, 1);
                assert!(reason.contains("Duplicate"));
            }
            _ => panic!("Expected MigrationFailed error"),
        }
    }

    #[test]
    fn detects_version_gaps() {
        struct GapV1;
        impl Migration for GapV1 {
            fn version(&self) -> u32 {
                1
            }
            fn deprecated_column_families(&self) -> &'static [&'static str] {
                &[]
            }
            fn description(&self) -> &'static str {
                "V1"
            }
        }

        struct GapV3;
        impl Migration for GapV3 {
            fn version(&self) -> u32 {
                3
            }
            fn deprecated_column_families(&self) -> &'static [&'static str] {
                &[]
            }
            fn description(&self) -> &'static str {
                "V3 (skipping V2)"
            }
        }

        // MigrationRunner::from auto-sorts by version
        let migrations: Vec<Box<dyn Migration>> = vec![Box::new(GapV1), Box::new(GapV3)];
        let runner = MigrationRunner::from(migrations);

        let result = runner.validate_order();
        assert!(result.is_err());
        match result {
            Err(MigrationError::MigrationFailed { reason, .. }) => {
                assert!(reason.contains("Gap"));
            }
            _ => panic!("Expected MigrationFailed error"),
        }
    }

    #[test]
    fn detects_non_one_start() {
        struct NonOneStartV2;
        impl Migration for NonOneStartV2 {
            fn version(&self) -> u32 {
                2
            }
            fn deprecated_column_families(&self) -> &'static [&'static str] {
                &[]
            }
            fn description(&self) -> &'static str {
                "V2 (no V1)"
            }
        }

        let migrations: Vec<Box<dyn Migration>> = vec![Box::new(NonOneStartV2)];
        let runner = MigrationRunner::from(migrations);

        let result = runner.validate_order();
        assert!(result.is_err());
        match result {
            Err(MigrationError::MigrationFailed { reason, .. }) => {
                assert!(reason.contains("start from version 1"));
            }
            _ => panic!("Expected MigrationFailed error"),
        }
    }
}

/// Tests for `MigrationError` formatting.
mod error_tests {
    use super::*;

    #[test]
    fn error_types_format_correctly() {
        let version_parse_err = MigrationError::VersionParse("invalid bytes".to_string());
        assert!(
            format!("{}", version_parse_err).contains("version"),
            "VersionParse error should format correctly"
        );

        let migration_failed_err = MigrationError::MigrationFailed {
            version: 1,
            reason: "test failure".to_string(),
        };
        assert!(
            format!("{}", migration_failed_err).contains("Migration 1 failed"),
            "MigrationFailed error should format correctly"
        );

        let downgrade_err = MigrationError::CannotDowngrade {
            current: 5,
            target: 3,
        };
        assert!(
            format!("{}", downgrade_err).contains("5")
                && format!("{}", downgrade_err).contains("3"),
            "CannotDowngrade error should contain version numbers"
        );
    }

    #[test]
    fn invalid_schema_version_bytes_handled() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // Create a database with corrupted schema version
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let cf_descriptors = vec![
                ColumnFamilyDescriptor::new("default", Options::default()),
                ColumnFamilyDescriptor::new(SCHEMA_VERSION_CF, Options::default()),
            ];

            let db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();

            // Write invalid bytes to schema version (not 4 bytes)
            let cf = db.cf_handle(SCHEMA_VERSION_CF).unwrap();
            db.put_cf(&cf, SCHEMA_VERSION_KEY, &[1, 2]).unwrap();
        }

        // Try to read schema version
        {
            let opts = default_db_options();
            let existing_cfs = DB::list_cf(&opts, path).unwrap();
            let cf_descriptors: Vec<ColumnFamilyDescriptor> = existing_cfs
                .iter()
                .map(|name| ColumnFamilyDescriptor::new(name.clone(), Options::default()))
                .collect();

            let db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();

            let result = MigrationRunner::read_schema_version(&db);
            assert!(result.is_err());
            match result {
                Err(MigrationError::VersionParse(msg)) => {
                    assert!(msg.contains("Invalid version bytes length"));
                }
                _ => panic!("Expected VersionParse error, got {:?}", result),
            }
        }
    }
}

/// Tests for basic database operations with migrations.
mod database_tests {
    use super::*;

    #[test]
    fn open_fresh_database_with_no_migrations() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let opts = default_db_options();
        let current_cfs = vec!["test_cf"];
        let db = open_db(&opts, path, &current_cfs).unwrap();

        assert!(db.cf_handle("test_cf").is_some());
        assert!(db.cf_handle(SCHEMA_VERSION_CF).is_some());

        let version = MigrationRunner::read_schema_version(&db).unwrap();
        assert_eq!(version, Some(0)); // No migrations, version stays at 0
    }

    #[test]
    fn open_fresh_database_with_migrations() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let opts = default_db_options();
        let current_cfs = vec!["test_cf"];
        let migrations = test_migrations::v1_migrations();

        let db = open_db_with_migrations(&opts, path, &current_cfs, migrations).unwrap();

        assert!(db.cf_handle("test_cf").is_some());
        assert!(db.cf_handle(SCHEMA_VERSION_CF).is_some());

        let version = MigrationRunner::read_schema_version(&db).unwrap();
        assert_eq!(version, Some(1)); // V1 migration applied
    }

    #[test]
    fn list_cf_error_for_existing_db_is_propagated() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Simulate a directory that *looks* like an existing RocksDB (has a CURRENT file),
        // but is not a valid DB. `open_db_with_migrations` should not treat this as a
        // brand-new database.
        std::fs::write(path.join("CURRENT"), b"not_a_manifest\n").unwrap();

        let opts = default_db_options();
        let result = open_db(&opts, path.to_str().unwrap(), &["test_cf"]);

        assert!(matches!(
            result,
            Err(DatabaseError::Migration(MigrationError::RocksDb(_)))
        ));
    }

    #[test]
    fn migrations_are_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let opts = default_db_options();
        let current_cfs = vec!["test_cf"];
        let migrations = test_migrations::v1_migrations();

        // Open and run migrations first time
        {
            let db = open_db_with_migrations(&opts, path, &current_cfs, migrations).unwrap();
            let version = MigrationRunner::read_schema_version(&db).unwrap();
            assert_eq!(version, Some(1));
        }

        // Open again - migrations should not run again
        {
            let migrations = test_migrations::v1_migrations();
            let db = open_db_with_migrations(&opts, path, &current_cfs, migrations).unwrap();
            let version = MigrationRunner::read_schema_version(&db).unwrap();
            assert_eq!(version, Some(1));
        }
    }

    #[test]
    fn migration_drops_deprecated_cfs() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // Create a database with deprecated column families
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let old_cfs = vec![
                ColumnFamilyDescriptor::new("default", Options::default()),
                ColumnFamilyDescriptor::new(test_migrations::V1_CF_A, Options::default()),
                ColumnFamilyDescriptor::new(test_migrations::V1_CF_B, Options::default()),
                ColumnFamilyDescriptor::new("current_cf", Options::default()),
            ];

            let _db = DB::open_cf_descriptors(&opts, path, old_cfs).unwrap();
        }

        // Open with migrations
        let opts = default_db_options();
        let current_cfs = vec!["current_cf"];
        let migrations = test_migrations::v1_migrations();
        let db = open_db_with_migrations(&opts, path, &current_cfs, migrations).unwrap();

        assert!(db.cf_handle("current_cf").is_some());
        assert!(db.cf_handle(test_migrations::V1_CF_A).is_none());
        assert!(db.cf_handle(test_migrations::V1_CF_B).is_none());
    }

    #[test]
    fn migration_preserves_data_in_active_cfs() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let test_key = b"test_key";
        let test_value = b"important_data_that_must_survive";

        // Create database with data
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let cfs = vec![
                ColumnFamilyDescriptor::new("default", Options::default()),
                ColumnFamilyDescriptor::new("active_cf", Options::default()),
                ColumnFamilyDescriptor::new(test_migrations::V1_CF_A, Options::default()),
            ];

            let db = DB::open_cf_descriptors(&opts, path, cfs).unwrap();
            let active_cf = db.cf_handle("active_cf").unwrap();
            db.put_cf(&active_cf, test_key, test_value).unwrap();
        }

        // Open with migrations
        let opts = default_db_options();
        let current_cfs = vec!["active_cf"];
        let migrations = test_migrations::v1_migrations();
        let db = open_db_with_migrations(&opts, path, &current_cfs, migrations).unwrap();

        let active_cf = db.cf_handle("active_cf").unwrap();
        let read_value = db.get_cf(&active_cf, test_key).unwrap();
        assert!(read_value.is_some());
        assert_eq!(&read_value.unwrap()[..], test_value);
    }

    #[test]
    fn migration_with_no_deprecated_cfs_present() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // Create a clean database without deprecated CFs
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

        let opts = default_db_options();
        let current_cfs = vec!["clean_cf"];
        let migrations = test_migrations::v1_migrations();
        let db = open_db_with_migrations(&opts, path, &current_cfs, migrations).unwrap();

        let cf = db.cf_handle("clean_cf").unwrap();
        assert_eq!(&db.get_cf(&cf, b"key").unwrap().unwrap()[..], b"value");
        assert_eq!(MigrationRunner::read_schema_version(&db).unwrap(), Some(1));
    }

    #[test]
    fn database_works_normally_after_migration() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // Create old database
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let cfs = vec![
                ColumnFamilyDescriptor::new("default", Options::default()),
                ColumnFamilyDescriptor::new("my_cf", Options::default()),
                ColumnFamilyDescriptor::new(test_migrations::V1_CF_A, Options::default()),
            ];

            let _db = DB::open_cf_descriptors(&opts, path, cfs).unwrap();
        }

        let opts = default_db_options();
        let current_cfs = vec!["my_cf"];
        let migrations = test_migrations::v1_migrations();
        let db = open_db_with_migrations(&opts, path, &current_cfs, migrations).unwrap();

        let cf = db.cf_handle("my_cf").unwrap();

        // Write, read, delete operations
        db.put_cf(&cf, b"key1", b"value1").unwrap();
        db.put_cf(&cf, b"key2", b"value2").unwrap();
        assert_eq!(&db.get_cf(&cf, b"key1").unwrap().unwrap()[..], b"value1");

        db.delete_cf(&cf, b"key1").unwrap();
        assert!(db.get_cf(&cf, b"key1").unwrap().is_none());

        // Verify iteration
        let iter = db.iterator_cf(&cf, rocksdb::IteratorMode::Start);
        assert_eq!(iter.count(), 1);

        // Close and reopen
        drop(db);
        let migrations = test_migrations::v1_migrations();
        let db = open_db_with_migrations(&opts, path, &current_cfs, migrations).unwrap();
        let cf = db.cf_handle("my_cf").unwrap();
        assert_eq!(&db.get_cf(&cf, b"key2").unwrap().unwrap()[..], b"value2");
    }

    #[test]
    fn schema_version_cf_created_automatically() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let opts = default_db_options();
        let migrations = test_migrations::v1_migrations();
        let db = open_db_with_migrations(&opts, path, &["my_cf"], migrations).unwrap();

        assert!(db.cf_handle(SCHEMA_VERSION_CF).is_some());
        assert_eq!(MigrationRunner::read_schema_version(&db).unwrap(), Some(1));
    }

    #[test]
    fn empty_current_cfs_works() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let opts = default_db_options();
        let db = open_db(&opts, path, &[]).unwrap();

        assert!(db.cf_handle("default").is_some());
        assert!(db.cf_handle(SCHEMA_VERSION_CF).is_some());
    }

    #[test]
    fn migration_runner_handles_empty_migrations() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf_descriptors = vec![
            ColumnFamilyDescriptor::new("default", Options::default()),
            ColumnFamilyDescriptor::new(SCHEMA_VERSION_CF, Options::default()),
        ];

        let mut db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();

        // Run with empty migrations - should succeed and return 0
        let runner = MigrationRunner::new(vec![]);
        let result = runner.run_pending(&mut db);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn open_db_rejects_database_with_higher_schema_version() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // First, create a database with migrations (schema version 1)
        {
            let opts = default_db_options();
            let current_cfs = vec!["test_cf"];
            let migrations = test_migrations::v1_migrations();
            let db = open_db_with_migrations(&opts, path, &current_cfs, migrations).unwrap();
            assert_eq!(MigrationRunner::read_schema_version(&db).unwrap(), Some(1));
        }

        // Now try to open with open_db() (no migrations) - should fail with CannotDowngrade
        {
            let opts = default_db_options();
            let result = open_db(&opts, path, &["test_cf"]);

            assert!(result.is_err());
            match result {
                Err(DatabaseError::Migration(MigrationError::CannotDowngrade {
                    current,
                    target,
                })) => {
                    assert_eq!(current, 1);
                    assert_eq!(target, 0);
                }
                _ => panic!("Expected CannotDowngrade error, got {:?}", result),
            }
        }
    }

    #[test]
    fn open_db_succeeds_for_fresh_database() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // Fresh database should work fine
        let opts = default_db_options();
        let db = open_db(&opts, path, &["test_cf"]).unwrap();

        assert_eq!(MigrationRunner::read_schema_version(&db).unwrap(), Some(0));
        assert!(db.cf_handle("test_cf").is_some());
    }

    #[test]
    fn open_db_succeeds_for_existing_version_0_database() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // Create a database with open_db (version 0)
        {
            let opts = default_db_options();
            let _db = open_db(&opts, path, &["test_cf"]).unwrap();
        }

        // Opening again with open_db should succeed
        {
            let opts = default_db_options();
            let db = open_db(&opts, path, &["test_cf"]).unwrap();
            assert_eq!(MigrationRunner::read_schema_version(&db).unwrap(), Some(0));
        }
    }

    #[test]
    fn detects_corrupted_directory_with_artifacts_but_no_current() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        let path_str = path.to_str().unwrap();

        // Create RocksDB artifacts without a CURRENT file (simulating corruption)
        std::fs::write(path.join("000001.sst"), b"fake sst data").unwrap();
        std::fs::write(path.join("MANIFEST-000001"), b"fake manifest").unwrap();

        // Try to open - should fail with InvalidColumnFamilyConfig
        let opts = default_db_options();
        let result = open_db(&opts, path_str, &["test_cf"]);

        assert!(result.is_err());
        match result {
            Err(DatabaseError::Migration(MigrationError::InvalidColumnFamilyConfig(msg))) => {
                assert!(msg.contains("RocksDB artifacts"));
                assert!(msg.contains("no CURRENT file"));
            }
            _ => panic!("Expected InvalidColumnFamilyConfig error, got {:?}", result),
        }
    }

    #[test]
    fn detects_corrupted_directory_with_log_files() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        let path_str = path.to_str().unwrap();

        // Create only a .log file (another RocksDB artifact)
        std::fs::write(path.join("000001.log"), b"fake wal log").unwrap();

        // Try to open - should fail
        let opts = default_db_options();
        let result = open_db(&opts, path_str, &["test_cf"]);

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(DatabaseError::Migration(
                MigrationError::InvalidColumnFamilyConfig(_)
            ))
        ));
    }

    #[test]
    fn allows_empty_directory_without_current_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        let path_str = path.to_str().unwrap();

        // Empty directory - should be treated as new database
        let opts = default_db_options();
        let db = open_db(&opts, path_str, &["test_cf"]).unwrap();

        assert!(db.cf_handle("test_cf").is_some());
        assert_eq!(MigrationRunner::read_schema_version(&db).unwrap(), Some(0));
    }

    #[test]
    fn allows_directory_with_non_rocksdb_files() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        let path_str = path.to_str().unwrap();

        // Create non-RocksDB files (these should be ignored)
        std::fs::write(path.join("readme.txt"), b"some readme").unwrap();
        std::fs::write(path.join("data.json"), b"{}").unwrap();
        std::fs::create_dir(path.join("subdir")).unwrap();

        // Should be treated as new database
        let opts = default_db_options();
        let db = open_db(&opts, path_str, &["test_cf"]).unwrap();

        assert!(db.cf_handle("test_cf").is_some());
    }
}

/// Test-only migrations for multi-version testing.
///
/// These migrations use distinct test-only CF names to avoid coupling with
/// production migrations. This ensures tests remain independent and don't
/// break if production migrations change.
mod test_migrations {
    use super::*;

    // V1 deprecated column family names
    pub const V1_CF_A: &str = "test_deprecated_v1_cf_a";
    pub const V1_CF_B: &str = "test_deprecated_v1_cf_b";
    pub const V1_CF_C: &str = "test_deprecated_v1_cf_c";

    // V2 deprecated column family names
    pub const V2_CF: &str = "test_deprecated_v2_cf";
    pub const V2_CF_INDEX: &str = "test_deprecated_v2_cf_index";

    // V3 deprecated column family names
    pub const V3_CF: &str = "test_deprecated_v3_cf";

    /// Test-only V1 migration for testing the migration framework.
    pub struct TestV1Migration;

    impl Migration for TestV1Migration {
        fn version(&self) -> u32 {
            1
        }

        fn deprecated_column_families(&self) -> &'static [&'static str] {
            &[V1_CF_A, V1_CF_B, V1_CF_C]
        }

        fn description(&self) -> &'static str {
            "Test V1 migration - removes test deprecated CFs"
        }
    }

    /// Test-only V2 migration for multi-version testing.
    pub struct TestV2Migration;

    impl Migration for TestV2Migration {
        fn version(&self) -> u32 {
            2
        }

        fn deprecated_column_families(&self) -> &'static [&'static str] {
            &[V2_CF, V2_CF_INDEX]
        }

        fn description(&self) -> &'static str {
            "Test V2 migration - removes test deprecated CFs"
        }
    }

    /// Test-only V3 migration for multi-version testing.
    pub struct TestV3Migration;

    impl Migration for TestV3Migration {
        fn version(&self) -> u32 {
            3
        }

        fn deprecated_column_families(&self) -> &'static [&'static str] {
            &[V3_CF]
        }

        fn description(&self) -> &'static str {
            "Test V3 migration - removes another test deprecated CF"
        }
    }

    /// Returns all test migrations for testing migration chains.
    pub fn all_test_migrations() -> Vec<Box<dyn Migration>> {
        vec![
            Box::new(TestV1Migration),
            Box::new(TestV2Migration),
            Box::new(TestV3Migration),
        ]
    }

    /// Returns just the V1 migration for single-migration tests.
    pub fn v1_migrations() -> Vec<Box<dyn Migration>> {
        vec![Box::new(TestV1Migration)]
    }
}

/// Tests for multi-version migration scenarios.
mod multi_version_tests {
    use super::test_migrations::*;
    use super::*;

    #[test]
    fn migration_chain_v0_to_v3() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let all_migrations = all_test_migrations();

        let all_deprecated_cfs: Vec<&str> = all_migrations
            .iter()
            .flat_map(|m| m.deprecated_column_families().iter().copied())
            .collect();

        // Create database at version 0 with all deprecated CFs
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let mut cf_descriptors: Vec<ColumnFamilyDescriptor> = vec![
                ColumnFamilyDescriptor::new("default", Options::default()),
                ColumnFamilyDescriptor::new("current_cf", Options::default()),
                ColumnFamilyDescriptor::new(SCHEMA_VERSION_CF, Options::default()),
            ];

            for cf_name in &all_deprecated_cfs {
                cf_descriptors.push(ColumnFamilyDescriptor::new(*cf_name, Options::default()));
            }

            let db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();

            let cf = db.cf_handle("current_cf").unwrap();
            db.put_cf(&cf, b"key", b"important_data").unwrap();

            for cf_name in &all_deprecated_cfs {
                if let Some(cf) = db.cf_handle(cf_name) {
                    db.put_cf(&cf, b"old_key", b"old_data").unwrap();
                }
            }

            // No version written yet - should be None
            assert_eq!(MigrationRunner::read_schema_version(&db).unwrap(), None);
        }

        // Run all migrations
        {
            let opts = default_db_options();
            let existing_cfs = DB::list_cf(&opts, path).unwrap();
            let cf_descriptors: Vec<ColumnFamilyDescriptor> = existing_cfs
                .iter()
                .map(|name| ColumnFamilyDescriptor::new(name.clone(), Options::default()))
                .collect();

            let mut db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();

            // Re-create the migrations list since we can't clone Box<dyn Migration>
            let all_migrations = all_test_migrations();
            let runner = MigrationRunner::from(all_migrations);
            let final_version = runner.run_pending(&mut db).unwrap();

            assert_eq!(final_version, 3);
            assert_eq!(MigrationRunner::read_schema_version(&db).unwrap(), Some(3));

            let cf = db.cf_handle("current_cf").unwrap();
            assert_eq!(
                &db.get_cf(&cf, b"key").unwrap().unwrap()[..],
                b"important_data"
            );

            for cf_name in &all_deprecated_cfs {
                assert!(
                    db.cf_handle(cf_name).is_none(),
                    "Deprecated CF '{}' should have been removed",
                    cf_name
                );
            }
        }
    }

    #[test]
    fn unsorted_migrations_are_applied_in_version_order() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // Create database with all deprecated CFs from v1, v2, v3
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let cf_descriptors = vec![
                ColumnFamilyDescriptor::new("default", Options::default()),
                ColumnFamilyDescriptor::new("current_cf", Options::default()),
                ColumnFamilyDescriptor::new(V1_CF_A, Options::default()),
                ColumnFamilyDescriptor::new(V2_CF, Options::default()),
                ColumnFamilyDescriptor::new(V3_CF, Options::default()),
            ];

            let _db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();
        }

        // Pass migrations in reverse order - MigrationRunner should sort them
        let migrations: Vec<Box<dyn Migration>> = vec![
            Box::new(TestV3Migration),
            Box::new(TestV1Migration),
            Box::new(TestV2Migration),
        ];

        let opts = default_db_options();
        let db = open_db_with_migrations(&opts, path, &["current_cf"], migrations).unwrap();

        // All deprecated CFs should be dropped (proves all migrations ran)
        assert!(db.cf_handle(V1_CF_A).is_none());
        assert!(db.cf_handle(V2_CF).is_none());
        assert!(db.cf_handle(V3_CF).is_none());

        // Schema version should be at v3
        assert_eq!(MigrationRunner::read_schema_version(&db).unwrap(), Some(3));
    }

    #[test]
    fn partial_migration_from_existing_version() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // Create database already at version 1
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let cf_descriptors = vec![
                ColumnFamilyDescriptor::new("default", Options::default()),
                ColumnFamilyDescriptor::new(SCHEMA_VERSION_CF, Options::default()),
                ColumnFamilyDescriptor::new(V2_CF, Options::default()),
                ColumnFamilyDescriptor::new(V3_CF, Options::default()),
            ];

            let db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();
            MigrationRunner::write_schema_version(&db, 1).unwrap();
        }

        // Run migrations - should only apply V2 and V3
        {
            let all_migrations = all_test_migrations();

            let opts = default_db_options();
            let existing_cfs = DB::list_cf(&opts, path).unwrap();
            let cf_descriptors: Vec<ColumnFamilyDescriptor> = existing_cfs
                .iter()
                .map(|name| ColumnFamilyDescriptor::new(name.clone(), Options::default()))
                .collect();

            let mut db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();

            assert_eq!(MigrationRunner::read_schema_version(&db).unwrap(), Some(1));

            let runner = MigrationRunner::from(all_migrations);
            let final_version = runner.run_pending(&mut db).unwrap();

            assert_eq!(final_version, 3);
            assert!(db.cf_handle(V2_CF).is_none());
            assert!(db.cf_handle(V3_CF).is_none());
        }
    }
}

/// Tests for downgrade prevention.
mod downgrade_tests {
    use super::*;

    #[test]
    fn prevents_downgrade() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // Create database at schema version 99
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let cf_descriptors = vec![
                ColumnFamilyDescriptor::new("default", Options::default()),
                ColumnFamilyDescriptor::new(SCHEMA_VERSION_CF, Options::default()),
            ];

            let db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();
            MigrationRunner::write_schema_version(&db, 99).unwrap();
        }

        // Try to open with current migration runner
        {
            let opts = default_db_options();
            let existing_cfs = DB::list_cf(&opts, path).unwrap();
            let cf_descriptors: Vec<ColumnFamilyDescriptor> = existing_cfs
                .iter()
                .map(|name| ColumnFamilyDescriptor::new(name.clone(), Options::default()))
                .collect();

            let mut db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();
            let migrations = test_migrations::v1_migrations();
            let runner = MigrationRunner::from(migrations);
            let result = runner.run_pending(&mut db);

            assert!(result.is_err());
            match result {
                Err(MigrationError::CannotDowngrade { current, target }) => {
                    assert_eq!(current, 99);
                    assert_eq!(target, 1); // V1 is the latest in our test migrations
                }
                _ => panic!("Expected CannotDowngrade error"),
            }
        }
    }

    #[test]
    fn no_downgrade_error_when_at_current_version() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let migrations = test_migrations::v1_migrations();
        let current_version = MigrationRunner::from(migrations).latest_version();

        // Create database at current schema version
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let cf_descriptors = vec![
                ColumnFamilyDescriptor::new("default", Options::default()),
                ColumnFamilyDescriptor::new(SCHEMA_VERSION_CF, Options::default()),
                ColumnFamilyDescriptor::new("test_cf", Options::default()),
            ];

            let db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();
            MigrationRunner::write_schema_version(&db, current_version).unwrap();
        }

        let opts = default_db_options();
        let migrations = test_migrations::v1_migrations();
        let result = open_db_with_migrations(&opts, path, &["test_cf"], migrations);

        assert!(result.is_ok());
        assert_eq!(
            MigrationRunner::read_schema_version(&result.unwrap()).unwrap(),
            Some(current_version)
        );
    }
}

/// Tests for column family configuration guardrails.
mod cf_guardrail_tests {
    use super::*;

    #[test]
    fn rejects_deprecated_cf_in_current_schema() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let opts = default_db_options();

        // Try to include a deprecated CF name from V1 migration
        let current_cfs = vec![
            "some_valid_cf",
            test_migrations::V1_CF_A, // This is deprecated!
        ];

        let migrations = test_migrations::v1_migrations();
        let result = open_db_with_migrations(&opts, path, &current_cfs, migrations);

        assert!(result.is_err());
        match result {
            Err(MigrationError::InvalidColumnFamilyConfig(msg)) => {
                assert!(msg.contains(test_migrations::V1_CF_A));
                assert!(msg.contains("cannot be reused"));
            }
            _ => panic!("Expected InvalidColumnFamilyConfig error, got {:?}", result),
        }
    }

    #[test]
    fn rejects_schema_version_cf_in_current_schema() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let opts = default_db_options();

        // Try to include the reserved schema version CF
        let current_cfs = vec!["some_valid_cf", SCHEMA_VERSION_CF];

        let result = open_db(&opts, path, &current_cfs);

        assert!(result.is_err());
        match result {
            Err(DatabaseError::Migration(MigrationError::InvalidColumnFamilyConfig(msg))) => {
                assert!(msg.contains(SCHEMA_VERSION_CF));
                assert!(msg.contains("reserved"));
            }
            _ => panic!("Expected InvalidColumnFamilyConfig error, got {:?}", result),
        }
    }

    #[test]
    fn all_deprecated_column_families_returns_expected_cfs() {
        let migrations = test_migrations::v1_migrations();
        let runner = MigrationRunner::from(migrations);
        let deprecated = runner.all_deprecated_column_families();

        // V1 migration deprecates 3 CFs
        assert!(deprecated.contains(test_migrations::V1_CF_A));
        assert!(deprecated.contains(test_migrations::V1_CF_B));
        assert!(deprecated.contains(test_migrations::V1_CF_C));
    }
}

/// Tests for resilient deprecated CF cleanup.
mod cleanup_resilience_tests {
    use super::*;

    #[test]
    fn cleanup_drops_deprecated_cf_even_when_schema_at_latest() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let migrations = test_migrations::v1_migrations();
        let latest_version = MigrationRunner::from(migrations).latest_version();

        // Create a database at latest schema version but WITH a deprecated CF still present
        // (simulating manual tampering or partial migration failure)
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let cf_descriptors = vec![
                ColumnFamilyDescriptor::new("default", Options::default()),
                ColumnFamilyDescriptor::new(SCHEMA_VERSION_CF, Options::default()),
                ColumnFamilyDescriptor::new("current_cf", Options::default()),
                // This deprecated CF should NOT exist at latest version, but we create it anyway
                ColumnFamilyDescriptor::new(test_migrations::V1_CF_A, Options::default()),
            ];

            let db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();

            // Set schema version to latest
            MigrationRunner::write_schema_version(&db, latest_version).unwrap();

            // Verify the deprecated CF exists and has data
            let cf = db.cf_handle(test_migrations::V1_CF_A).unwrap();
            db.put_cf(&cf, b"tampered_key", b"tampered_value").unwrap();
        }

        // Now open with the migration system
        let opts = default_db_options();
        let migrations = test_migrations::v1_migrations();
        let db = open_db_with_migrations(&opts, path, &["current_cf"], migrations).unwrap();

        // Deprecated CF should be gone despite schema version already being latest
        assert!(
            db.cf_handle(test_migrations::V1_CF_A).is_none(),
            "Deprecated CF should have been cleaned up"
        );

        // Current CF should still work
        assert!(db.cf_handle("current_cf").is_some());

        // Schema version should still be at latest
        assert_eq!(
            MigrationRunner::read_schema_version(&db).unwrap(),
            Some(latest_version)
        );
    }
}
