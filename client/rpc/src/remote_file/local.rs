use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use bytes::Bytes;
use tokio::{
    fs::{self, File},
    io::{self, AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
};
use url::Url;

use super::{RemoteFileConfig, RemoteFileError, RemoteFileHandler};

#[derive(Debug, Clone)]
pub struct LocalFileHandler {
    file_path: PathBuf,
    config: RemoteFileConfig,

    file_exists_and_valid: bool,
    has_write_permission: bool,
}

impl LocalFileHandler {
    pub fn new(url: &Url, config: RemoteFileConfig) -> Result<Self, RemoteFileError> {
        let file_path = Self::url_to_path(url)?;

        // Check file permissions
        let mut current_path = file_path.as_path();
        let mut traversed = false;

        // Find the first existing path
        let existing_path = loop {
            if current_path.exists() {
                break current_path;
            }

            traversed = true;
            current_path = match current_path.parent() {
                Some(p) if !p.as_os_str().is_empty() => p,
                _ => std::path::Path::new("."),
            };
        };

        let (file_exists_and_valid, has_write_permission) = if traversed {
            // File doesn't exist, check write permissions on parent directory
            // by trying to create a temporary file
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let temp_name = format!(".storagehub_test_{}_{}", std::process::id(), timestamp);
            let temp_path = existing_path.join(&temp_name);

            let has_write = match std::fs::File::create(&temp_path) {
                Ok(_) => {
                    // Clean up the test file
                    let _ = std::fs::remove_file(&temp_path);
                    true
                }
                _ => false,
            };

            (false, has_write)
        } else {
            // File exists, check if it's a valid file and permissions
            let metadata = std::fs::metadata(&file_path).map_err(RemoteFileError::IoError)?;
            let is_valid_file = metadata.is_file();

            // Try to open for reading to verify read permissions
            std::fs::File::open(&file_path).map_err(RemoteFileError::IoError)?;

            // Try to open for writing to check write permissions
            let has_write = match std::fs::OpenOptions::new().write(true).open(&file_path) {
                Ok(_) => true,
                _ => false,
            };

            (is_valid_file, has_write)
        };

        Ok(Self {
            file_path,
            config,
            // Technically we could have a "time of check vs time of use" problem,
            // where we had the right permissions at this point in time but not later
            file_exists_and_valid,
            has_write_permission,
        })
    }

    fn url_to_path(url: &Url) -> Result<PathBuf, RemoteFileError> {
        let path = match url.scheme() {
            "" => PathBuf::from(url.path()),
            "file" => {
                // Try to convert to file path first
                url.to_file_path().map_err(|err| {
                    RemoteFileError::InvalidUrl(format!("Invalid file URL ({url}): {err:?}"))
                })?
            }
            scheme => return Err(RemoteFileError::UnsupportedProtocol(scheme.to_string())),
        };

        // Convert relative paths to absolute
        if path.is_absolute() {
            Ok(path)
        } else {
            std::env::current_dir()
                .map_err(|e| RemoteFileError::IoError(e))
                .map(|cwd| cwd.join(path))
        }
    }

    async fn get_metadata(&self) -> Result<std::fs::Metadata, RemoteFileError> {
        tokio::fs::metadata(&self.file_path).await.map_err(|e| {
            // Preserve original IO errors to maintain OS error messages
            RemoteFileError::IoError(e)
        })
    }

    fn check_file_valid(&self) -> Result<(), RemoteFileError> {
        if !self.file_exists_and_valid {
            return Err(RemoteFileError::Other(format!(
                "Path is not a valid file: {}",
                self.file_path.display()
            )));
        }
        Ok(())
    }

    fn check_write_permission(&self) -> Result<(), RemoteFileError> {
        if !self.has_write_permission {
            return Err(RemoteFileError::AccessDenied);
        }
        Ok(())
    }
}

#[async_trait]
impl RemoteFileHandler for LocalFileHandler {
    async fn get_file_size(&self) -> Result<u64, RemoteFileError> {
        self.check_file_valid()?;

        let metadata = self.get_metadata().await?;
        Ok(metadata.len())
    }

    async fn stream_file(&self) -> Result<Box<dyn AsyncRead + Send + Unpin>, RemoteFileError> {
        self.check_file_valid()?;

        let file = File::open(&self.file_path).await?;
        // Wrap file in a buffered reader that uses the configured chunk size
        let buffered_reader = tokio::io::BufReader::with_capacity(self.config.chunk_size, file);
        Ok(Box::new(buffered_reader))
    }

    async fn download_chunk(&self, offset: u64, length: u64) -> Result<Bytes, RemoteFileError> {
        self.check_file_valid()?;

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
        self.check_write_permission()?;

        if let Some(parent) = self.file_path.parent() {
            // Ensure path exists
            fs::create_dir_all(parent).await.map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    RemoteFileError::AccessDenied
                } else {
                    RemoteFileError::IoError(e)
                }
            })?;
        }

        let mut file = File::create(&self.file_path).await.map_err(|e| {
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
    use serial_test::serial;

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

    #[test]
    fn test_permission_validation() {
        use tempfile::TempDir;

        // Create a unique temporary directory
        let temp_dir = TempDir::new().unwrap();

        // Test with existing readable file
        let readable_file = temp_dir.path().join("readable_test.txt");
        std::fs::write(&readable_file, b"test").unwrap();

        let url = Url::from_file_path(&readable_file).unwrap();
        let result = LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE));
        assert!(result.is_ok(), "Should accept readable file");

        // Test with non-existent file in writable directory
        let new_file = temp_dir.path().join("new_file.txt");
        let url = Url::from_file_path(&new_file).unwrap();
        let result = LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE));
        assert!(
            result.is_ok(),
            "Should accept non-existent file in writable directory"
        );

        // Test that non-existent parent directory is allowed
        let path_with_nonexistent_parent = temp_dir.path().join("non/existent/directory/file.txt");
        let url = Url::from_file_path(&path_with_nonexistent_parent).unwrap();
        let result = LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE));
        assert!(
            result.is_ok(),
            "Should allow file with non-existent parent directory"
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_access_denied() {
        use std::os::unix::fs::PermissionsExt;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // Create a file and make it unreadable
        let unreadable_file = temp_dir.path().join("unreadable_test.txt");
        std::fs::write(&unreadable_file, b"test").unwrap();
        let mut perms = std::fs::metadata(&unreadable_file).unwrap().permissions();
        perms.set_mode(0o000); // No permissions
        std::fs::set_permissions(&unreadable_file, perms).unwrap();

        let url = Url::from_file_path(&unreadable_file).unwrap();
        let result = LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE));

        // Restore permissions before asserting (in case of panic)
        let mut perms = std::fs::metadata(&unreadable_file).unwrap().permissions();
        perms.set_mode(0o644);
        std::fs::set_permissions(&unreadable_file, perms).unwrap();

        assert!(
            matches!(result, Err(RemoteFileError::IoError(_))),
            "Should return IoError for unreadable file"
        );
    }

    #[test]
    #[serial(cwd)]
    fn test_relative_path_handling() {
        use tempfile::TempDir;

        // Create a unique temporary directory and set it as current dir
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test_relative.txt");
        std::fs::write(&test_file, b"test").unwrap();

        // Save old current dir and change to temp dir
        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Test with relative path through empty scheme URL
        let url = Url::parse(":./test_relative.txt").unwrap();
        let result = LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE));
        assert!(
            result.is_ok(),
            "Should handle relative path: {:?}",
            result.err()
        );

        // Restore old current dir
        std::env::set_current_dir(old_dir).unwrap();
    }

    #[test]
    #[serial(cwd)]
    fn test_bare_path_handling() {
        use tempfile::TempDir;

        // Create a unique temporary directory and set it as current dir
        let temp_dir = TempDir::new().unwrap();
        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create test directory structure inside temp dir
        let test_dir = temp_dir.path().join("test_bare_dir");
        std::fs::create_dir_all(&test_dir).unwrap();
        let test_file = test_dir.join("bar.txt");
        std::fs::write(&test_file, b"test").unwrap();

        // Test with bare paths through empty scheme URLs
        let bare_paths = vec![
            "test_bare_dir/bar.txt",
            "test_bare_dir/../test_bare_dir/bar.txt",
        ];

        for path in &bare_paths {
            let url = Url::parse(&format!(":{}", path)).unwrap();
            let result = LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE));
            assert!(
                result.is_ok(),
                "Should handle bare path '{}': {:?}",
                path,
                result.err()
            );
        }

        // Test with a bare filename that doesn't exist (should still work for file creation)
        let url = Url::parse(":nonexistent_file.txt").unwrap();
        let result = LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE));
        assert!(
            result.is_ok(),
            "Should handle bare filename for file creation: {:?}",
            result.err()
        );

        // Additional bare path edge cases
        let edge_cases = vec![
            "file with spaces.txt",
            "файл.txt",
            ".hiddenfile",
            "README",
            "weird!@#$.txt",
        ];

        for case in &edge_cases {
            // Create the file
            std::fs::write(case, b"test").unwrap();
            let url = Url::parse(&format!(":{}", case)).unwrap();
            let result = LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE));
            assert!(
                result.is_ok(),
                "Should handle bare path edge case '{}': {:?}",
                case,
                result.err()
            );
            // Clean up
            let _ = std::fs::remove_file(case);
        }

        // Test directory with is_file check (should fail)
        let url = Url::parse(":test_bare_dir").unwrap();
        let result = LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE));
        // This should succeed in constructor but fail when trying to use as file
        assert!(result.is_ok());

        // Restore old current dir
        std::env::set_current_dir(old_dir).unwrap();
    }
}
