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
}

/// Test placeholder IDs - properly formatted 32-byte hex strings for testing
pub mod placeholder_ids {
    use hex_literal::hex;

    // Bucket IDs (32 bytes each)
    pub const NONEXISTENT_BUCKET_ID: &[u8; 32] =
        &hex!("0000000000000000000000000000000000000000000000000000000000000000");
    pub const OTHER_BUCKET_ID: &[u8; 32] =
        &hex!("1111111111111111111111111111111111111111111111111111111111111111");
    pub const BUCKET1_ID: &[u8; 32] =
        &hex!("2222222222222222222222222222222222222222222222222222222222222222");
    pub const BUCKET2_ID: &[u8; 32] =
        &hex!("3333333333333333333333333333333333333333333333333333333333333333");
    pub const BUCKET3_ID: &[u8; 32] =
        &hex!("4444444444444444444444444444444444444444444444444444444444444444");
    pub const USER_BUCKET_ID: &[u8; 32] =
        &hex!("5555555555555555555555555555555555555555555555555555555555555555");
    pub const OTHER1_ID: &[u8; 32] =
        &hex!("6666666666666666666666666666666666666666666666666666666666666666");
    pub const OTHER2_ID: &[u8; 32] =
        &hex!("7777777777777777777777777777777777777777777777777777777777777777");
    pub const MSP1_BUCKET_ID: &[u8; 32] =
        &hex!("8888888888888888888888888888888888888888888888888888888888888888");
    pub const MSP2_BUCKET_ID: &[u8; 32] =
        &hex!("9999999999999999999999999999999999999999999999999999999999999999");
    pub const WITH_MSP_ID: &[u8; 32] =
        &hex!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    pub const NO_MSP_ID: &[u8; 32] =
        &hex!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");

    // Additional bucket IDs for postgres tests
    pub const ADDITIONAL_BUCKET_ID: &[u8; 32] =
        &hex!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
    pub const EMPTY_BUCKET_ID: &[u8; 32] =
        &hex!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");
    pub const USER_BUCKET2_ID: &[u8; 32] =
        &hex!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
    pub const USER_BUCKET3_ID: &[u8; 32] =
        &hex!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    pub const PAGINATION_BUCKET2_ID: &[u8; 32] =
        &hex!("1010101010101010101010101010101010101010101010101010101010101010");
    pub const PAGINATION_BUCKET3_ID: &[u8; 32] =
        &hex!("2020202020202020202020202020202020202020202020202020202020202020");
    pub const OTHER_USER_BUCKET_ID: &[u8; 32] =
        &hex!("3030303030303030303030303030303030303030303030303030303030303030");
    pub const MSP1_USER_BUCKET_ID: &[u8; 32] =
        &hex!("4040404040404040404040404040404040404040404040404040404040404040");
    pub const NO_MSP_BUCKET_ID: &[u8; 32] =
        &hex!("5050505050505050505050505050505050505050505050505050505050505050");

    // File keys (32 bytes each)
    pub const NONEXISTENT_FILE_KEY: &[u8; 32] =
        &hex!("abababababababababababababababababababababababababababababababab");
    pub const TEST_FILE_KEY1: &[u8; 32] =
        &hex!("0101010101010101010101010101010101010101010101010101010101010101");
    pub const TEST_FILE_KEY2: &[u8; 32] =
        &hex!("0202020202020202020202020202020202020202020202020202020202020202");
    pub const TEST_FILE_KEY3: &[u8; 32] =
        &hex!("0303030303030303030303030303030303030303030303030303030303030303");
    pub const TEST_FILE_KEY4: &[u8; 32] =
        &hex!("0404040404040404040404040404040404040404040404040404040404040404");
    pub const TEST_FILE_KEY5: &[u8; 32] =
        &hex!("0505050505050505050505050505050505050505050505050505050505050505");
    pub const TEST_FILE_KEY6: &[u8; 32] =
        &hex!("0606060606060606060606060606060606060606060606060606060606060606");
    pub const TEST_FILE_KEY7: &[u8; 32] =
        &hex!("0707070707070707070707070707070707070707070707070707070707070707");

    // File fingerprints
    pub const TEST_FILE_FINGERPRINT: &[u8; 32] =
        &hex!("0000000000000000000000000000000000000000000000000000000000000002");

    // Test account IDs (32 bytes)
    pub const TEST_FILE_ACCOUNT: &[u8; 32] =
        &hex!("20d81e86ed5b986d1d6ddbe416627f96f740252c4a80ab8ed91db58f7ecf9657");
    pub const TEST_FILE_KEY: &[u8; 32] =
        &hex!("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890");
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
