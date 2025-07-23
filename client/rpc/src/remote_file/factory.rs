use super::{
    ftp::FtpFileHandler, http::HttpFileHandler, local::LocalFileHandler, RemoteFileConfig,
    RemoteFileError, RemoteFileHandler,
};
use std::path::PathBuf;
use std::sync::Arc;
use url::Url;

pub struct RemoteFileHandlerFactory;

impl RemoteFileHandlerFactory {
    pub fn create(
        url: &Url,
        config: RemoteFileConfig,
    ) -> Result<(Arc<dyn RemoteFileHandler>, Url), RemoteFileError> {
        match url.scheme() {
            "" | "file" => LocalFileHandler::new(url, config)
                .map(|h| (Arc::new(h) as Arc<dyn RemoteFileHandler>, url.clone())),

            "http" | "https" => HttpFileHandler::new(config, url)
                .map(|h| (Arc::new(h) as Arc<dyn RemoteFileHandler>, url.clone()))
                .map_err(|e| {
                    RemoteFileError::Other(format!("Failed to create HTTP handler: {}", e))
                }),

            "ftp" | "ftps" => FtpFileHandler::new(config, url)
                .map(|h| (Arc::new(h) as Arc<dyn RemoteFileHandler>, url.clone())),

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

        Self::create(&url, config)
    }

    pub fn supported_protocols() -> &'static [&'static str] {
        &["file", "http", "https", "ftp", "ftps"]
    }
}
