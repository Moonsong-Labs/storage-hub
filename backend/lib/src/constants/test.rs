//! Test constants for use across all backend tests
//!
//! This module provides centralized test data constants to ensure consistency
//! and clarity in tests. Using these constants prevents accidental mismatches
//! and makes it clear where test data originates from.

/// Default test database URL (used when TEST_DATABASE_URL env var is not set)
pub const DEFAULT_TEST_DATABASE_URL: &str = "postgres://test:test@localhost/test_db";

/// Test account and owner identifiers
pub mod accounts {
    /// Test BSP account string
    pub const TEST_BSP_ACCOUNT_STR: &str = "test_account";

    /// Test MSP account string
    pub const TEST_MSP_ACCOUNT_STR: &str = "msp_test_account";
}

/// Test BSP (Backup Storage Provider) data
pub mod bsp {
    /// Default BSP capacity
    pub const DEFAULT_CAPACITY: i64 = 1000;

    /// Updated BSP capacity
    pub const UPDATED_CAPACITY: i64 = 2000;

    /// Default BSP stake
    pub const DEFAULT_STAKE: i64 = 100;

    /// Default BSP ID
    pub const DEFAULT_BSP_ID: i64 = 1;
}

/// Test MSP (Main Storage Provider) data
pub mod msp {
    /// Default MSP capacity
    pub const DEFAULT_CAPACITY: i64 = 5000;

    /// Default MSP value proposition
    pub const DEFAULT_VALUE_PROP: &str = "Test MSP Value Proposition";
}

/// Test merkle tree data
pub mod merkle {
    /// Alternative merkle root for BSP
    pub const BSP_MERKLE_ROOT: &[u8] = &[1, 2, 3];
}

/// Mock connection test data
pub mod mock_rpc {
    /// Test method names
    pub const SAMPLE_METHOD: &str = "system_health";
    pub const SAMPLE_FIELD: &str = "field";
    pub const SAMPLE_VALUE: &str = "value";

    /// Error simulation parameters
    pub const FAIL_AFTER_N_CALLS_THRESHOLD: usize = 2;
    pub const ERROR_MESSAGE_FAIL_AFTER_N: &str = "Simulated failure after N calls";

    /// Transport error messages
    pub const TEST_TRANSPORT_ERROR_MSG: &str = "Test transport error";
    pub const TEST_RPC_ERROR_MSG: &str = "Test RPC error";
}
