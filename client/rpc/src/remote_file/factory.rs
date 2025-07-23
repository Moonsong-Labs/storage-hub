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
        let url = match Url::parse(url_str) {
            Ok(url) => url,
            Err(_) => {
                // Try to parse as a local file path
                {
                    // Validate that we have a non-empty string
                    if url_str.is_empty() {
                        return Err(RemoteFileError::InvalidUrl("Empty path".to_string()));
                    }

                    // Check if this looks like a malformed URL (contains :// but failed to parse)
                    if url_str.contains("://") {
                        return Err(RemoteFileError::InvalidUrl(format!(
                            "Invalid URL: {}",
                            url_str
                        )));
                    }

                    // Accept any non-URL string as a local path and create a simple file URL
                    // The LocalFileHandler will handle path resolution and validation
                    Url::parse(&format!("file://{}", url_str)).map_err(|_| {
                        RemoteFileError::InvalidUrl(format!(
                            "Unable to convert given URL to a valid file URL"
                        ))
                    })?
                }
            }
        };

        let handler = Self::create(&url, config)?;
        Ok((handler, url))
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
        ];

        for (url_str, expected_scheme) in test_cases {
            let url = Url::parse(url_str).unwrap();
            let handler = RemoteFileHandlerFactory::create(&url, config.clone()).unwrap();

            assert!(
                handler.is_supported(&url),
                "Handler should support {} URLs",
                expected_scheme
            );
        }
    }

    #[test]
    fn test_factory_handles_path_without_scheme() {
        use tempfile::TempDir;
        let config = default_config();

        // Create a unique temporary directory
        let temp_dir = TempDir::new().unwrap();
        let temp_file = temp_dir.path().join("test_file.txt");
        std::fs::write(&temp_file, b"test").unwrap();

        let path = temp_file.to_str().unwrap();
        let (_handler, _url) =
            RemoteFileHandlerFactory::create_from_string(path, config.clone()).unwrap();
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
