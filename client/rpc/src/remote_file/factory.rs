//! Factory for creating remote file handlers based on URL protocol

use super::{
    ftp::FtpFileHandler, http::HttpFileHandler, local::LocalFileHandler, RemoteFileConfig,
    RemoteFileError, RemoteFileHandler,
};
use std::sync::Arc;
use url::Url;

/// Factory for creating appropriate remote file handlers
pub struct RemoteFileHandlerFactory;

impl RemoteFileHandlerFactory {
    /// Create a new remote file handler based on the URL protocol
    ///
    /// # Arguments
    /// * `url` - The URL to create a handler for
    /// * `config` - Configuration for the handler
    ///
    /// # Returns
    /// * `Ok(Arc<dyn RemoteFileHandler>)` - The appropriate handler for the URL
    /// * `Err(RemoteFileError)` - If the URL scheme is unsupported or handler creation fails
    pub fn create(
        url: &Url,
        config: RemoteFileConfig,
    ) -> Result<Arc<dyn RemoteFileHandler>, RemoteFileError> {
        match url.scheme() {
            // Local file handler for file:// URLs or paths without scheme
            "" | "file" => Ok(Arc::new(LocalFileHandler::new()) as Arc<dyn RemoteFileHandler>),

            // HTTP/HTTPS handler
            "http" | "https" => HttpFileHandler::new(config)
                .map(|h| Arc::new(h) as Arc<dyn RemoteFileHandler>)
                .map_err(|e| {
                    RemoteFileError::Other(format!("Failed to create HTTP handler: {}", e))
                }),

            // FTP/FTPS handler
            "ftp" | "ftps" => {
                Ok(Arc::new(FtpFileHandler::new(config)) as Arc<dyn RemoteFileHandler>)
            }

            // Unsupported protocol
            scheme => Err(RemoteFileError::UnsupportedProtocol(scheme.to_string())),
        }
    }

    /// Create a handler from a string URL
    ///
    /// This is a convenience method that parses the URL string first.
    /// If the string is a plain path without a scheme, it will be treated as a file:// URL.
    ///
    /// # Arguments
    /// * `url_str` - The URL string to parse and create a handler for
    /// * `config` - Configuration for the handler
    ///
    /// # Returns
    /// * `Ok(Arc<dyn RemoteFileHandler>)` - The appropriate handler for the URL
    /// * `Err(RemoteFileError)` - If the URL is invalid or handler creation fails
    pub fn create_from_string(
        url_str: &str,
        config: RemoteFileConfig,
    ) -> Result<Arc<dyn RemoteFileHandler>, RemoteFileError> {
        // Try to parse as URL first
        let url = match Url::parse(url_str) {
            Ok(url) => url,
            Err(_) => {
                // If it fails, check if it's a plain path
                if url_str.starts_with('/') || url_str.starts_with("./") || url_str.starts_with("../") {
                    // Convert to file:// URL
                    Url::parse(&format!("file://{}", url_str))
                        .map_err(|e| RemoteFileError::InvalidUrl(format!("{}: {}", url_str, e)))?
                } else {
                    return Err(RemoteFileError::InvalidUrl(format!("Invalid URL: {}", url_str)));
                }
            }
        };
        Self::create(&url, config)
    }

    /// Get the supported protocols
    pub fn supported_protocols() -> &'static [&'static str] {
        &["file", "http", "https", "ftp", "ftps"]
    }

    /// Check if a protocol is supported
    pub fn is_protocol_supported(scheme: &str) -> bool {
        matches!(scheme, "" | "file" | "http" | "https" | "ftp" | "ftps")
    }
}

// Unit tests for this module are in remote_file/tests.rs (factory_tests module)
// to keep all handler tests organized in one place
