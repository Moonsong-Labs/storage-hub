use super::{
    ftp::FtpFileHandler, http::HttpFileHandler, local::LocalFileHandler, RemoteFileConfig,
    RemoteFileError, RemoteFileHandler,
};
use std::sync::Arc;
use url::Url;

pub struct RemoteFileHandlerFactory;

impl RemoteFileHandlerFactory {
    pub fn create(
        url: &Url,
        config: RemoteFileConfig,
    ) -> Result<Arc<dyn RemoteFileHandler>, RemoteFileError> {
        match url.scheme() {
            "" | "file" => LocalFileHandler::new(url, config)
                .map(|h| Arc::new(h) as Arc<dyn RemoteFileHandler>),

            "http" | "https" => HttpFileHandler::new(config, url)
                .map(|h| Arc::new(h) as Arc<dyn RemoteFileHandler>)
                .map_err(|e| {
                    RemoteFileError::Other(format!("Failed to create HTTP handler: {}", e))
                }),

            "ftp" | "ftps" => {
                FtpFileHandler::new(config, url).map(|h| Arc::new(h) as Arc<dyn RemoteFileHandler>)
            }

            scheme => Err(RemoteFileError::UnsupportedProtocol(scheme.to_string())),
        }
    }

    pub fn create_from_string(
        url_str: &str,
        config: RemoteFileConfig,
    ) -> Result<(Arc<dyn RemoteFileHandler>, Url), RemoteFileError> {
        // Try to parse as URL
        if let Ok(url) = Url::parse(url_str) {
            let handler = Self::create(&url, config)?;
            return Ok((handler, url));
        }

        // Check if this looks like a malformed URL (contains :// but failed to parse)
        if url_str.contains("://") {
            return Err(RemoteFileError::InvalidUrl(format!(
                "Invalid URL: {}",
                url_str
            )));
        }

        // Treat as local path - use local file handler and get canonical url from it
        let handler = LocalFileHandler::new_from_path(url_str, config)?;
        let canonical_url = handler.get_canonical_url()?;

        let handler = Arc::new(handler) as Arc<dyn RemoteFileHandler>;

        Ok((handler, canonical_url))
    }

    pub fn supported_protocols() -> &'static [&'static str] {
        &["file", "http", "https", "ftp", "ftps"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB for tests

    fn default_config() -> RemoteFileConfig {
        RemoteFileConfig::new(TEST_MAX_FILE_SIZE)
    }

    #[test]
    fn test_factory_creates_correct_handler_for_each_scheme() {
        let config = default_config();

        let test_cases = vec![
            ("http://example.com/file.txt", "http"),
            ("https://example.com/file.txt", "https"),
            ("ftp://example.com/file.txt", "ftp"),
            ("ftps://example.com/file.txt", "ftps"),
            ("file:///path/to/file.txt", "file"),
            // ensure backwards compatibility
            // scheme will be autodetected
            ("/foo.txt", "file"),
            ("foo.txt", "file"),
            ("./foo.txt", "file"),
            ("subdir/foo.txt", "file"),
            ("../foo.txt", "file"),
        ];

        for (url_str, expected_scheme) in test_cases {
            let (handler, url) =
                RemoteFileHandlerFactory::create_from_string(url_str, config.clone()).unwrap();

            assert!(
                handler.is_supported(&url),
                "Handler should support {} URLs",
                expected_scheme
            );
        }
    }

    #[test]
    fn test_factory_rejects_unsupported_schemes() {
        let config = default_config();

        let unsupported_schemes = vec![
            "sftp://example.com/file.txt",
            "ssh://example.com/file.txt",
            "smb://example.com/file.txt",
            "custom://example.com/file.txt",
            "invalid://example.com/file.txt",
        ];

        for url_str in unsupported_schemes {
            let url = Url::parse(url_str).unwrap();
            let result = RemoteFileHandlerFactory::create(&url, config.clone());

            assert!(
                matches!(result, Err(RemoteFileError::UnsupportedProtocol(_))),
                "Should reject unsupported scheme in URL: {}",
                url_str
            );
        }
    }

    #[test]
    fn test_factory_validates_url_format() {
        let config = default_config();

        let invalid_urls = vec!["", "://example.com", "http://[invalid"];

        for invalid_url in invalid_urls {
            let result = RemoteFileHandlerFactory::create_from_string(invalid_url, config.clone());

            assert!(
                matches!(result, Err(RemoteFileError::InvalidUrl(_))),
                "Should reject invalid URL: {}",
                invalid_url
            );
        }
    }
}
