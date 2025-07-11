//! Factory for creating remote file handlers based on URL protocol

use super::{http::HttpFileHandler, local::LocalFileHandler, RemoteFileConfig, RemoteFileHandler};
use std::sync::Arc;
use url::Url;

/// Factory for creating appropriate remote file handlers
pub struct RemoteFileHandlerFactory;

impl RemoteFileHandlerFactory {
    /// Create a new remote file handler based on the URL protocol
    pub fn create(url: &Url, config: RemoteFileConfig) -> Option<Arc<dyn RemoteFileHandler>> {
        match url.scheme() {
            "" | "file" => Some(Arc::new(LocalFileHandler::new())),
            "http" | "https" => HttpFileHandler::new(config)
                .ok()
                .map(|h| Arc::new(h) as Arc<dyn RemoteFileHandler>),
            _ => None, // FTP will be added in later steps
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_http_handler() {
        let config = RemoteFileConfig::default();
        let url = Url::parse("http://example.com/file.txt").unwrap();
        let handler = RemoteFileHandlerFactory::create(&url, config);
        assert!(handler.is_some());
        assert!(handler.unwrap().is_supported(&url));
    }

    #[test]
    fn test_create_https_handler() {
        let config = RemoteFileConfig::default();
        let url = Url::parse("https://example.com/file.txt").unwrap();
        let handler = RemoteFileHandlerFactory::create(&url, config);
        assert!(handler.is_some());
        assert!(handler.unwrap().is_supported(&url));
    }

    #[test]
    fn test_create_file_handler() {
        let config = RemoteFileConfig::default();
        let url = Url::parse("file:///tmp/file.txt").unwrap();
        let handler = RemoteFileHandlerFactory::create(&url, config);
        assert!(handler.is_some());
        assert!(handler.unwrap().is_supported(&url));
    }

    #[test]
    fn test_unsupported_protocol() {
        let config = RemoteFileConfig::default();
        let url = Url::parse("ftp://example.com/file.txt").unwrap();
        let handler = RemoteFileHandlerFactory::create(&url, config);
        assert!(handler.is_none());
    }
}
