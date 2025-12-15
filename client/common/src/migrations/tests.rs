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

/// Tests for `MigrationRunner` static methods.
mod migration_runner_tests {
    use super::*;

    #[test]
    fn latest_version_is_at_least_one() {
        let latest = MigrationRunner::latest_version();
        assert!(latest >= 1, "Should have at least v1 migration");
    }

    #[test]
    fn validate_migration_order_passes_for_valid_migrations() {
        let result = MigrationRunner::validate_migration_order();
        assert!(result.is_ok(), "Valid migrations should pass validation");
    }
}

/// Tests for migration validation logic.
mod validation_tests {
    use super::*;

    /// Helper to test with custom migrations
    fn validate_custom_migrations(
        migrations: Vec<Box<dyn Migration>>,
    ) -> Result<(), MigrationError> {
        if migrations.is_empty() {
            return Ok(());
        }

        let mut seen_versions = HashSet::new();
        for migration in &migrations {
            if !seen_versions.insert(migration.version()) {
                return Err(MigrationError::MigrationFailed {
                    version: migration.version(),
                    reason: format!("Duplicate migration version: {}", migration.version()),
                });
            }
        }

        if migrations.first().map(|m| m.version()) != Some(1) {
            return Err(MigrationError::MigrationFailed {
                version: 0,
                reason: "Migrations must start from version 1".to_string(),
            });
        }

        for (i, migration) in migrations.iter().enumerate() {
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

        let result = validate_custom_migrations(migrations);
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

        let mut migrations: Vec<Box<dyn Migration>> = vec![Box::new(GapV1), Box::new(GapV3)];
        migrations.sort_by_key(|m| m.version());

        let result = validate_custom_migrations(migrations);
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

        let result = validate_custom_migrations(migrations);
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
    fn open_fresh_database() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let opts = default_db_options();
        let current_cfs = vec!["test_cf"];

        let db = open_db_with_migrations(&opts, path, &current_cfs).unwrap();

        assert!(db.cf_handle("test_cf").is_some());
        assert!(db.cf_handle(SCHEMA_VERSION_CF).is_some());

        let version = MigrationRunner::read_schema_version(&db).unwrap();
        assert_eq!(version, MigrationRunner::latest_version());
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
        let result = open_db_with_migrations(&opts, path.to_str().unwrap(), &["test_cf"]);

        assert!(matches!(result, Err(MigrationError::RocksDb(_))));
    }

    #[test]
    fn migrations_are_idempotent() {
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
        }

        // Open with migrations
        let opts = default_db_options();
        let current_cfs = vec!["current_cf"];
        let db = open_db_with_migrations(&opts, path, &current_cfs).unwrap();

        assert!(db.cf_handle("current_cf").is_some());
        assert!(db
            .cf_handle("pending_msp_respond_storage_request")
            .is_none());
        assert!(db
            .cf_handle("pending_msp_respond_storage_request_left_index")
            .is_none());
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
                ColumnFamilyDescriptor::new(
                    "pending_msp_respond_storage_request",
                    Options::default(),
                ),
            ];

            let db = DB::open_cf_descriptors(&opts, path, cfs).unwrap();
            let active_cf = db.cf_handle("active_cf").unwrap();
            db.put_cf(&active_cf, test_key, test_value).unwrap();
        }

        // Open with migrations
        let opts = default_db_options();
        let current_cfs = vec!["active_cf"];
        let db = open_db_with_migrations(&opts, path, &current_cfs).unwrap();

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
        let db = open_db_with_migrations(&opts, path, &current_cfs).unwrap();

        let cf = db.cf_handle("clean_cf").unwrap();
        assert_eq!(&db.get_cf(&cf, b"key").unwrap().unwrap()[..], b"value");
        assert_eq!(
            MigrationRunner::read_schema_version(&db).unwrap(),
            MigrationRunner::latest_version()
        );
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
                ColumnFamilyDescriptor::new(
                    "pending_msp_respond_storage_request",
                    Options::default(),
                ),
            ];

            let _db = DB::open_cf_descriptors(&opts, path, cfs).unwrap();
        }

        let opts = default_db_options();
        let current_cfs = vec!["my_cf"];
        let db = open_db_with_migrations(&opts, path, &current_cfs).unwrap();

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
        let db = open_db_with_migrations(&opts, path, &current_cfs).unwrap();
        let cf = db.cf_handle("my_cf").unwrap();
        assert_eq!(&db.get_cf(&cf, b"key2").unwrap().unwrap()[..], b"value2");
    }

    #[test]
    fn schema_version_cf_created_automatically() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let opts = default_db_options();
        let db = open_db_with_migrations(&opts, path, &["my_cf"]).unwrap();

        assert!(db.cf_handle(SCHEMA_VERSION_CF).is_some());
        assert_eq!(
            MigrationRunner::read_schema_version(&db).unwrap(),
            MigrationRunner::latest_version()
        );
    }

    #[test]
    fn empty_current_cfs_works() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let opts = default_db_options();
        let db = open_db_with_migrations(&opts, path, &[]).unwrap();

        assert!(db.cf_handle("default").is_some());
        assert!(db.cf_handle(SCHEMA_VERSION_CF).is_some());
    }

    #[test]
    fn migration_runner_handles_empty_migrations() {
        fn run_with_empty_migrations(db: &mut DB) -> Result<u32, MigrationError> {
            let current_version = MigrationRunner::read_schema_version(db)?;
            let migrations: Vec<Box<dyn Migration>> = vec![];

            if migrations.is_empty() {
                return Ok(current_version);
            }
            Ok(0)
        }

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

        let result = run_with_empty_migrations(&mut db);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }
}

/// Test-only migrations for multi-version testing.
mod test_migrations {
    use super::*;

    /// Test-only V2 migration for multi-version testing.
    pub struct TestV2Migration;

    impl Migration for TestV2Migration {
        fn version(&self) -> u32 {
            2
        }

        fn deprecated_column_families(&self) -> &'static [&'static str] {
            &["test_deprecated_v2_cf", "test_deprecated_v2_cf_index"]
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
            &["test_deprecated_v3_cf"]
        }

        fn description(&self) -> &'static str {
            "Test V3 migration - removes another test deprecated CF"
        }
    }
}

/// Helper function to run migrations with a custom migration list.
fn run_migrations_with_list(
    db: &mut DB,
    migrations: Vec<Box<dyn Migration>>,
) -> Result<u32, MigrationError> {
    let current_version = MigrationRunner::read_schema_version(db)?;
    let latest_version = migrations.last().map(|m| m.version()).unwrap_or(0);

    if current_version > latest_version {
        return Err(MigrationError::CannotDowngrade {
            current: current_version,
            target: latest_version,
        });
    }

    let pending: Vec<_> = migrations
        .iter()
        .filter(|m| m.version() > current_version)
        .collect();

    if pending.is_empty() {
        return Ok(current_version);
    }

    let mut applied_version = current_version;

    for migration in pending {
        for cf_name in migration.deprecated_column_families() {
            if db.cf_handle(cf_name).is_some() {
                db.drop_cf(cf_name)
                    .map_err(|e| MigrationError::MigrationFailed {
                        version: migration.version(),
                        reason: format!("Failed to drop column family '{}': {}", cf_name, e),
                    })?;
            }
        }

        MigrationRunner::write_schema_version(db, migration.version())?;
        applied_version = migration.version();
    }

    Ok(applied_version)
}

/// Tests for multi-version migration scenarios.
mod multi_version_tests {
    use super::test_migrations::*;
    use super::*;

    #[test]
    fn migration_chain_v0_to_v3() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let all_migrations: Vec<Box<dyn Migration>> = vec![
            Box::new(v1::V1Migration),
            Box::new(TestV2Migration),
            Box::new(TestV3Migration),
        ];

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

            assert_eq!(MigrationRunner::read_schema_version(&db).unwrap(), 0);
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
            let all_migrations: Vec<Box<dyn Migration>> = vec![
                Box::new(v1::V1Migration),
                Box::new(TestV2Migration),
                Box::new(TestV3Migration),
            ];
            let final_version = run_migrations_with_list(&mut db, all_migrations).unwrap();

            assert_eq!(final_version, 3);
            assert_eq!(MigrationRunner::read_schema_version(&db).unwrap(), 3);

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
    fn migrations_applied_in_version_order() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // Create migrations in non-sorted order
        let mut all_migrations: Vec<Box<dyn Migration>> = vec![
            Box::new(TestV3Migration),
            Box::new(v1::V1Migration),
            Box::new(TestV2Migration),
        ];

        all_migrations.sort_by_key(|m| m.version());

        assert_eq!(all_migrations[0].version(), 1);
        assert_eq!(all_migrations[1].version(), 2);
        assert_eq!(all_migrations[2].version(), 3);

        // Create database with deprecated CFs
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let cf_descriptors = vec![
                ColumnFamilyDescriptor::new("default", Options::default()),
                ColumnFamilyDescriptor::new(SCHEMA_VERSION_CF, Options::default()),
                ColumnFamilyDescriptor::new(
                    "pending_msp_respond_storage_request",
                    Options::default(),
                ),
                ColumnFamilyDescriptor::new("test_deprecated_v2_cf", Options::default()),
                ColumnFamilyDescriptor::new("test_deprecated_v3_cf", Options::default()),
            ];

            let _db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();
        }

        // Run migrations
        {
            let opts = default_db_options();
            let existing_cfs = DB::list_cf(&opts, path).unwrap();
            let cf_descriptors: Vec<ColumnFamilyDescriptor> = existing_cfs
                .iter()
                .map(|name| ColumnFamilyDescriptor::new(name.clone(), Options::default()))
                .collect();

            let mut db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();
            let final_version = run_migrations_with_list(&mut db, all_migrations).unwrap();

            assert_eq!(final_version, 3);
            assert!(db
                .cf_handle("pending_msp_respond_storage_request")
                .is_none());
            assert!(db.cf_handle("test_deprecated_v2_cf").is_none());
            assert!(db.cf_handle("test_deprecated_v3_cf").is_none());
        }
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
                ColumnFamilyDescriptor::new("test_deprecated_v2_cf", Options::default()),
                ColumnFamilyDescriptor::new("test_deprecated_v3_cf", Options::default()),
            ];

            let db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();
            MigrationRunner::write_schema_version(&db, 1).unwrap();
        }

        // Run migrations - should only apply V2 and V3
        {
            let all_migrations: Vec<Box<dyn Migration>> = vec![
                Box::new(v1::V1Migration),
                Box::new(TestV2Migration),
                Box::new(TestV3Migration),
            ];

            let opts = default_db_options();
            let existing_cfs = DB::list_cf(&opts, path).unwrap();
            let cf_descriptors: Vec<ColumnFamilyDescriptor> = existing_cfs
                .iter()
                .map(|name| ColumnFamilyDescriptor::new(name.clone(), Options::default()))
                .collect();

            let mut db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();

            assert_eq!(MigrationRunner::read_schema_version(&db).unwrap(), 1);

            let final_version = run_migrations_with_list(&mut db, all_migrations).unwrap();

            assert_eq!(final_version, 3);
            assert!(db.cf_handle("test_deprecated_v2_cf").is_none());
            assert!(db.cf_handle("test_deprecated_v3_cf").is_none());
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
            let result = MigrationRunner::run_pending(&mut db);

            assert!(result.is_err());
            match result {
                Err(MigrationError::CannotDowngrade { current, target }) => {
                    assert_eq!(current, 99);
                    assert_eq!(target, MigrationRunner::latest_version());
                }
                _ => panic!("Expected CannotDowngrade error"),
            }
        }
    }

    #[test]
    fn downgrade_prevention_via_open_db_with_migrations() {
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
                ColumnFamilyDescriptor::new("test_cf", Options::default()),
            ];

            let db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();
            MigrationRunner::write_schema_version(&db, 99).unwrap();
        }

        let opts = default_db_options();
        let result = open_db_with_migrations(&opts, path, &["test_cf"]);

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(MigrationError::CannotDowngrade { .. })
        ));
    }

    #[test]
    fn no_downgrade_error_when_at_current_version() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let current_version = MigrationRunner::latest_version();

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
        let result = open_db_with_migrations(&opts, path, &["test_cf"]);

        assert!(result.is_ok());
        assert_eq!(
            MigrationRunner::read_schema_version(&result.unwrap()).unwrap(),
            current_version
        );
    }

    #[test]
    fn error_propagation_from_open_db_with_migrations() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // Create database with high schema version
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let cf_descriptors = vec![
                ColumnFamilyDescriptor::new("default", Options::default()),
                ColumnFamilyDescriptor::new(SCHEMA_VERSION_CF, Options::default()),
            ];

            let db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();
            MigrationRunner::write_schema_version(&db, 999).unwrap();
        }

        let opts = default_db_options();
        let result = open_db_with_migrations(&opts, path, &["some_cf"]);

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(MigrationError::CannotDowngrade { .. })
        ));
    }
}

/// Tests simulating real store migration scenarios.
mod store_simulation_tests {
    use super::*;

    #[test]
    fn blockchain_service_store_migration() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let current_store_cfs = vec![
            "last_processed_block_number",
            "pending_confirm_storing_request_left_index",
            "pending_confirm_storing_request_right_index",
            "pending_confirm_storing_request",
            "pending_stop_storing_for_insolvent_user_request_left_index",
            "pending_stop_storing_for_insolvent_user_request_right_index",
            "pending_stop_storing_for_insolvent_user_request",
            "pending_file_deletion_request_left_index",
            "pending_file_deletion_request_right_index",
            "pending_file_deletion_request",
        ];

        // Create pre-migration database
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let mut cf_descriptors: Vec<ColumnFamilyDescriptor> =
                vec![ColumnFamilyDescriptor::new("default", Options::default())];

            for cf_name in &current_store_cfs {
                cf_descriptors.push(ColumnFamilyDescriptor::new(*cf_name, Options::default()));
            }

            let v1_migration = v1::V1Migration;
            for cf_name in v1_migration.deprecated_column_families() {
                cf_descriptors.push(ColumnFamilyDescriptor::new(*cf_name, Options::default()));
            }

            let db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();

            let cf = db.cf_handle("last_processed_block_number").unwrap();
            db.put_cf(&cf, b"value", 42u64.to_le_bytes()).unwrap();

            let deprecated_cf = db.cf_handle("pending_msp_respond_storage_request").unwrap();
            db.put_cf(&deprecated_cf, b"old_request_key", b"old_request_data")
                .unwrap();
        }

        // Open with migration system
        {
            let opts = default_db_options();
            let db = open_db_with_migrations(&opts, path, &current_store_cfs).unwrap();

            let cf = db.cf_handle("last_processed_block_number").unwrap();
            let value = db.get_cf(&cf, b"value").unwrap();
            assert!(value.is_some());
            let bytes: [u8; 8] = value.unwrap()[..].try_into().unwrap();
            assert_eq!(u64::from_le_bytes(bytes), 42);

            let v1_migration = v1::V1Migration;
            for cf_name in v1_migration.deprecated_column_families() {
                assert!(db.cf_handle(cf_name).is_none());
            }

            assert_eq!(
                MigrationRunner::read_schema_version(&db).unwrap(),
                MigrationRunner::latest_version()
            );
        }
    }

    #[test]
    fn download_state_store_migration() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let current_store_cfs = vec![
            "missing_chunks",
            "file_metadata",
            "pending_bucket_downloads",
        ];

        // Create database
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let mut cf_descriptors: Vec<ColumnFamilyDescriptor> =
                vec![ColumnFamilyDescriptor::new("default", Options::default())];

            for cf_name in &current_store_cfs {
                cf_descriptors.push(ColumnFamilyDescriptor::new(*cf_name, Options::default()));
            }

            let db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();

            let cf = db.cf_handle("file_metadata").unwrap();
            db.put_cf(&cf, b"file_key_123", b"metadata_123").unwrap();
        }

        // Open with migration system
        {
            let opts = default_db_options();
            let db = open_db_with_migrations(&opts, path, &current_store_cfs).unwrap();

            let cf = db.cf_handle("file_metadata").unwrap();
            let value = db.get_cf(&cf, b"file_key_123").unwrap();
            assert_eq!(&value.unwrap()[..], b"metadata_123");

            for cf_name in &current_store_cfs {
                assert!(db.cf_handle(cf_name).is_some());
            }
        }
    }

    #[test]
    fn bsp_peer_manager_store_migration() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let current_store_cfs = vec![
            "bsp_peer_stats",
            "bsp_peer_file_keys",
            "bsp_peer_last_update",
        ];

        // Create database
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let mut cf_descriptors: Vec<ColumnFamilyDescriptor> =
                vec![ColumnFamilyDescriptor::new("default", Options::default())];

            for cf_name in &current_store_cfs {
                cf_descriptors.push(ColumnFamilyDescriptor::new(*cf_name, Options::default()));
            }

            let db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();

            let cf = db.cf_handle("bsp_peer_stats").unwrap();
            db.put_cf(&cf, b"peer_id_abc", b"stats_data").unwrap();
        }

        // Open with migration system
        {
            let opts = default_db_options();
            let db = open_db_with_migrations(&opts, path, &current_store_cfs).unwrap();

            let cf = db.cf_handle("bsp_peer_stats").unwrap();
            assert_eq!(
                &db.get_cf(&cf, b"peer_id_abc").unwrap().unwrap()[..],
                b"stats_data"
            );

            for cf_name in &current_store_cfs {
                assert!(db.cf_handle(cf_name).is_some());
            }
        }
    }

    #[test]
    fn store_reopening_after_migration() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let current_cfs = vec!["data_cf", "index_cf"];

        // First open
        {
            let opts = default_db_options();
            let db = open_db_with_migrations(&opts, path, &current_cfs).unwrap();

            let cf = db.cf_handle("data_cf").unwrap();
            db.put_cf(&cf, b"key1", b"value1").unwrap();

            assert_eq!(
                MigrationRunner::read_schema_version(&db).unwrap(),
                MigrationRunner::latest_version()
            );
        }

        // Second open
        {
            let opts = default_db_options();
            let db = open_db_with_migrations(&opts, path, &current_cfs).unwrap();

            let cf = db.cf_handle("data_cf").unwrap();
            assert_eq!(&db.get_cf(&cf, b"key1").unwrap().unwrap()[..], b"value1");

            db.put_cf(&cf, b"key2", b"value2").unwrap();
        }

        // Third open
        {
            let opts = default_db_options();
            let db = open_db_with_migrations(&opts, path, &current_cfs).unwrap();

            let cf = db.cf_handle("data_cf").unwrap();
            assert_eq!(&db.get_cf(&cf, b"key1").unwrap().unwrap()[..], b"value1");
            assert_eq!(&db.get_cf(&cf, b"key2").unwrap().unwrap()[..], b"value2");
        }
    }

    #[test]
    fn full_v1_migration_scenario() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let v1_migration = v1::V1Migration;
        let deprecated_cfs = v1_migration.deprecated_column_families();
        assert_eq!(deprecated_cfs.len(), 3, "V1 should deprecate exactly 3 CFs");

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

        // Create old database
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let mut all_cfs: Vec<ColumnFamilyDescriptor> = current_cfs
                .iter()
                .map(|name| ColumnFamilyDescriptor::new(*name, Options::default()))
                .collect();

            for cf_name in deprecated_cfs {
                all_cfs.push(ColumnFamilyDescriptor::new(*cf_name, Options::default()));
            }
            all_cfs.push(ColumnFamilyDescriptor::new("default", Options::default()));

            let db = DB::open_cf_descriptors(&opts, path, all_cfs).unwrap();

            let cf = db.cf_handle("last_processed_block_number").unwrap();
            db.put_cf(&cf, b"block", b"12345").unwrap();

            let deprecated_cf = db.cf_handle("pending_msp_respond_storage_request").unwrap();
            db.put_cf(&deprecated_cf, b"old_key", b"old_value").unwrap();
        }

        // Open with migration
        let opts = default_db_options();
        let db = open_db_with_migrations(&opts, path, &current_cfs).unwrap();

        for cf_name in &current_cfs {
            assert!(db.cf_handle(cf_name).is_some());
        }

        for cf_name in deprecated_cfs {
            assert!(db.cf_handle(cf_name).is_none());
        }

        let cf = db.cf_handle("last_processed_block_number").unwrap();
        assert_eq!(&db.get_cf(&cf, b"block").unwrap().unwrap()[..], b"12345");

        assert_eq!(MigrationRunner::read_schema_version(&db).unwrap(), 1);
    }

    #[test]
    fn migration_does_not_fail_on_nonexistent_cf_drop() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // Create database WITHOUT deprecated CFs
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let cf_descriptors = vec![
                ColumnFamilyDescriptor::new("default", Options::default()),
                ColumnFamilyDescriptor::new(SCHEMA_VERSION_CF, Options::default()),
                ColumnFamilyDescriptor::new("current_cf", Options::default()),
            ];

            let _db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();
        }

        // Should succeed even though deprecated CFs don't exist
        let opts = default_db_options();
        let db = open_db_with_migrations(&opts, path, &["current_cf"]).unwrap();

        assert_eq!(
            MigrationRunner::read_schema_version(&db).unwrap(),
            MigrationRunner::latest_version()
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
            "pending_msp_respond_storage_request", // This is deprecated!
        ];

        let result = open_db_with_migrations(&opts, path, &current_cfs);

        assert!(result.is_err());
        match result {
            Err(MigrationError::InvalidColumnFamilyConfig(msg)) => {
                assert!(msg.contains("pending_msp_respond_storage_request"));
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

        let result = open_db_with_migrations(&opts, path, &current_cfs);

        assert!(result.is_err());
        match result {
            Err(MigrationError::InvalidColumnFamilyConfig(msg)) => {
                assert!(msg.contains(SCHEMA_VERSION_CF));
                assert!(msg.contains("reserved"));
            }
            _ => panic!("Expected InvalidColumnFamilyConfig error, got {:?}", result),
        }
    }

    #[test]
    fn all_deprecated_column_families_returns_expected_cfs() {
        let deprecated = MigrationRunner::all_deprecated_column_families();

        // V1 migration deprecates 3 CFs
        assert!(deprecated.contains("pending_msp_respond_storage_request"));
        assert!(deprecated.contains("pending_msp_respond_storage_request_left_index"));
        assert!(deprecated.contains("pending_msp_respond_storage_request_right_index"));
    }
}

/// Tests for resilient deprecated CF cleanup.
mod cleanup_resilience_tests {
    use super::*;

    #[test]
    fn cleanup_drops_deprecated_cf_even_when_schema_at_latest() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let latest_version = MigrationRunner::latest_version();

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
                ColumnFamilyDescriptor::new(
                    "pending_msp_respond_storage_request",
                    Options::default(),
                ),
            ];

            let db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();

            // Set schema version to latest
            MigrationRunner::write_schema_version(&db, latest_version).unwrap();

            // Verify the deprecated CF exists and has data
            let cf = db.cf_handle("pending_msp_respond_storage_request").unwrap();
            db.put_cf(&cf, b"tampered_key", b"tampered_value").unwrap();
        }

        // Now open with the migration system
        let opts = default_db_options();
        let db = open_db_with_migrations(&opts, path, &["current_cf"]).unwrap();

        // Deprecated CF should be gone despite schema version already being latest
        assert!(
            db.cf_handle("pending_msp_respond_storage_request")
                .is_none(),
            "Deprecated CF should have been cleaned up"
        );

        // Current CF should still work
        assert!(db.cf_handle("current_cf").is_some());

        // Schema version should still be at latest
        assert_eq!(
            MigrationRunner::read_schema_version(&db).unwrap(),
            latest_version
        );
    }

    #[test]
    fn cleanup_drops_multiple_deprecated_cfs_at_latest_version() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let latest_version = MigrationRunner::latest_version();

        // Create a database with ALL deprecated CFs present but at latest schema version
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let mut cf_descriptors = vec![
                ColumnFamilyDescriptor::new("default", Options::default()),
                ColumnFamilyDescriptor::new(SCHEMA_VERSION_CF, Options::default()),
                ColumnFamilyDescriptor::new("current_cf", Options::default()),
            ];

            // Add all deprecated CFs from V1
            let v1_migration = v1::V1Migration;
            for cf_name in v1_migration.deprecated_column_families() {
                cf_descriptors.push(ColumnFamilyDescriptor::new(*cf_name, Options::default()));
            }

            let db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();
            MigrationRunner::write_schema_version(&db, latest_version).unwrap();
        }

        // Open with migration system
        let opts = default_db_options();
        let db = open_db_with_migrations(&opts, path, &["current_cf"]).unwrap();

        // All deprecated CFs should be gone
        let v1_migration = v1::V1Migration;
        for cf_name in v1_migration.deprecated_column_families() {
            assert!(
                db.cf_handle(cf_name).is_none(),
                "Deprecated CF '{}' should have been cleaned up",
                cf_name
            );
        }
    }

    #[test]
    fn cleanup_is_idempotent_when_deprecated_cfs_already_gone() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let latest_version = MigrationRunner::latest_version();

        // Create a clean database at latest version with no deprecated CFs
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let cf_descriptors = vec![
                ColumnFamilyDescriptor::new("default", Options::default()),
                ColumnFamilyDescriptor::new(SCHEMA_VERSION_CF, Options::default()),
                ColumnFamilyDescriptor::new("current_cf", Options::default()),
            ];

            let db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();
            MigrationRunner::write_schema_version(&db, latest_version).unwrap();
        }

        // Open multiple times - should not fail
        for _ in 0..3 {
            let opts = default_db_options();
            let db = open_db_with_migrations(&opts, path, &["current_cf"]).unwrap();
            assert_eq!(
                MigrationRunner::read_schema_version(&db).unwrap(),
                latest_version
            );
        }
    }
}
