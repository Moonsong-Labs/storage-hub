use super::factory::RemoteFileHandlerFactory;
use super::*;
use std::sync::Arc;
use url::Url;

#[cfg(test)]
mod factory_tests {
    use super::*;
    use percent_encoding;

    const TEST_MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB for tests

    fn default_config() -> RemoteFileConfig {
        RemoteFileConfig::new(TEST_MAX_FILE_SIZE)
    }

    #[test]
    fn test_factory_creates_correct_handler_for_each_scheme() {
        let config = default_config();

        let test_cases = vec![
            ("http://example.com/file.txt", "http"),
            ("https://example.com/file.txt", "https"),
            ("ftp://example.com/file.txt", "ftp"),
            ("ftps://example.com/file.txt", "ftps"),
            ("file:///path/to/file.txt", "file"),
        ];

        for (url_str, expected_scheme) in test_cases {
            let url = Url::parse(url_str).unwrap();
            let (handler, returned_url) =
                RemoteFileHandlerFactory::create(&url, config.clone()).unwrap();

            assert_eq!(url, returned_url, "Returned URL should match input URL");
            assert!(
                handler.is_supported(&url),
                "Handler should support {} URLs",
                expected_scheme
            );

            let other_url = Url::parse("sftp://example.com/file.txt").unwrap();
            assert!(
                !handler.is_supported(&other_url),
                "Handler for {} should not support sftp URLs",
                expected_scheme
            );
        }
    }

    #[test]
    fn test_factory_handles_path_without_scheme() {
        use tempfile::TempDir;
        let config = default_config();

        // Create a unique temporary directory
        let temp_dir = TempDir::new().unwrap();
        let temp_file = temp_dir.path().join("test_file.txt");
        std::fs::write(&temp_file, b"test").unwrap();

        let path = temp_file.to_str().unwrap();
        let (handler, returned_url) =
            RemoteFileHandlerFactory::create_from_string(path, config.clone()).unwrap();

        assert!(handler.is_supported(&returned_url));
        assert_eq!(returned_url.scheme(), "file");
        // No manual cleanup needed; TempDir cleans up automatically
    }

    #[test]
    fn test_factory_rejects_unsupported_schemes() {
        let config = default_config();

        let unsupported_schemes = vec![
            "sftp://example.com/file.txt",
            "ssh://example.com/file.txt",
            "smb://example.com/file.txt",
            "custom://example.com/file.txt",
            "invalid://example.com/file.txt",
        ];

        for url_str in unsupported_schemes {
            let url = Url::parse(url_str).unwrap();
            let result = RemoteFileHandlerFactory::create(&url, config.clone());

            assert!(
                matches!(result, Err(RemoteFileError::UnsupportedProtocol(_))),
                "Should reject unsupported scheme in URL: {}",
                url_str
            );
        }
    }

    #[test]
    fn test_factory_validates_url_format() {
        let config = default_config();

        let invalid_urls = vec!["", "://example.com", "http://[invalid"];

        for invalid_url in invalid_urls {
            let result = RemoteFileHandlerFactory::create_from_string(invalid_url, config.clone());

            assert!(
                matches!(result, Err(RemoteFileError::InvalidUrl(_))),
                "Should reject invalid URL: {}",
                invalid_url
            );
        }
    }

    #[test]
    fn test_supported_protocols_list() {
        let protocols = RemoteFileHandlerFactory::supported_protocols();

        assert!(protocols.contains(&"file"));
        assert!(protocols.contains(&"http"));
        assert!(protocols.contains(&"https"));
        assert!(protocols.contains(&"ftp"));
        assert!(protocols.contains(&"ftps"));

        assert_eq!(protocols.len(), 5);
    }


    #[test]
    fn test_is_protocol_supported_comprehensive() {
        let supported = RemoteFileHandlerFactory::supported_protocols();

        // Empty string is handled specially in create() method
        let result = RemoteFileHandlerFactory::create(
            &Url::parse("file:///test").unwrap(),
            RemoteFileConfig::new(TEST_MAX_FILE_SIZE),
        );
        assert!(result.is_ok());

        assert!(supported.contains(&"file"));
        assert!(supported.contains(&"http"));
        assert!(supported.contains(&"https"));
        assert!(supported.contains(&"ftp"));
        assert!(supported.contains(&"ftps"));

        assert!(!supported.contains(&"sftp"));
        assert!(!supported.contains(&"ssh"));
        assert!(!supported.contains(&"smb"));
        assert!(!supported.contains(&"custom"));

        assert!(!supported.contains(&"HTTP"));
        assert!(!supported.contains(&"File"));
    }
}

#[cfg(test)]
mod url_parsing_tests {
    use super::*;

    const TEST_MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB for tests

    #[test]
    fn test_url_with_authentication() {
        let config = RemoteFileConfig::new(TEST_MAX_FILE_SIZE);

        let url_str = "ftp://user:pass@example.com/file.txt";
        let (handler, returned_url) =
            RemoteFileHandlerFactory::create_from_string(url_str, config.clone()).unwrap();
        let url = Url::parse(url_str).unwrap();
        assert_eq!(url, returned_url);
        assert!(handler.is_supported(&url));
    }

    #[test]
    fn test_url_with_port() {
        let config = RemoteFileConfig::new(TEST_MAX_FILE_SIZE);

        let urls_with_ports = vec![
            "http://example.com:8080/file.txt",
            "https://example.com:443/file.txt",
            "ftp://example.com:21/file.txt",
        ];

        for url_str in urls_with_ports {
            let (handler, returned_url) =
                RemoteFileHandlerFactory::create_from_string(url_str, config.clone()).unwrap();
            let url = Url::parse(url_str).unwrap();
            assert_eq!(url, returned_url);
            assert!(handler.is_supported(&url));
        }
    }

    #[test]
    fn test_url_with_query_parameters() {
        let config = RemoteFileConfig::new(TEST_MAX_FILE_SIZE);

        let url_str = "https://example.com/file.txt?version=1.0&token=abc123";
        let (handler, returned_url) =
            RemoteFileHandlerFactory::create_from_string(url_str, config.clone()).unwrap();
        let url = Url::parse(url_str).unwrap();
        assert_eq!(url, returned_url);
        assert!(handler.is_supported(&url));
    }

    #[test]
    fn test_url_with_fragment() {
        let config = RemoteFileConfig::new(TEST_MAX_FILE_SIZE);

        let url_str = "https://example.com/file.txt#section1";
        let (handler, returned_url) =
            RemoteFileHandlerFactory::create_from_string(url_str, config.clone()).unwrap();
        let url = Url::parse(url_str).unwrap();
        assert_eq!(url, returned_url);
        assert!(handler.is_supported(&url));
    }

    #[test]
    fn test_url_encoding() {
        let config = RemoteFileConfig::new(TEST_MAX_FILE_SIZE);

        let url_str = "https://example.com/path%20with%20spaces/file%20name.txt";
        let (handler, returned_url) =
            RemoteFileHandlerFactory::create_from_string(url_str, config.clone()).unwrap();
        let url = Url::parse(url_str).unwrap();
        assert_eq!(url, returned_url);
        assert!(handler.is_supported(&url));
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let errors = vec![
            RemoteFileError::InvalidUrl("bad url".to_string()),
            RemoteFileError::UnsupportedProtocol("custom".to_string()),
            RemoteFileError::Other("Connection refused".to_string()),
            RemoteFileError::Timeout,
            RemoteFileError::NotFound,
            RemoteFileError::AccessDenied,
            RemoteFileError::Other("Unknown error".to_string()),
        ];

        for error in errors {
            let display = format!("{}", error);
            assert!(!display.is_empty(), "Error display should not be empty");
        }
    }

    #[test]
    fn test_error_conversions() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let remote_error: RemoteFileError = io_error.into();
        assert!(matches!(remote_error, RemoteFileError::IoError(_)));
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = RemoteFileConfig::default();

        assert_eq!(config.max_file_size, 5 * 1024 * 1024 * 1024);
        assert_eq!(config.connection_timeout, 30);
        assert_eq!(config.read_timeout, 300);
        assert_eq!(config.max_redirects, 10);
        assert!(config.follow_redirects);
        assert_eq!(config.user_agent, "StorageHub-Client/1.0");
    }

    #[test]
    fn test_config_custom_values() {
        let config = RemoteFileConfig {
            max_file_size: 10 * 1024 * 1024 * 1024,
            connection_timeout: 10,
            read_timeout: 30,
            max_redirects: 5,
            follow_redirects: false,
            user_agent: "CustomAgent/1.0".to_string(),
            chunk_size: 16384,
        };

        assert_eq!(config.max_file_size, 10 * 1024 * 1024 * 1024);
        assert_eq!(config.connection_timeout, 10);
        assert_eq!(config.read_timeout, 30);
        assert_eq!(config.max_redirects, 5);
        assert!(!config.follow_redirects);
        assert_eq!(config.user_agent, "CustomAgent/1.0");
        assert_eq!(config.chunk_size, 16384);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tokio;

    const TEST_MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB for tests

    #[tokio::test]
    async fn test_handler_lifecycle() {
        let config = RemoteFileConfig::new(TEST_MAX_FILE_SIZE);
        let url_str = "https://httpbin.org/bytes/100";
        let url = Url::parse(url_str).unwrap();

        let (handler, returned_url) = RemoteFileHandlerFactory::create(&url, config).unwrap();

        assert_eq!(url, returned_url);
        assert!(handler.is_supported(&url));
    }

    #[tokio::test]
    async fn test_multiple_handlers_concurrent() {
        let config = RemoteFileConfig::new(TEST_MAX_FILE_SIZE);

        let urls = vec![
            "http://example.com/file1.txt",
            "https://example.com/file2.txt",
            "ftp://example.com/file3.txt",
            "file:///tmp/file4.txt",
        ];

        let handlers: Vec<(Arc<dyn RemoteFileHandler>, Url)> = urls
            .iter()
            .map(|url_str| {
                RemoteFileHandlerFactory::create_from_string(url_str, config.clone()).unwrap()
            })
            .collect();

        assert_eq!(handlers.len(), 4);

        for (i, url_str) in urls.iter().enumerate() {
            let url = Url::parse(url_str).unwrap();
            assert_eq!(url, handlers[i].1);
            assert!(handlers[i].0.is_supported(&url));
        }
    }

    #[test]
    fn test_handler_thread_safety() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<Arc<dyn RemoteFileHandler>>();
    }
}

#[cfg(test)]
mod external_service_tests {
    use super::*;
    use mockito::Server;

    const TEST_MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB for tests

    #[tokio::test]
    async fn test_http_download_with_mock_server() {
        let config = RemoteFileConfig::new(TEST_MAX_FILE_SIZE);

        let mut server = Server::new_async().await;
        let test_data = vec![42u8; 100];
        let _m = server
            .mock("GET", "/bytes/100")
            .with_status(200)
            .with_header("content-type", "application/octet-stream")
            .with_header("content-length", "100")
            .with_body(&test_data)
            .create_async()
            .await;

        let url = Url::parse(&format!("{}/bytes/100", server.url())).unwrap();
        let (handler, returned_url) = RemoteFileHandlerFactory::create(&url, config).unwrap();
        assert_eq!(url, returned_url);
        let mut stream = handler.stream_file().await.unwrap();
        let mut data = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut stream, &mut data)
            .await
            .unwrap();

        assert_eq!(data.len(), 100);
        assert_eq!(data, test_data);
    }

    #[tokio::test]
    #[ignore = "Mockito has issues with HEAD requests and content-length headers"]
    async fn test_http_download_large_file_mock() {
        let config = RemoteFileConfig {
            max_file_size: 1024 * 1024,
            ..RemoteFileConfig::new(TEST_MAX_FILE_SIZE)
        };
        let mut server = Server::new_async().await;
        let _m = server
            .mock("HEAD", "/large-file.bin")
            .with_status(200)
            .with_header("content-length", "2097152")
            .with_header("content-type", "application/octet-stream")
            .create_async()
            .await;

        let url = Url::parse(&format!("{}/large-file.bin", server.url())).unwrap();
        let (handler, returned_url) = RemoteFileHandlerFactory::create(&url, config).unwrap();
        assert_eq!(url, returned_url);

        let result = handler.get_file_size().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2097152);
    }

    #[tokio::test]
    async fn test_http_with_redirects_mock() {
        let config = RemoteFileConfig {
            follow_redirects: true,
            max_redirects: 5,
            ..RemoteFileConfig::new(TEST_MAX_FILE_SIZE)
        };
        let mut server = Server::new_async().await;

        let final_content = b"Final destination content";

        let _m1 = server
            .mock("GET", "/start")
            .with_status(302)
            .with_header("Location", &format!("{}/final", server.url()))
            .create_async()
            .await;

        let _m2 = server
            .mock("GET", "/final")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_header("content-length", &final_content.len().to_string())
            .with_body(final_content)
            .create_async()
            .await;

        let url = Url::parse(&format!("{}/start", server.url())).unwrap();
        let (handler, returned_url) = RemoteFileHandlerFactory::create(&url, config).unwrap();
        assert_eq!(url, returned_url);

        let mut stream = handler.stream_file().await.unwrap();
        let mut data = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut stream, &mut data)
            .await
            .unwrap();

        assert_eq!(data, final_content);
    }

    #[tokio::test]
    async fn test_http_authentication_mock() {
        let config = RemoteFileConfig::new(TEST_MAX_FILE_SIZE);

        let mut server = Server::new_async().await;
        let _m_head = server
            .mock("HEAD", "/protected/resource")
            .with_status(401)
            .with_header("WWW-Authenticate", "Basic realm=\"Protected\"")
            .create_async()
            .await;

        let url_no_auth = Url::parse(&format!("{}/protected/resource", server.url())).unwrap();
        let (handler_no_auth, returned_url) =
            RemoteFileHandlerFactory::create(&url_no_auth, config).unwrap();
        assert_eq!(url_no_auth, returned_url);
        let result = handler_no_auth.get_file_size().await;
        assert!(matches!(result, Err(RemoteFileError::AccessDenied)));
    }

    #[tokio::test]
    async fn test_http_timeout_handling() {
        let config = RemoteFileConfig {
            connection_timeout: 1,
            read_timeout: 1,
            ..RemoteFileConfig::new(TEST_MAX_FILE_SIZE)
        };

        let url = Url::parse("http://10.255.255.1/timeout-test").unwrap();
        let (handler, returned_url) = RemoteFileHandlerFactory::create(&url, config).unwrap();
        assert_eq!(url, returned_url);
        let result = handler.stream_file().await;
        assert!(matches!(result, Err(RemoteFileError::Timeout)));
    }
}

#[cfg(test)]
mod handler_trait_tests {
    use super::*;
    use async_trait::async_trait;
    use bytes::Bytes;
    use std::io::Cursor;
    use tokio::io::AsyncRead;

    struct MockHandler {
        supported_scheme: String,
        file_content: Vec<u8>,
        file_size: u64,
    }

    #[async_trait]
    impl RemoteFileHandler for MockHandler {
        async fn get_file_size(&self) -> Result<u64, RemoteFileError> {
            Ok(self.file_size)
        }

        async fn stream_file(&self) -> Result<Box<dyn AsyncRead + Send + Unpin>, RemoteFileError> {
            let cursor = Cursor::new(self.file_content.clone());
            Ok(Box::new(cursor))
        }

        async fn download_chunk(&self, offset: u64, length: u64) -> Result<Bytes, RemoteFileError> {
            let end = std::cmp::min(offset + length, self.file_content.len() as u64) as usize;
            let start = offset as usize;
            Ok(Bytes::from(self.file_content[start..end].to_vec()))
        }

        fn is_supported(&self, url: &Url) -> bool {
            url.scheme() == self.supported_scheme
        }

        async fn upload_file(
            &self,
            _data: Box<dyn tokio::io::AsyncRead + Send + Unpin>,
            _size: u64,
            _content_type: Option<String>,
        ) -> Result<(), RemoteFileError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_mock_handler_get_file_size() {
        let handler = MockHandler {
            supported_scheme: "mock".to_string(),
            file_content: b"test content".to_vec(),
            file_size: 12,
        };

        let _url = Url::parse("mock://example.com/file.txt").unwrap();
        let size = handler.get_file_size().await.unwrap();

        assert_eq!(size, 12);
    }

    #[tokio::test]
    async fn test_mock_handler_stream() {
        let handler = MockHandler {
            supported_scheme: "mock".to_string(),
            file_content: b"streaming data".to_vec(),
            file_size: 14,
        };

        let _url = Url::parse("mock://example.com/file.txt").unwrap();
        let mut stream = handler.stream_file().await.unwrap();

        let mut buffer = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut stream, &mut buffer)
            .await
            .unwrap();

        assert_eq!(buffer, b"streaming data");
    }

    #[tokio::test]
    async fn test_mock_handler_chunk() {
        let handler = MockHandler {
            supported_scheme: "mock".to_string(),
            file_content: b"0123456789abcdef".to_vec(),
            file_size: 16,
        };

        let _url = Url::parse("mock://example.com/file.txt").unwrap();

        let chunk = handler.download_chunk(5, 5).await.unwrap();
        assert_eq!(chunk.as_ref(), b"56789");
        let chunk = handler.download_chunk(10, 10).await.unwrap();
        assert_eq!(chunk.as_ref(), b"abcdef");
    }

    #[tokio::test]
    async fn test_mock_handler_unsupported() {
        let handler = MockHandler {
            supported_scheme: "mock".to_string(),
            file_content: vec![],
            file_size: 0,
        };

        let url = Url::parse("http://example.com/file.txt").unwrap();

        assert!(!handler.is_supported(&url));

        let result = handler.get_file_size().await;
        assert!(result.is_ok());
    }
}
