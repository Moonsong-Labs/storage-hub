use std::fmt::Debug;

use codec::decode_from_bytes;
use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use sp_core::Hasher;
use sp_core::H256;
use sp_trie::CompactProof;
use trie_db::TrieLayout;

/// The file chunk size in bytes. This is the size of the leaf nodes in the Merkle tree.
pub const FILE_CHUNK_SIZE: usize = 1024 * 1024;

/// The hash type of trie node keys
pub type HashT<T> = <T as TrieLayout>::Hash;
pub type HasherOutT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;

// TODO: this is currently a placeholder in order to define Storage interface.
/// FileKey is the identifier for a file.
/// Computed as the hash of the FileMetadata.
pub type Key = Vec<u8>;

// TODO: this is currently a placeholder in order to define Storage interface.
/// This type mirrors the `FileLocation<T>` type from the runtime, which is a BoundedVec.
type FileLocation = Vec<u8>;

// TODO: this is currently a placeholder in order to define Storage interface.
/// Metadata contains information about a file.
/// Most importantly, the fingerprint which is the root Merkle hash of the file.
#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub owner: String,
    pub location: FileLocation,
    pub size: u64,
    pub fingerprint: Key,
}

impl Metadata {
    pub fn new(owner: String, location: Vec<u8>, size: u64, fingerprint: Key) -> Self {
        Self {
            owner,
            location,
            size,
            fingerprint,
        }
    }

    pub fn chunk_count(&self) -> u64 {
        let full_chunks = self.size / (FILE_CHUNK_SIZE as u64);
        if self.size % (FILE_CHUNK_SIZE as u64) > 0 {
            return full_chunks + 1;
        }
        full_chunks
    }

    pub fn chunk_ids(&self) -> impl Iterator<Item = ChunkId> {
        0..self.chunk_count()
    }

    /// Compute the hash of the SCALE encoded metadata using [`Hasher`].
    pub fn key<H: Hasher>(&self) -> H::Out {
        H::hash(&self.encode())
    }

    /// Decode metadata from the SCALE encoded metadata bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, codec::Error> {
        decode_from_bytes(bytes.into())
    }
}

/// Typed u64 representing the index of a file [`Chunk`]. Indexed from 0.
pub type ChunkId = u64;

// TODO: this is currently a placeholder in order to define Storage interface.
/// Typed chunk of a file. This is what is stored in the leaf of the stored Merkle tree.
pub type Chunk = Vec<u8>;

/// Leaf in the Forest or File trie.
#[derive(Clone, Serialize, Deserialize)]
pub struct Leaf<K, D: Debug> {
    pub key: K,
    pub data: D,
}

impl<K, D: Debug> Leaf<K, D> {
    pub fn new(key: K, data: D) -> Self {
        Self { key, data }
    }
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
pub struct ForestProof<T: TrieLayout> {
    /// The file key that was proven.
    pub proven: Vec<Proven<HasherOutT<T>, Metadata>>,
    /// The compact proof.
    pub proof: CompactProof,
    /// The root hash of the trie.
    pub root: HasherOutT<T>,
}

/// Storage proof in compact form.
#[derive(Clone, Serialize, Deserialize)]
pub struct SerializableCompactProof {
    pub encoded_nodes: Vec<Vec<u8>>,
}

impl From<CompactProof> for SerializableCompactProof {
    fn from(proof: CompactProof) -> Self {
        Self {
            encoded_nodes: proof.encoded_nodes,
        }
    }
}

impl Into<CompactProof> for SerializableCompactProof {
    fn into(self) -> CompactProof {
        CompactProof {
            encoded_nodes: self.encoded_nodes,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FileProof {
    /// The file chunk (and id) that was proven.
    pub proven: Leaf<ChunkId, Chunk>,
    /// The compact proof.
    pub proof: SerializableCompactProof,
    /// The root hash of the trie, also known as the fingerprint of the file.
    pub root: H256,
}
