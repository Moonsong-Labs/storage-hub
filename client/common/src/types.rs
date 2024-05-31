use std::fmt::Debug;

use codec::{Decode, Encode};
use polkadot_primitives::BlakeTwo256;
use serde::{Deserialize, Serialize};
use shp_traits::CommitmentVerifier;
use sp_core::Hasher;
use sp_trie::CompactProof;
use storage_hub_runtime::Runtime;
pub use storage_hub_runtime::{FILE_CHUNK_SIZE, FILE_SIZE_TO_CHALLENGES};
use trie_db::TrieLayout;

/// The hash type of trie node keys
pub type HashT<T> = <T as TrieLayout>::Hash;
pub type HasherOutT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;

/// FileKey is the identifier for a file.
/// Computed as the hash of the FileMetadata.
#[derive(
    Encode, Decode, Clone, Copy, Debug, PartialEq, Eq, Default, Hash, Serialize, Deserialize,
)]
pub struct FileKey(Hash);

impl From<Hash> for FileKey {
    fn from(hash: Hash) -> Self {
        Self(hash)
    }
}

impl Into<Hash> for FileKey {
    fn into(self) -> Hash {
        self.0
    }
}

impl From<&[u8]> for FileKey {
    fn from(bytes: &[u8]) -> Self {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes);
        Self(hash)
    }
}

impl AsRef<[u8]> for FileKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<&[u8; 32]> for FileKey {
    fn from(bytes: &[u8; 32]) -> Self {
        Self(*bytes)
    }
}

impl AsRef<[u8; 32]> for FileKey {
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

pub type KeyVerifier = <Runtime as pallet_proofs_dealer::Config>::KeyVerifier;
pub type FileKeyProof = <KeyVerifier as CommitmentVerifier>::Proof;

pub const H_LENGTH: usize = BlakeTwo256::LENGTH;

pub type Hash = shp_file_key_verifier::Hash<H_LENGTH>;
pub type Fingerprint = shp_file_key_verifier::Fingerprint<H_LENGTH>;
pub type FileMetadata =
    shp_file_key_verifier::FileMetadata<H_LENGTH, FILE_CHUNK_SIZE, FILE_SIZE_TO_CHALLENGES>;

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
    pub proven: Vec<Proven<HasherOutT<T>, ()>>,
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
    pub proven: Vec<Leaf<ChunkId, Chunk>>,
    /// The compact proof.
    pub proof: SerializableCompactProof,
    /// The root hash of the trie, also known as the fingerprint of the file.
    pub root: Fingerprint,
}

impl FileProof {
    pub fn verify(&self) -> bool {
        // TODO: implement this using the verifier from runtime after we have it.
        true
    }
}
