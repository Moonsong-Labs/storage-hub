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
            "" | "file" => Ok((
                Arc::new(LocalFileHandler::new()) as Arc<dyn RemoteFileHandler>,
                url.clone(),
            )),

            "http" | "https" => HttpFileHandler::new(config)
                .map(|h| (Arc::new(h) as Arc<dyn RemoteFileHandler>, url.clone()))
                .map_err(|e| {
                    RemoteFileError::Other(format!("Failed to create HTTP handler: {}", e))
                }),

            "ftp" | "ftps" => Ok((
                Arc::new(FtpFileHandler::new(config)) as Arc<dyn RemoteFileHandler>,
                url.clone(),
            )),

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
                // Handle local paths
                if url_str.starts_with('/')
                    || url_str.starts_with("./")
                    || url_str.starts_with("../")
                {
                    // Validate local file permissions before creating the URL
                    let path = PathBuf::from(url_str);

                    // Check if the file exists and is readable OR if we can create it
                    if path.exists() {
                        // Check if we can read the existing file
                        std::fs::File::open(&path).map_err(|e| match e.kind() {
                            std::io::ErrorKind::PermissionDenied => RemoteFileError::AccessDenied,
                            std::io::ErrorKind::NotFound => RemoteFileError::NotFound,
                            _ => RemoteFileError::IoError(e),
                        })?;
                    } else {
                        // Check if we can create the file (test write permissions on parent directory)
                        if let Some(parent) = path.parent() {
                            if !parent.exists() {
                                return Err(RemoteFileError::InvalidUrl(format!(
                                    "Parent directory does not exist for path: {}",
                                    url_str
                                )));
                            }
                            // Test write permissions by checking if parent is writable
                            let metadata = std::fs::metadata(parent)
                                .map_err(|e| RemoteFileError::IoError(e))?;
                            if metadata.permissions().readonly() {
                                return Err(RemoteFileError::AccessDenied);
                            }
                        }
                    }

                    Url::parse(&format!("file://{}", url_str))
                        .map_err(|e| RemoteFileError::InvalidUrl(format!("{}: {}", url_str, e)))?
                } else {
                    return Err(RemoteFileError::InvalidUrl(format!(
                        "Invalid URL: {}",
                        url_str
                    )));
                }
            }
        };
        Self::create(&url, config)
    }

    pub fn supported_protocols() -> &'static [&'static str] {
        &["file", "http", "https", "ftp", "ftps"]
    }
}
