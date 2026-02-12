use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::io::IsTerminal;
use tracing::warn;

use crate::constants::{
    api::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE},
    auth::{
        DEFAULT_AUTH_NONCE_EXPIRATION_SECONDS, DEFAULT_JWT_EXPIRY_OFFSET_MINUTES,
        DEFAULT_SIWE_DOMAIN,
    },
    database::DEFAULT_DATABASE_URL,
    download::MAX_DOWNLOAD_SESSIONS,
    rpc::{
        DEFAULT_MAX_CONCURRENT_REQUESTS, DEFAULT_MSP_CALLBACK_URL,
        DEFAULT_MSP_TRUSTED_FILE_TRANSFER_SERVER_URL, DEFAULT_RPC_URL, DEFAULT_TIMEOUT_SECS,
        DEFAULT_UPLOAD_RETRY_ATTEMPTS, DEFAULT_UPLOAD_RETRY_DELAY_SECS,
    },
    server::{DEFAULT_HOST, DEFAULT_PORT},
    upload::MAX_UPLOAD_SESSIONS,
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
    pub msp: MspConfig,
    pub database: DatabaseConfig,
    pub file_transfer: FileTransferConfig,
    #[serde(default)]
    pub node_health: NodeHealthConfig,
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
        std::env::var("SH_BACKEND_LOG_FORMAT")
            .ok()
            .and_then(|s| match s.to_lowercase().as_str() {
                "text" => Some(Self::Text),
                "json" => Some(Self::Json),
                "auto" => Some(Self::Auto),
                _ => {
                    warn!(value = %s, "Invalid SH_BACKEND_LOG_FORMAT value, using default");
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

/// File transfer configuration for upload and download session management
///
/// Controls the maximum number of concurrent file transfers to prevent resource
/// exhaustion and potential race conditions in file storage operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferConfig {
    /// Maximum number of concurrent file uploads allowed
    /// Prevents concurrent uploads of the same file key
    pub max_upload_sessions: usize,
    /// Maximum number of concurrent file downloads allowed
    pub max_download_sessions: usize,
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
    /// When enabled, uses mock RPC operations for testing
    #[cfg(feature = "mocks")]
    pub mock_mode: bool,
}

/// MSP-specific configuration
///
/// Configures MSP service behavior including upload retries and callback URLs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MspConfig {
    /// URL for the node to reach the MSP backend
    pub callback_url: String,
    /// URL for the MSP trusted file transfer server
    pub trusted_file_transfer_server_url: String,
    /// Number of retry attempts for file upload operations
    pub upload_retry_attempts: u32,
    /// Delay in seconds between file upload retry attempts
    pub upload_retry_delay_secs: u64,
    // TODO: Remove this field once legacy upload is deprecated
    /// If true, use legacy RPC-based upload method instead of trusted file transfer server
    pub use_legacy_upload_method: bool,
}

/// Node health monitoring configuration
///
/// Configures thresholds for the `/node-health` endpoint, which checks
/// whether the MSP node is operating correctly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHealthConfig {
    /// How many seconds the indexer's `updated_at` can lag behind `now`
    /// before it is considered stuck.
    pub indexer_stale_threshold_secs: u64,
    /// Maximum acceptable block lag between the indexer and the finalized chain head.
    pub indexer_lag_blocks_threshold: u64,
    /// Time window in seconds for counting recent storage requests.
    pub request_window_secs: u64,
    /// Minimum number of total requests in the window before the acceptance ratio matters.
    pub request_min_threshold: u64,
    /// How many seconds the on-chain nonce can remain unchanged (with pending extrinsics)
    /// before it is considered stuck.
    pub nonce_stuck_threshold_secs: u64,
}

impl Default for NodeHealthConfig {
    fn default() -> Self {
        Self {
            indexer_stale_threshold_secs: 120,
            indexer_lag_blocks_threshold: 10,
            request_window_secs: 600,
            request_min_threshold: 10,
            nonce_stuck_threshold_secs: 600,
        }
    }
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
                session_expiration_minutes: DEFAULT_JWT_EXPIRY_OFFSET_MINUTES,
                nonce_expiration_seconds: DEFAULT_AUTH_NONCE_EXPIRATION_SECONDS,
                siwe_domain: DEFAULT_SIWE_DOMAIN.to_string(),
            },
            storage_hub: StorageHubConfig {
                rpc_url: DEFAULT_RPC_URL.to_string(),
                timeout_secs: Some(DEFAULT_TIMEOUT_SECS),
                max_concurrent_requests: Some(DEFAULT_MAX_CONCURRENT_REQUESTS),
                verify_tls: true,
                #[cfg(feature = "mocks")]
                mock_mode: true,
            },
            msp: MspConfig {
                callback_url: DEFAULT_MSP_CALLBACK_URL.to_string(),
                trusted_file_transfer_server_url: DEFAULT_MSP_TRUSTED_FILE_TRANSFER_SERVER_URL
                    .to_string(),
                upload_retry_attempts: DEFAULT_UPLOAD_RETRY_ATTEMPTS,
                upload_retry_delay_secs: DEFAULT_UPLOAD_RETRY_DELAY_SECS,
                use_legacy_upload_method: false,
            },
            database: DatabaseConfig {
                url: DEFAULT_DATABASE_URL.to_string(),
                #[cfg(feature = "mocks")]
                mock_mode: true,
            },
            file_transfer: FileTransferConfig {
                max_upload_sessions: MAX_UPLOAD_SESSIONS,
                max_download_sessions: MAX_DOWNLOAD_SESSIONS,
            },
            node_health: NodeHealthConfig::default(),
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
