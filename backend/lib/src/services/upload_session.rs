use std::collections::HashSet;
use std::sync::{Arc, RwLock};

/// Manages active upload sessions to prevent concurrent uploads of the same file key.
///
/// Each session tracks a file key that is currently being uploaded, ensuring that
/// only one upload per file key can be active at any given time.
#[derive(Debug)]
pub struct UploadSessionManager {
    sessions: Arc<RwLock<HashSet<String>>>,
    max_sessions: usize,
}

impl UploadSessionManager {
    pub fn new(max_sessions: usize) -> Self {
        UploadSessionManager {
            sessions: Arc::new(RwLock::new(HashSet::new())),
            max_sessions,
        }
    }

    /// Atomically registers a new upload session for the given file key.
    /// Returns a guard that will automatically clean up the session when dropped.
    /// Returns an error if there is already an active upload for this file key
    /// or if the maximum number of concurrent uploads has been reached.
    pub fn start_upload(&self, file_key: String) -> Result<UploadSessionGuard, String> {
        let mut sessions = self
            .sessions
            .write()
            .expect("Upload sessions lock poisoned");

        if sessions.len() >= self.max_sessions {
            return Err(format!(
                "Maximum number of {} concurrent uploads reached",
                self.max_sessions
            ));
        }

        if sessions.contains(&file_key) {
            return Err(format!(
                "File key {} is already being uploaded. Please wait for the current upload to complete.",
                file_key
            ));
        }

        sessions.insert(file_key.clone());

        Ok(UploadSessionGuard {
            manager: self.clone(),
            file_key,
        })
    }

    /// Removes an upload session for the given file_key.
    /// This is called automatically by the guard's Drop implementation.
    fn end_upload(&self, file_key: &str) {
        self.sessions
            .write()
            .expect("Upload sessions lock poisoned")
            .remove(file_key);
    }

    /// Checks if an upload is currently active for the given file_key.
    pub fn is_uploading(&self, file_key: &str) -> bool {
        self.sessions
            .read()
            .expect("Upload sessions lock poisoned")
            .contains(file_key)
    }
}

impl Clone for UploadSessionManager {
    fn clone(&self) -> Self {
        UploadSessionManager {
            sessions: Arc::clone(&self.sessions),
            max_sessions: self.max_sessions,
        }
    }
}

/// RAII guard that ensures upload sessions are always cleaned up.
/// The upload session will be automatically removed when this guard is dropped.
#[derive(Debug)]
pub struct UploadSessionGuard {
    manager: UploadSessionManager,
    file_key: String,
}

impl Drop for UploadSessionGuard {
    fn drop(&mut self) {
        self.manager.end_upload(&self.file_key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_upload_success() {
        let manager = UploadSessionManager::new(100);
        let file_key = "test_file_key";

        let _guard = manager.start_upload(file_key.to_string()).unwrap();
        assert!(manager.is_uploading(file_key));
    }

    #[test]
    fn test_start_upload_duplicate_fails() {
        let manager = UploadSessionManager::new(100);
        let file_key = "test_file_key";

        let _guard = manager.start_upload(file_key.to_string()).unwrap();
        let result = manager.start_upload(file_key.to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("is already being uploaded"));
    }

    #[test]
    fn test_guard_cleanup_on_drop() {
        let manager = UploadSessionManager::new(100);
        let file_key = "test_file_key";

        {
            let _guard = manager.start_upload(file_key.to_string()).unwrap();
            assert!(manager.is_uploading(file_key));
        } // guard dropped here

        assert!(!manager.is_uploading(file_key));

        // Should be able to start a new upload after guard is dropped
        let _guard = manager.start_upload(file_key.to_string()).unwrap();
        assert!(manager.is_uploading(file_key));
    }

    #[test]
    fn test_multiple_different_files() {
        let manager = UploadSessionManager::new(100);

        let _guard1 = manager.start_upload("file1".to_string()).unwrap();
        let _guard2 = manager.start_upload("file2".to_string()).unwrap();
        let _guard3 = manager.start_upload("file3".to_string()).unwrap();

        assert!(manager.is_uploading("file1"));
        assert!(manager.is_uploading("file2"));
        assert!(manager.is_uploading("file3"));

        drop(_guard2);
        assert!(manager.is_uploading("file1"));
        assert!(!manager.is_uploading("file2"));
        assert!(manager.is_uploading("file3"));
    }

    #[test]
    fn test_max_sessions_limit() {
        let manager = UploadSessionManager::new(2);

        let _guard1 = manager.start_upload("file1".to_string()).unwrap();
        let _guard2 = manager.start_upload("file2".to_string()).unwrap();

        // Third upload should fail due to max sessions reached
        let result = manager.start_upload("file3".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Maximum number"));
    }
}
