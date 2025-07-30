use async_trait::async_trait;
use bytes::Bytes;
use futures_util::TryStreamExt;
use reqwest::{header, Body, Client, StatusCode};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio_util::io::{ReaderStream, StreamReader};
use url::Url;

use crate::remote_file::{RemoteFileConfig, RemoteFileError, RemoteFileHandler};

pub struct HttpFileHandler {
    client: Client,
    config: RemoteFileConfig,
    base_url: Url,
}

impl HttpFileHandler {
    pub fn new(config: RemoteFileConfig, url: &Url) -> Result<Self, RemoteFileError> {
        let client = Client::builder()
            .user_agent(&config.user_agent)
            .connect_timeout(Duration::from_secs(config.connection_timeout))
            .timeout(Duration::from_secs(config.read_timeout))
            .redirect(if config.follow_redirects {
                reqwest::redirect::Policy::limited(config.max_redirects as usize)
            } else {
                reqwest::redirect::Policy::none()
            })
            .build()
            .map_err(|e| RemoteFileError::Other(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self {
            client,
            config,
            base_url: url.clone(),
        })
    }

    fn status_to_error(status: StatusCode) -> RemoteFileError {
        match status {
            StatusCode::NOT_FOUND => RemoteFileError::NotFound,
            StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => RemoteFileError::AccessDenied,
            StatusCode::REQUEST_TIMEOUT | StatusCode::GATEWAY_TIMEOUT => RemoteFileError::Timeout,
            _ => RemoteFileError::Other(format!("HTTP error: {}", status)),
        }
    }

    fn map_request_error(e: reqwest::Error) -> RemoteFileError {
        if e.is_timeout() {
            RemoteFileError::Timeout
        } else {
            RemoteFileError::HttpError(e)
        }
    }

    fn get_content_length(response: &reqwest::Response) -> Result<u64, RemoteFileError> {
        // We need to access the header directly as the `content_length` method determines it from the response body
        match response
            .headers()
            .get("content-length")
            .ok_or_else(|| RemoteFileError::Other("Content-Length header missing".to_string()))?
            .to_str()
            .map(|val| val.parse::<u64>())
        {
            Ok(Ok(val)) => Ok(val),
            _ => Err(RemoteFileError::Other(
                "Invalid Content-Length header value".to_string(),
            )),
        }
    }

    fn validate_file_size(&self, size: u64) -> Result<(), RemoteFileError> {
        if size > self.config.max_file_size {
            Err(RemoteFileError::Other(format!(
                "File size {} exceeds maximum allowed size {}",
                size, self.config.max_file_size
            )))
        } else {
            Ok(())
        }
    }

    async fn handle_full_content(
        &self,
        response: reqwest::Response,
    ) -> Result<Bytes, RemoteFileError> {
        // For servers that don't support range requests, we accept the full content
        response.bytes().await.map_err(Self::map_request_error)
    }

    async fn handle_partial_content(
        &self,
        response: reqwest::Response,
        offset: u64,
        length: u64,
    ) -> Result<Bytes, RemoteFileError> {
        // Optionally verify the Content-Range header
        if let Some(content_range) = response.headers().get(header::CONTENT_RANGE) {
            let content_range_str = content_range.to_str().unwrap_or("");
            if let Err(e) = self.parse_and_validate_content_range(content_range_str, offset, length)
            {
                return Err(e);
            }
        }
        // Note: We don't error if Content-Range header is missing as some servers
        // may return 206 without it

        response.bytes().await.map_err(Self::map_request_error)
    }

    fn parse_and_validate_content_range(
        &self,
        header: &str,
        expected_offset: u64,
        expected_length: u64,
    ) -> Result<(u64, u64), RemoteFileError> {
        // Parse Content-Range header (format: "bytes start-end/total")
        let (start_str, end_str) = (|| {
            let range_part = header.strip_prefix("bytes ")?;

            let slash_pos = range_part.find('/')?;

            let range_values = &range_part[..slash_pos];
            let dash_pos = range_values.find('-')?;

            let start_str = &range_values[..dash_pos];
            let end_str = &range_values[dash_pos + 1..];

            Some((start_str, end_str))
        })()
        .ok_or_else(|| RemoteFileError::Other("Invalid Content-Range header format".to_string()))?;

        let actual_start = start_str.parse::<u64>().map_err(|_| {
            RemoteFileError::Other("Invalid start value in Content-Range".to_string())
        })?;
        let actual_end = end_str.parse::<u64>().map_err(|_| {
            RemoteFileError::Other("Invalid end value in Content-Range".to_string())
        })?;

        let expected_end = expected_offset + expected_length - 1;
        if actual_start != expected_offset || actual_end != expected_end {
            return Err(RemoteFileError::Other(format!(
                "Server returned incorrect range: expected {}-{}, got {}-{}",
                expected_offset, expected_end, actual_start, actual_end
            )));
        }

        Ok((actual_start, actual_end))
    }

    // TODO: This might be used when we do pagination, remove if it's not needed
    #[allow(dead_code)]
    async fn download_chunk(&self, offset: u64, length: u64) -> Result<Bytes, RemoteFileError> {
        let range = format!("bytes={}-{}", offset, offset + length - 1);

        let response = self
            .client
            .get(self.base_url.as_str())
            .header("Range", range)
            .send()
            .await
            .map_err(Self::map_request_error)?;

        match response.status() {
            StatusCode::OK => self.handle_full_content(response).await,
            StatusCode::PARTIAL_CONTENT => {
                self.handle_partial_content(response, offset, length).await
            }
            status => Err(Self::status_to_error(status)),
        }
    }

    pub async fn download(&self) -> Result<Bytes, RemoteFileError> {
        let mut reader = self.download_file().await?;

        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).await?;

        Ok(buffer.into())
    }
}

#[async_trait]
impl RemoteFileHandler for HttpFileHandler {
    async fn get_file_size(&self) -> Result<u64, RemoteFileError> {
        let response = self
            .client
            .head(self.base_url.as_str())
            .send()
            .await
            .map_err(Self::map_request_error)?;

        match response.status() {
            status if status.is_success() => {
                let content_length = Self::get_content_length(&response)?;

                Ok(content_length)
            }
            status => Err(Self::status_to_error(status)),
        }
    }

    async fn download_file(&self) -> Result<Box<dyn AsyncRead + Send + Unpin>, RemoteFileError> {
        let response = self
            .client
            .get(self.base_url.as_str())
            .send()
            .await
            .map_err(Self::map_request_error)?;

        match response.status() {
            status if status.is_success() => {
                Self::get_content_length(&response)
                    .and_then(|size| self.validate_file_size(size))?;

                let stream = response.bytes_stream();
                let reader = StreamReader::new(
                    stream.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
                );

                // Wrap the reader in a buffered reader with buffer size based on chunks_buffer
                let buffer_size = self.config.chunks_buffer.max(1) * shc_common::types::FILE_CHUNK_SIZE as usize;
                let buffered_reader =
                    tokio::io::BufReader::with_capacity(buffer_size, reader);

                Ok(Box::new(buffered_reader) as Box<dyn AsyncRead + Send + Unpin>)
            }
            status => Err(Self::status_to_error(status)),
        }
    }

    fn is_supported(&self, url: &Url) -> bool {
        matches!(url.scheme(), "http" | "https")
    }

    async fn upload_file(
        &self,
        data: Box<dyn AsyncRead + Send + Unpin>,
        size: u64,
        content_type: Option<String>,
    ) -> Result<(), RemoteFileError> {
        let stream = ReaderStream::new(data);
        let body = Body::wrap_stream(stream);

        // Upload to the configured base URL
        let mut request = self.client.put(self.base_url.as_str()).body(body);

        request = request.header("Content-Length", size.to_string());

        if let Some(ct) = content_type {
            request = request.header("Content-Type", ct);
        }

        if let Some(password) = self.base_url.password() {
            request = request.basic_auth(self.base_url.username(), Some(password));
        }

        let response = request.send().await.map_err(Self::map_request_error)?;

        match response.status() {
            status if status.is_success() => Ok(()),
            status => Err(Self::status_to_error(status)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    const TEST_MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB for tests

    fn create_test_handler(url: &Url) -> HttpFileHandler {
        let config = RemoteFileConfig {
            max_file_size: 1024 * 1024,
            connection_timeout: 5,
            read_timeout: 10,
            follow_redirects: true,
            max_redirects: 3,
            user_agent: "Test-Agent".to_string(),
            chunk_size: 8192,
            chunks_buffer: 512,
        };
        HttpFileHandler::new(config, url).unwrap()
    }

    #[tokio::test]
    async fn test_is_supported() {
        let url = Url::parse("http://example.com/file.txt").unwrap();
        let handler = create_test_handler(&url);

        assert!(handler.is_supported(&Url::parse("http://example.com/file.txt").unwrap()));
        assert!(handler.is_supported(&Url::parse("https://example.com/file.txt").unwrap()));
        assert!(!handler.is_supported(&Url::parse("ftp://example.com/file.txt").unwrap()));
        assert!(!handler.is_supported(&Url::parse("file:///tmp/file.txt").unwrap()));
    }

    /**  File size tests */

    #[tokio::test]
    async fn test_file_size() {
        let mut server = Server::new_async().await;

        let _m = server
            .mock("HEAD", "/test.txt")
            .with_status(200)
            .with_header("content-length", "1024")
            .create();

        let url = Url::parse(&format!("{}/test.txt", server.url())).unwrap();
        let handler = create_test_handler(&url);

        let result = handler.get_file_size().await;

        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
        assert_eq!(result.unwrap(), 1024);
    }

    #[tokio::test]
    async fn test_file_size_not_found() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("HEAD", "/missing.txt")
            .with_status(404)
            .create();

        let url = Url::parse(&format!("{}/missing.txt", server.url())).unwrap();
        let handler = create_test_handler(&url);

        let result = handler.get_file_size().await;

        assert!(matches!(result, Err(RemoteFileError::NotFound)));
    }

    #[tokio::test]
    async fn test_file_size_forbidden() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("HEAD", "/forbidden.txt")
            .with_status(403)
            .create();

        let url = Url::parse(&format!("{}/forbidden.txt", server.url())).unwrap();
        let handler = create_test_handler(&url);

        let result = handler.get_file_size().await;

        assert!(matches!(result, Err(RemoteFileError::AccessDenied)));
    }

    #[tokio::test]
    async fn test_file_size_too_large() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("HEAD", "/large.txt")
            .with_status(200)
            .with_header("content-length", "2097152")
            .create();

        let url = Url::parse(&format!("{}/large.txt", server.url())).unwrap();
        let handler = create_test_handler(&url);

        let result = handler.get_file_size().await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2097152);
    }

    #[tokio::test]
    async fn test_file_size_no_content_length_header() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("HEAD", "/no-length.txt")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .create();

        let url = Url::parse(&format!("{}/no-length.txt", server.url())).unwrap();
        let handler = create_test_handler(&url);
        let result = handler.get_file_size().await;

        assert!(result.is_err());
        assert!(matches!(result, Err(RemoteFileError::Other(_))));
        if let Err(RemoteFileError::Other(msg)) = result {
            assert!(msg.contains("Content-Length header missing"));
        }
    }

    /**  Download tests */

    #[tokio::test]
    async fn test_download_success() {
        let mut server = Server::new_async().await;
        let content = b"Hello, World!";
        let _m = server
            .mock("GET", "/test.txt")
            .with_status(200)
            .with_body(content)
            .create();

        let url = Url::parse(&format!("{}/test.txt", server.url())).unwrap();
        let handler = create_test_handler(&url);

        let data = handler.download().await.unwrap();

        assert_eq!(data.as_ref(), content);
    }

    #[tokio::test]
    async fn test_download_chunk_success() {
        let mut server = Server::new_async().await;
        let content = b"Hello";
        let _m = server
            .mock("GET", "/test.txt")
            .match_header("range", "bytes=6-10")
            .with_status(206)
            .with_body(content)
            .create();

        let url = Url::parse(&format!("{}/test.txt", server.url())).unwrap();
        let handler = create_test_handler(&url);

        let chunk = handler.download_chunk(6, 5).await.unwrap();

        assert_eq!(chunk.as_ref(), content);
    }

    #[tokio::test]
    async fn test_download_chunk_server_no_range_support() {
        let full_content = b"This is the full content of the file";

        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/no-range.txt")
            .match_header("range", "bytes=5-9")
            .with_status(200)
            .with_body(full_content)
            .create();

        let url = Url::parse(&format!("{}/no-range.txt", server.url())).unwrap();
        let handler = create_test_handler(&url);
        let chunk = handler.download_chunk(5, 5).await.unwrap();

        assert_eq!(chunk.as_ref(), full_content);
    }

    #[tokio::test]
    async fn test_download_with_timeout() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/slow.txt")
            .with_status(200)
            .with_header("content-length", "0")
            .with_chunked_body(|_| {
                std::thread::sleep(std::time::Duration::from_secs(2));
                Ok(())
            })
            .create();

        let url = Url::parse(&format!("{}/slow.txt", server.url())).unwrap();
        let config = RemoteFileConfig {
            connection_timeout: 1,
            read_timeout: 1,
            ..RemoteFileConfig::new(TEST_MAX_FILE_SIZE)
        };
        let handler = HttpFileHandler::new(config, &url).unwrap();

        let result = handler.download().await.unwrap_err();

        assert!(result.to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn test_download_forbidden() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/auth-required.txt")
            .with_status(401)
            .create();

        let url = Url::parse(&format!("{}/auth-required.txt", server.url())).unwrap();
        let handler = create_test_handler(&url);
        let result = handler.download().await;

        assert!(matches!(result, Err(RemoteFileError::AccessDenied)));
    }

    #[tokio::test]
    async fn test_download_chunk_with_content_range_validation() {
        let mut server = Server::new_async().await;
        let content = b"Hello";
        let _m = server
            .mock("GET", "/test.txt")
            .match_header("range", "bytes=6-10")
            .with_status(206)
            .with_header("Content-Range", "bytes 6-10/100")
            .with_body(content)
            .create();

        let url = Url::parse(&format!("{}/test.txt", server.url())).unwrap();
        let handler = create_test_handler(&url);
        let chunk = handler.download_chunk(6, 5).await.unwrap();

        assert_eq!(chunk.as_ref(), content);
    }

    #[tokio::test]
    async fn test_download_chunk_incorrect_range_validation() {
        let mut server = Server::new_async().await;
        let content = b"Hello";
        let _m = server
            .mock("GET", "/test.txt")
            .match_header("range", "bytes=6-10")
            .with_status(206)
            .with_header("Content-Range", "bytes 0-4/100") // Wrong range
            .with_body(content)
            .create();

        let url = Url::parse(&format!("{}/test.txt", server.url())).unwrap();
        let handler = create_test_handler(&url);
        let result = handler.download_chunk(6, 5).await;

        assert!(matches!(result, Err(RemoteFileError::Other(_))));
        if let Err(RemoteFileError::Other(msg)) = result {
            assert!(msg.contains("Server returned incorrect range"));
        }
    }

    /** Upload tests */

    #[tokio::test]
    async fn test_upload_file() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("PUT", "/upload.txt")
            .match_header("content-length", "13")
            .with_status(200)
            .create();

        let data = b"Hello, World!";
        let reader = Box::new(std::io::Cursor::new(data));

        let url = Url::parse(&format!("{}/upload.txt", server.url())).unwrap();
        let handler = create_test_handler(&url);

        handler
            .upload_file(reader, data.len() as u64, None)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_upload_file_with_basic_auth() {
        let mut server = Server::new_async().await;

        let _m = server
            .mock("PUT", "/secure-upload.txt")
            // this is the encoded user pass auth of the URL below
            .match_header("authorization", "Basic dXNlcjpwYXNz")
            .match_header("content-length", "6")
            .with_status(200)
            .create();

        let data = b"secure";
        let reader = Box::new(std::io::Cursor::new(data));

        let url = Url::parse(&format!(
            "http://user:pass@{}/secure-upload.txt",
            server.host_with_port()
        ))
        .unwrap();
        let handler = create_test_handler(&url);

        handler
            .upload_file(reader, data.len() as u64, None)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_upload_file_forbidden() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("PUT", "/forbidden-upload.txt")
            .with_status(403)
            .create();

        let data = b"data";
        let reader = Box::new(std::io::Cursor::new(data));

        let url = Url::parse(&format!("{}/forbidden-upload.txt", server.url())).unwrap();
        let handler = create_test_handler(&url);

        let result = handler.upload_file(reader, data.len() as u64, None).await;

        assert!(matches!(result, Err(RemoteFileError::AccessDenied)));
    }

    #[tokio::test]
    async fn test_upload_file_with_error() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("PUT", "/error-upload.txt")
            .with_status(500)
            .create();

        let data = b"data";
        let reader = Box::new(std::io::Cursor::new(data));

        let url = Url::parse(&format!("{}/error-upload.txt", server.url())).unwrap();
        let handler = create_test_handler(&url);

        let result = handler.upload_file(reader, data.len() as u64, None).await;

        assert!(matches!(result, Err(RemoteFileError::Other(_))));
    }

    #[tokio::test]
    async fn test_upload_file_timeout() {
        // can't trigger timeout with mockito for this
        let url = Url::parse("http://10.255.255.1/slow.txt").unwrap();
        let config = RemoteFileConfig {
            connection_timeout: 1,
            read_timeout: 1,
            ..RemoteFileConfig::new(TEST_MAX_FILE_SIZE)
        };
        let handler = HttpFileHandler::new(config, &url).unwrap();

        let data = b"data";
        let reader = Box::new(std::io::Cursor::new(data));

        let result = handler
            .upload_file(reader, data.len() as u64, None)
            .await
            .unwrap_err();

        assert!(result.to_string().contains("timed out"));
    }

    /** Misc tests */

    #[tokio::test]
    async fn test_follow_redirects() {
        let mut server = Server::new_async().await;

        let _m1 = server
            .mock("GET", "/redirect1")
            .with_status(302)
            .with_header("Location", &format!("{}/redirect2", server.url()))
            .create();

        let _m2 = server
            .mock("GET", "/redirect2")
            .with_status(302)
            .with_header("Location", &format!("{}/final", server.url()))
            .create();

        let _m3 = server
            .mock("GET", "/final")
            .with_status(200)
            .with_body(b"Final content")
            .create();

        let url = Url::parse(&format!("{}/redirect1", server.url())).unwrap();
        let handler = create_test_handler(&url);
        let data = handler.download().await.unwrap();

        assert_eq!(data.as_ref(), b"Final content");
    }

    #[tokio::test]
    async fn test_too_many_redirects() {
        let mut server = Server::new_async().await;
        let _m1 = server
            .mock("GET", "/redirect1")
            .with_status(302)
            .with_header("Location", &format!("{}/redirect2", server.url()))
            .create();

        let _m2 = server
            .mock("GET", "/redirect2")
            .with_status(302)
            .with_header("Location", &format!("{}/redirect3", server.url()))
            .create();

        let _m3 = server
            .mock("GET", "/redirect3")
            .with_status(302)
            .with_header("Location", &format!("{}/final", server.url()))
            .create();

        let url = Url::parse(&format!("{}/redirect1", server.url())).unwrap();
        let config = RemoteFileConfig {
            max_redirects: 2,
            ..RemoteFileConfig::new(TEST_MAX_FILE_SIZE)
        };
        let handler = HttpFileHandler::new(config, &url).unwrap();
        let result = handler.download().await;

        assert!(result.is_err());
    }
}
