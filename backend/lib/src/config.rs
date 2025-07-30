use serde::{Deserialize, Serialize};

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
            port: 8080,
            storage_hub: StorageHubConfig {
                rpc_url: "ws://localhost:9944".to_string(),
                timeout_secs: Some(30),
                max_concurrent_requests: Some(100),
                verify_tls: true,
                #[cfg(feature = "mocks")]
                mock_mode: true,
            },
            database: DatabaseConfig {
                url: "postgres://localhost:5432/storage_hub".to_string(),
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
