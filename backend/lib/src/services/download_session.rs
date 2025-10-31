use std::collections::{hash_map::Entry, HashMap};
use std::sync::{Arc, RwLock};

use axum::body::Bytes;
use tokio::sync::mpsc;

use crate::constants::download::MAX_DOWNLOAD_SESSIONS;

/// Manages active download sessions for streaming files from MSP nodes to clients.
///
/// Each session maps a file key to a channel sender, allowing the internal upload
/// endpoint (which receives chunks from the MSP node) to forward them to the
/// download endpoint (which streams them to the client).
pub struct DownloadSessionManager {
    sessions: Arc<RwLock<HashMap<String, mpsc::Sender<Result<Bytes, std::io::Error>>>>>,
}

impl DownloadSessionManager {
    pub fn new() -> Self {
        DownloadSessionManager {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Atomically adds a new download session for the given file key.
    /// Fails if there is already an active session for the file.
    pub fn add_session(
        &self,
        id: &String,
        sender: mpsc::Sender<Result<Bytes, std::io::Error>>,
    ) -> Result<(), String> {
        let mut sessions = self
            .sessions
            .write()
            .expect("Download sessions lock poisoned");

        if sessions.len() >= MAX_DOWNLOAD_SESSIONS {
            return Err(format!(
                "Maximum number of {} download sessions reached",
                MAX_DOWNLOAD_SESSIONS
            ));
        }

        match sessions.entry(id.clone()) {
            Entry::Occupied(_) => Err("File is already being downloaded".to_string()),
            Entry::Vacant(entry) => {
                entry.insert(sender);
                Ok(())
            }
        }
    }

    pub fn remove_session(&self, id: &str) {
        self.sessions
            .write()
            .expect("Download sessions lock poisoned")
            .remove(id);
    }

    pub fn get_session(&self, id: &str) -> Option<mpsc::Sender<Result<Bytes, std::io::Error>>> {
        self.sessions
            .read()
            .expect("Download sessions lock poisoned")
            .get(id)
            .cloned()
    }
}
