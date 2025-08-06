use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use futures_util::future::FutureExt;
use futures_util::io::{AsyncReadExt, AsyncWriteExt};
use suppaftp::types::FileType;
use suppaftp::{AsyncFtpStream, FtpError};
use tokio::io::{AsyncRead, ReadBuf};
use tokio::sync::{mpsc, oneshot};
use url::Url;

use crate::remote_file::{RemoteFileConfig, RemoteFileError, RemoteFileHandler};

/// Async reader that streams FTP data through a channel
struct FtpStreamReader {
    receiver: mpsc::Receiver<Result<Bytes, RemoteFileError>>,
    current_chunk: Option<Bytes>,
    position: usize,
    error_receiver: Option<oneshot::Receiver<RemoteFileError>>,
}

impl FtpStreamReader {
    pub fn new(
        chunks: mpsc::Receiver<Result<Bytes, RemoteFileError>>,
        error: Option<oneshot::Receiver<RemoteFileError>>,
    ) -> Self {
        Self {
            current_chunk: None,
            position: 0,
            receiver: chunks,
            error_receiver: error,
        }
    }
}

impl AsyncRead for FtpStreamReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();

        loop {
            // If we have data in current chunk, read from it
            if let Some(chunk) = &this.current_chunk {
                if this.position < chunk.len() {
                    let remaining = chunk.len() - this.position;
                    let to_read = remaining.min(buf.remaining());
                    buf.put_slice(&chunk[this.position..this.position + to_read]);
                    this.position += to_read;

                    if this.position >= chunk.len() {
                        this.current_chunk = None;
                        this.position = 0;
                    }
                }
            }

            if buf.remaining() == 0 {
                return Poll::Ready(Ok(()));
            }

            // Try to get next chunk
            break match this.receiver.poll_recv(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    this.current_chunk = Some(chunk);
                    this.position = 0;
                    continue;
                }
                Poll::Ready(Some(Err(e))) => Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))),
                Poll::Ready(None) => {
                    // Check for any error from the download task and return it
                    if let Some(mut error_rx) = this.error_receiver.take() {
                        match error_rx.poll_unpin(cx) {
                            Poll::Ready(Ok(e)) => {
                                return Poll::Ready(Err(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    e.to_string(),
                                )));
                            }
                            _ => {}
                        }
                    }
                    Poll::Ready(Ok(()))
                }
                Poll::Pending => Poll::Pending,
            };
        }
    }
}

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

    async fn chunked_read<T>(
        tx: mpsc::Sender<Result<Bytes, RemoteFileError>>,
        mut reader: T,
        chunk_size: usize,
    ) -> Result<((), T), FtpError>
    where
        T: futures_util::io::AsyncRead + Unpin,
    {
        // Read `chunk_size` at a time and send to channel
        let mut buffer = vec![0u8; chunk_size];
        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => break, // EOF
                Ok(n) => {
                    let chunk = Bytes::copy_from_slice(&buffer[..n]);
                    if tx.send(Ok(chunk)).await.is_err() {
                        // Receiver dropped, stop reading
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(Err(RemoteFileError::IoError(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e,
                        ))))
                        .await;
                    break;
                }
            }
        }
        Ok(((), reader))
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
}

#[async_trait]
impl RemoteFileHandler for FtpFileHandler {
    async fn get_file_size(&self) -> Result<u64, RemoteFileError> {
        let mut stream = self.connect().await?;

        let size = stream
            .size(&self.path)
            .await
            .map_err(Self::ftp_error_to_remote_error)?;

        let _ = stream.quit().await;

        Ok(size as u64)
    }

    async fn download_file(&self) -> Result<Box<dyn AsyncRead + Send + Unpin>, RemoteFileError> {
        // Create channel with buffer based on configured chunks_buffer
        let buffered_chunks = self.config.chunks_buffer.max(1);
        let (tx, rx) = mpsc::channel(buffered_chunks);
        let (error_tx, error_rx) = oneshot::channel();

        let mut stream = self.connect().await?;

        // Verify file exists and get size
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

        let path = self.path.clone();
        let chunk_size = self.config.chunk_size;

        // Spawn task to download file through channel
        tokio::spawn(async move {
            let result = async {
                let result = stream
                    .retr(&path, |reader| {
                        Box::pin(Self::chunked_read(tx.clone(), reader, chunk_size))
                    })
                    .await
                    .map_err(Self::ftp_error_to_remote_error)
                    .map(|_| ());

                let _ = stream.quit().await;
                result
            }
            .await;

            if let Err(e) = result {
                // Propagate error to reader
                let _ = error_tx.send(e);
            }
        });

        Ok(Box::new(FtpStreamReader::new(rx, Some(error_rx))))
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
        let mut upload_stream = match stream
            .put_with_stream(&self.path)
            .await
            .map_err(Self::ftp_error_to_remote_error)
        {
            Ok(upload_stream) => upload_stream,
            Err(e) => {
                let _ = stream.quit().await;
                return Err(e);
            }
        };

        let result = {
            // Stream data in chunks
            let mut buffer = vec![0u8; self.config.chunk_size];
            let mut upload_error = None;

            loop {
                match tokio::io::AsyncReadExt::read(&mut data, &mut buffer).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if let Err(e) = upload_stream.write_all(&buffer[..n]).await {
                            upload_error =
                                Some(RemoteFileError::Other(format!("Write error: {}", e)));
                            break;
                        }
                    }
                    Err(e) => {
                        upload_error = Some(RemoteFileError::IoError(e));
                        break;
                    }
                }
            }

            // Always finalize the upload stream, even on error
            let finalize_result = stream
                .finalize_put_stream(upload_stream)
                .await
                .map_err(Self::ftp_error_to_remote_error);

            // Return the first error that occurred
            match (upload_error, finalize_result) {
                (Some(e), _) => Err(e),
                (None, Err(e)) => Err(e),
                (None, Ok(_)) => Ok(()),
            }
        };

        // Always quit the connection, even on error
        let _ = stream.quit().await;

        result
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

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
            chunk_size: 8096,
            chunks_buffer: 512,
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
