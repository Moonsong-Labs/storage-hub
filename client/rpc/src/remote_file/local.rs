use async_trait::async_trait;
use bytes::Bytes;
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use url::Url;

use super::{RemoteFileConfig, RemoteFileError, RemoteFileHandler};

#[derive(Debug, Clone)]
pub struct LocalFileHandler {
    file_path: PathBuf,
    config: RemoteFileConfig,
}

impl LocalFileHandler {
    pub fn new(url: &Url, config: RemoteFileConfig) -> Result<Self, RemoteFileError> {
        let file_path = Self::url_to_path(url)?;
        Ok(Self { file_path, config })
    }

    fn url_to_path(url: &Url) -> Result<PathBuf, RemoteFileError> {
        match url.scheme() {
            "" => Ok(PathBuf::from(url.path())),
            "file" => url
                .to_file_path()
                .map_err(|_| RemoteFileError::InvalidUrl(format!("Invalid file URL: {}", url))),
            scheme => Err(RemoteFileError::UnsupportedProtocol(scheme.to_string())),
        }
    }

    async fn validate_file(path: &Path) -> Result<(), RemoteFileError> {
        let metadata = tokio::fs::metadata(path).await.map_err(|e| {
            // Preserve original IO errors to maintain OS error messages
            RemoteFileError::IoError(e)
        })?;

        if !metadata.is_file() {
            return Err(RemoteFileError::Other(format!(
                "Path is not a file: {}",
                path.display()
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl RemoteFileHandler for LocalFileHandler {
    async fn get_file_size(&self) -> Result<u64, RemoteFileError> {
        Self::validate_file(&self.file_path).await?;

        let metadata = tokio::fs::metadata(&self.file_path).await?;
        Ok(metadata.len())
    }

    async fn stream_file(&self) -> Result<Box<dyn AsyncRead + Send + Unpin>, RemoteFileError> {
        Self::validate_file(&self.file_path).await?;

        let file = File::open(&self.file_path).await?;
        // Wrap file in a buffered reader that uses the configured chunk size
        let buffered_reader = tokio::io::BufReader::with_capacity(self.config.chunk_size, file);
        Ok(Box::new(buffered_reader))
    }

    async fn download_chunk(&self, offset: u64, length: u64) -> Result<Bytes, RemoteFileError> {
        Self::validate_file(&self.file_path).await?;

        let mut file = File::open(&self.file_path).await?;

        file.seek(std::io::SeekFrom::Start(offset)).await?;

        let mut buffer = vec![0u8; length as usize];
        file.read_exact(&mut buffer).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                RemoteFileError::Other("Requested chunk extends beyond file size".to_string())
            } else {
                RemoteFileError::IoError(e)
            }
        })?;

        Ok(Bytes::from(buffer))
    }

    fn is_supported(&self, url: &Url) -> bool {
        matches!(url.scheme(), "" | "file")
    }

    async fn upload_file(
        &self,
        mut data: Box<dyn tokio::io::AsyncRead + Send + Unpin>,
        _size: u64,
        _content_type: Option<String>,
    ) -> Result<(), RemoteFileError> {
        // Upload to the configured file path
        let path = &self.file_path;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    RemoteFileError::AccessDenied
                } else {
                    RemoteFileError::IoError(e)
                }
            })?;
        }

        let mut file = File::create(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                RemoteFileError::AccessDenied
            } else {
                RemoteFileError::IoError(e)
            }
        })?;

        io::copy(&mut data, &mut file).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                RemoteFileError::AccessDenied
            } else if e.kind() == std::io::ErrorKind::Other {
                let error_str = e.to_string().to_lowercase();
                if error_str.contains("space") || error_str.contains("disk full") {
                    RemoteFileError::Other("Insufficient disk space".to_string())
                } else {
                    RemoteFileError::IoError(e)
                }
            } else {
                RemoteFileError::IoError(e)
            }
        })?;

        file.flush().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    const TEST_MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB for tests

    #[tokio::test]
    async fn test_local_file_size() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_content = b"Hello, StorageHub!";
        temp_file.write_all(test_content).unwrap();
        temp_file.flush().unwrap();

        let url = Url::parse(&format!("file://{}", temp_file.path().display())).unwrap();
        let handler =
            LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE)).unwrap();

        let size = handler.get_file_size().await.unwrap();
        assert_eq!(size, test_content.len() as u64);
    }

    #[tokio::test]
    async fn test_local_file_stream() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_content = b"Hello, StorageHub!";
        temp_file.write_all(test_content).unwrap();
        temp_file.flush().unwrap();

        let url = Url::parse(&format!("file://{}", temp_file.path().display())).unwrap();
        let handler =
            LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE)).unwrap();

        let mut stream = handler.stream_file().await.unwrap();
        let mut buffer = Vec::new();
        stream.read_to_end(&mut buffer).await.unwrap();

        assert_eq!(buffer, test_content);
    }

    #[tokio::test]
    async fn test_local_file_chunk_download() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_content = b"Hello, StorageHub! This is a test file.";
        temp_file.write_all(test_content).unwrap();
        temp_file.flush().unwrap();

        let url = Url::parse(&format!("file://{}", temp_file.path().display())).unwrap();
        let handler =
            LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE)).unwrap();

        let chunk = handler.download_chunk(7, 10).await.unwrap();
        assert_eq!(&chunk[..], &test_content[7..17]);
    }

    #[tokio::test]
    async fn test_file_not_found() {
        let url = Url::parse("file:///non/existent/file.txt").unwrap();
        let handler =
            LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE)).unwrap();

        let result = handler.get_file_size().await;
        assert!(matches!(result, Err(RemoteFileError::IoError(_))));
    }

    #[tokio::test]
    async fn test_url_schemes() {
        let file_url = Url::parse("file:///path/to/file.txt").unwrap();
        let handler =
            LocalFileHandler::new(&file_url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE)).unwrap();
        assert!(handler.is_supported(&file_url));

        // Test that regular paths can be converted to URLs
        let path_url = Url::from_file_path("/path/to/file.txt").unwrap();
        assert!(handler.is_supported(&path_url));

        let http_url = Url::parse("http://example.com/file.txt").unwrap();
        assert!(!handler.is_supported(&http_url));
    }

    #[tokio::test]
    async fn test_upload_file_with_file_url() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("uploaded_file.txt");
        let file_url = format!("file://{}", file_path.display());
        let url = Url::parse(&file_url).unwrap();
        let handler =
            LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE)).unwrap();

        let test_content = b"Hello, uploaded file!";
        let data: Box<dyn AsyncRead + Send + Unpin> = Box::new(std::io::Cursor::new(test_content));

        handler
            .upload_file(data, test_content.len() as u64, None)
            .await
            .unwrap();

        let content = tokio::fs::read(&file_path).await.unwrap();
        assert_eq!(content, test_content);
    }

    #[tokio::test]
    async fn test_upload_file_with_plain_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("uploaded_file2.txt");
        let url = Url::from_file_path(&file_path).unwrap();
        let handler =
            LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE)).unwrap();

        let test_content = b"Plain path upload test";
        let data: Box<dyn AsyncRead + Send + Unpin> = Box::new(std::io::Cursor::new(test_content));

        handler
            .upload_file(
                data,
                test_content.len() as u64,
                Some("text/plain".to_string()),
            )
            .await
            .unwrap();

        let content = tokio::fs::read(&file_path).await.unwrap();
        assert_eq!(content, test_content);
    }

    #[tokio::test]
    async fn test_upload_file_creates_parent_directories() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("nested/dirs/uploaded_file.txt");
        let url = Url::from_file_path(&file_path).unwrap();
        // Create handler with target path
        let handler =
            LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE)).unwrap();

        let test_content = b"Nested directory test";
        let data: Box<dyn AsyncRead + Send + Unpin> = Box::new(std::io::Cursor::new(test_content));

        handler
            .upload_file(data, test_content.len() as u64, None)
            .await
            .unwrap();

        assert!(file_path.exists());
        let content = tokio::fs::read(&file_path).await.unwrap();
        assert_eq!(content, test_content);
    }

    #[tokio::test]
    async fn test_upload_file_overwrites_existing() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Old content").unwrap();
        temp_file.flush().unwrap();

        let url = Url::from_file_path(temp_file.path()).unwrap();
        let handler =
            LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE)).unwrap();

        let test_content = b"New content";
        let data: Box<dyn AsyncRead + Send + Unpin> = Box::new(std::io::Cursor::new(test_content));

        handler
            .upload_file(data, test_content.len() as u64, None)
            .await
            .unwrap();

        let content = tokio::fs::read(temp_file.path()).await.unwrap();
        assert_eq!(content, test_content);
    }

    #[tokio::test]
    async fn test_upload_large_file_streaming() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("large_file.bin");
        let url = Url::from_file_path(&file_path).unwrap();
        let handler =
            LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE)).unwrap();

        let large_content = vec![0xAB; 1024 * 1024];
        let data: Box<dyn AsyncRead + Send + Unpin> =
            Box::new(std::io::Cursor::new(large_content.clone()));

        handler
            .upload_file(data, large_content.len() as u64, None)
            .await
            .unwrap();

        let metadata = tokio::fs::metadata(&file_path).await.unwrap();
        assert_eq!(metadata.len(), large_content.len() as u64);

        let content = tokio::fs::read(&file_path).await.unwrap();
        assert_eq!(content.len(), large_content.len());
        assert_eq!(content[0], 0xAB);
        assert_eq!(content[content.len() - 1], 0xAB);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_upload_file_permission_denied() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("no_permission.txt");
        let url = Url::from_file_path(&file_path).unwrap();
        let handler =
            LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE)).unwrap();

        tokio::fs::set_permissions(temp_dir.path(), std::fs::Permissions::from_mode(0o555))
            .await
            .unwrap();

        let test_content = b"Should fail";
        let data: Box<dyn AsyncRead + Send + Unpin> = Box::new(std::io::Cursor::new(test_content));

        let result = handler
            .upload_file(data, test_content.len() as u64, None)
            .await;

        tokio::fs::set_permissions(temp_dir.path(), std::fs::Permissions::from_mode(0o755))
            .await
            .unwrap();

        assert!(matches!(result, Err(RemoteFileError::AccessDenied)));
    }
}
