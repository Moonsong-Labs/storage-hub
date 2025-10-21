use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::io::IsTerminal;
use tracing::warn;

use crate::constants::{
    api::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE},
    database::DEFAULT_DATABASE_URL,
    rpc::{
        DEFAULT_MAX_CONCURRENT_REQUESTS, DEFAULT_MSP_CALLBACK_URL, DEFAULT_RPC_URL,
        DEFAULT_TIMEOUT_SECS,
    },
    server::{DEFAULT_HOST, DEFAULT_PORT},
};

/// Backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// The backend will serve requests at this host
    pub host: String,
    /// The backend will serve requests at this port
    pub port: u16,
    /// Log format (text, json, or auto-detect)
    #[serde(default)]
    pub log_format: LogFormat,
    pub api: ApiConfig,
    pub auth: AuthConfig,
    pub storage_hub: StorageHubConfig,
    pub database: DatabaseConfig,
}

/// Log format configuration
///
/// This determines the format of the logs emitted by the backend.
///
/// The default value is `Auto`, which will select the format based on
/// whether the backend is running in a TTY (i.e. a terminal) or not.
///
/// The possible values are:
/// - `Text`: Human-readable text format
/// - `Json`: JSON format for machine parsing
/// - `Auto`: Auto-detect based on TTY (JSON if non-TTY, Text if TTY)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// Human-readable text format (default for development)
    Text,
    /// JSON format for machine parsing (recommended for production)
    Json,
    /// Auto-detect based on TTY (JSON if non-TTY, Text if TTY)
    Auto,
}

impl Default for LogFormat {
    fn default() -> Self {
        Self::Auto
    }
}

impl LogFormat {
    /// Parse log format from environment variable
    pub fn from_env() -> Self {
        std::env::var("STORAGEHUB_LOG_FORMAT")
            .ok()
            .and_then(|s| match s.to_lowercase().as_str() {
                "text" => Some(Self::Text),
                "json" => Some(Self::Json),
                "auto" => Some(Self::Auto),
                _ => {
                    warn!(value = %s, "Invalid STORAGEHUB_LOG_FORMAT value, using default");
                    None
                }
            })
            .unwrap_or_default()
    }

    /// Resolve the actual format to use
    pub fn resolve(&self) -> LogFormat {
        match self {
            Self::Auto => {
                // Use JSON if output is not a TTY (e.g., piped, redirected, or in production)
                if std::io::stdout().is_terminal() {
                    Self::Text
                } else {
                    Self::Json
                }
            }
            format => *format,
        }
    }
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

/// Authentication configuration for JWT tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// JWT secret key for signing and verifying tokens
    /// Must be at least 32 bytes for HS256 algorithm
    /// Can be set in config or loaded from JWT_SECRET environment variable
    /// Environment variable takes precedence over config value
    pub jwt_secret: Option<String>,

    /// When enabled, do not verify JWT signature
    #[cfg(feature = "mocks")]
    pub mock_mode: bool,
}

impl AuthConfig {
    /// Generate a random JWT secret for development/testing
    pub(crate) fn generate_random_secret() -> String {
        let mut data = [0u8; 32];

        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut data);

        hex::encode(data)
    }
}

/// StorageHub RPC configuration for blockchain interaction
///
/// Configures the connection and behavior parameters for communicating
/// with the StorageHub parachain node via JSON-RPC interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageHubConfig {
    /// WebSocket URL for the StorageHub RPC endpoint
    /// (e.g., `ws://localhost:9944`)
    pub rpc_url: String,
    /// Request timeout in seconds for RPC calls
    pub timeout_secs: Option<u64>,
    /// Maximum number of concurrent RPC requests allowed
    pub max_concurrent_requests: Option<usize>,
    /// Whether to verify TLS certificates for secure connections
    pub verify_tls: bool,
    /// URL for the node to reach the MSP backend
    pub msp_callback_url: String,
    /// When enabled, uses mock RPC operations for testing
    #[cfg(feature = "mocks")]
    pub mock_mode: bool,
}

/// Database configuration for PostgreSQL connection
///
/// Manages the connection parameters for the PostgreSQL database
/// where blockchain data is indexed and stored for efficient querying.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL in the format:
    /// `postgresql://[user[:password]@][host][:port][/dbname]`
    pub url: String,
    /// When enabled, uses mock database operations for testing
    #[cfg(feature = "mocks")]
    pub mock_mode: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
            log_format: LogFormat::from_env(),
            api: ApiConfig {
                default_page_size: DEFAULT_PAGE_SIZE,
                max_page_size: MAX_PAGE_SIZE,
            },
            auth: AuthConfig {
                jwt_secret: std::env::var("JWT_SECRET").ok().or_else(|| {
                    warn!("JWT_SECRET not set, using random secret for development");
                    Some(AuthConfig::generate_random_secret())
                }),
                #[cfg(feature = "mocks")]
                mock_mode: true,
            },
            storage_hub: StorageHubConfig {
                rpc_url: DEFAULT_RPC_URL.to_string(),
                timeout_secs: Some(DEFAULT_TIMEOUT_SECS),
                max_concurrent_requests: Some(DEFAULT_MAX_CONCURRENT_REQUESTS),
                verify_tls: true,
                msp_callback_url: DEFAULT_MSP_CALLBACK_URL.to_string(),
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
        let mut config: Self = toml::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Override JWT secret with environment variable if present
        if let Ok(jwt_secret) = std::env::var("JWT_SECRET") {
            config.auth.jwt_secret = Some(jwt_secret);
        }

        Ok(config)
    }
}

#[cfg(test)]
impl Config {
    /// Helper to get a DecodingKey
    pub fn get_jwt_key(&self) -> jsonwebtoken::DecodingKey {
        let jwt_secret = self
            .auth
            .jwt_secret
            .as_ref()
            .expect("JWT secret should be set in tests");
        jsonwebtoken::DecodingKey::from_secret(jwt_secret.as_bytes())
    }
}
