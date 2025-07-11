//! Local file system handler implementation
//!
//! This module provides functionality for handling local files using the RemoteFileHandler trait.
//! It supports both absolute paths and file:// URLs.

use super::{RemoteFileError, RemoteFileHandler};
use async_trait::async_trait;
use bytes::Bytes;
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use url::Url;

/// Handler for local file system operations
#[derive(Debug, Clone)]
pub struct LocalFileHandler;

impl LocalFileHandler {
    /// Create a new LocalFileHandler instance
    pub fn new() -> Self {
        Self
    }

    /// Convert a URL to a local file path
    fn url_to_path(url: &Url) -> Result<PathBuf, RemoteFileError> {
        match url.scheme() {
            "" => {
                // No scheme - treat as local path
                Ok(PathBuf::from(url.path()))
            }
            "file" => {
                // file:// URL - convert to path
                url.to_file_path()
                    .map_err(|_| RemoteFileError::InvalidUrl(format!("Invalid file URL: {}", url)))
            }
            scheme => Err(RemoteFileError::UnsupportedProtocol(scheme.to_string())),
        }
    }

    /// Check if a path exists and is a file
    async fn validate_file(path: &Path) -> Result<(), RemoteFileError> {
        let metadata = tokio::fs::metadata(path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                RemoteFileError::NotFound
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                RemoteFileError::AccessDenied
            } else {
                RemoteFileError::IoError(e)
            }
        })?;

        if !metadata.is_file() {
            return Err(RemoteFileError::Other(format!(
                "Path is not a file: {}",
                path.display()
            )));
        }

        Ok(())
    }

    /// Parse a URI string to a local file path
    /// Handles both file:// URLs and plain paths
    fn parse_uri_to_path(uri: &str) -> Result<PathBuf, RemoteFileError> {
        // Try to parse as URL first
        if let Ok(url) = Url::parse(uri) {
            Self::url_to_path(&url)
        } else {
            // Not a valid URL, treat as plain path
            Ok(PathBuf::from(uri))
        }
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

        // Try to determine content type from file extension
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

        // Seek to the requested offset
        file.seek(std::io::SeekFrom::Start(offset)).await?;

        // Read the requested chunk
        let mut buffer = vec![0u8; length as usize];
        let bytes_read = file.read_exact(&mut buffer).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                RemoteFileError::Other("Requested chunk extends beyond file size".to_string())
            } else {
                RemoteFileError::IoError(e)
            }
        })?;

        buffer.truncate(bytes_read);
        Ok(Bytes::from(buffer))
    }

    fn is_supported(&self, url: &Url) -> bool {
        matches!(url.scheme(), "" | "file")
    }

    async fn upload_file(
        &self,
        uri: &str,
        mut data: Box<dyn tokio::io::AsyncRead + Send + Unpin>,
        _size: u64,
        _content_type: Option<String>,
    ) -> Result<(), RemoteFileError> {
        // Parse the URI to get the file path
        let path = Self::parse_uri_to_path(uri)?;

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    RemoteFileError::AccessDenied
                } else {
                    RemoteFileError::IoError(e)
                }
            })?;
        }

        // Create the file
        let mut file = File::create(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                RemoteFileError::AccessDenied
            } else {
                RemoteFileError::IoError(e)
            }
        })?;

        // Stream data from AsyncRead to the file
        io::copy(&mut data, &mut file).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                RemoteFileError::AccessDenied
            } else if e.kind() == std::io::ErrorKind::Other {
                // Check if the error message contains disk space related keywords
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

        // Ensure all data is written to disk
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
        // Create a temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_content = b"Hello, StorageHub!";
        temp_file.write_all(test_content).unwrap();
        temp_file.flush().unwrap();

        let handler = LocalFileHandler::new();

        // Test with absolute path
        let url = Url::parse(&format!("file://{}", temp_file.path().display())).unwrap();
        let (size, content_type) = handler.fetch_metadata(&url).await.unwrap();
        assert_eq!(size, test_content.len() as u64);
    }

    #[tokio::test]
    async fn test_local_file_stream() {
        // Create a temporary file
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
        // Create a temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_content = b"Hello, StorageHub! This is a test file.";
        temp_file.write_all(test_content).unwrap();
        temp_file.flush().unwrap();

        let handler = LocalFileHandler::new();
        let url = Url::parse(&format!("file://{}", temp_file.path().display())).unwrap();

        // Download a chunk from offset 7 with length 10
        let chunk = handler.download_chunk(&url, 7, 10).await.unwrap();
        assert_eq!(&chunk[..], &test_content[7..17]);
    }

    #[tokio::test]
    async fn test_file_not_found() {
        let handler = LocalFileHandler::new();
        let url = Url::parse("file:///non/existent/file.txt").unwrap();

        let result = handler.fetch_metadata(&url).await;
        assert!(matches!(result, Err(RemoteFileError::NotFound)));
    }

    #[tokio::test]
    async fn test_url_schemes() {
        let handler = LocalFileHandler::new();

        // file:// scheme should be supported
        let file_url = Url::parse("file:///path/to/file.txt").unwrap();
        assert!(handler.is_supported(&file_url));

        // Empty scheme (absolute path) should be supported
        let path_url = Url::parse("/path/to/file.txt").unwrap();
        assert!(handler.is_supported(&path_url));

        // HTTP should not be supported
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
        
        handler
            .upload_file(&file_url, data, test_content.len() as u64, None)
            .await
            .unwrap();
        
        // Verify the file was created with correct content
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
        
        handler
            .upload_file(
                file_path.to_str().unwrap(),
                data,
                test_content.len() as u64,
                Some("text/plain".to_string()),
            )
            .await
            .unwrap();
        
        // Verify the file was created with correct content
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
        
        handler
            .upload_file(
                file_path.to_str().unwrap(),
                data,
                test_content.len() as u64,
                None,
            )
            .await
            .unwrap();
        
        // Verify the file and directories were created
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
        
        handler
            .upload_file(
                temp_file.path().to_str().unwrap(),
                data,
                test_content.len() as u64,
                None,
            )
            .await
            .unwrap();
        
        // Verify the file was overwritten
        let content = tokio::fs::read(temp_file.path()).await.unwrap();
        assert_eq!(content, test_content);
    }

    #[tokio::test]
    async fn test_upload_large_file_streaming() {
        let handler = LocalFileHandler::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("large_file.bin");
        
        // Create a large content (1MB)
        let large_content = vec![0xAB; 1024 * 1024];
        let data: Box<dyn AsyncRead + Send + Unpin> = Box::new(std::io::Cursor::new(large_content.clone()));
        
        handler
            .upload_file(
                file_path.to_str().unwrap(),
                data,
                large_content.len() as u64,
                None,
            )
            .await
            .unwrap();
        
        // Verify the file was created with correct size
        let metadata = tokio::fs::metadata(&file_path).await.unwrap();
        assert_eq!(metadata.len(), large_content.len() as u64);
        
        // Verify content (sample check)
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
        
        // Make directory read-only
        tokio::fs::set_permissions(temp_dir.path(), std::fs::Permissions::from_mode(0o555))
            .await
            .unwrap();
        
        let file_path = temp_dir.path().join("no_permission.txt");
        let test_content = b"Should fail";
        let data: Box<dyn AsyncRead + Send + Unpin> = Box::new(std::io::Cursor::new(test_content));
        
        let result = handler
            .upload_file(
                file_path.to_str().unwrap(),
                data,
                test_content.len() as u64,
                None,
            )
            .await;
        
        // Reset permissions before cleanup
        tokio::fs::set_permissions(temp_dir.path(), std::fs::Permissions::from_mode(0o755))
            .await
            .unwrap();
        
        assert!(matches!(result, Err(RemoteFileError::AccessDenied)));
    }
}
