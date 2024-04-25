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
    pub owner: String,
    pub location: String,
    pub size: u64,
    pub fingerprint: H256,
}

impl Metadata {
    pub fn chunk_count(&self) -> u64 {
        let full_chunks = self.size / (FILE_CHUNK_SIZE as u64);
        if self.size % (FILE_CHUNK_SIZE as u64) > 0 {
            return full_chunks + 1;
        }
        full_chunks
    }

    pub fn chunk_ids(&self) -> impl Iterator<Item = ChunkId> {
        (0..self.chunk_count()).map(|n| ChunkId(n))
    }
}

/// Typed u64 representing the index of a file [`Chunk`]. Indexed from 0.
#[derive(Clone)]
pub struct ChunkId(pub u64);

impl ChunkId {
    pub fn get_be_key(&self) -> [u8; 8] {
        self.0.to_be_bytes()
    }
}

// TODO: this is currently a placeholder in order to define Storage interface.
/// Typed chunk of a file. This is what is stored in the leaf of the stored Merkle tree.
pub type Chunk = Vec<u8>;

/// Leaf in the Forest or File trie.
pub struct Leaf<K, D: Debug> {
    pub key: K,
    pub data: D,
}

/// Proving either the exact key or the neighbour keys of the challenged key.
pub enum Proven<K, D: Debug> {
    ExactKey(Leaf<K, D>),
    NeighbourKeys((Option<Leaf<K, D>>, Option<Leaf<K, D>>)),
}

impl<K, D: Debug> Proven<K, D> {
    pub fn new_exact_key(key: K, data: D) -> Self {
        Proven::ExactKey(Leaf { key, data })
    }

    pub fn new_neighbour_keys(
        left: Option<Leaf<K, D>>,
        right: Option<Leaf<K, D>>,
    ) -> Result<Self, &'static str> {
        match (left, right) {
            (None, None) => Err("Both left and right leaves cannot be None"),
            (left, right) => Ok(Proven::NeighbourKeys((left, right))),
        }
    }
}

/// Proof of file key(s) in the forest trie.
pub struct ForestProof<K: AsRef<[u8]>> {
    /// The file key that was proven.
    pub proven: Vec<Proven<K, Metadata>>,
    /// The compact proof.
    pub proof: CompactProof,
    /// The root hash of the trie.
    pub root: H256,
}

pub struct FileProof {
    /// The file chunk (and id) that was proven.
    pub proven: Leaf<ChunkId, Chunk>,
    /// The compact proof.
    pub proof: CompactProof,
    /// The root hash of the trie, also known as the fingerprint of the file.
    pub root: H256,
}
