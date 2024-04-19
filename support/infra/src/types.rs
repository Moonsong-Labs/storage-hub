use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use sp_core::H256;
use sp_trie::CompactProof;

use crate::constants::FILE_CHUNK_SIZE;

// TODO: this is currently a placeholder in order to define Storage interface.
/// FileKey is the identifier for a file.
/// Computed as the hash of the FileMetadata.
pub type Key = H256;

// TODO: this is currently a placeholder in order to define Storage interface.
/// Metadata contains information about a file.
/// Most importantly, the fingerprint which is the root Merkle hash of the file.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Metadata {
    pub location: String,
    pub size: u64,
    pub fingerprint: Key,
}

// TODO: this is currently a placeholder in order to define Storage interface.
/// Typed chunk of a file. This is what is stored in the leaf of the stored Merkle tree.
pub type Chunk = [u8; FILE_CHUNK_SIZE];

/// Leaf in the Forest or File trie.
pub struct Leaf<K: AsRef<[u8]>, D: Debug> {
    pub key: K,
    pub data: D,
}

/// Proving either the exact key or the neighbour keys of the challenged key.
pub enum Proven<K: AsRef<[u8]>, D: Debug> {
    ExactKey(Leaf<K, D>),
    NeighbourKeys((Option<Leaf<K, D>>, Option<Leaf<K, D>>)),
}

impl<K: AsRef<[u8]>, D: Debug> Proven<K, D> {
    pub fn new_exact_key(key: K, data: D) -> Self {
        Proven::ExactKey(Leaf { key, data })
    }

    pub fn new_neighbour_keys(left: Option<Leaf<K, D>>, right: Option<Leaf<K, D>>) -> Self {
        Proven::NeighbourKeys((left, right))
    }
}

/// Proof of file key(s) in the forest trie.
pub struct ForestProof<K: AsRef<[u8]>> {
    /// The file key that was proven.
    pub proven: Proven<K, Metadata>,
    /// The compact proof.
    pub proof: CompactProof,
    /// The root hash of the trie.
    pub root: [u8; 32],
}

pub struct FileProof<K: AsRef<[u8]>> {
    /// The file key that was proven.
    pub proven: Proven<K, Chunk>,
    /// The compact proof.
    pub proof: CompactProof,
    /// The root hash of the trie.
    pub root_hash: H256,
}
