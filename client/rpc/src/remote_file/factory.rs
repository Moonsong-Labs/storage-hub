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
            "" | "file" => Ok(Arc::new(LocalFileHandler::new()) as Arc<dyn RemoteFileHandler>),

            "http" | "https" => HttpFileHandler::new(config)
                .map(|h| Arc::new(h) as Arc<dyn RemoteFileHandler>)
                .map_err(|e| {
                    RemoteFileError::Other(format!("Failed to create HTTP handler: {}", e))
                }),

            "ftp" | "ftps" => {
                Ok(Arc::new(FtpFileHandler::new(config)) as Arc<dyn RemoteFileHandler>)
            }

            scheme => Err(RemoteFileError::UnsupportedProtocol(scheme.to_string())),
        }
    }

    pub fn create_from_string(
        url_str: &str,
        config: RemoteFileConfig,
    ) -> Result<Arc<dyn RemoteFileHandler>, RemoteFileError> {
        let url = match Url::parse(url_str) {
            Ok(url) => url,
            Err(_) => {
                if url_str.starts_with('/')
                    || url_str.starts_with("./")
                    || url_str.starts_with("../")
                {
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
