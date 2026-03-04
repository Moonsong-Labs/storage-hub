use std::collections::{hash_map::Entry, HashMap};
use std::sync::{Arc, RwLock};

use axum::body::Bytes;
use tokio::sync::mpsc;

type SessionMap = HashMap<String, mpsc::Sender<Result<Bytes, std::io::Error>>>;

/// Manages active download sessions for streaming files from MSP nodes to clients.
///
/// Each session maps a session ID to a channel sender, allowing the internal upload
/// endpoint (which receives chunks from the MSP node) to forward them to the
/// download endpoint (which streams them to the client).
#[derive(Debug)]
pub struct DownloadSessionManager {
    sessions: Arc<RwLock<SessionMap>>,
    max_sessions: usize,
}

impl DownloadSessionManager {
    pub fn new(max_sessions: usize) -> Self {
        DownloadSessionManager {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            max_sessions,
        }
    }

    /// Atomically registers a new download session for the given session ID.
    /// Returns a guard that will automatically clean up the session when dropped.
    /// Returns an error if there is already an active session with this ID
    /// or if the maximum number of concurrent downloads has been reached.
    pub fn start_session(
        &self,
        session_id: String,
        sender: mpsc::Sender<Result<Bytes, std::io::Error>>,
    ) -> Result<DownloadSessionGuard, String> {
        let mut sessions = self
            .sessions
            .write()
            .expect("Download sessions lock poisoned");

        if sessions.len() >= self.max_sessions {
            return Err(format!(
                "Maximum number of {} download sessions reached",
                self.max_sessions
            ));
        }

        match sessions.entry(session_id.clone()) {
            Entry::Occupied(_) => Err(format!(
                "Session ID {} is already active. Please retry with a new session ID.",
                session_id
            )),
            Entry::Vacant(entry) => {
                entry.insert(sender);
                Ok(DownloadSessionGuard {
                    manager: self.clone(),
                    session_id,
                })
            }
        }
    }

    /// Removes a download session for the given session ID.
    /// This is called automatically by the guard's Drop implementation.
    fn end_session(&self, session_id: &str) {
        self.sessions
            .write()
            .expect("Download sessions lock poisoned")
            .remove(session_id);
    }

    /// Retrieves the channel sender for the given session ID.
    /// Used by internal_upload_by_key to forward chunks to the client.
    pub fn get_session(&self, id: &str) -> Option<mpsc::Sender<Result<Bytes, std::io::Error>>> {
        self.sessions
            .read()
            .expect("Download sessions lock poisoned")
            .get(id)
            .cloned()
    }
}

impl Clone for DownloadSessionManager {
    fn clone(&self) -> Self {
        DownloadSessionManager {
            sessions: Arc::clone(&self.sessions),
            max_sessions: self.max_sessions,
        }
    }
}

/// RAII guard that ensures download sessions are always cleaned up.
/// The download session will be automatically removed when this guard is dropped,
/// regardless of whether the download succeeded, failed, or panicked.
#[derive(Debug)]
pub struct DownloadSessionGuard {
    manager: DownloadSessionManager,
    session_id: String,
}

impl Drop for DownloadSessionGuard {
    fn drop(&mut self) {
        self.manager.end_session(&self.session_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_session_success() {
        let manager = DownloadSessionManager::new(100);
        let session_id = "test_session_123";
        let (tx, _rx) = mpsc::channel(10);

        let _guard = manager.start_session(session_id.to_string(), tx).unwrap();

        // Session should exist
        assert!(manager.get_session(session_id).is_some());
    }

    #[test]
    fn test_start_session_duplicate_fails() {
        let manager = DownloadSessionManager::new(100);
        let session_id = "test_session_123";
        let (tx1, _rx1) = mpsc::channel(10);
        let (tx2, _rx2) = mpsc::channel(10);

        let _guard = manager.start_session(session_id.to_string(), tx1).unwrap();
        let result = manager.start_session(session_id.to_string(), tx2);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("is already active"));
    }

    #[test]
    fn test_guard_cleanup_on_drop() {
        let manager = DownloadSessionManager::new(100);
        let session_id = "test_session_123";
        let (tx1, _rx1) = mpsc::channel(10);
        let (tx2, _rx2) = mpsc::channel(10);

        {
            let _guard = manager.start_session(session_id.to_string(), tx1).unwrap();
            assert!(manager.get_session(session_id).is_some());
        } // guard dropped here

        assert!(manager.get_session(session_id).is_none());

        // Should be able to start a new session after guard is dropped
        let _guard = manager.start_session(session_id.to_string(), tx2).unwrap();
        assert!(manager.get_session(session_id).is_some());
    }

    #[test]
    fn test_max_sessions_limit() {
        let manager = DownloadSessionManager::new(2);
        let (tx1, _rx1) = mpsc::channel(10);
        let (tx2, _rx2) = mpsc::channel(10);
        let (tx3, _rx3) = mpsc::channel(10);

        let _guard1 = manager.start_session("session1".to_string(), tx1).unwrap();
        let _guard2 = manager.start_session("session2".to_string(), tx2).unwrap();

        // Third session should fail due to max sessions reached
        let result = manager.start_session("session3".to_string(), tx3);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Maximum number"));
    }

    #[test]
    fn test_multiple_different_sessions() {
        let manager = DownloadSessionManager::new(100);
        let (tx1, _rx1) = mpsc::channel(10);
        let (tx2, _rx2) = mpsc::channel(10);
        let (tx3, _rx3) = mpsc::channel(10);

        let _guard1 = manager.start_session("session1".to_string(), tx1).unwrap();
        let _guard2 = manager.start_session("session2".to_string(), tx2).unwrap();
        let _guard3 = manager.start_session("session3".to_string(), tx3).unwrap();

        assert!(manager.get_session("session1").is_some());
        assert!(manager.get_session("session2").is_some());
        assert!(manager.get_session("session3").is_some());

        drop(_guard2);
        assert!(manager.get_session("session1").is_some());
        assert!(manager.get_session("session2").is_none());
        assert!(manager.get_session("session3").is_some());
    }

    #[tokio::test]
    async fn test_guard_cleanup_on_task_failure() {
        let manager = DownloadSessionManager::new(100);
        let session_id = "test_session_123";
        let (tx, _rx) = mpsc::channel(10);

        let guard = manager.start_session(session_id.to_string(), tx).unwrap();
        assert!(manager.get_session(session_id).is_some());

        // Simulate what happens in download_by_key: move guard into task
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            let _guard = guard;
            // Simulate RPC failure
            Err::<(), String>("RPC call failed".to_string())
        });

        // Wait for task to complete
        let _ = handle.await;

        // Session should be cleaned up even though task failed
        assert!(manager_clone.get_session(session_id).is_none());
    }
}
