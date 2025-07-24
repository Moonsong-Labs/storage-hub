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
    absolute_file_path: PathBuf,
    config: RemoteFileConfig,

    file_exists_and_valid: bool,
    has_write_permission: bool,
}

impl LocalFileHandler {
    /// Creates a LocalFileHandler from a URL
    pub fn new(url: &Url, config: RemoteFileConfig) -> Result<Self, RemoteFileError> {
        let file_path = Self::url_to_path(url)?;
        Self::new_from_path_internal(file_path, config)
    }

    /// Creates a LocalFileHandler from a path string
    ///
    /// Handles relative paths by joining with the current working directory
    pub fn new_from_path(
        path_str: &str,
        config: RemoteFileConfig,
    ) -> Result<Self, RemoteFileError> {
        // Validate that we have a non-empty string
        if path_str.is_empty() {
            return Err(RemoteFileError::InvalidUrl("Empty path".to_string()));
        }

        let file_path = PathBuf::from(path_str);

        let file_path = if file_path.is_absolute() {
            file_path
        } else {
            // Join with current directory for relative paths
            std::env::current_dir()
                .map_err(RemoteFileError::IoError)?
                .join(file_path)
        };

        Self::new_from_path_internal(file_path, config)
    }

    /// Internal constructor that handles the actual initialization
    ///
    /// * `path` must be an absolute path
    fn new_from_path_internal(
        path: PathBuf,
        config: RemoteFileConfig,
    ) -> Result<Self, RemoteFileError> {
        assert!(
            path.is_absolute(),
            "can only instantiate handler for an absolute path"
        );

        // Check file permissions
        let mut current_path = path.as_path();
        let mut traversed = false;

        // Find the first existing path
        let existing_path = loop {
            if current_path.exists() {
                break current_path;
            }

            traversed = true;
            current_path = match current_path.parent() {
                Some(p) if !p.as_os_str().is_empty() => p,
                // TODO: check if this can actually happen:
                // 1. None is returned if `current_path` is a root folder (/ or windows prefix)
                // 2. Some("") is returned if `current_path` is a relative path
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
            let metadata = std::fs::metadata(&path).map_err(RemoteFileError::IoError)?;
            let is_valid_file = metadata.is_file();

            // Try to open for reading to verify read permissions
            std::fs::File::open(&path).map_err(RemoteFileError::IoError)?;

            // Try to open for writing to check write permissions
            let has_write = match std::fs::OpenOptions::new().write(true).open(&path) {
                Ok(_) => true,
                _ => false,
            };

            (is_valid_file, has_write)
        };

        Ok(Self {
            absolute_file_path: path,
            config,
            // Technically we could have a "time of check vs time of use" problem,
            // where we had the right permissions at this point in time but not later
            file_exists_and_valid,
            has_write_permission,
        })
    }

    fn url_to_path(url: &Url) -> Result<PathBuf, RemoteFileError> {
        match url.scheme() {
            "" => Ok(PathBuf::from(url.path())),
            "file" => {
                // Try to convert to file path first
                url.to_file_path()
                    .map_err(|_| RemoteFileError::InvalidUrl(format!("Invalid file URL: {url}")))
            }
            scheme => Err(RemoteFileError::UnsupportedProtocol(scheme.to_string())),
        }
    }

    /// Returns a canonical URL representation of the file path
    pub fn get_canonical_url(&self) -> Result<Url, RemoteFileError> {
        // TODO: this should only be able to fail in windows (bad disk prefix or UNC prefix)
        Url::from_file_path(&self.absolute_file_path).map_err(|_| {
            RemoteFileError::InvalidUrl(format!(
                "Cannot convert path to URL: {}",
                self.absolute_file_path.display()
            ))
        })
    }

    async fn get_metadata(&self) -> Result<std::fs::Metadata, RemoteFileError> {
        tokio::fs::metadata(&self.absolute_file_path)
            .await
            .map_err(|e| {
                // Preserve original IO errors to maintain OS error messages
                RemoteFileError::IoError(e)
            })
    }

    fn check_file_valid(&self) -> Result<(), RemoteFileError> {
        if !self.file_exists_and_valid {
            return Err(RemoteFileError::Other(format!(
                "Path is not a valid file: {}",
                self.absolute_file_path.display()
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

    /// Maps IO errors to RemoteFileError, converting PermissionDenied to AccessDenied
    fn map_io_error(e: std::io::Error) -> RemoteFileError {
        match e.kind() {
            std::io::ErrorKind::PermissionDenied => RemoteFileError::AccessDenied,
            std::io::ErrorKind::Other => {
                let error_str = e.to_string().to_lowercase();
                if error_str.contains("space") || error_str.contains("disk full") {
                    RemoteFileError::Other("Insufficient disk space".to_string())
                } else {
                    RemoteFileError::IoError(e)
                }
            }
            _ => RemoteFileError::IoError(e),
        }
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

        let file = File::open(&self.absolute_file_path).await?;

        // Wrap file in a buffered reader that uses the configured chunk size
        let buffered_reader = tokio::io::BufReader::with_capacity(self.config.chunk_size, file);
        Ok(Box::new(buffered_reader))
    }

    async fn download_chunk(&self, offset: u64, length: u64) -> Result<Bytes, RemoteFileError> {
        self.check_file_valid()?;

        let mut file = File::open(&self.absolute_file_path).await?;
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

        if let Some(parent) = self.absolute_file_path.parent() {
            // Ensure path exists
            fs::create_dir_all(parent)
                .await
                .map_err(Self::map_io_error)?;
        }

        let mut file = File::create(&self.absolute_file_path)
            .await
            .map_err(Self::map_io_error)?;

        io::copy(&mut data, &mut file)
            .await
            .map_err(Self::map_io_error)?;

        file.flush().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use serial_test::serial;
    use tempfile::{NamedTempFile, TempDir};

    use super::*;

    const TEST_MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB for tests

    #[tokio::test]
    async fn test_local_file_size() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_content = b"Hello, StorageHub!";
        temp_file.write_all(test_content).unwrap();
        temp_file.flush().unwrap();

        let handler = LocalFileHandler::new_from_path(
            temp_file.path().to_str().unwrap(),
            RemoteFileConfig::new(TEST_MAX_FILE_SIZE),
        )
        .unwrap();

        let size = handler.get_file_size().await.unwrap();
        assert_eq!(size, test_content.len() as u64);
    }

    #[tokio::test]
    async fn test_local_file_stream() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_content = b"Hello, StorageHub!";
        temp_file.write_all(test_content).unwrap();
        temp_file.flush().unwrap();

        let handler = LocalFileHandler::new_from_path(
            temp_file.path().to_str().unwrap(),
            RemoteFileConfig::new(TEST_MAX_FILE_SIZE),
        )
        .unwrap();

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

        let handler = LocalFileHandler::new_from_path(
            temp_file.path().to_str().unwrap(),
            RemoteFileConfig::new(TEST_MAX_FILE_SIZE),
        )
        .unwrap();

        let chunk = handler.download_chunk(7, 10).await.unwrap();
        assert_eq!(&chunk[..], &test_content[7..17]);
    }

    #[tokio::test]
    async fn test_file_not_found() {
        let handler = LocalFileHandler::new_from_path(
            "/non/existent/file.txt",
            RemoteFileConfig::new(TEST_MAX_FILE_SIZE),
        )
        .unwrap();

        let result = handler.get_file_size().await;
        assert!(matches!(result, Err(RemoteFileError::Other(_))));
    }

    #[tokio::test]
    async fn test_url_constructor() {
        // This test explicitly verifies the URL constructor path still works
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_content = b"Testing URL constructor";
        temp_file.write_all(test_content).unwrap();
        temp_file.flush().unwrap();

        // Create handler using URL constructor
        let url = Url::from_file_path(temp_file.path()).unwrap();
        let handler =
            LocalFileHandler::new(&url, RemoteFileConfig::new(TEST_MAX_FILE_SIZE)).unwrap();
        let canonical_url = handler.get_canonical_url().unwrap();

        // Check canonical URL matches between both constructors
        let str_handler = LocalFileHandler::new_from_path(
            temp_file.path().to_string_lossy().as_ref(),
            RemoteFileConfig::new(TEST_MAX_FILE_SIZE),
        )
        .unwrap();
        let canonical_url_str = str_handler.get_canonical_url().unwrap();

        assert_eq!(canonical_url, canonical_url_str);
    }

    #[tokio::test]
    async fn test_upload_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("uploaded_file2.txt");
        let handler = LocalFileHandler::new_from_path(
            file_path.to_str().unwrap(),
            RemoteFileConfig::new(TEST_MAX_FILE_SIZE),
        )
        .unwrap();

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
        // Create handler with target path
        let handler = LocalFileHandler::new_from_path(
            file_path.to_str().unwrap(),
            RemoteFileConfig::new(TEST_MAX_FILE_SIZE),
        )
        .unwrap();

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

        let handler = LocalFileHandler::new_from_path(
            temp_file.path().to_str().unwrap(),
            RemoteFileConfig::new(TEST_MAX_FILE_SIZE),
        )
        .unwrap();

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
        let handler = LocalFileHandler::new_from_path(
            file_path.to_str().unwrap(),
            RemoteFileConfig::new(TEST_MAX_FILE_SIZE),
        )
        .unwrap();

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
        let handler = LocalFileHandler::new_from_path(
            file_path.to_str().unwrap(),
            RemoteFileConfig::new(TEST_MAX_FILE_SIZE),
        )
        .unwrap();

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

        let handler = LocalFileHandler::new_from_path(
            readable_file.to_string_lossy().as_ref(),
            RemoteFileConfig::new(TEST_MAX_FILE_SIZE),
        )
        .expect("Should accept readable file");
        assert_eq!(handler.file_exists_and_valid, true);

        // Test with non-existent file in writable directory
        let new_file = temp_dir.path().join("new_file.txt");
        let handler = LocalFileHandler::new_from_path(
            new_file.to_string_lossy().as_ref(),
            RemoteFileConfig::new(TEST_MAX_FILE_SIZE),
        )
        .expect("Should accept non-existent file in writable directory");
        assert_eq!(handler.file_exists_and_valid, false);
        assert_eq!(handler.has_write_permission, true);

        // Test that non-existent parent directory is allowed
        let path_with_nonexistent_parent = temp_dir.path().join("non/existent/directory/file.txt");
        let handler = LocalFileHandler::new_from_path(
            path_with_nonexistent_parent.to_string_lossy().as_ref(),
            RemoteFileConfig::new(TEST_MAX_FILE_SIZE),
        )
        .expect("Should allow file with non-existent parent directory");
        assert_eq!(handler.file_exists_and_valid, false);
        assert_eq!(handler.has_write_permission, true);
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

        let result = LocalFileHandler::new_from_path(
            unreadable_file.to_string_lossy().as_ref(),
            RemoteFileConfig::new(TEST_MAX_FILE_SIZE),
        );

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
        let old_dir = std::env::current_dir().unwrap();

        // Create a unique temporary directory and set it as current dir
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Test with bare paths through empty scheme URLs
        let relative_paths = vec![
            "relative_dir/relative.txt",
            "relative_dir/../relative_dir/relative.txt",
            "relative.txt",
            "./relative.txt",
        ];

        for path in &relative_paths {
            let handler = match LocalFileHandler::new_from_path(
                path,
                RemoteFileConfig::new(TEST_MAX_FILE_SIZE),
            ) {
                Ok(handler) => handler,
                Err(e) => panic!("Should handle bare path '{path}' : {e:?}"),
            };

            let url = handler.get_canonical_url().unwrap();
            assert_eq!(url.scheme(), "file");
            assert!(url.path().ends_with("relative.txt"));
        }

        // Restore old current dir
        std::env::set_current_dir(old_dir).unwrap();
    }
}
