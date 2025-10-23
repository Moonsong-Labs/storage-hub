use axum::body::Bytes;
use std::collections::HashMap;
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

    pub fn add_session(&self, id: &String, sender: mpsc::Sender<Result<Bytes, std::io::Error>>) {
        self.sessions
            .write()
            .expect("Download sessions lock poisoned")
            .insert(id.clone(), sender);
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
