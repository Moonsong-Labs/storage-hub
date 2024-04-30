use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use sp_core::H256;
use sp_trie::CompactProof;

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
    pub fingerprint: Key,
}

// TODO: this is currently a placeholder in order to define Storage interface.
/// Typed chunk of a file. This is what is stored in the leaf of the stored Merkle tree.
pub type Chunk = Vec<u8>;

/// Leaf in the Forest or File trie.
#[derive(Serialize, Deserialize)]
pub struct Leaf<K, D> {
    pub key: K,
    pub data: D,
}

/// Proving either the exact key or the neighbour keys of the challenged key.
pub enum Proven<K, D> {
    ExactKey(Leaf<K, D>),
    NeighbourKeys((Option<Leaf<K, D>>, Option<Leaf<K, D>>)),
}

impl<K, D> Proven<K, D>
where
    K: Serialize + for<'a> Deserialize<'a> + AsRef<[u8]>,
    D: Serialize + for<'a> Deserialize<'a> + Debug,
{
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
pub struct ForestProof<K>
where
    K: Serialize + for<'a> Deserialize<'a> + AsRef<[u8]>,
{
    /// The file key that was proven.
    pub proven: Vec<Proven<K, Metadata>>,
    /// The compact proof.
    pub proof: CompactProof,
    /// The root hash of the trie.
    pub root: [u8; 32],
}

/// Storage proof in compact form.
#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub struct FileProof<K>
where
    K: Serialize + for<'a> Deserialize<'a> + AsRef<[u8]>,
{
    /// The file key that was proven.
    pub proven: Proven<K, Chunk>,
    /// The compact proof.
    pub proof: SerializableCompactProof,
    /// The root hash of the trie.
    pub root_hash: H256,
}
