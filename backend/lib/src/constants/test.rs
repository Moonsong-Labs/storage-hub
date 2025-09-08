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

    /// Default bucket name
    pub const DEFAULT_BUCKET_NAME: &str = "test_bucket";

    /// Default bucket onchain ID (valid 32-byte hex string = 64 hex chars)
    pub const DEFAULT_BUCKET_ID: [u8; 32] =
        hex!("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");

    /// Default bucket is public
    pub const DEFAULT_IS_PUBLIC: bool = true;
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

/// Repository test constants for database tests
pub mod repository {
    use hex_literal::hex;

    /// Additional test bucket for filtering tests
    pub const ADDITIONAL_BUCKET_ID: &[u8] = b"additional-bucket";
    pub const ADDITIONAL_FILE_KEY: [u8; 32] =
        hex!("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890");
    pub const ADDITIONAL_FILE_ACCOUNT: [u8; 32] =
        hex!("20d81e86ed5b986d1d6ddbe416627f96f740252c4a80ab8ed91db58f7ecf9657");
    pub const ADDITIONAL_FILE_FINGERPRINT: [u8; 32] =
        hex!("0000000000000000000000000000000000000000000000000000000000000002");
    pub const ADDITIONAL_FILE_LOCATION: &[u8] = b"file.txt";
    pub const ADDITIONAL_FILE_SIZE: i64 = 12345;

    /// Empty bucket test MSP
    pub const EMPTY_BUCKET_MSP_ID: [u8; 32] =
        hex!("0000000000000000000000000000000000000000000000000000000000000999");
    pub const EMPTY_BUCKET_MSP_ACCOUNT: &str = "5EmptyMspAccountAddressForTestingPurpose";
    pub const EMPTY_BUCKET_NAME: &[u8] = b"empty-bucket";
    pub const EMPTY_BUCKET_ID: &[u8] = b"empty-bucket-id";
    pub const EMPTY_BUCKET_USER: &str = "0xemptybucketuser";

    /// Pagination test buckets
    pub const PAGINATION_BUCKET_2_NAME: &[u8] = b"pagination-bucket-2";
    pub const PAGINATION_BUCKET_2_ID: &[u8] = b"pb2";
    pub const PAGINATION_BUCKET_3_NAME: &[u8] = b"pagination-bucket-3";
    pub const PAGINATION_BUCKET_3_ID: &[u8] = b"pb3";

    /// User filtering test buckets
    pub const USER_BUCKET_2_NAME: &[u8] = b"user-bucket2";
    pub const USER_BUCKET_2_ID: &[u8] = b"b2";
    pub const USER_BUCKET_3_NAME: &[u8] = b"user-bucket3";
    pub const USER_BUCKET_3_ID: &[u8] = b"b3";
    pub const OTHER_USER_ACCOUNT: &str = "0xotheruser";
    pub const OTHER_USER_BUCKET_NAME: &[u8] = b"other-user-bucket";
    pub const OTHER_USER_BUCKET_ID: &[u8] = b"oub1";

    /// MSP filtering test
    pub const MSP1_BUCKET_NAME: &[u8] = b"user-msp1-bucket";
    pub const MSP1_BUCKET_ID: &[u8] = b"mb1";

    /// No MSP test
    pub const NO_MSP_BUCKET_NAME: &[u8] = b"no-msp-bucket";
    pub const NO_MSP_BUCKET_ID: &[u8] = b"nmb1";

    /// BSP deletion test
    pub const TEST_BSP_ID: [u8; 32] =
        hex!("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");
    pub const TEST_BSP_ACCOUNT: &str = "5TestBspAccountAddressForDeletionTesting";
    pub const TEST_BSP_CAPACITY: i64 = 1000000;
    pub const TEST_BSP_STAKE: i64 = 50000;

    /// Not found test keys
    pub const NONEXISTENT_BUCKET_ID: &[u8] = b"nonexistent_bucket_id";
    pub const NONEXISTENT_FILE_KEY: &[u8] = b"non-existing-file-key";
}
