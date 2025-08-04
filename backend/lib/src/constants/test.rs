//! Test constants for use across all backend tests
//!
//! This module provides centralized test data constants to ensure consistency
//! and clarity in tests. Using these constants prevents accidental mismatches
//! and makes it clear where test data originates from.

use serde_json;

/// Configuration constants for test environments
/// RPC timeout for test environments (seconds)
pub const RPC_TIMEOUT_SECS: u64 = 60;

/// Maximum concurrent requests for tests
pub const MAX_CONCURRENT_REQUESTS: usize = 200;

/// Maximum database connections for tests
pub const DB_MAX_CONNECTIONS: u32 = 3;

/// Test file keys for various scenarios
pub mod file_keys {
    /// Standard test file key
    pub const TEST_FILE_KEY: &[u8] = &[1, 2, 3];

    /// Alternative file key for testing multiple files
    pub const ALTERNATIVE_FILE_KEY: &[u8] = &[4, 5, 6];

    /// Empty file key for edge case testing
    pub const EMPTY_FILE_KEY: &[u8] = &[];
}

/// Test account and owner identifiers
pub mod accounts {
    /// Standard test owner account
    pub const TEST_OWNER: &[u8] = &[4, 5, 6];

    /// Test MSP account
    pub const TEST_MSP_ACCOUNT: &[u8] = &[10, 11, 12, 13];

    /// Test user account
    pub const TEST_USER_ACCOUNT: &[u8] = &[1, 2, 3];

    /// Alternative account for multi-account testing
    pub const ALTERNATIVE_ACCOUNT: &[u8] = &[50, 51, 52, 53];
}

/// Test bucket identifiers
pub mod buckets {
    /// Standard test bucket ID
    pub const TEST_BUCKET_ID: &[u8] = &[7, 8, 9];

    /// Alternative bucket ID
    pub const ALTERNATIVE_BUCKET_ID: &[u8] = &[30, 31, 32, 33];

    /// Test bucket name
    pub const TEST_BUCKET_NAME: &[u8] = &[110, 111, 112, 113];
}

/// Test file metadata
pub mod file_metadata {
    /// Test file location
    pub const TEST_LOCATION: &[u8] = &[10, 11, 12];

    /// Alternative location
    pub const ALTERNATIVE_LOCATION: &[u8] = &[7, 8, 9];

    /// Test file fingerprint
    pub const TEST_FINGERPRINT: &[u8] = &[13, 14, 15];

    /// Alternative fingerprint
    pub const ALTERNATIVE_FINGERPRINT: &[u8] = &[10, 11, 12];

    /// Standard test file size
    pub const TEST_FILE_SIZE: u64 = 1024;

    /// Large file size for testing
    pub const LARGE_FILE_SIZE: u64 = 2048;
}

/// Test peer identifiers
pub mod peers {
    /// First test peer ID
    pub const TEST_PEER_1: &[u8] = &[16, 17];

    /// Second test peer ID
    pub const TEST_PEER_2: &[u8] = &[18, 19];

    /// Alternative peer IDs for storage request testing
    pub const ALTERNATIVE_PEER_1: &[u8] = &[7, 8];
    pub const ALTERNATIVE_PEER_2: &[u8] = &[9, 10];
}

/// Test blockchain data
pub mod blockchain {
    /// Test block number
    pub const TEST_BLOCK_NUMBER: u64 = 12345;

    /// Alternative block number
    pub const ALTERNATIVE_BLOCK_NUMBER: u64 = 100;

    /// Test block hash
    pub const TEST_BLOCK_HASH: &[u8] = &[11, 12, 13];

    /// Test transaction hash
    pub const TEST_TX_HASH: &str = "0x1234567890abcdef";

    /// Test extrinsic index
    pub const TEST_EXTRINSIC_INDEX: u32 = 5;
}

/// Test MSP (Main Storage Provider) data
pub mod msp {
    /// Default MSP ID
    pub const DEFAULT_MSP_ID: i64 = 1;

    /// Test MSP onchain ID
    pub const TEST_MSP_ONCHAIN_ID: &[u8] = &[1, 2, 3, 4];

    /// Test MSP value proposition
    pub const TEST_MSP_VALUE_PROP: &[u8] = &[100, 101, 102];
}

/// Test merkle tree data
pub mod merkle {
    /// Test merkle root
    pub const TEST_MERKLE_ROOT: &[u8] = &[40, 41, 42, 43];
}

/// Test timestamps
pub mod timestamps {
    /// Standard test timestamp (2023-11-14 22:13:20 UTC)
    pub const TEST_TIMESTAMP: i64 = 1_700_000_000;
}

/// Helper functions for creating test data
pub mod helpers {
    use super::*;

    /// Creates a standard test file metadata response
    pub fn create_test_file_metadata() -> serde_json::Value {
        serde_json::json!({
            "owner": accounts::TEST_OWNER,
            "bucket_id": buckets::TEST_BUCKET_ID,
            "location": file_metadata::TEST_LOCATION,
            "fingerprint": file_metadata::TEST_FINGERPRINT,
            "size": file_metadata::TEST_FILE_SIZE,
            "peer_ids": [peers::TEST_PEER_1, peers::TEST_PEER_2]
        })
    }

    /// Creates a test transaction receipt
    pub fn create_test_transaction_receipt() -> serde_json::Value {
        serde_json::json!({
            "block_hash": blockchain::TEST_BLOCK_HASH,
            "block_number": blockchain::ALTERNATIVE_BLOCK_NUMBER,
            "extrinsic_index": blockchain::TEST_EXTRINSIC_INDEX,
            "success": true
        })
    }

    /// Creates test peer IDs vector
    pub fn create_test_peer_ids() -> Vec<Vec<u8>> {
        vec![
            peers::ALTERNATIVE_PEER_1.to_vec(),
            peers::ALTERNATIVE_PEER_2.to_vec(),
        ]
    }
}
