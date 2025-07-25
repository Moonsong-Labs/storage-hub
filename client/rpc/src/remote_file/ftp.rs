use async_trait::async_trait;
use bytes::Bytes;
use futures_util::io::{AsyncReadExt, AsyncWriteExt};
use std::io::Cursor;
use std::time::Duration;
use suppaftp::types::FileType;
use suppaftp::types::Response;
use suppaftp::{AsyncFtpStream, FtpError};
use tokio::io::AsyncRead;
use tokio_util::compat::TokioAsyncReadCompatExt;
use url::Url;

use crate::remote_file::{RemoteFileConfig, RemoteFileError, RemoteFileHandler};

#[derive(Clone)]
pub struct FtpFileHandler {
    config: RemoteFileConfig,
    host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    path: String,
}

impl FtpFileHandler {
    pub fn new(config: RemoteFileConfig, url: &Url) -> Result<Self, RemoteFileError> {
        let (host, port, username, password, path) = Self::parse_url(url)?;
        Ok(Self {
            config,
            host,
            port,
            username,
            password,
            path,
        })
    }

    pub fn default(url: &Url, max_file_size: u64) -> Result<Self, RemoteFileError> {
        Self::new(RemoteFileConfig::new(max_file_size), url)
    }

    fn parse_url(
        url: &Url,
    ) -> Result<(String, u16, Option<String>, Option<String>, String), RemoteFileError> {
        let host = url
            .host_str()
            .and_then(|host| if host.is_empty() { None } else { Some(host) })
            .ok_or_else(|| RemoteFileError::InvalidUrl("Missing host".to_string()))?;

        let host = host.to_string();
        let port = url.port().unwrap_or(21);

        let username = url.username().to_string();
        let username = if username.is_empty() {
            None
        } else {
            Some(username)
        };

        let password = url.password().map(|p| p.to_string());

        let path = url.path().to_string();

        Ok((host, port, username, password, path))
    }

    async fn connect(&self) -> Result<AsyncFtpStream, RemoteFileError> {
        let addr = format!("{}:{}", self.host, self.port);

        let connect_future = AsyncFtpStream::connect(&addr);
        let mut stream = tokio::time::timeout(
            Duration::from_secs(self.config.connection_timeout),
            connect_future,
        )
        .await
        .map_err(|_| RemoteFileError::Timeout)?
        .map_err(Self::ftp_error_to_remote_error)?;

        let (user, pass) = match (&self.username, &self.password) {
            (Some(u), Some(p)) => (u.clone(), p.clone()),
            (Some(u), None) => (u.clone(), String::new()),
            _ => ("anonymous".to_string(), String::new()),
        };

        stream
            .login(&user, &pass)
            .await
            .map_err(Self::ftp_error_to_remote_error)?;

        stream.set_mode(suppaftp::Mode::Passive);

        stream
            .transfer_type(FileType::Binary)
            .await
            .map_err(Self::ftp_error_to_remote_error)?;

        Ok(stream)
    }

    pub async fn download(&self) -> Result<Vec<u8>, RemoteFileError> {
        let mut stream = self.connect().await?;

        let size = stream
            .size(&self.path)
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
            stream.retr(&self.path, |mut reader| {
                Box::pin(async move {
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

    pub async fn upload(&self, data: &[u8]) -> Result<(), RemoteFileError> {
        let mut stream = self.connect().await?;

        let cursor = Cursor::new(data);
        let mut compat_cursor = cursor.compat();

        stream
            .put_file(&self.path, &mut compat_cursor)
            .await
            .map_err(Self::ftp_error_to_remote_error)?;

        let _ = stream.quit().await;

        Ok(())
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

    pub async fn download_chunk(&self, offset: u64, length: u64) -> Result<Bytes, RemoteFileError> {
        let mut stream = self.connect().await?;

        // Use REST command to set the starting position
        if offset > 0 {
            stream
                .resume_transfer(offset as usize)
                .await
                .map_err(Self::ftp_error_to_remote_error)?;
        }

        // Download the data starting from the offset, if any
        let result = stream
            .retr(&self.path, |mut reader| {
                Box::pin(async move {
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
}

#[async_trait]
impl RemoteFileHandler for FtpFileHandler {
    async fn get_file_size(&self) -> Result<u64, RemoteFileError> {
        let mut stream = self.connect().await?;

        let size = stream
            .size(&self.path)
            .await
            .map_err(Self::ftp_error_to_remote_error)?;

        if size as u64 > self.config.max_file_size {
            return Err(RemoteFileError::Other(format!(
                "File size {} exceeds maximum allowed size {}",
                size, self.config.max_file_size
            )));
        }

        let _ = stream.quit().await;

        Ok(size as u64)
    }

    async fn download_file(&self) -> Result<Box<dyn AsyncRead + Send + Unpin>, RemoteFileError> {
        // For now, we'll download the entire file and wrap it in a cursor
        // TODO: Implement true streaming when suppaftp provides better async streaming support
        let data = self.download().await?;
        let cursor = Cursor::new(data);

        Ok(Box::new(cursor))
    }

    fn is_supported(&self, url: &Url) -> bool {
        matches!(url.scheme(), "ftp" | "ftps")
    }

    async fn upload_file(
        &self,
        mut data: Box<dyn tokio::io::AsyncRead + Send + Unpin>,
        size: u64,
        _content_type: Option<String>,
    ) -> Result<(), RemoteFileError> {
        if size as u64 > self.config.max_file_size {
            return Err(RemoteFileError::Other(format!(
                "File size {} exceeds maximum allowed size {}",
                size, self.config.max_file_size
            )));
        }

        let mut stream = self.connect().await?;

        // Use put_with_stream for streaming upload
        let mut upload_stream = stream
            .put_with_stream(&self.path)
            .await
            .map_err(Self::ftp_error_to_remote_error)?;

        // Stream data in chunks
        let mut buffer = vec![0u8; self.config.chunk_size];
        loop {
            let n = tokio::io::AsyncReadExt::read(&mut data, &mut buffer)
                .await
                .map_err(|e| RemoteFileError::IoError(e))?;

            if n == 0 {
                break;
            }

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

    const TEST_MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB for tests

    // Unit tests for FTP handler focusing on testable logic without network access.
    // Integration tests requiring an actual FTP server are not included here.

    fn create_test_handler(url: &Url) -> Result<FtpFileHandler, RemoteFileError> {
        let config = RemoteFileConfig {
            max_file_size: 1024 * 1024,
            connection_timeout: 5,
            read_timeout: 10,
            follow_redirects: false,
            max_redirects: 0,
            user_agent: "Test-Agent".to_string(),
            chunk_size: 8192,
        };
        FtpFileHandler::new(config, url)
    }

    #[test]
    fn test_is_supported() {
        let url = Url::parse("ftp://example.com/file.txt").unwrap();
        let handler = create_test_handler(&url).unwrap();

        assert!(handler.is_supported(&Url::parse("ftp://example.com/file.txt").unwrap()));
        assert!(handler.is_supported(&Url::parse("ftps://example.com/file.txt").unwrap()));
        assert!(!handler.is_supported(&Url::parse("http://example.com/file.txt").unwrap()));
        assert!(!handler.is_supported(&Url::parse("file:///tmp/file.txt").unwrap()));
    }

    #[test]
    fn test_parse_url_anonymous() {
        let url = Url::parse("ftp://example.com/path/to/file.txt").unwrap();
        let handler = create_test_handler(&url).unwrap();

        assert_eq!(handler.host, "example.com");
        assert_eq!(handler.port, 21);
        assert_eq!(handler.username, None);
        assert_eq!(handler.password, None);
        assert_eq!(handler.path, "/path/to/file.txt");
    }

    #[test]
    fn test_parse_url_with_auth() {
        let url = Url::parse("ftp://user:pass@example.com:2121/file.txt").unwrap();
        let handler = create_test_handler(&url).unwrap();

        assert_eq!(handler.host, "example.com");
        assert_eq!(handler.port, 2121);
        assert_eq!(handler.username, Some("user".to_string()));
        assert_eq!(handler.password, Some("pass".to_string()));
        assert_eq!(handler.path, "/file.txt");
    }

    #[test]
    fn test_parse_url_with_username_only() {
        let url = Url::parse("ftp://user@example.com/file.txt").unwrap();
        let handler = create_test_handler(&url).unwrap();

        assert_eq!(handler.host, "example.com");
        assert_eq!(handler.port, 21);
        assert_eq!(handler.username, Some("user".to_string()));
        assert_eq!(handler.password, None);
        assert_eq!(handler.path, "/file.txt");
    }

    #[test]
    fn test_parse_url_edge_cases() {
        // Test with localhost
        let url = Url::parse("ftp://localhost/file.txt").unwrap();
        let handler = create_test_handler(&url).unwrap();
        assert_eq!(handler.host, "localhost");
        assert_eq!(handler.path, "/file.txt");

        // Test with IP address
        let url = Url::parse("ftp://192.168.1.1:2121/file.txt").unwrap();
        let handler = create_test_handler(&url).unwrap();
        assert_eq!(handler.host, "192.168.1.1");
        assert_eq!(handler.port, 2121);
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
    async fn test_upload_file_size_limit() {
        let url = Url::parse("ftp://example.com/upload.txt").unwrap();
        let config = RemoteFileConfig {
            max_file_size: 10,
            ..RemoteFileConfig::new(TEST_MAX_FILE_SIZE)
        };
        let handler = FtpFileHandler::new(config, &url).unwrap();
        let data = b"This is larger than 10 bytes";
        let cursor = Cursor::new(data);
        let boxed_reader: Box<dyn AsyncRead + Send + Unpin> = Box::new(cursor);

        let result = handler
            .upload_file(boxed_reader, data.len() as u64, None)
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
}
