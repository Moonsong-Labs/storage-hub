use crate::remote_file::{RemoteFileConfig, RemoteFileError, RemoteFileHandler};
use async_trait::async_trait;
use bytes::Bytes;
use std::io::Cursor;
use std::time::Duration;
use suppaftp::types::FileType;
use suppaftp::types::Response;
use suppaftp::{AsyncFtpStream, FtpError};
use tokio::io::AsyncRead;
use tokio_util::compat::TokioAsyncReadCompatExt;
use url::Url;

#[derive(Clone)]
pub struct FtpFileHandler {
    config: RemoteFileConfig,
}

impl FtpFileHandler {
    pub fn new(config: RemoteFileConfig) -> Self {
        Self { config }
    }

    pub fn default() -> Self {
        Self::new(RemoteFileConfig::default())
    }

    fn parse_url(
        url: &Url,
    ) -> Result<(String, u16, Option<String>, Option<String>, String), RemoteFileError> {
        let host = url
            .host_str()
            .ok_or_else(|| RemoteFileError::InvalidUrl("Missing host".to_string()))?;

        if host.is_empty() {
            return Err(RemoteFileError::InvalidUrl("Missing host".to_string()));
        }

        let host = host.to_string();

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

    async fn connect(&self, url: &Url) -> Result<AsyncFtpStream, RemoteFileError> {
        let (host, port, username, password, _) = Self::parse_url(url)?;

        let addr = format!("{}:{}", host, port);

        let connect_future = AsyncFtpStream::connect(&addr);
        let mut stream = tokio::time::timeout(
            Duration::from_secs(self.config.connection_timeout),
            connect_future,
        )
        .await
        .map_err(|_| RemoteFileError::Timeout)?
        .map_err(|e| RemoteFileError::FtpError(e))?;

        let (user, pass) = match (username, password) {
            (Some(u), Some(p)) => (u, p),
            (Some(u), None) => (u, String::new()),
            _ => ("anonymous".to_string(), String::new()),
        };

        stream.login(&user, &pass).await.map_err(|e| match e {
            FtpError::UnexpectedResponse(ref resp) if resp.status == 530.into() => {
                RemoteFileError::AccessDenied
            }
            _ => RemoteFileError::FtpError(e),
        })?;

        stream.set_mode(suppaftp::Mode::Passive);

        stream
            .transfer_type(FileType::Binary)
            .await
            .map_err(|e| RemoteFileError::FtpError(e))?;

        Ok(stream)
    }

    fn ftp_error_to_remote_error(error: FtpError) -> RemoteFileError {
        match error {
            FtpError::UnexpectedResponse(ref resp) => match resp.status {
                s if s == 550.into() => RemoteFileError::NotFound,
                s if s == 530.into() => RemoteFileError::AccessDenied,
                _ => RemoteFileError::FtpError(error),
            },
            _ => RemoteFileError::FtpError(error),
        }
    }

    pub async fn download(&self, url: &Url) -> Result<Vec<u8>, RemoteFileError> {
        let (_, _, _, _, path) = Self::parse_url(url)?;
        let mut stream = self.connect(url).await?;

        let size = stream
            .size(&path)
            .await
            .map_err(Self::ftp_error_to_remote_error)?;

        if size as u64 > self.config.max_file_size {
            return Err(RemoteFileError::Other(format!(
                "File size {} exceeds maximum allowed size {}",
                size, self.config.max_file_size
            )));
        }

        let data = tokio::time::timeout(
            Duration::from_secs(self.config.read_timeout),
            stream.retr(&path, |mut reader| {
                Box::pin(async move {
                    use futures_util::io::AsyncReadExt;
                    let mut buffer = Vec::new();
                    reader.read_to_end(&mut buffer).await.map_err(|e| {
                        FtpError::UnexpectedResponse(Response::new(
                            0.into(),
                            format!("IO error: {}", e).into_bytes(),
                        ))
                    })?;
                    Ok((buffer, reader))
                })
            }),
        )
        .await
        .map_err(|_| RemoteFileError::Timeout)?
        .map_err(Self::ftp_error_to_remote_error)?;

        let _ = stream.quit().await;

        Ok(data)
    }

    pub async fn upload(&self, url: &Url, data: &[u8]) -> Result<(), RemoteFileError> {
        let (_, _, _, _, path) = Self::parse_url(url)?;
        let mut stream = self.connect(url).await?;

        let cursor = Cursor::new(data);
        let mut compat_cursor = cursor.compat();

        stream
            .put_file(&path, &mut compat_cursor)
            .await
            .map_err(Self::ftp_error_to_remote_error)?;

        let _ = stream.quit().await;

        Ok(())
    }
}

#[async_trait]
impl RemoteFileHandler for FtpFileHandler {
    async fn fetch_metadata(&self, url: &Url) -> Result<(u64, Option<String>), RemoteFileError> {
        let (_, _, _, _, path) = Self::parse_url(url)?;
        let mut stream = self.connect(url).await?;

        let size = stream
            .size(&path)
            .await
            .map_err(Self::ftp_error_to_remote_error)?;

        if size as u64 > self.config.max_file_size {
            return Err(RemoteFileError::Other(format!(
                "File size {} exceeds maximum allowed size {}",
                size, self.config.max_file_size
            )));
        }

        let _ = stream.quit().await;

        Ok((size as u64, None))
    }

    async fn stream_file(
        &self,
        url: &Url,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>, RemoteFileError> {
        // For now, we'll download the entire file and wrap it in a cursor
        // TODO: Implement true streaming when suppaftp provides better async streaming support
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

        // Use REST command to set the starting position
        if offset > 0 {
            stream
                .resume_transfer(offset as usize)
                .await
                .map_err(Self::ftp_error_to_remote_error)?;
        }

        // Download the data starting from the offset
        let result = stream
            .retr(&path, |mut reader| {
                Box::pin(async move {
                    use futures_util::io::AsyncReadExt;
                    let mut buffer = vec![0u8; length as usize];
                    let bytes_read = reader.read(&mut buffer).await.map_err(|e| {
                        FtpError::UnexpectedResponse(Response::new(
                            0.into(),
                            format!("IO error: {}", e).into_bytes(),
                        ))
                    })?;
                    buffer.truncate(bytes_read);
                    Ok((buffer, reader))
                })
            })
            .await
            .map_err(Self::ftp_error_to_remote_error)?;

        // Reset REST position for future commands
        if offset > 0 {
            stream
                .resume_transfer(0)
                .await
                .map_err(Self::ftp_error_to_remote_error)?;
        }

        let _ = stream.quit().await;

        Ok(Bytes::from(result))
    }

    fn is_supported(&self, url: &Url) -> bool {
        matches!(url.scheme(), "ftp" | "ftps")
    }

    async fn upload_file(
        &self,
        url: &Url,
        mut data: Box<dyn tokio::io::AsyncRead + Send + Unpin>,
        size: u64,
        _content_type: Option<String>,
    ) -> Result<(), RemoteFileError> {
        if !self.is_supported(url) {
            return Err(RemoteFileError::UnsupportedProtocol(
                url.scheme().to_string(),
            ));
        }

        if size as u64 > self.config.max_file_size {
            return Err(RemoteFileError::Other(format!(
                "File size {} exceeds maximum allowed size {}",
                size, self.config.max_file_size
            )));
        }

        let (_, _, _, _, path) = Self::parse_url(url)?;
        let mut stream = self.connect(url).await?;

        // Use put_with_stream for streaming upload
        let mut upload_stream = stream
            .put_with_stream(&path)
            .await
            .map_err(Self::ftp_error_to_remote_error)?;

        // Stream data in chunks
        let mut buffer = vec![0u8; 8192]; // 8KB chunks
        loop {
            let n = tokio::io::AsyncReadExt::read(&mut data, &mut buffer)
                .await
                .map_err(|e| RemoteFileError::IoError(e))?;

            if n == 0 {
                break;
            }

            use futures_util::io::AsyncWriteExt;
            upload_stream
                .write_all(&buffer[..n])
                .await
                .map_err(|e| RemoteFileError::Other(format!("Write error: {}", e)))?;
        }

        // Finalize the upload
        stream
            .finalize_put_stream(upload_stream)
            .await
            .map_err(Self::ftp_error_to_remote_error)?;

        let _ = stream.quit().await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Current Testing Limitations:
    //
    // The FTP handler is tightly coupled to suppaftp::AsyncFtpStream, which makes
    // unit testing challenging without an actual FTP server. The current tests only
    // cover:
    // - URL parsing logic
    // - Protocol support checks
    // - Error handling for invalid inputs
    // - Size limit validation
    //
    // What we CANNOT test without a real/mock FTP server:
    // - Actual file downloads/uploads
    // - REST command for partial downloads
    // - Streaming functionality
    // - Connection timeouts
    // - Authentication failures
    // - Network errors
    //
    // To properly test the FTP handler, we would need one of:
    // 1. A mock FTP server library (not currently available in Rust ecosystem)
    // 2. Docker-based integration tests with a real FTP server
    // 3. Refactoring to use dependency injection for the FTP client

    fn create_test_handler() -> FtpFileHandler {
        let config = RemoteFileConfig {
            max_file_size: 1024 * 1024,
            connection_timeout: 5,
            read_timeout: 10,
            follow_redirects: false,
            max_redirects: 0,
            user_agent: "Test-Agent".to_string(),
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
    fn test_parse_url_edge_cases() {
        // Test with localhost
        let url = Url::parse("ftp://localhost/file.txt").unwrap();
        let (host, _, _, _, path) = FtpFileHandler::parse_url(&url).unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(path, "/file.txt");

        // Test with IP address
        let url = Url::parse("ftp://192.168.1.1:2121/file.txt").unwrap();
        let (host, port, _, _, _) = FtpFileHandler::parse_url(&url).unwrap();
        assert_eq!(host, "192.168.1.1");
        assert_eq!(port, 2121);
    }

    #[test]
    fn test_parse_url_invalid() {
        // URL without host should fail
        let url = Url::parse("ftp:///file.txt").unwrap();
        let result = FtpFileHandler::parse_url(&url);
        // This actually parses as host="file.txt", path="/"
        assert!(result.is_ok());
        if let Ok((host, _, _, _, path)) = result {
            assert_eq!(host, "file.txt");
            assert_eq!(path, "/");
        }
    }

    #[tokio::test]
    async fn test_upload_file_invalid_protocol() {
        let handler = create_test_handler();
        let url = Url::parse("http://example.com/upload.txt").unwrap();
        let data = b"test";
        let cursor = Cursor::new(data);
        let boxed_reader: Box<dyn AsyncRead + Send + Unpin> = Box::new(cursor);

        let result = handler
            .upload_file(&url, boxed_reader, data.len() as u64, None)
            .await;

        assert!(matches!(
            result,
            Err(RemoteFileError::UnsupportedProtocol(_))
        ));
    }

    #[tokio::test]
    async fn test_upload_file_size_limit() {
        let config = RemoteFileConfig {
            max_file_size: 10,
            ..RemoteFileConfig::default()
        };
        let handler = FtpFileHandler::new(config);
        let url = Url::parse("ftp://example.com/upload.txt").unwrap();
        let data = b"This is larger than 10 bytes";
        let cursor = Cursor::new(data);
        let boxed_reader: Box<dyn AsyncRead + Send + Unpin> = Box::new(cursor);

        let result = handler
            .upload_file(&url, boxed_reader, data.len() as u64, None)
            .await;

        assert!(result.is_err());
        if let Err(e) = result {
            match e {
                RemoteFileError::Other(msg) => {
                    assert!(msg.contains("exceeds maximum allowed size"))
                }
                _ => panic!("Expected Other error with size limit message, got {:?}", e),
            }
        }
    }

    #[tokio::test]
    async fn test_ftp_error_conversion() {
        use suppaftp::types::Response;

        // Test 550 error (file not found)
        let error = FtpError::UnexpectedResponse(Response::new(550.into(), vec![]));
        let converted = FtpFileHandler::ftp_error_to_remote_error(error);
        assert!(matches!(converted, RemoteFileError::NotFound));

        // Test 530 error (access denied)
        let error = FtpError::UnexpectedResponse(Response::new(530.into(), vec![]));
        let converted = FtpFileHandler::ftp_error_to_remote_error(error);
        assert!(matches!(converted, RemoteFileError::AccessDenied));

        // Test other errors
        let error = FtpError::UnexpectedResponse(Response::new(500.into(), vec![]));
        let converted = FtpFileHandler::ftp_error_to_remote_error(error);
        assert!(matches!(converted, RemoteFileError::FtpError(_)));
    }

    // Integration tests would go here, but they require an actual FTP server
    // For unit tests, we focus on testing the logic that doesn't require network access
}
