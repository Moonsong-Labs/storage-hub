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

/// Test bucket data
pub mod bucket {
    use hex_literal::hex;
    use shp_types::Hash;

    /// Default bucket name
    pub const DEFAULT_BUCKET_NAME: &str = "test_bucket";

    /// Bucket ID expected by the SDK tests to be owned by MOCK_ADDRESS
    pub const BUCKET1_BUCKET_ID: [u8; 32] =
        hex!("d8793e4187f5642e96016a96fb33849a7e03eda91358b311bbd426ed38b26692");

    /// Default bucket is public
    pub const DEFAULT_IS_PUBLIC: bool = true;

    /// Default merkle root for repository (single zero byte)
    pub const DEFAULT_MERKLE_ROOT: &[u8] = &[0u8];

    /// Default value prop id (Hash::zero)
    pub const DEFAULT_VALUE_PROP_ID: Hash = Hash::zero();

    /// Default number of files in a bucket for new buckets
    pub const DEFAULT_FILE_COUNT: i64 = 0;

    /// Default bucket size in bytes for new buckets
    pub const DEFAULT_BUCKET_SIZE: i64 = 0;
}

/// Test file data
pub mod file {
    use hex_literal::hex;

    /// File key expected by the SDK tests to be in [`super::bucket::BUCKET1_BUCKET_ID`]
    pub const BUCKET1_FILE1_KEY: [u8; 32] =
        hex!("e901c8d212325fe2f18964fd2ea6e7375e2f90835b638ddb3c08692edd7840f7");

    /// File key expected by the SDK tests to be in [`super::bucket::BUCKET1_BUCKET_ID`]
    pub const BUCKET1_FILE3_KEY: [u8; 32] =
        hex!("c4344065c2f4c1155008caf5d56bcbf59d2f37b276e566b2dcad4713904d88e8");

    /// File fingerprint expected by the SDK tests for FILE3
    pub const BUCKET1_FILE3_FINGERPRINT: [u8; 32] =
        hex!("34eb5f637e05fc18f857ccb013250076534192189894d174ee3aa6d3525f6970");

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
