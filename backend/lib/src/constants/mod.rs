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

    /// Default MSP callback URL
    pub const DEFAULT_MSP_CALLBACK_URL: &str = "http://localhost:8080";

    /// Timeout multiplier for simulating network delays in mocks
    pub const TIMEOUT_MULTIPLIER: u64 = 10;

    /// Default number of retry attempts for RPC connection
    pub const DEFAULT_RPC_RETRY_ATTEMPTS: u32 = 30;

    /// Default delay between RPC connection retries in seconds
    pub const DEFAULT_RPC_RETRY_DELAY_SECS: u64 = 2;

    /// Default number of retry attempts for file upload operations
    pub const DEFAULT_UPLOAD_RETRY_ATTEMPTS: u32 = 3;

    /// Default delay between file upload retries in seconds
    pub const DEFAULT_UPLOAD_RETRY_DELAY_SECS: u64 = 1;
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

    /// MSP cache time-to-live in seconds
    pub const MSP_CACHE_TTL_SECS: u64 = 300; // 5 minutes
}

/// API configuration constants
pub mod api {
    /// Default page size for paginated API responses
    pub const DEFAULT_PAGE_SIZE: usize = 20;

    /// Maximum allowed page size for paginated API responses
    pub const MAX_PAGE_SIZE: usize = 100;
}

/// Auth configuration constants
pub mod auth {
    /// The endpoint for the nonce authentication
    ///
    /// This is here as a constant because it is used both in the
    /// routing of the API, and in the construction of the SIWE message.
    /// This way, if we change the endpoint, we only need to change it in one place.
    pub const AUTH_NONCE_ENDPOINT: &str = "/auth/nonce";

    /// The 'domain' to use for the SIWE message
    pub const DEFAULT_SIWE_DOMAIN: &str = "localhost";

    /// Default nonce expiration, in seconds
    pub const DEFAULT_AUTH_NONCE_EXPIRATION_SECONDS: usize = 300; // 5 minutes

    /// Default authentication JWT token expiration (in minutes)
    // TODO: temporarily set to 5 hours to extend sessions
    pub const DEFAULT_JWT_EXPIRY_OFFSET_MINUTES: usize = 60 * 5;

    // TODO(MOCK): retrieve ens from token?
    pub const MOCK_ENS: &str = "user.eth";
}

/// Retry and backoff configuration
pub mod retry {
    /// Stepped backoff delays (in seconds) for retry operations.
    /// Sequence: 1s → 2s → 5s → 10s → 15s → 20s → 60s → 90s → 150s → 240s
    pub const BACKOFF_DELAYS_SECS: &[u64] = &[1, 2, 5, 10, 15, 20, 60, 90, 150, 240];

    /// Maximum backoff delay (in seconds) for retry operations
    /// Used when all stepped delays have been exhausted.
    pub const MAX_BACKOFF_DELAY_SECS: u64 = 300; // 5 minutes

    /// Calculates the retry delay based on the attempt number using the stepped backoff strategy.
    pub fn get_retry_delay(attempt: u32) -> u64 {
        BACKOFF_DELAYS_SECS
            .get(attempt as usize)
            .copied()
            .unwrap_or(MAX_BACKOFF_DELAY_SECS)
    }
}

pub mod mocks {
    use alloy_core::primitives::{address, Address};

    /// The user address to mock
    pub const MOCK_ADDRESS: Address = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

    /// Shared mock file content used by tests and RPC mocks
    pub const DOWNLOAD_FILE_CONTENT: &str = "GoodFla mock file content for download";

    /// Mock price per giga unit
    pub const MOCK_PRICE_PER_GIGA_UNIT: u128 = 100;
}
