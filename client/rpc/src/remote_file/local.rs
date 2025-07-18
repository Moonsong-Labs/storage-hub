use super::{RemoteFileError, RemoteFileHandler};
use async_trait::async_trait;
use bytes::Bytes;
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use url::Url;

#[derive(Debug, Clone)]
pub struct LocalFileHandler;

impl LocalFileHandler {
    pub fn new() -> Self {
        Self
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

impl Default for LocalFileHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RemoteFileHandler for LocalFileHandler {
    async fn fetch_metadata(&self, url: &Url) -> Result<(u64, Option<String>), RemoteFileError> {
        let path = Self::url_to_path(url)?;
        Self::validate_file(&path).await?;

        let metadata = tokio::fs::metadata(&path).await?;
        let size = metadata.len();

        let content_type = path
            .extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| {
                mime_guess::from_ext(ext)
                    .first()
                    .map(|mime| mime.to_string())
            });

        Ok((size, content_type))
    }

    async fn stream_file(
        &self,
        url: &Url,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>, RemoteFileError> {
        let path = Self::url_to_path(url)?;
        Self::validate_file(&path).await?;

        let file = File::open(&path).await?;
        Ok(Box::new(file))
    }

    async fn download_chunk(
        &self,
        url: &Url,
        offset: u64,
        length: u64,
    ) -> Result<Bytes, RemoteFileError> {
        let path = Self::url_to_path(url)?;
        Self::validate_file(&path).await?;

        let mut file = File::open(&path).await?;

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
        url: &Url,
        mut data: Box<dyn tokio::io::AsyncRead + Send + Unpin>,
        _size: u64,
        _content_type: Option<String>,
    ) -> Result<(), RemoteFileError> {
        let path = Self::url_to_path(url)?;

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

    #[tokio::test]
    async fn test_local_file_metadata() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_content = b"Hello, StorageHub!";
        temp_file.write_all(test_content).unwrap();
        temp_file.flush().unwrap();

        let handler = LocalFileHandler::new();

        let url = Url::parse(&format!("file://{}", temp_file.path().display())).unwrap();
        let (size, _content_type) = handler.fetch_metadata(&url).await.unwrap();
        assert_eq!(size, test_content.len() as u64);
    }

    #[tokio::test]
    async fn test_local_file_stream() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_content = b"Hello, StorageHub!";
        temp_file.write_all(test_content).unwrap();
        temp_file.flush().unwrap();

        let handler = LocalFileHandler::new();
        let url = Url::parse(&format!("file://{}", temp_file.path().display())).unwrap();

        let mut stream = handler.stream_file(&url).await.unwrap();
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

        let handler = LocalFileHandler::new();
        let url = Url::parse(&format!("file://{}", temp_file.path().display())).unwrap();

        let chunk = handler.download_chunk(&url, 7, 10).await.unwrap();
        assert_eq!(&chunk[..], &test_content[7..17]);
    }

    #[tokio::test]
    async fn test_file_not_found() {
        let handler = LocalFileHandler::new();
        let url = Url::parse("file:///non/existent/file.txt").unwrap();

        let result = handler.fetch_metadata(&url).await;
        assert!(matches!(result, Err(RemoteFileError::IoError(_))));
    }

    #[tokio::test]
    async fn test_url_schemes() {
        let handler = LocalFileHandler::new();

        let file_url = Url::parse("file:///path/to/file.txt").unwrap();
        assert!(handler.is_supported(&file_url));

        // Test that regular paths can be converted to URLs
        let path_url = Url::from_file_path("/path/to/file.txt").unwrap();
        assert!(handler.is_supported(&path_url));

        let http_url = Url::parse("http://example.com/file.txt").unwrap();
        assert!(!handler.is_supported(&http_url));
    }

    #[tokio::test]
    async fn test_upload_file_with_file_url() {
        let handler = LocalFileHandler::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("uploaded_file.txt");
        let file_url = format!("file://{}", file_path.display());

        let test_content = b"Hello, uploaded file!";
        let data: Box<dyn AsyncRead + Send + Unpin> = Box::new(std::io::Cursor::new(test_content));

        let url = Url::parse(&file_url).unwrap();
        handler
            .upload_file(&url, data, test_content.len() as u64, None)
            .await
            .unwrap();

        let content = tokio::fs::read(&file_path).await.unwrap();
        assert_eq!(content, test_content);
    }

    #[tokio::test]
    async fn test_upload_file_with_plain_path() {
        let handler = LocalFileHandler::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("uploaded_file2.txt");

        let test_content = b"Plain path upload test";
        let data: Box<dyn AsyncRead + Send + Unpin> = Box::new(std::io::Cursor::new(test_content));

        let url = Url::from_file_path(&file_path).unwrap();
        handler
            .upload_file(
                &url,
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
        let handler = LocalFileHandler::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("nested/dirs/uploaded_file.txt");

        let test_content = b"Nested directory test";
        let data: Box<dyn AsyncRead + Send + Unpin> = Box::new(std::io::Cursor::new(test_content));

        let url = Url::from_file_path(&file_path).unwrap();
        handler
            .upload_file(&url, data, test_content.len() as u64, None)
            .await
            .unwrap();

        assert!(file_path.exists());
        let content = tokio::fs::read(&file_path).await.unwrap();
        assert_eq!(content, test_content);
    }

    #[tokio::test]
    async fn test_upload_file_overwrites_existing() {
        let handler = LocalFileHandler::new();
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Old content").unwrap();
        temp_file.flush().unwrap();

        let test_content = b"New content";
        let data: Box<dyn AsyncRead + Send + Unpin> = Box::new(std::io::Cursor::new(test_content));

        let url = Url::from_file_path(temp_file.path()).unwrap();
        handler
            .upload_file(&url, data, test_content.len() as u64, None)
            .await
            .unwrap();

        let content = tokio::fs::read(temp_file.path()).await.unwrap();
        assert_eq!(content, test_content);
    }

    #[tokio::test]
    async fn test_upload_large_file_streaming() {
        let handler = LocalFileHandler::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("large_file.bin");

        let large_content = vec![0xAB; 1024 * 1024];
        let data: Box<dyn AsyncRead + Send + Unpin> =
            Box::new(std::io::Cursor::new(large_content.clone()));

        let url = Url::from_file_path(&file_path).unwrap();
        handler
            .upload_file(&url, data, large_content.len() as u64, None)
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

        let handler = LocalFileHandler::new();
        let temp_dir = tempfile::tempdir().unwrap();

        tokio::fs::set_permissions(temp_dir.path(), std::fs::Permissions::from_mode(0o555))
            .await
            .unwrap();

        let file_path = temp_dir.path().join("no_permission.txt");
        let test_content = b"Should fail";
        let data: Box<dyn AsyncRead + Send + Unpin> = Box::new(std::io::Cursor::new(test_content));

        let url = Url::from_file_path(&file_path).unwrap();
        let result = handler
            .upload_file(&url, data, test_content.len() as u64, None)
            .await;

        tokio::fs::set_permissions(temp_dir.path(), std::fs::Permissions::from_mode(0o755))
            .await
            .unwrap();

        assert!(matches!(result, Err(RemoteFileError::AccessDenied)));
    }
}
