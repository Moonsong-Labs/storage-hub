use std::fmt::Debug;

use codec::decode_from_bytes;
use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use sp_core::Hasher;
use sp_trie::CompactProof;
pub use storage_hub_runtime::FILE_CHUNK_SIZE;
use trie_db::TrieLayout;

// TODO: this is currently a placeholder in order to define Storage interface.
/// This type mirrors the `FileLocation<T>` type from the runtime, which is a BoundedVec.
type FileLocation = Vec<u8>;
/// The hash type of trie node keys
pub type HashT<T> = <T as TrieLayout>::Hash;
pub type HasherOutT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;

type Hash = [u8; 32];

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

#[derive(Encode, Decode, Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Fingerprint(Hash);

impl Fingerprint {
    /// Returns the hash of the fingerprint.
    pub fn hash(&self) -> Hash {
        self.0
    }
}

impl From<Hash> for Fingerprint {
    fn from(hash: Hash) -> Self {
        Self(hash)
    }
}

impl Into<Hash> for Fingerprint {
    fn into(self) -> Hash {
        self.0
    }
}

impl From<&[u8]> for Fingerprint {
    fn from(bytes: &[u8]) -> Self {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes);
        Self(hash)
    }
}

impl AsRef<[u8]> for Fingerprint {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

// TODO: this is currently a placeholder in order to define Storage interface.
/// FileMetadata contains information about a file.
/// Most importantly, the fingerprint which is the root Merkle hash of the file.
#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
pub struct FileMetadata {
    pub owner: String,
    pub location: FileLocation,
    pub size: u64,
    pub fingerprint: Fingerprint,
}

impl FileMetadata {
    pub fn new(owner: String, location: Vec<u8>, size: u64, fingerprint: Fingerprint) -> Self {
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

    /// Generates a hash key from the file metadata using the provided [`Hasher`].
    ///
    /// The key is created by combining the [SCALE](https://wentelteefje.github.io/parity-scale-codec-page/) encoded representations of the file's
    /// `owner`, `location`, `size`, and `fingerprint` hash. This order must be respected to ensure the same resultant hash is generated for the same metadata.
    /// The encoded values are flattened into a single byte vector and finally hashed.
    pub fn key<H: Hasher>(&self) -> H::Out {
        H::hash(
            &[
                &self.owner.encode(),
                &self.location.encode(),
                &self.size.encode(),
                &self.fingerprint.hash().encode(),
            ]
            .into_iter()
            .flatten()
            .cloned()
            .collect::<Vec<u8>>(),
        )
    }

    /// Decode metadata from the [SCALE](https://wentelteefje.github.io/parity-scale-codec-page/) encoded metadata bytes.
    pub fn from_scale_encoded(bytes: Vec<u8>) -> Result<Self, codec::Error> {
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
