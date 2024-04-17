use sp_core::H256;
use sp_trie::CompactProof;

use crate::types::{Chunk, Key, Metadata};

/// Leaf in the Forest or File trie.
pub struct Leaf<D> {
    pub key: Key,
    pub data: D,
}

/// Proving either the exact key or the neighbour keys of the challenged key.
pub enum Proven<D> {
    ExactKey(Leaf<D>),
    NeighbourKeys((Leaf<D>, Leaf<D>)),
}

pub struct FileProof {
    /// The file key that was proven.
    pub proven: Proven<Chunk>,
    /// The compact proof.
    pub proof: CompactProof,
    /// The root hash of the trie.
    pub root_hash: Key,
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

pub struct ForestProof {
    /// The file key that was proven.
    pub proven: Proven<Metadata>,
    /// The compact proof.
    pub proof: CompactProof,
    /// The root hash of the trie.
    pub root_hash: H256,
}

/// Forest storage interface to be implemented by the storage providers.
pub trait ForestStorage: 'static {
    /// Generate proof for file key(s).
    fn generate_proof(&self, challenged_file_key: &Key) -> ForestProof;

    /// Insert a file key and generate a proof for it.
    fn insert_file_key(&self, file_key: &Key) -> ForestProof;

    /// Delete a file key and generate a proof for it.
    fn delete_file_key(&self, file_key: &Key) -> ForestProof;
}
