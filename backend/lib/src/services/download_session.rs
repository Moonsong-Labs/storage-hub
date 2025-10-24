use axum::body::Bytes;
use std::collections::{hash_map::Entry, HashMap};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

/// Manages active download sessions for streaming files from MSP nodes to clients.
///
/// Each session maps a file key to a channel sender, allowing the internal upload
/// endpoint (which receives chunks from the MSP node) to forward them to the
/// download endpoint (which streams them to the client).
pub struct DownloadSession {
    sessions: Arc<RwLock<HashMap<String, mpsc::Sender<Result<Bytes, std::io::Error>>>>>,
}

impl DownloadSession {
    pub fn new() -> Self {
        DownloadSession {
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
