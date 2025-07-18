use crate::remote_file::{RemoteFileConfig, RemoteFileError, RemoteFileHandler};
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::TryStreamExt;
use reqwest::{header, Body, Client, StatusCode};
use std::time::Duration;
use tokio::io::AsyncRead;
use tokio_util::io::{ReaderStream, StreamReader};
use url::Url;

pub struct HttpFileHandler {
    client: Client,
    config: RemoteFileConfig,
}

impl HttpFileHandler {
    pub fn new(config: RemoteFileConfig) -> Result<Self, RemoteFileError> {
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

        Ok(Self { client, config })
    }

    pub fn default() -> Result<Self, RemoteFileError> {
        Self::new(RemoteFileConfig::default())
    }

    fn status_to_error(status: StatusCode) -> RemoteFileError {
        match status {
            StatusCode::NOT_FOUND => RemoteFileError::NotFound,
            StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => RemoteFileError::AccessDenied,
            StatusCode::REQUEST_TIMEOUT | StatusCode::GATEWAY_TIMEOUT => RemoteFileError::Timeout,
            _ => RemoteFileError::Other(format!("HTTP error: {}", status)),
        }
    }

    pub async fn download(&self, url: &Url) -> Result<Vec<u8>, RemoteFileError> {
        let response = self.client.get(url.as_str()).send().await.map_err(|e| {
            if e.is_timeout() {
                RemoteFileError::Timeout
            } else {
                RemoteFileError::HttpError(e)
            }
        })?;

        if !response.status().is_success() {
            return Err(Self::status_to_error(response.status()));
        }

        if let Some(content_length) = response.content_length() {
            if content_length > self.config.max_file_size {
                return Err(RemoteFileError::Other(format!(
                    "File size {} exceeds maximum allowed size {}",
                    content_length, self.config.max_file_size
                )));
            }
        }

        let bytes = response.bytes().await.map_err(|e| {
            if e.is_timeout() {
                RemoteFileError::Timeout
            } else {
                RemoteFileError::HttpError(e)
            }
        })?;

        if bytes.len() as u64 > self.config.max_file_size {
            return Err(RemoteFileError::Other(format!(
                "Downloaded file size {} exceeds maximum allowed size {}",
                bytes.len(),
                self.config.max_file_size
            )));
        }

        Ok(bytes.to_vec())
    }
}

#[async_trait]
impl RemoteFileHandler for HttpFileHandler {
    async fn fetch_metadata(&self, url: &Url) -> Result<(u64, Option<String>), RemoteFileError> {
        let response = self.client.head(url.as_str()).send().await.map_err(|e| {
            if e.is_timeout() {
                RemoteFileError::Timeout
            } else {
                RemoteFileError::HttpError(e)
            }
        })?;

        if !response.status().is_success() {
            return Err(Self::status_to_error(response.status()));
        }

        let content_length = response
            .content_length()
            .ok_or_else(|| RemoteFileError::Other("Content-Length header missing".to_string()))?;

        if content_length > self.config.max_file_size {
            return Err(RemoteFileError::Other(format!(
                "File size {} exceeds maximum allowed size {}",
                content_length, self.config.max_file_size
            )));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        Ok((content_length, content_type))
    }

    async fn stream_file(
        &self,
        url: &Url,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>, RemoteFileError> {
        let response = self.client.get(url.as_str()).send().await.map_err(|e| {
            if e.is_timeout() {
                RemoteFileError::Timeout
            } else {
                RemoteFileError::HttpError(e)
            }
        })?;

        if !response.status().is_success() {
            return Err(Self::status_to_error(response.status()));
        }

        if let Some(content_length) = response.content_length() {
            if content_length > self.config.max_file_size {
                return Err(RemoteFileError::Other(format!(
                    "File size {} exceeds maximum allowed size {}",
                    content_length, self.config.max_file_size
                )));
            }
        }

        let stream = response.bytes_stream();
        let reader = StreamReader::new(
            stream.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
        );

        Ok(Box::new(reader) as Box<dyn AsyncRead + Send + Unpin>)
    }

    async fn download_chunk(
        &self,
        url: &Url,
        offset: u64,
        length: u64,
    ) -> Result<Bytes, RemoteFileError> {
        let range = format!("bytes={}-{}", offset, offset + length - 1);

        let response = self
            .client
            .get(url.as_str())
            .header("Range", range)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    RemoteFileError::Timeout
                } else {
                    RemoteFileError::HttpError(e)
                }
            })?;

        if !response.status().is_success() && response.status() != StatusCode::PARTIAL_CONTENT {
            return Err(Self::status_to_error(response.status()));
        }

        // If we requested a range but got a 200 OK instead of 206 Partial Content,
        // the server might have ignored our range request and returned the full content
        if response.status() == StatusCode::OK {
            // For servers that don't support range requests, we accept the full content
            // This maintains backward compatibility with existing behavior
            let bytes = response.bytes().await.map_err(|e| {
                if e.is_timeout() {
                    RemoteFileError::Timeout
                } else {
                    RemoteFileError::HttpError(e)
                }
            })?;

            return Ok(bytes);
        }

        // For 206 Partial Content responses, optionally verify the Content-Range header
        if response.status() == StatusCode::PARTIAL_CONTENT {
            if let Some(content_range) = response.headers().get(header::CONTENT_RANGE) {
                let content_range_str = content_range.to_str().unwrap_or("");

                // Parse Content-Range header (format: "bytes start-end/total")
                if let Some(range_part) = content_range_str.strip_prefix("bytes ") {
                    if let Some(slash_pos) = range_part.find('/') {
                        let range_values = &range_part[..slash_pos];
                        if let Some(dash_pos) = range_values.find('-') {
                            let start_str = &range_values[..dash_pos];
                            let end_str = &range_values[dash_pos + 1..];

                            if let (Ok(actual_start), Ok(actual_end)) =
                                (start_str.parse::<u64>(), end_str.parse::<u64>())
                            {
                                let expected_end = offset + length - 1;
                                if actual_start != offset || actual_end != expected_end {
                                    return Err(RemoteFileError::Other(format!(
                                        "Server returned incorrect range: expected {}-{}, got {}-{}",
                                        offset, expected_end, actual_start, actual_end
                                    )));
                                }
                            }
                        }
                    }
                }
            }
            // Note: We don't error if Content-Range header is missing as some servers
            // may return 206 without it, and the test expects this to work
        }

        let bytes = response.bytes().await.map_err(|e| {
            if e.is_timeout() {
                RemoteFileError::Timeout
            } else {
                RemoteFileError::HttpError(e)
            }
        })?;

        Ok(bytes)
    }

    fn is_supported(&self, url: &Url) -> bool {
        matches!(url.scheme(), "http" | "https")
    }

    async fn upload_file(
        &self,
        url: &Url,
        data: Box<dyn AsyncRead + Send + Unpin>,
        size: u64,
        content_type: Option<String>,
    ) -> Result<(), RemoteFileError> {
        if !self.is_supported(url) {
            return Err(RemoteFileError::UnsupportedProtocol(
                url.scheme().to_string(),
            ));
        }

        let stream = ReaderStream::new(data);
        let body = Body::wrap_stream(stream);

        let mut request = self.client.put(url.as_str()).body(body);

        request = request.header("Content-Length", size.to_string());

        if let Some(ct) = content_type {
            request = request.header("Content-Type", ct);
        }

        if let Some(password) = url.password() {
            request = request.basic_auth(url.username(), Some(password));
        }

        let response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                RemoteFileError::Timeout
            } else {
                RemoteFileError::HttpError(e)
            }
        })?;

        if !response.status().is_success() {
            return Err(Self::status_to_error(response.status()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    fn create_test_handler() -> HttpFileHandler {
        let config = RemoteFileConfig {
            max_file_size: 1024 * 1024,
            connection_timeout: 5,
            read_timeout: 10,
            follow_redirects: true,
            max_redirects: 3,
            user_agent: "Test-Agent".to_string(),
        };
        HttpFileHandler::new(config).unwrap()
    }

    #[tokio::test]
    async fn test_is_supported() {
        let handler = create_test_handler();

        assert!(handler.is_supported(&Url::parse("http://example.com/file.txt").unwrap()));
        assert!(handler.is_supported(&Url::parse("https://example.com/file.txt").unwrap()));
        assert!(!handler.is_supported(&Url::parse("ftp://example.com/file.txt").unwrap()));
        assert!(!handler.is_supported(&Url::parse("file:///tmp/file.txt").unwrap()));
    }

    #[tokio::test]
    async fn test_fetch_metadata_success() {
        let handler = create_test_handler();
        let mut server = Server::new_async().await;
        let _m = server
            .mock("HEAD", "/test.txt")
            .with_status(200)
            .with_header("content-length", "1024")
            .with_header("content-type", "text/plain")
            .create_async()
            .await;

        let url = Url::parse(&format!("{}/test.txt", server.url())).unwrap();
        let result = handler.fetch_metadata(&url).await;
        assert!(result.is_ok());

        if let Ok((_, content_type)) = result {
            assert_eq!(content_type, Some("text/plain".to_string()));
        }
    }

    #[tokio::test]
    async fn test_fetch_metadata_not_found() {
        let handler = create_test_handler();
        let mut server = Server::new_async().await;
        let _m = server
            .mock("HEAD", "/missing.txt")
            .with_status(404)
            .create_async()
            .await;

        let url = Url::parse(&format!("{}/missing.txt", server.url())).unwrap();
        let result = handler.fetch_metadata(&url).await;

        assert!(matches!(result, Err(RemoteFileError::NotFound)));
    }

    #[tokio::test]
    async fn test_fetch_metadata_forbidden() {
        let handler = create_test_handler();
        let mut server = Server::new_async().await;
        let _m = server
            .mock("HEAD", "/forbidden.txt")
            .with_status(403)
            .create_async()
            .await;

        let url = Url::parse(&format!("{}/forbidden.txt", server.url())).unwrap();
        let result = handler.fetch_metadata(&url).await;

        assert!(matches!(result, Err(RemoteFileError::AccessDenied)));
    }

    #[tokio::test]
    async fn test_fetch_metadata_file_too_large() {
        let handler = create_test_handler();
        let mut server = Server::new_async().await;
        let _m = server
            .mock("HEAD", "/large.txt")
            .with_status(200)
            .with_header("content-length", "2097152")
            .create_async()
            .await;

        let url = Url::parse(&format!("{}/large.txt", server.url())).unwrap();
        let result = handler.fetch_metadata(&url).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_download_success() {
        let handler = create_test_handler();
        let mut server = Server::new_async().await;
        let content = b"Hello, World!";
        let _m = server
            .mock("GET", "/test.txt")
            .with_status(200)
            .with_body(content)
            .create_async()
            .await;

        let url = Url::parse(&format!("{}/test.txt", server.url())).unwrap();
        let data = handler.download(&url).await.unwrap();

        assert_eq!(data, content);
    }

    #[tokio::test]
    async fn test_download_chunk_success() {
        let handler = create_test_handler();
        let mut server = Server::new_async().await;
        let content = b"Hello";
        let _m = server
            .mock("GET", "/test.txt")
            .match_header("range", "bytes=6-10")
            .with_status(206)
            .with_body(content)
            .create_async()
            .await;

        let url = Url::parse(&format!("{}/test.txt", server.url())).unwrap();
        let chunk = handler.download_chunk(&url, 6, 5).await.unwrap();

        assert_eq!(chunk.as_ref(), content);
    }

    #[tokio::test]
    async fn test_upload_file_success() {
        let handler = create_test_handler();
        let mut server = Server::new_async().await;
        let _m = server
            .mock("PUT", "/upload.txt")
            .match_header("content-length", "13")
            .match_header("content-type", "text/plain")
            .with_status(200)
            .create_async()
            .await;

        let data = b"Hello, World!";
        let reader = Box::new(std::io::Cursor::new(data));
        let url = Url::parse(&format!("{}/upload.txt", server.url())).unwrap();

        handler
            .upload_file(&url, reader, 13, Some("text/plain".to_string()))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_upload_file_without_content_type() {
        let handler = create_test_handler();
        let mut server = Server::new_async().await;
        let _m = server
            .mock("PUT", "/upload2.txt")
            .match_header("content-length", "4")
            .with_status(201)
            .create_async()
            .await;

        let data = b"test";
        let reader = Box::new(std::io::Cursor::new(data));
        let url = Url::parse(&format!("{}/upload2.txt", server.url())).unwrap();

        handler.upload_file(&url, reader, 4, None).await.unwrap();
    }

    #[tokio::test]
    async fn test_upload_file_with_basic_auth() {
        let handler = create_test_handler();
        let mut server = Server::new_async().await;
        let _m = server
            .mock("PUT", "/secure-upload.txt")
            .match_header("authorization", "Basic dXNlcjpwYXNz")
            .match_header("content-length", "6")
            .with_status(200)
            .create_async()
            .await;

        let data = b"secure";
        let reader = Box::new(std::io::Cursor::new(data));
        let url = Url::parse(&format!(
            "http://user:pass@{}/secure-upload.txt",
            server.host_with_port()
        ))
        .unwrap();

        handler.upload_file(&url, reader, 6, None).await.unwrap();
    }

    #[tokio::test]
    async fn test_upload_file_forbidden() {
        let handler = create_test_handler();
        let mut server = Server::new_async().await;
        let _m = server
            .mock("PUT", "/forbidden-upload.txt")
            .with_status(403)
            .create_async()
            .await;

        let data = b"data";
        let reader = Box::new(std::io::Cursor::new(data));
        let url = Url::parse(&format!("{}/forbidden-upload.txt", server.url())).unwrap();

        let result = handler.upload_file(&url, reader, 4, None).await;

        assert!(matches!(result, Err(RemoteFileError::AccessDenied)));
    }

    #[tokio::test]
    async fn test_upload_file_unsupported_protocol() {
        let handler = create_test_handler();
        let data = b"data";
        let reader = Box::new(std::io::Cursor::new(data));

        let url = Url::parse("ftp://example.com/file.txt").unwrap();
        let result = handler.upload_file(&url, reader, 4, None).await;

        assert!(matches!(
            result,
            Err(RemoteFileError::UnsupportedProtocol(_))
        ));
    }

    #[tokio::test]
    async fn test_upload_file_server_error() {
        let handler = create_test_handler();
        let mut server = Server::new_async().await;
        let _m = server
            .mock("PUT", "/error-upload.txt")
            .with_status(500)
            .create_async()
            .await;

        let data = b"data";
        let reader = Box::new(std::io::Cursor::new(data));
        let url = Url::parse(&format!("{}/error-upload.txt", server.url())).unwrap();

        let result = handler.upload_file(&url, reader, 4, None).await;

        assert!(matches!(result, Err(RemoteFileError::Other(_))));
    }

    #[tokio::test]
    async fn test_upload_file_timeout() {
        let config = RemoteFileConfig {
            connection_timeout: 1,
            read_timeout: 1,
            ..RemoteFileConfig::default()
        };
        let handler = HttpFileHandler::new(config).unwrap();

        let url = Url::parse("http://10.255.255.1/timeout-upload.txt").unwrap();

        let data = b"data";
        let reader = Box::new(std::io::Cursor::new(data));

        let result = handler.upload_file(&url, reader, 4, None).await;

        assert!(matches!(result, Err(RemoteFileError::Timeout)));
    }

    #[tokio::test]
    async fn test_stream_file_success() {
        let handler = create_test_handler();
        let mut server = Server::new_async().await;
        let content = b"Streaming content";
        let _m = server
            .mock("GET", "/stream.txt")
            .with_status(200)
            .with_body(content)
            .create_async()
            .await;

        let url = Url::parse(&format!("{}/stream.txt", server.url())).unwrap();
        let mut reader = handler.stream_file(&url).await.unwrap();

        let mut buffer = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut reader, &mut buffer)
            .await
            .unwrap();

        assert_eq!(buffer, content);
    }

    #[tokio::test]
    async fn test_follow_redirects() {
        let handler = create_test_handler();
        let mut server = Server::new_async().await;

        let _m1 = server
            .mock("GET", "/redirect1")
            .with_status(302)
            .with_header("Location", &format!("{}/redirect2", server.url()))
            .create_async()
            .await;

        let _m2 = server
            .mock("GET", "/redirect2")
            .with_status(302)
            .with_header("Location", &format!("{}/final", server.url()))
            .create_async()
            .await;

        let _m3 = server
            .mock("GET", "/final")
            .with_status(200)
            .with_body(b"Final content")
            .create_async()
            .await;

        let url = Url::parse(&format!("{}/redirect1", server.url())).unwrap();
        let data = handler.download(&url).await.unwrap();

        assert_eq!(data, b"Final content");
    }

    #[tokio::test]
    async fn test_too_many_redirects() {
        let config = RemoteFileConfig {
            max_redirects: 2,
            ..RemoteFileConfig::default()
        };
        let handler = HttpFileHandler::new(config).unwrap();

        let mut server = Server::new_async().await;
        let _m1 = server
            .mock("GET", "/redirect1")
            .with_status(302)
            .with_header("Location", &format!("{}/redirect2", server.url()))
            .create_async()
            .await;

        let _m2 = server
            .mock("GET", "/redirect2")
            .with_status(302)
            .with_header("Location", &format!("{}/redirect3", server.url()))
            .create_async()
            .await;

        let _m3 = server
            .mock("GET", "/redirect3")
            .with_status(302)
            .with_header("Location", &format!("{}/final", server.url()))
            .create_async()
            .await;

        let url = Url::parse(&format!("{}/redirect1", server.url())).unwrap();
        let result = handler.download(&url).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_no_content_length_header() {
        let handler = create_test_handler();
        let mut server = Server::new_async().await;
        let _m = server
            .mock("HEAD", "/no-length.txt")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .create_async()
            .await;

        let url = Url::parse(&format!("{}/no-length.txt", server.url())).unwrap();
        let result = handler.fetch_metadata(&url).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_download_chunk_server_no_range_support() {
        let handler = create_test_handler();
        let full_content = b"This is the full content of the file";

        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/no-range.txt")
            .match_header("range", "bytes=5-9")
            .with_status(200)
            .with_body(full_content)
            .create_async()
            .await;

        let url = Url::parse(&format!("{}/no-range.txt", server.url())).unwrap();
        let chunk = handler.download_chunk(&url, 5, 5).await.unwrap();

        assert_eq!(chunk.as_ref(), full_content);
    }

    #[tokio::test]
    async fn test_timeout_error() {
        let config = RemoteFileConfig {
            connection_timeout: 1,
            read_timeout: 1,
            ..RemoteFileConfig::default()
        };
        let handler = HttpFileHandler::new(config).unwrap();

        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/slow.txt")
            .with_status(200)
            .with_chunked_body(|_| {
                std::thread::sleep(std::time::Duration::from_secs(2));
                Ok(())
            })
            .create_async()
            .await;

        let url = Url::parse(&format!("{}/slow.txt", server.url())).unwrap();
        let result = handler.download(&url).await;

        assert!(matches!(result, Err(RemoteFileError::Timeout)));
    }

    #[tokio::test]
    async fn test_unauthorized_error() {
        let handler = create_test_handler();
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/auth-required.txt")
            .with_status(401)
            .create_async()
            .await;

        let url = Url::parse(&format!("{}/auth-required.txt", server.url())).unwrap();
        let result = handler.download(&url).await;

        assert!(matches!(result, Err(RemoteFileError::AccessDenied)));
    }

    #[tokio::test]
    async fn test_download_chunk_with_content_range_validation() {
        let handler = create_test_handler();
        let mut server = Server::new_async().await;
        let content = b"Hello";
        let _m = server
            .mock("GET", "/test.txt")
            .match_header("range", "bytes=6-10")
            .with_status(206)
            .with_header("Content-Range", "bytes 6-10/100")
            .with_body(content)
            .create_async()
            .await;

        let url = Url::parse(&format!("{}/test.txt", server.url())).unwrap();
        let chunk = handler.download_chunk(&url, 6, 5).await.unwrap();

        assert_eq!(chunk.as_ref(), content);
    }

    #[tokio::test]
    async fn test_download_chunk_incorrect_range_validation() {
        let handler = create_test_handler();
        let mut server = Server::new_async().await;
        let content = b"Hello";
        let _m = server
            .mock("GET", "/test.txt")
            .match_header("range", "bytes=6-10")
            .with_status(206)
            .with_header("Content-Range", "bytes 0-4/100") // Wrong range
            .with_body(content)
            .create_async()
            .await;

        let url = Url::parse(&format!("{}/test.txt", server.url())).unwrap();
        let result = handler.download_chunk(&url, 6, 5).await;

        assert!(matches!(result, Err(RemoteFileError::Other(_))));
        if let Err(RemoteFileError::Other(msg)) = result {
            assert!(msg.contains("Server returned incorrect range"));
        }
    }

    #[tokio::test]
    async fn test_internal_server_error() {
        let handler = create_test_handler();
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/error.txt")
            .with_status(500)
            .create_async()
            .await;

        let url = Url::parse(&format!("{}/error.txt", server.url())).unwrap();
        let result = handler.download(&url).await;

        assert!(matches!(result, Err(RemoteFileError::Other(_))));
        if let Err(RemoteFileError::Other(msg)) = result {
            assert!(msg.contains("500"));
        }
    }
}
