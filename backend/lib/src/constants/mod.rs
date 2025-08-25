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

/// Counter service constants
/// TODO(SCAFFOLDING): Counter constants are for demonstration only
/// Remove this entire module when implementing real MSP features
pub mod counter {
    /// Default counter increment value
    pub const DEFAULT_INCREMENT: i64 = 1;

    /// Default counter key
    pub const DEFAULT_COUNTER_KEY: &str = "default";
}
