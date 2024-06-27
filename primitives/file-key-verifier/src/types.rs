use codec::{Compact, Decode, Encode};
use core::fmt::Debug;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use shp_traits::AsCompact;
use sp_std::vec::Vec;
use sp_trie::{CompactProof, TrieDBBuilder, TrieLayout};
use trie_db::Trie;

#[derive(Clone, Debug, PartialEq, Eq, TypeInfo, Encode, Decode)]
pub struct FileKeyProof<const H_LENGTH: usize, const CHUNK_SIZE: u64, const SIZE_TO_CHALLENGES: u64>
{
    pub file_metadata: FileMetadata<H_LENGTH, CHUNK_SIZE, SIZE_TO_CHALLENGES>,
    pub proof: CompactProof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvenFileKeyError {
    /// The fingerprint from FileMetadata can not be converted to the output of the trie's hasher.
    FingerprintAndTrieHashMismatch,
    /// The root hash of the trie does not match the expected root hash.
    TrieAndExpectedRootMismatch,
    /// Trie internal error: failed to get trie key iterator.
    FailedToGetTrieKeyIterator,
    /// Trie internal error: failed to get trie key from the iterator.
    FailedToGetTrieKey,
    /// Trie internal error: failed to get trie value.
    FailedToGetTrieValue,
    /// Internal error: the key is not found in the trie.
    KeyNotFoundInTrie,
    /// Internal error: failed to convert trie key to ChunkId.
    ChunkIdFromKeyError(ChunkIdError),
}

impl<const H_LENGTH: usize, const CHUNK_SIZE: u64, const SIZE_TO_CHALLENGES: u64>
    FileKeyProof<H_LENGTH, CHUNK_SIZE, SIZE_TO_CHALLENGES>
{
    pub fn new(
        owner: Vec<u8>,
        bucket_id: Vec<u8>,
        location: Vec<u8>,
        size: u64,
        fingerprint: Fingerprint<H_LENGTH>,
        proof: CompactProof,
    ) -> Self {
        Self {
            file_metadata: FileMetadata::new(owner, bucket_id, location, size, fingerprint),
            proof,
        }
    }

    pub fn proven<T: TrieLayout>(&self) -> Result<Vec<Leaf<ChunkId, Chunk>>, ProvenFileKeyError>
    where
        <T::Hash as sp_core::Hasher>::Out: TryFrom<[u8; H_LENGTH]>,
    {
        // Convert the fingerprint from the proof to the output of the hasher.
        let expected_root: &[u8; H_LENGTH] = &self.file_metadata.fingerprint.into();
        let expected_root: <T::Hash as sp_core::Hasher>::Out = (*expected_root)
            .try_into()
            .map_err(|_| ProvenFileKeyError::FingerprintAndTrieHashMismatch)?;

        // This generates a partial trie based on the proof and checks that the root hash matches the `expected_root`.
        let (memdb, root) = self
            .proof
            .to_memory_db::<<T as TrieLayout>::Hash>(Some(&expected_root))
            .map_err(|_| ProvenFileKeyError::TrieAndExpectedRootMismatch)?;

        let trie = TrieDBBuilder::<T>::new(&memdb, &root).build();
        let mut trie_iter = trie
            .key_iter()
            .map_err(|_| ProvenFileKeyError::FailedToGetTrieKeyIterator)?;

        let mut proven = Vec::new();

        while let Some(key) = trie_iter.next() {
            // Only add chunks to `proven` if they are present in the trie.
            // Ignore them otherwise.
            if let Ok(key) = key {
                let chunk_id = ChunkId::from_trie_key(&key)
                    .map_err(|e| ProvenFileKeyError::ChunkIdFromKeyError(e))?;
                let chunk = trie
                    .get(&key)
                    .map_err(|_| ProvenFileKeyError::FailedToGetTrieValue)?
                    .ok_or_else(|| ProvenFileKeyError::KeyNotFoundInTrie)?;
                proven.push(Leaf::new(chunk_id, chunk));
            }
        }

        Ok(proven)
    }
}

/// A hash type of arbitrary length `H_LENGTH`.
pub type Hash<const H_LENGTH: usize> = [u8; H_LENGTH];

/// A fingerprint is something that uniquely identifies a file by its content.
/// In the context of this verifier, a fingerprint is the root hash of a Merkle Patricia Trie
/// of the merklised file.
#[derive(Encode, Decode, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct Fingerprint<const H_LENGTH: usize>(Hash<H_LENGTH>);

impl<const H_LENGTH: usize> Default for Fingerprint<H_LENGTH> {
    fn default() -> Self {
        Self([0u8; H_LENGTH])
    }
}

impl<const H_LENGTH: usize> Serialize for Fingerprint<H_LENGTH> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        self.0.to_vec().serialize(serializer)
    }
}

impl<'de, const H_LENGTH: usize> Deserialize<'de> for Fingerprint<H_LENGTH> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let vec = Vec::<u8>::deserialize(deserializer)?;
        let mut hash = [0u8; H_LENGTH];
        hash.copy_from_slice(&vec);
        Ok(Self(hash))
    }
}

impl<const H_LENGTH: usize> Fingerprint<H_LENGTH> {
    /// Returns the hash of the fingerprint.
    pub fn as_hash(&self) -> Hash<H_LENGTH> {
        self.0
    }
}

impl<const H_LENGTH: usize> From<Hash<H_LENGTH>> for Fingerprint<H_LENGTH> {
    fn from(hash: Hash<H_LENGTH>) -> Self {
        Self(hash)
    }
}

impl<const H_LENGTH: usize> Into<Hash<H_LENGTH>> for Fingerprint<H_LENGTH> {
    fn into(self) -> Hash<H_LENGTH> {
        self.0
    }
}

impl<const H_LENGTH: usize> From<&[u8]> for Fingerprint<H_LENGTH> {
    fn from(bytes: &[u8]) -> Self {
        let mut hash = [0u8; H_LENGTH];
        hash.copy_from_slice(&bytes);
        Self(hash)
    }
}

impl<const H_LENGTH: usize> AsRef<[u8]> for Fingerprint<H_LENGTH> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Typed u64 representing the index of a file [`Chunk`]. Indexed from 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TypeInfo, Encode, Decode)]
pub struct ChunkId(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChunkIdError {
    InvalidChunkId,
}

impl ChunkId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }

    pub fn as_trie_key(&self) -> Vec<u8> {
        AsCompact(self.0).encode()
    }

    pub fn from_trie_key(key: &Vec<u8>) -> Result<Self, ChunkIdError> {
        let id = Compact::<u64>::decode(&mut &key[..])
            .map_err(|_| ChunkIdError::InvalidChunkId)?
            .0;
        Ok(Self(id))
    }
}

// TODO: this is currently a placeholder in order to define Storage interface.
/// Typed chunk of a file. This is what is stored in the leaf of the stored Merkle tree.
pub type Chunk = Vec<u8>;

/// A leaf in the in a trie.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Leaf<K, D: Debug> {
    pub key: K,
    pub data: D,
}

impl<K, D: Debug> Leaf<K, D> {
    pub fn new(key: K, data: D) -> Self {
        Self { key, data }
    }
}

/// FileKey is the identifier for a file.
/// Computed as the hash of the SCALE-encoded FileMetadata.
#[derive(Encode, Decode, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FileKey<const H_LENGTH: usize>(Hash<H_LENGTH>);

impl<const H_LENGTH: usize> From<Hash<H_LENGTH>> for FileKey<H_LENGTH> {
    fn from(hash: Hash<H_LENGTH>) -> Self {
        Self(hash)
    }
}

impl<const H_LENGTH: usize> Into<Hash<H_LENGTH>> for FileKey<H_LENGTH> {
    fn into(self) -> Hash<H_LENGTH> {
        self.0
    }
}

impl<const H_LENGTH: usize> From<&[u8]> for FileKey<H_LENGTH> {
    fn from(bytes: &[u8]) -> Self {
        let mut hash = [0u8; H_LENGTH];
        hash.copy_from_slice(&bytes);
        Self(hash)
    }
}

impl<const H_LENGTH: usize> AsRef<[u8]> for FileKey<H_LENGTH> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl<const H_LENGTH: usize> From<&[u8; H_LENGTH]> for FileKey<H_LENGTH> {
    fn from(bytes: &[u8; H_LENGTH]) -> Self {
        Self(*bytes)
    }
}

impl<const H_LENGTH: usize> AsRef<[u8; H_LENGTH]> for FileKey<H_LENGTH> {
    fn as_ref(&self) -> &[u8; H_LENGTH] {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, TypeInfo, Encode, Decode, Serialize, Deserialize)]
pub struct FileMetadata<const H_LENGTH: usize, const CHUNK_SIZE: u64, const SIZE_TO_CHALLENGES: u64>
{
    pub owner: Vec<u8>,
    pub bucket_id: Vec<u8>,
    pub location: Vec<u8>,
    #[codec(compact)]
    pub size: u64,
    pub fingerprint: Fingerprint<H_LENGTH>,
}

impl<const H_LENGTH: usize, const CHUNK_SIZE: u64, const SIZE_TO_CHALLENGES: u64>
    FileMetadata<H_LENGTH, CHUNK_SIZE, SIZE_TO_CHALLENGES>
{
    pub fn new(
        owner: Vec<u8>,
        bucket_id: Vec<u8>,
        location: Vec<u8>,
        size: u64,
        fingerprint: Fingerprint<H_LENGTH>,
    ) -> Self {
        Self {
            owner,
            bucket_id,
            location,
            size,
            fingerprint,
        }
    }

    pub fn file_key<T: sp_core::Hasher>(&self) -> T::Out {
        T::hash(self.encode().as_slice())
    }

    pub fn chunks_to_check(&self) -> u64 {
        self.size / SIZE_TO_CHALLENGES + (self.size % SIZE_TO_CHALLENGES != 0) as u64
    }

    pub fn chunks_count(&self) -> u64 {
        self.size / CHUNK_SIZE + (self.size % CHUNK_SIZE != 0) as u64
    }
}
