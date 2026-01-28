extern crate alloc;

use alloc::vec::Vec;
use codec::{Decode, DecodeWithMemTracking, Encode};
use core::fmt::Debug;
use scale_info::TypeInfo;
use shp_file_metadata::{
    Chunk, ChunkId, ChunkIdError, ChunkWithId, FileMetadata, Fingerprint, Leaf,
};
use shp_traits::ShpCompactProof;
use sp_trie::{CompactProof, TrieDBBuilder, TrieLayout};
use trie_db::Trie;

#[derive(Clone, Debug, PartialEq, Eq, TypeInfo, Encode, Decode, DecodeWithMemTracking)]
pub struct FileKeyProof<const H_LENGTH: usize, const CHUNK_SIZE: u64, const SIZE_TO_CHALLENGES: u64>
{
    pub file_metadata: FileMetadata<H_LENGTH, CHUNK_SIZE, SIZE_TO_CHALLENGES>,
    pub proof: ShpCompactProof,
}

/// Implement the `From<ShpCompactProof>` trait for the `FileKeyProof` struct.
impl<const H_LENGTH: usize, const CHUNK_SIZE: u64, const SIZE_TO_CHALLENGES: u64>
    From<ShpCompactProof> for FileKeyProof<H_LENGTH, CHUNK_SIZE, SIZE_TO_CHALLENGES>
{
    fn from(proof: ShpCompactProof) -> Self {
        Self {
            file_metadata: Default::default(),
            proof,
        }
    }
}

/// Implement the `From<CompactProof>` trait for the `FileKeyProof` struct.
impl<const H_LENGTH: usize, const CHUNK_SIZE: u64, const SIZE_TO_CHALLENGES: u64> From<CompactProof>
    for FileKeyProof<H_LENGTH, CHUNK_SIZE, SIZE_TO_CHALLENGES>
{
    fn from(proof: CompactProof) -> Self {
        Self {
            file_metadata: Default::default(),
            proof: proof.into(),
        }
    }
}

/// Implement the `Into<ShpCompactProof>` trait for the `FileKeyProof` struct.
impl<const H_LENGTH: usize, const CHUNK_SIZE: u64, const SIZE_TO_CHALLENGES: u64>
    Into<ShpCompactProof> for FileKeyProof<H_LENGTH, CHUNK_SIZE, SIZE_TO_CHALLENGES>
{
    fn into(self) -> ShpCompactProof {
        self.proof
    }
}

/// Implement the `Into<CompactProof>` trait for the `FileKeyProof` struct.
impl<const H_LENGTH: usize, const CHUNK_SIZE: u64, const SIZE_TO_CHALLENGES: u64> Into<CompactProof>
    for FileKeyProof<H_LENGTH, CHUNK_SIZE, SIZE_TO_CHALLENGES>
{
    fn into(self) -> CompactProof {
        self.proof.into_inner()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvenFileKeyError {
    /// The file metadata can not be created.
    FailedToCreateFileMetadata,
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
    /// Internal error: failed to decode chunk from proof.
    FailedToDecodeChunkFromProof,
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
        proof: impl Into<ShpCompactProof>,
    ) -> Result<Self, ProvenFileKeyError> {
        let file_metadata = FileMetadata::new(owner, bucket_id, location, size, fingerprint)
            .map_err(|_| ProvenFileKeyError::FailedToCreateFileMetadata)?;

        Ok(Self {
            file_metadata,
            proof: proof.into(),
        })
    }

    /// Verifies and extracts proven chunks from a Merkle trie proof.
    ///
    /// Returns a `Vec<Leaf<ChunkId, Chunk>>` of proven chunks with their IDs (keys) and data (values)
    pub fn proven<T: TrieLayout>(&self) -> Result<Vec<Leaf<ChunkId, Chunk>>, ProvenFileKeyError>
    where
        <T::Hash as sp_core::Hasher>::Out: TryFrom<[u8; H_LENGTH]>,
    {
        // Convert the fingerprint from the proof to the output of the hasher.
        let expected_root: &[u8; H_LENGTH] = &self.file_metadata.fingerprint().as_hash();
        let expected_root: <T::Hash as sp_core::Hasher>::Out = (*expected_root)
            .try_into()
            .map_err(|_| ProvenFileKeyError::FingerprintAndTrieHashMismatch)?;

        // This generates a partial trie based on the proof and checks that the root hash matches the `expected_root`.
        let (memdb, root) = self
            .proof
            .inner()
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
                // Get the chunk ID from the trie key.
                let chunk_id = ChunkId::from_trie_key(&key)
                    .map_err(|e| ProvenFileKeyError::ChunkIdFromKeyError(e))?;

                // Get the encoded chunk from the trie.
                let encoded_chunk = trie
                    .get(&key)
                    .map_err(|_| ProvenFileKeyError::FailedToGetTrieValue)?
                    .ok_or_else(|| ProvenFileKeyError::KeyNotFoundInTrie)?;

                // Decode the chunk into its chunk ID and data.
                let decoded_chunk = ChunkWithId::decode(&mut encoded_chunk.as_slice())
                    .map_err(|_| ProvenFileKeyError::FailedToDecodeChunkFromProof)?;

                proven.push(Leaf::new(chunk_id, decoded_chunk.data));
            }
        }

        Ok(proven)
    }
}
