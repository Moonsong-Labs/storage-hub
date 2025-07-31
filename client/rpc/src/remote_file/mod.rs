use async_trait::async_trait;
use thiserror::Error;
use tokio::io::AsyncRead;
use url::Url;

#[derive(Debug, Error)]
pub enum RemoteFileError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Protocol '{0}' is not supported")]
    UnsupportedProtocol(String),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("FTP error: {0}")]
    FtpError(#[from] suppaftp::FtpError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("File not found")]
    NotFound,

    #[error("Access denied")]
    AccessDenied,

    #[error("Operation timed out")]
    Timeout,

    #[error("{0}")]
    Other(String),
}

#[async_trait]
pub trait RemoteFileHandler: Send + Sync {
    async fn get_file_size(&self) -> Result<u64, RemoteFileError>;

    // TODO: add pagination?
    async fn download_file(&self) -> Result<Box<dyn AsyncRead + Send + Unpin>, RemoteFileError>;

    fn is_supported(&self, url: &Url) -> bool;

    // TODO: add pagination?
    async fn upload_file(
        &self,
        data: Box<dyn tokio::io::AsyncRead + Send + Unpin>,
        size: u64,
        content_type: Option<String>,
    ) -> Result<(), RemoteFileError>;
}

#[derive(Debug, Clone)]
pub struct RemoteFileConfig {
    pub max_file_size: u64,
    pub connection_timeout: u64,
    pub read_timeout: u64,
    pub follow_redirects: bool,
    pub max_redirects: u32,
    pub user_agent: String,
    pub chunk_size: usize,
    /// Number of FILE_CHUNK_SIZE chunks to buffer (minimum 1, default 512)
    pub chunks_buffer: usize,
}

impl RemoteFileConfig {
    /// Create a new config with explicit max_file_size
    pub fn new(max_file_size: u64) -> Self {
        Self {
            max_file_size,
            connection_timeout: 30,
            read_timeout: 300,
            follow_redirects: true,
            max_redirects: 10,
            user_agent: "StorageHub-Client/1.0".to_string(),
            chunk_size: 8192,   // 8KB default
            chunks_buffer: 512, // 512 FILE_CHUNK_SIZE chunks default (512KB)
        }
    }
}

impl Default for RemoteFileConfig {
    fn default() -> Self {
        Self::new(5 * 1024 * 1024 * 1024) // 5GB default
    }
}

pub mod factory;
pub mod ftp;
pub mod http;
pub mod local;

pub use factory::RemoteFileHandlerFactory;
