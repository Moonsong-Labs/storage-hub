use sp_core::H256;

use crate::constants::FILE_CHUNK_SIZE;

// TODO: this is currently a placeholder in order to define Storage interface.
/// FileKey is the identifier for a file.
/// Computed as the hash of the FileMetadata.
pub type Key = H256;

// TODO: this is currently a placeholder in order to define Storage interface.
/// Metadata contains information about a file.
/// Most importantly, the fingerprint which is the root Merkle hash of the file.
pub struct Metadata {
    pub size: u64,
    pub fingerprint: Key,
}

// TODO: this is currently a placeholder in order to define Storage interface.
/// Typed chunk of a file. This is what is stored in the leaf of the stored Merkle tree.
pub type Chunk = [u8; FILE_CHUNK_SIZE];

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
