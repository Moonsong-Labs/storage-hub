use sp_core::H256;
use sp_trie::CompactProof;

use storage_hub_infra::types::{Chunk, Key, Metadata, Proven};

pub struct FileProof {
    /// The file key that was proven.
    pub proven: Proven<Chunk>,
    /// The compact proof.
    pub proof: CompactProof,
    /// The root hash of the trie.
    pub root_hash: H256,
}

/// Storage interface to be implemented by the storage providers.
pub trait FileStorage: 'static {
    /// Generate proof.
    fn generate_proof(&self, challenged_key: &Key) -> FileProof;

    /// Remove a file from storage.
    fn delete_file(&self, key: &Key);

    /// Get metadata for a file.
    fn get_metadata(&self, key: &Key) -> Option<Metadata>;

    /// Set metadata for a file.
    fn set_metadata(&self, key: &Key, metadata: &Metadata);

    /// Get a file chunk from storage.
    fn get_chunk(&self, key: &Key, chunk: u64) -> Option<Chunk>;

    /// Write a file chunk in storage.
    fn write_chunk(&self, key: &str, chunk: u64, data: &Chunk);
}
