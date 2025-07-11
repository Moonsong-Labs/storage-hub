//! Remote file handling module
//!
//! This module provides functionality for fetching files from remote locations
//! during the storage deal processing. It supports HTTP/HTTPS and FTP protocols.

use async_trait::async_trait;
use bytes::Bytes;
use std::error::Error as StdError;
use std::fmt;
use tokio::io::AsyncRead;
use url::Url;

/// Error types for remote file operations
#[derive(Debug)]
pub enum RemoteFileError {
    /// Invalid URL format
    InvalidUrl(String),
    /// Unsupported protocol (only http, https, ftp are supported)
    UnsupportedProtocol(String),
    /// HTTP request failed
    HttpError(reqwest::Error),
    /// FTP operation failed
    FtpError(suppaftp::FtpError),
    /// IO operation failed
    IoError(std::io::Error),
    /// File not found at the remote location
    NotFound,
    /// Access denied to the remote resource
    AccessDenied,
    /// Remote server timeout
    Timeout,
    /// Other errors with custom message
    Other(String),
}

impl fmt::Display for RemoteFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl(url) => write!(f, "Invalid URL: {}", url),
            Self::UnsupportedProtocol(protocol) => {
                write!(f, "Unsupported protocol: {}", protocol)
            }
            Self::HttpError(e) => write!(f, "HTTP error: {}", e),
            Self::FtpError(e) => write!(f, "FTP error: {}", e),
            Self::IoError(e) => write!(f, "IO error: {}", e),
            Self::NotFound => write!(f, "File not found"),
            Self::AccessDenied => write!(f, "Access denied"),
            Self::Timeout => write!(f, "Operation timed out"),
            Self::Other(msg) => write!(f, "Remote file error: {}", msg),
        }
    }
}

impl StdError for RemoteFileError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::HttpError(e) => Some(e),
            Self::FtpError(e) => Some(e),
            Self::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for RemoteFileError {
    fn from(error: reqwest::Error) -> Self {
        RemoteFileError::HttpError(error)
    }
}

impl From<suppaftp::FtpError> for RemoteFileError {
    fn from(error: suppaftp::FtpError) -> Self {
        RemoteFileError::FtpError(error)
    }
}

impl From<std::io::Error> for RemoteFileError {
    fn from(error: std::io::Error) -> Self {
        RemoteFileError::IoError(error)
    }
}

/// Trait for handling remote file operations
#[async_trait]
pub trait RemoteFileHandler: Send + Sync {
    /// Fetch file metadata from a remote URL
    ///
    /// Returns the file size and optionally the content type
    async fn fetch_metadata(&self, url: &Url) -> Result<(u64, Option<String>), RemoteFileError>;

    /// Stream a file from a remote URL
    ///
    /// Returns an async reader that streams the file content
    async fn stream_file(
        &self,
        url: &Url,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>, RemoteFileError>;

    /// Download a file chunk from a remote URL
    ///
    /// # Arguments
    /// * `url` - The remote file URL
    /// * `offset` - Starting byte offset
    /// * `length` - Number of bytes to read
    ///
    /// Returns the requested bytes
    async fn download_chunk(
        &self,
        url: &Url,
        offset: u64,
        length: u64,
    ) -> Result<Bytes, RemoteFileError>;

    /// Validate if a URL is supported
    fn is_supported(&self, url: &Url) -> bool;
}

/// Configuration for remote file operations
#[derive(Debug, Clone)]
pub struct RemoteFileConfig {
    /// Maximum file size allowed (in bytes)
    pub max_file_size: u64,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Read timeout in seconds
    pub read_timeout: u64,
    /// Whether to follow redirects
    pub follow_redirects: bool,
    /// Maximum number of redirects to follow
    pub max_redirects: u32,
    /// User agent string for HTTP requests
    pub user_agent: String,
}

impl Default for RemoteFileConfig {
    fn default() -> Self {
        Self {
            max_file_size: 5 * 1024 * 1024 * 1024, // 5GB
            connection_timeout: 30,
            read_timeout: 300,
            follow_redirects: true,
            max_redirects: 10,
            user_agent: "StorageHub-Client/1.0".to_string(),
        }
    }
}

// Re-export handler implementations
pub mod factory;
pub mod ftp;
pub mod http;
pub mod local;

#[cfg(test)]
mod tests;

pub use factory::RemoteFileHandlerFactory;
