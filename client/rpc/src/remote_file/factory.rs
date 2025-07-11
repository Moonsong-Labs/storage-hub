//! Factory for creating remote file handlers based on URL protocol

use super::{local::LocalFileHandler, RemoteFileConfig, RemoteFileHandler};
use std::sync::Arc;
use url::Url;

/// Factory for creating appropriate remote file handlers
pub struct RemoteFileHandlerFactory;

impl RemoteFileHandlerFactory {
    /// Create a new remote file handler based on the URL protocol
    pub fn create(url: &Url, _config: RemoteFileConfig) -> Option<Arc<dyn RemoteFileHandler>> {
        match url.scheme() {
            "" | "file" => Some(Arc::new(LocalFileHandler::new())),
            _ => None, // HTTP and FTP will be added in later steps
        }
    }
}
