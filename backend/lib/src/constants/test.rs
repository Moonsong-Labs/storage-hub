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
    use shc_indexer_db::OnchainBspId;
    use shp_types::Hash;

    /// Default BSP capacity
    pub const DEFAULT_CAPACITY: i64 = 1000;

    /// Updated BSP capacity
    pub const UPDATED_CAPACITY: i64 = 2000;

    /// Default BSP stake
    pub const DEFAULT_STAKE: i64 = 100;

    /// Default BSP ID
    pub const DEFAULT_BSP_ID: OnchainBspId = OnchainBspId::new(Hash::zero());

    /// Default merkle root for repository (single zero byte)
    pub const DEFAULT_MERKLE_ROOT: &[u8] = &[0u8];

    /// Default last tick proven value for repository
    pub const DEFAULT_LAST_TICK_PROVEN: i64 = 0;
}

/// Test MSP (Main Storage Provider) data
pub mod msp {
    use bigdecimal::BigDecimal;

    /// Default MSP capacity
    pub const DEFAULT_CAPACITY: i64 = 5000;

    /// Default MSP value proposition
    pub const DEFAULT_VALUE_PROP: &str = "Test MSP Value Proposition";

    /// Default MSP capacity for repository (zero)
    pub fn default_repository_capacity() -> BigDecimal {
        BigDecimal::from(0)
    }

    /// Default MSP value proposition for repository (empty string)
    pub const DEFAULT_REPOSITORY_VALUE_PROP: &str = "";
}

/// Test merkle tree data
pub mod merkle {
    /// Alternative merkle root for BSP
    pub const BSP_MERKLE_ROOT: &[u8] = &[1, 2, 3];
}

/// Test bucket data
pub mod bucket {
    use hex_literal::hex;

    /// Default bucket name
    pub const DEFAULT_BUCKET_NAME: &str = "test_bucket";

    /// Default bucket onchain ID (valid 32-byte hex string = 64 hex chars)
    pub const DEFAULT_BUCKET_ID: [u8; 32] =
        hex!("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");

    /// Default bucket is public
    pub const DEFAULT_IS_PUBLIC: bool = true;

    /// Default merkle root for repository (single zero byte)
    pub const DEFAULT_MERKLE_ROOT: &[u8] = &[0u8];
}

/// Test file data
pub mod file {
    /// Default file key
    pub const DEFAULT_FILE_KEY: &str = "test_file.txt";

    /// Default file location
    pub const DEFAULT_LOCATION: &str = "/files/test_file.txt";

    /// Default file fingerprint (32 bytes)
    pub const DEFAULT_FINGERPRINT: &[u8; 32] = &[
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32,
    ];

    /// Default file size
    pub const DEFAULT_SIZE: i64 = 1024;

    /// Default file step (0 = requested, 1 = fulfilled)
    pub const DEFAULT_STEP: i32 = 1;

    /// Default file step for repository (0 = Requested)
    pub const DEFAULT_REPOSITORY_STEP: i32 = 0;
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
