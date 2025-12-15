//! Version 1 Migration
//!
//! This migration removes the deprecated MSP respond storage request column families.
//! These column families were replaced with in-memory queueing in `MspHandler`.

use super::Migration;

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
pub struct V1Migration;

impl Migration for V1Migration {
    fn version(&self) -> u32 {
        1
    }

    fn deprecated_column_families(&self) -> &'static [&'static str] {
        &[
            "pending_msp_respond_storage_request",
            "pending_msp_respond_storage_request_left_index",
            "pending_msp_respond_storage_request_right_index",
        ]
    }

    fn description(&self) -> &'static str {
        "Remove deprecated MSP respond storage request column families (replaced with in-memory queueing)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v1_migration_version() {
        let migration = V1Migration;
        assert_eq!(migration.version(), 1);
    }

    #[test]
    fn test_v1_migration_deprecated_cfs() {
        let migration = V1Migration;
        let cfs = migration.deprecated_column_families();
        assert_eq!(cfs.len(), 3);
        assert!(cfs.contains(&"pending_msp_respond_storage_request"));
        assert!(cfs.contains(&"pending_msp_respond_storage_request_left_index"));
        assert!(cfs.contains(&"pending_msp_respond_storage_request_right_index"));
    }

    #[test]
    fn test_v1_migration_description() {
        let migration = V1Migration;
        let desc = migration.description();
        assert!(!desc.is_empty());
        assert!(desc.contains("MSP"));
    }
}
