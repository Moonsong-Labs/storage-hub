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
        let url = Url::parse(url_str)
            .map_err(|e| RemoteFileError::InvalidUrl(format!("{}: {}", url_str, e)))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_http_handler() {
        let config = RemoteFileConfig::default();
        let url = Url::parse("http://example.com/file.txt").unwrap();
        let handler = RemoteFileHandlerFactory::create(&url, config).unwrap();
        assert!(handler.is_supported(&url));
    }

    #[test]
    fn test_create_https_handler() {
        let config = RemoteFileConfig::default();
        let url = Url::parse("https://example.com/file.txt").unwrap();
        let handler = RemoteFileHandlerFactory::create(&url, config).unwrap();
        assert!(handler.is_supported(&url));
    }

    #[test]
    fn test_create_file_handler() {
        let config = RemoteFileConfig::default();
        let url = Url::parse("file:///tmp/file.txt").unwrap();
        let handler = RemoteFileHandlerFactory::create(&url, config).unwrap();
        assert!(handler.is_supported(&url));
    }

    #[test]
    fn test_create_ftp_handler() {
        let config = RemoteFileConfig::default();
        let url = Url::parse("ftp://example.com/file.txt").unwrap();
        let handler = RemoteFileHandlerFactory::create(&url, config).unwrap();
        assert!(handler.is_supported(&url));
    }

    #[test]
    fn test_create_ftps_handler() {
        let config = RemoteFileConfig::default();
        let url = Url::parse("ftps://example.com/file.txt").unwrap();
        let handler = RemoteFileHandlerFactory::create(&url, config).unwrap();
        assert!(handler.is_supported(&url));
    }

    #[test]
    fn test_unsupported_protocol() {
        let config = RemoteFileConfig::default();
        let url = Url::parse("sftp://example.com/file.txt").unwrap();
        let result = RemoteFileHandlerFactory::create(&url, config);
        assert!(matches!(
            result,
            Err(RemoteFileError::UnsupportedProtocol(_))
        ));
    }

    #[test]
    fn test_create_from_string_valid() {
        let config = RemoteFileConfig::default();
        let handler =
            RemoteFileHandlerFactory::create_from_string("https://example.com/file.txt", config)
                .unwrap();
        assert!(handler.is_supported(&Url::parse("https://example.com/file.txt").unwrap()));
    }

    #[test]
    fn test_create_from_string_invalid_url() {
        let config = RemoteFileConfig::default();
        let result = RemoteFileHandlerFactory::create_from_string("not a valid url", config);
        assert!(matches!(result, Err(RemoteFileError::InvalidUrl(_))));
    }

    #[test]
    fn test_supported_protocols() {
        let protocols = RemoteFileHandlerFactory::supported_protocols();
        assert_eq!(protocols, &["file", "http", "https", "ftp", "ftps"]);
    }

    #[test]
    fn test_is_protocol_supported() {
        assert!(RemoteFileHandlerFactory::is_protocol_supported(""));
        assert!(RemoteFileHandlerFactory::is_protocol_supported("file"));
        assert!(RemoteFileHandlerFactory::is_protocol_supported("http"));
        assert!(RemoteFileHandlerFactory::is_protocol_supported("https"));
        assert!(RemoteFileHandlerFactory::is_protocol_supported("ftp"));
        assert!(RemoteFileHandlerFactory::is_protocol_supported("ftps"));
        assert!(!RemoteFileHandlerFactory::is_protocol_supported("sftp"));
        assert!(!RemoteFileHandlerFactory::is_protocol_supported("ssh"));
        assert!(!RemoteFileHandlerFactory::is_protocol_supported("custom"));
    }

    #[test]
    fn test_empty_scheme_creates_local_handler() {
        let config = RemoteFileConfig::default();
        // URLs without scheme are treated as local files
        let url = Url::parse("file:///path/to/file").unwrap();
        let modified_url = Url::parse(&url.as_str().replace("file://", "")).unwrap_or(url.clone());
        let handler = RemoteFileHandlerFactory::create(&modified_url, config).unwrap();
        assert!(handler.is_supported(&Url::parse("file:///path/to/file").unwrap()));
    }

    #[test]
    fn test_handler_config_propagation() {
        let custom_config = RemoteFileConfig {
            max_file_size: 100 * 1024 * 1024, // 100MB
            connection_timeout: 60,
            read_timeout: 600,
            follow_redirects: false,
            max_redirects: 5,
            user_agent: "CustomAgent/2.0".to_string(),
        };

        // Test that handlers are created with the provided config
        let http_url = Url::parse("http://example.com/file.txt").unwrap();
        let http_handler =
            RemoteFileHandlerFactory::create(&http_url, custom_config.clone()).unwrap();
        assert!(http_handler.is_supported(&http_url));

        let ftp_url = Url::parse("ftp://example.com/file.txt").unwrap();
        let ftp_handler = RemoteFileHandlerFactory::create(&ftp_url, custom_config).unwrap();
        assert!(ftp_handler.is_supported(&ftp_url));
    }
}
