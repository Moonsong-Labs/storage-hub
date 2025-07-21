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
            "" | "file" => LocalFileHandler::new(url)
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
        Self::create_from_string_with_mode(url_str, config, false)
    }

    pub fn create_from_string_for_write(
        url_str: &str,
        config: RemoteFileConfig,
    ) -> Result<(Arc<dyn RemoteFileHandler>, Url), RemoteFileError> {
        Self::create_from_string_with_mode(url_str, config, true)
    }

    fn create_from_string_with_mode(
        url_str: &str,
        config: RemoteFileConfig,
        _for_write: bool,
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

                    // Accept any non-URL string as a local path (absolute, relative, or bare file names)

                    // Validate local file permissions before creating the URL
                    let path = PathBuf::from(url_str);

                    // Check if the file exists and is readable OR if we can create it
                    if path.exists() {
                        // Check if we can read the existing file
                        std::fs::File::open(&path).map_err(|e| {
                            // Preserve original IO errors to maintain OS error messages
                            RemoteFileError::IoError(e)
                        })?;
                    } else {
                        // Check if we can create the file (test write permissions on parent directory)
                        let parent = match path.parent() {
                            Some(p) if !p.as_os_str().is_empty() => p,
                            _ => std::path::Path::new("."),
                        };
                        // Only validate parent directory permissions if it exists
                        if parent.exists() {
                            let metadata = std::fs::metadata(parent)
                                .map_err(|e| RemoteFileError::IoError(e))?;
                            if metadata.permissions().readonly() {
                                return Err(RemoteFileError::AccessDenied);
                            }
                        }
                    }

                    // Always use absolute paths for file:// URLs
                    let abs_path = if path.is_absolute() {
                        path
                    } else {
                        std::env::current_dir()
                            .map_err(|e| RemoteFileError::IoError(e))?
                            .join(&path)
                    };
                    Url::from_file_path(&abs_path).map_err(|_| {
                        RemoteFileError::InvalidUrl(format!(
                            "Could not convert '{}' to file URL",
                            abs_path.display()
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
