//! FTP remote file handler implementation

use crate::remote_file::{RemoteFileConfig, RemoteFileError, RemoteFileHandler};
use async_trait::async_trait;
use bytes::Bytes;
use std::io::Cursor;
use std::time::Duration;
use suppaftp::types::FileType;
use suppaftp::{AsyncFtpStream, FtpError};
use tokio::io::{AsyncRead, AsyncReadExt};
use url::Url;

/// FTP/FTPS file handler
pub struct FtpFileHandler {
    config: RemoteFileConfig,
}

impl FtpFileHandler {
    /// Create a new FTP file handler with the given configuration
    pub fn new(config: RemoteFileConfig) -> Self {
        Self { config }
    }

    /// Create a new FTP file handler with default configuration
    pub fn default() -> Self {
        Self::new(RemoteFileConfig::default())
    }

    /// Parse FTP URL and extract components
    fn parse_url(
        url: &Url,
    ) -> Result<(String, u16, Option<String>, Option<String>, String), RemoteFileError> {
        let host = url
            .host_str()
            .ok_or_else(|| RemoteFileError::InvalidUrl("Missing host".to_string()))?
            .to_string();

        let port = url.port().unwrap_or(21);

        let username = if url.username().is_empty() {
            None
        } else {
            Some(url.username().to_string())
        };

        let password = url.password().map(|p| p.to_string());

        let path = url.path().to_string();

        Ok((host, port, username, password, path))
    }

    /// Connect to FTP server
    async fn connect(&self, url: &Url) -> Result<AsyncFtpStream, RemoteFileError> {
        let (host, port, username, password, _) = Self::parse_url(url)?;

        // Create connection string
        let addr = format!("{}:{}", host, port);

        // Connect with timeout
        let connect_future = AsyncFtpStream::connect(&addr);
        let mut stream = tokio::time::timeout(
            Duration::from_secs(self.config.connection_timeout),
            connect_future,
        )
        .await
        .map_err(|_| RemoteFileError::Timeout)?
        .map_err(|e| RemoteFileError::FtpError(e))?;

        // Login
        let (user, pass) = match (username, password) {
            (Some(u), Some(p)) => (u, p),
            (Some(u), None) => (u, String::new()),
            _ => ("anonymous".to_string(), "anonymous@example.com".to_string()),
        };

        stream.login(&user, &pass).await.map_err(|e| match e {
            FtpError::UnexpectedResponse(ref resp) if resp.status == 530 => {
                RemoteFileError::AccessDenied
            }
            _ => RemoteFileError::FtpError(e),
        })?;

        // Set binary transfer mode
        stream
            .transfer_type(FileType::Binary)
            .await
            .map_err(|e| RemoteFileError::FtpError(e))?;

        Ok(stream)
    }

    /// Convert FTP error to RemoteFileError
    fn ftp_error_to_remote_error(error: FtpError) -> RemoteFileError {
        match error {
            FtpError::UnexpectedResponse(ref resp) => match resp.status {
                550 => RemoteFileError::NotFound,
                530 => RemoteFileError::AccessDenied,
                _ => RemoteFileError::FtpError(error),
            },
            _ => RemoteFileError::FtpError(error),
        }
    }

    /// Download file from FTP URL
    pub async fn download(&self, url: &Url) -> Result<Vec<u8>, RemoteFileError> {
        let (_, _, _, _, path) = Self::parse_url(url)?;
        let mut stream = self.connect(url).await?;

        // Get file size first
        let size = stream
            .size(&path)
            .await
            .map_err(Self::ftp_error_to_remote_error)?
            .ok_or_else(|| RemoteFileError::Other("Unable to determine file size".to_string()))?;

        if size > self.config.max_file_size {
            return Err(RemoteFileError::Other(format!(
                "File size {} exceeds maximum allowed size {}",
                size, self.config.max_file_size
            )));
        }

        // Retrieve file
        let data = tokio::time::timeout(
            Duration::from_secs(self.config.read_timeout),
            stream.retr_as_buffer(&path),
        )
        .await
        .map_err(|_| RemoteFileError::Timeout)?
        .map_err(Self::ftp_error_to_remote_error)?
        .into_inner();

        // Disconnect
        let _ = stream.quit().await;

        Ok(data)
    }

    /// Upload is not supported for FTP
    pub async fn upload(&self, _url: &Url, _data: &[u8]) -> Result<(), RemoteFileError> {
        Err(RemoteFileError::Other(
            "FTP upload is not implemented yet".to_string(),
        ))
    }
}

#[async_trait]
impl RemoteFileHandler for FtpFileHandler {
    async fn fetch_metadata(&self, url: &Url) -> Result<(u64, Option<String>), RemoteFileError> {
        let (_, _, _, _, path) = Self::parse_url(url)?;
        let mut stream = self.connect(url).await?;

        // Get file size
        let size = stream
            .size(&path)
            .await
            .map_err(Self::ftp_error_to_remote_error)?
            .ok_or_else(|| RemoteFileError::Other("Unable to determine file size".to_string()))?;

        if size > self.config.max_file_size {
            return Err(RemoteFileError::Other(format!(
                "File size {} exceeds maximum allowed size {}",
                size, self.config.max_file_size
            )));
        }

        // Disconnect
        let _ = stream.quit().await;

        // FTP doesn't provide content type information
        Ok((size, None))
    }

    async fn stream_file(
        &self,
        url: &Url,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>, RemoteFileError> {
        // For FTP, we need to download the entire file first
        // since suppaftp doesn't provide streaming interface
        let data = self.download(url).await?;
        let cursor = Cursor::new(data);

        Ok(Box::new(cursor) as Box<dyn AsyncRead + Send + Unpin>)
    }

    async fn download_chunk(
        &self,
        url: &Url,
        offset: u64,
        length: u64,
    ) -> Result<Bytes, RemoteFileError> {
        let (_, _, _, _, path) = Self::parse_url(url)?;
        let mut stream = self.connect(url).await?;

        // FTP REST command to set resume position
        stream
            .resume_transfer(offset)
            .await
            .map_err(|e| RemoteFileError::Other(format!("FTP REST command failed: {}", e)))?;

        // Retrieve file from offset
        let mut reader = stream
            .retr_as_stream(&path)
            .await
            .map_err(Self::ftp_error_to_remote_error)?;

        // Read only the requested length
        let mut buffer = vec![0u8; length as usize];
        let mut total_read = 0;

        while total_read < length as usize {
            let to_read = std::cmp::min(buffer.len() - total_read, 8192);
            let n = reader
                .read(&mut buffer[total_read..total_read + to_read])
                .await
                .map_err(|e| RemoteFileError::IoError(e))?;

            if n == 0 {
                break;
            }

            total_read += n;
        }

        buffer.truncate(total_read);

        // Finalize the transfer
        drop(reader);
        let _ = stream.finalize_retr_stream().await;

        // Disconnect
        let _ = stream.quit().await;

        Ok(Bytes::from(buffer))
    }

    fn is_supported(&self, url: &Url) -> bool {
        matches!(url.scheme(), "ftp" | "ftps")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_handler() -> FtpFileHandler {
        let config = RemoteFileConfig {
            max_file_size: 1024 * 1024, // 1MB for tests
            connection_timeout: 5,
            read_timeout: 10,
            follow_redirects: false,              // Not applicable for FTP
            max_redirects: 0,                     // Not applicable for FTP
            user_agent: "Test-Agent".to_string(), // Not used in FTP
        };
        FtpFileHandler::new(config)
    }

    #[test]
    fn test_is_supported() {
        let handler = create_test_handler();

        assert!(handler.is_supported(&Url::parse("ftp://example.com/file.txt").unwrap()));
        assert!(handler.is_supported(&Url::parse("ftps://example.com/file.txt").unwrap()));
        assert!(!handler.is_supported(&Url::parse("http://example.com/file.txt").unwrap()));
        assert!(!handler.is_supported(&Url::parse("file:///tmp/file.txt").unwrap()));
    }

    #[test]
    fn test_parse_url_anonymous() {
        let url = Url::parse("ftp://example.com/path/to/file.txt").unwrap();
        let (host, port, username, password, path) = FtpFileHandler::parse_url(&url).unwrap();

        assert_eq!(host, "example.com");
        assert_eq!(port, 21);
        assert_eq!(username, None);
        assert_eq!(password, None);
        assert_eq!(path, "/path/to/file.txt");
    }

    #[test]
    fn test_parse_url_with_auth() {
        let url = Url::parse("ftp://user:pass@example.com:2121/file.txt").unwrap();
        let (host, port, username, password, path) = FtpFileHandler::parse_url(&url).unwrap();

        assert_eq!(host, "example.com");
        assert_eq!(port, 2121);
        assert_eq!(username, Some("user".to_string()));
        assert_eq!(password, Some("pass".to_string()));
        assert_eq!(path, "/file.txt");
    }

    #[test]
    fn test_parse_url_with_username_only() {
        let url = Url::parse("ftp://user@example.com/file.txt").unwrap();
        let (host, port, username, password, path) = FtpFileHandler::parse_url(&url).unwrap();

        assert_eq!(host, "example.com");
        assert_eq!(port, 21);
        assert_eq!(username, Some("user".to_string()));
        assert_eq!(password, None);
        assert_eq!(path, "/file.txt");
    }

    #[test]
    fn test_parse_url_invalid() {
        let url = Url::parse("ftp:///file.txt").unwrap(); // Missing host
        let result = FtpFileHandler::parse_url(&url);

        assert!(matches!(result, Err(RemoteFileError::InvalidUrl(_))));
    }

    #[tokio::test]
    async fn test_upload_not_supported() {
        let handler = create_test_handler();
        let url = Url::parse("ftp://example.com/upload.txt").unwrap();
        let result = handler.upload(&url, b"data").await;

        assert!(matches!(result, Err(RemoteFileError::Other(_))));
        if let Err(RemoteFileError::Other(msg)) = result {
            assert_eq!(msg, "FTP upload is not implemented yet");
        }
    }

    // Note: The following tests would require a mock FTP server or test FTP server
    // In a real implementation, you might use a library like ftp-test-server or
    // set up a Docker container with an FTP server for integration tests

    #[tokio::test]
    #[ignore = "Requires FTP test server"]
    async fn test_fetch_metadata_success() {
        let handler = create_test_handler();
        let url = Url::parse("ftp://test.rebex.net/readme.txt").unwrap();
        let (size, content_type) = handler.fetch_metadata(&url).await.unwrap();

        assert!(size > 0);
        assert_eq!(content_type, None); // FTP doesn't provide content type
    }

    #[tokio::test]
    #[ignore = "Requires FTP test server"]
    async fn test_download_success() {
        let handler = create_test_handler();
        let url = Url::parse("ftp://test.rebex.net/readme.txt").unwrap();
        let data = handler.download(&url).await.unwrap();

        assert!(!data.is_empty());
    }

    #[tokio::test]
    #[ignore = "Requires FTP test server"]
    async fn test_stream_file_success() {
        let handler = create_test_handler();
        let url = Url::parse("ftp://test.rebex.net/readme.txt").unwrap();
        let mut reader = handler.stream_file(&url).await.unwrap();

        let mut buffer = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut reader, &mut buffer)
            .await
            .unwrap();

        assert!(!buffer.is_empty());
    }

    #[tokio::test]
    #[ignore = "Requires FTP test server"]
    async fn test_download_chunk_success() {
        let handler = create_test_handler();
        let url = Url::parse("ftp://test.rebex.net/readme.txt").unwrap();

        // Download first 100 bytes
        let chunk = handler.download_chunk(&url, 0, 100).await.unwrap();
        assert_eq!(chunk.len(), 100);

        // Download bytes 50-150
        let chunk2 = handler.download_chunk(&url, 50, 100).await.unwrap();
        assert_eq!(chunk2.len(), 100);

        // Verify overlap
        assert_eq!(&chunk[50..], &chunk2[..50]);
    }

    #[tokio::test]
    #[ignore = "Requires FTP test server"]
    async fn test_file_not_found() {
        let handler = create_test_handler();
        let url = Url::parse("ftp://test.rebex.net/nonexistent.txt").unwrap();
        let result = handler.fetch_metadata(&url).await;

        assert!(matches!(result, Err(RemoteFileError::NotFound)));
    }

    #[tokio::test]
    #[ignore = "Requires FTP test server"]
    async fn test_access_denied() {
        let handler = create_test_handler();
        // Try to access with wrong credentials
        let url = Url::parse("ftp://wronguser:wrongpass@test.rebex.net/readme.txt").unwrap();
        let result = handler.connect(&url).await;

        assert!(matches!(result, Err(RemoteFileError::AccessDenied)));
    }

    #[tokio::test]
    #[ignore = "Requires FTP test server"]
    async fn test_connection_timeout() {
        let config = RemoteFileConfig {
            connection_timeout: 1, // 1 second timeout
            ..RemoteFileConfig::default()
        };
        let handler = FtpFileHandler::new(config);

        // Use a non-routable IP to trigger timeout
        let url = Url::parse("ftp://192.0.2.1/file.txt").unwrap();
        let result = handler.connect(&url).await;

        assert!(matches!(result, Err(RemoteFileError::Timeout)));
    }

    #[tokio::test]
    #[ignore = "Requires FTP test server"]
    async fn test_file_too_large() {
        let config = RemoteFileConfig {
            max_file_size: 10, // Very small limit
            ..RemoteFileConfig::default()
        };
        let handler = FtpFileHandler::new(config);

        let url = Url::parse("ftp://test.rebex.net/readme.txt").unwrap();
        let result = handler.fetch_metadata(&url).await;

        assert!(matches!(result, Err(RemoteFileError::Other(_))));
        if let Err(RemoteFileError::Other(msg)) = result {
            assert!(msg.contains("exceeds maximum allowed size"));
        }
    }
}
