//! Blockchain Service Database Migrations
//!
//! This module contains store-specific migrations for the BlockchainServiceStateStore.
//! Each migration drops deprecated column families when upgrading the database schema.

use shc_common::rocksdb::Migration;

/// Version 1 migration module.
mod v1 {
    use super::Migration;

    /// Deprecated column family names for V1 migration.
    mod deprecated_cf_names {
        pub const PENDING_MSP_RESPOND_STORAGE_REQUEST: &str = "pending_msp_respond_storage_request";
        pub const PENDING_MSP_RESPOND_STORAGE_REQUEST_LEFT_INDEX: &str =
            "pending_msp_respond_storage_request_left_index";
        pub const PENDING_MSP_RESPOND_STORAGE_REQUEST_RIGHT_INDEX: &str =
            "pending_msp_respond_storage_request_right_index";
    }

    /// Version 1 migration that removes deprecated MSP respond storage request column families.
    ///
    /// ## Deprecated Column Families
    ///
    /// The following column families are removed by this migration:
    ///
    /// - `pending_msp_respond_storage_request`: Stored pending respond storage requests
    /// - `pending_msp_respond_storage_request_left_index`: Left index for the deque
    /// - `pending_msp_respond_storage_request_right_index`: Right index for the deque
    ///
    /// ## Reason for Deprecation
    ///
    /// The functionality has been replaced with in-memory queueing in `MspHandler`,
    /// eliminating the need for persistent storage of these requests.
    pub struct BlockchainServiceV1Migration;

    impl Migration for BlockchainServiceV1Migration {
        fn version(&self) -> u32 {
            1
        }

        fn deprecated_column_families(&self) -> &'static [&'static str] {
            &[
                deprecated_cf_names::PENDING_MSP_RESPOND_STORAGE_REQUEST,
                deprecated_cf_names::PENDING_MSP_RESPOND_STORAGE_REQUEST_LEFT_INDEX,
                deprecated_cf_names::PENDING_MSP_RESPOND_STORAGE_REQUEST_RIGHT_INDEX,
            ]
        }

        fn description(&self) -> &'static str {
            "Remove deprecated MSP respond storage request column families (replaced with in-memory queueing)"
        }
    }
}

/// Returns all migrations for the BlockchainServiceStateStore.
///
/// Migrations are returned in order of their version numbers.
/// Each migration drops deprecated column families from previous schema versions.
pub fn blockchain_service_migrations() -> Vec<Box<dyn Migration>> {
    vec![Box::new(v1::BlockchainServiceV1Migration)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v1_migration_version() {
        let migration = v1::BlockchainServiceV1Migration;
        assert_eq!(migration.version(), 1);
    }

    #[test]
    fn test_v1_migration_deprecated_cfs() {
        let migration = v1::BlockchainServiceV1Migration;
        let cfs = migration.deprecated_column_families();
        assert_eq!(cfs.len(), 3);
        assert!(cfs.contains(&"pending_msp_respond_storage_request"));
        assert!(cfs.contains(&"pending_msp_respond_storage_request_left_index"));
        assert!(cfs.contains(&"pending_msp_respond_storage_request_right_index"));
    }

    #[test]
    fn test_v1_migration_description() {
        let migration = v1::BlockchainServiceV1Migration;
        let desc = migration.description();
        assert!(!desc.is_empty());
        assert!(desc.contains("MSP"));
    }

    #[test]
    fn test_blockchain_service_migrations_order() {
        let migrations = blockchain_service_migrations();
        assert_eq!(migrations.len(), 1);
        assert_eq!(migrations[0].version(), 1);
    }
}
