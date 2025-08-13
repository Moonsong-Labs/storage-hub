use serde::{Deserialize, Serialize};

use crate::constants::{
    api::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE},
    database::DEFAULT_DATABASE_URL,
    rpc::{DEFAULT_MAX_CONCURRENT_REQUESTS, DEFAULT_RPC_URL, DEFAULT_TIMEOUT_SECS},
    server::DEFAULT_PORT,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub api: ApiConfig,
    pub storage_hub: StorageHubConfig,
    pub database: DatabaseConfig,
}

/// API configuration for unified pagination and request handling
///
/// These values are used directly by database query methods in the postgres module
/// to enforce consistent pagination limits across all queries. When implementing
/// API endpoints, use these values to set defaults and enforce limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Default number of items per page in paginated responses
    pub default_page_size: usize,
    /// Maximum allowed page size for paginated responses
    pub max_page_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageHubConfig {
    pub rpc_url: String,
    pub timeout_secs: Option<u64>,
    pub max_concurrent_requests: Option<usize>,
    pub verify_tls: bool,
    #[cfg(feature = "mocks")]
    pub mock_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    #[cfg(feature = "mocks")]
    pub mock_mode: bool,
}

impl Default for Config {
    fn default() -> Self {
        // these are just some sane defaults, most likely we will
        // have them overridden
        Self {
            host: "127.0.0.1".to_string(),
            port: DEFAULT_PORT,
            api: ApiConfig {
                default_page_size: DEFAULT_PAGE_SIZE,
                max_page_size: MAX_PAGE_SIZE,
            },
            storage_hub: StorageHubConfig {
                rpc_url: DEFAULT_RPC_URL.to_string(),
                timeout_secs: Some(DEFAULT_TIMEOUT_SECS),
                max_concurrent_requests: Some(DEFAULT_MAX_CONCURRENT_REQUESTS),
                verify_tls: true,
                #[cfg(feature = "mocks")]
                mock_mode: true,
            },
            database: DatabaseConfig {
                url: DEFAULT_DATABASE_URL.to_string(),
                #[cfg(feature = "mocks")]
                mock_mode: true,
            },
        }
    }
}

impl Config {
    pub fn from_file(path: &str) -> std::io::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        toml::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}
