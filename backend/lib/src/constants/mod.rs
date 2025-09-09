//! Configuration constants for the StorageHub backend

/// Test constants for use across all backend tests
#[cfg(any(feature = "mocks", test))]
pub mod test;

/// Default server configuration
pub mod server {
    /// Default HTTP listening host
    pub const DEFAULT_HOST: &str = "127.0.0.1";

    /// Default HTTP server port
    pub const DEFAULT_PORT: u16 = 8080;
}

/// RPC client configuration
pub mod rpc {
    use hex_literal::hex;

    /// Default RPC request timeout in seconds
    pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

    /// Default maximum concurrent RPC requests
    pub const DEFAULT_MAX_CONCURRENT_REQUESTS: usize = 100;

    /// Default RPC WebSocket URL
    pub const DEFAULT_RPC_URL: &str = "ws://localhost:9944";

    pub const DUMMY_MSP_ID: [u8; 32] =
        hex!("0000000000000000000000000000000000000000000000000000000000000300");

    /// Timeout multiplier for simulating network delays in mocks
    pub const TIMEOUT_MULTIPLIER: u64 = 10;
}

/// Database configuration
pub mod database {
    /// Default maximum database connections
    pub const DEFAULT_MAX_CONNECTIONS: u32 = 5;

    /// Default database connection timeout in seconds
    pub const DEFAULT_CONNECTION_TIMEOUT_SECS: u64 = 10;

    /// Default PostgreSQL database URL
    pub const DEFAULT_DATABASE_URL: &str = "postgres://localhost:5432/storage_hub";

    /// Default limit for requests with pagination
    pub const DEFAULT_PAGE_LIMIT: i64 = 100;
}

/// API configuration constants
pub mod api {
    /// Default page size for paginated API responses
    pub const DEFAULT_PAGE_SIZE: usize = 20;

    /// Maximum allowed page size for paginated API responses
    pub const MAX_PAGE_SIZE: usize = 100;
}

pub mod mocks {
    /// The user address to mock
    pub const MOCK_ADDRESS: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";

    // TODO: These are placeholder that are not indexed currently but we could compute.
    // For example, we could retrieve all files in the DB by bucket and compute it that way
    // using `File::get_by_onchain_bucket_id`

    pub const PLACEHOLDER_BUCKET_SIZE_BYTES: u64 = 0;
    pub const PLACEHOLDER_BUCKET_FILE_COUNT: u64 = 0;
}
