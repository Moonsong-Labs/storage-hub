use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::constants::{
    api::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE},
    auth::{
        DEFAULT_AUTH_NONCE_EXPIRATION_SECONDS, DEFAULT_JWT_EXPIRY_OFFSET_MINUTES,
        DEFAULT_SIWE_DOMAIN,
    },
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
    pub api: ApiConfig,
    pub auth: AuthConfig,
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

    /// The expiration time (in minutes) of the user session
    ///
    /// Recommended a relatively short duration (10 minutes) to represent a typical user session with the backend
    pub session_expiration_minutes: usize,

    /// The expiration time (in seconds) for user nonces
    ///
    /// Recommended a short duration (a few minutes) to allow users to authenticate themselves,
    /// whilst also cleaning up abandoned sessions
    pub nonce_expiration_seconds: usize,

    /// The domain to use for the generated SIWE message
    ///
    /// Recommended to match the domain which this backend is reachable at
    pub siwe_domain: String,
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
            api: ApiConfig {
                default_page_size: DEFAULT_PAGE_SIZE,
                max_page_size: MAX_PAGE_SIZE,
            },
            auth: AuthConfig {
                jwt_secret: std::env::var("JWT_SECRET").ok().or_else(|| {
                    tracing::warn!("JWT_SECRET not set, using random secret for development");
                    Some(AuthConfig::generate_random_secret())
                }),
                #[cfg(feature = "mocks")]
                mock_mode: true,
                session_expiration_minutes: DEFAULT_JWT_EXPIRY_OFFSET_MINUTES,
                nonce_expiration_seconds: DEFAULT_AUTH_NONCE_EXPIRATION_SECONDS,
                siwe_domain: DEFAULT_SIWE_DOMAIN.to_string(),
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
