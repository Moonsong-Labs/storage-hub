use serde::{Deserialize, Serialize};

use crate::constants::database::DEFAULT_DATABASE_URL;
use crate::constants::rpc::{
    DEFAULT_MAX_CONCURRENT_REQUESTS, DEFAULT_RPC_URL, DEFAULT_TIMEOUT_SECS,
};
use crate::constants::server::DEFAULT_PORT;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub storage_hub: StorageHubConfig,
    pub database: DatabaseConfig,
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
