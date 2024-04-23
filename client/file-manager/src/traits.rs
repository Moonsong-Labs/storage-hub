use storage_hub_infra::types::{FileProof, Metadata};

/// Storage interface to be implemented by the storage providers.
pub trait FileStorage: 'static {
    type Key: AsRef<[u8]>;
    type Value;

    /// Generate proof.
    fn generate_proof(&self, challenged_key: &Self::Key) -> FileProof<Self::Key>;

    /// Remove a file from storage.
    fn delete_file(&self, key: &Self::Key);

    /// Get metadata for a file.
    fn get_metadata(&self, key: &Self::Key) -> Option<Metadata>;

    /// Set metadata for a file.
    fn set_metadata(&self, key: &Self::Key, metadata: &Metadata);

    /// Get a file chunk from storage.
    fn get_chunk(&self, key: &Self::Key) -> Option<Self::Value>;

    /// Write a file chunk in storage.
    fn write_chunk(&self, key: &str, data: &Self::Value);
}
