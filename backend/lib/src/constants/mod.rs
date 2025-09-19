//! Configuration constants for the StorageHub backend

/// Test constants for use across all backend tests
#[cfg(test)]
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
    /// Default RPC request timeout in seconds
    pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

    /// Default maximum concurrent RPC requests
    pub const DEFAULT_MAX_CONCURRENT_REQUESTS: usize = 100;

    /// Default RPC WebSocket URL
    pub const DEFAULT_RPC_URL: &str = "ws://localhost:9944";

    /// Default MSP callback URL
    pub const DEFAULT_MSP_CALLBACK_URL: &str = "http://localhost:8080";

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

/// Auth configuration constants
pub mod auth {
    use chrono::Duration;

    /// The 'domain' to use for the SIWE message
    // TODO: make configurable
    pub const AUTH_SIWE_DOMAIN: &str = "localhost";

    /// Authentication nonce expiration, in seconds
    // TODO: make configurable
    pub const AUTH_NONCE_EXPIRATION_SECONDS: u64 = 300; // 5 minutes

    /// Authentication JWT token expiration
    // TODO: make configurable
    pub const JWT_EXPIRY_OFFSET: Duration = Duration::minutes(60 * 5); // 5 hours

    // TODO(MOCK): retrieve ens from token?
    pub const MOCK_ENS: &str = "user.eth";
}

pub mod mocks {
    pub const MOCK_ADDRESS: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
    /// Shared mock file content used by tests and RPC mocks
    pub const DOWNLOAD_FILE_CONTENT: &str = "GoodFla mock file content for download";
}
