//! Configuration constants for the StorageHub backend

/// Default server configuration
pub mod server {
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
}

/// Test configuration values
#[cfg(test)]
pub mod test {
    /// RPC timeout for test environments (seconds)
    pub const RPC_TIMEOUT_SECS: u64 = 60;
    
    /// Maximum concurrent requests for tests
    pub const MAX_CONCURRENT_REQUESTS: usize = 200;
    
    /// Maximum database connections for tests
    pub const DB_MAX_CONNECTIONS: u32 = 3;
}

/// Counter service constants
pub mod counter {
    /// Default counter increment value
    pub const DEFAULT_INCREMENT: i64 = 1;
    
    /// Default counter key
    pub const DEFAULT_COUNTER_KEY: &str = "default";
}