#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use codec::{Compact, Decode, DecodeWithMemTracking, Encode};
use core::fmt::{self, Debug};
use num_bigint::BigUint;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use shp_traits::{AsCompact, FileMetadataInterface};
use sp_arithmetic::traits::SaturatedConversion;
use sp_core::H256;

/// Maximum number of chunks a Storage Provider would need to prove for a file.
const MAX_CHUNKS_TO_CHECK: u32 = 10;

/// A struct containing all the information about a file in StorageHub.
///
/// It also provides utility functions like calculating the number of chunks in a file,
/// the last chunk ID, and generating a file key for a given file metadata.
#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    TypeInfo,
    Encode,
    Decode,
    DecodeWithMemTracking,
    Serialize,
    Deserialize,
)]
pub struct FileMetadata<const H_LENGTH: usize, const CHUNK_SIZE: u64, const SIZE_TO_CHALLENGES: u64>
{
    owner: Vec<u8>,
    bucket_id: Vec<u8>,
    location: Vec<u8>,
    #[codec(compact)]
    file_size: u64,
    fingerprint: Fingerprint<H_LENGTH>,
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
    ) -> Result<Self, FileMetadataError> {
        if owner.is_empty() {
            return Err(FileMetadataError::InvalidOwner);
        }

        if bucket_id.is_empty() {
            return Err(FileMetadataError::InvalidBucketId);
        }

        if location.is_empty() {
            return Err(FileMetadataError::InvalidLocation);
        }

        if size == 0 {
            return Err(FileMetadataError::InvalidFileSize);
        }

        if fingerprint.0.is_empty() {
            return Err(FileMetadataError::InvalidFingerprint);
        }

        Ok(Self {
            owner,
            bucket_id,
            location,
            file_size: size,
            fingerprint,
        })
    }

    pub fn owner(&self) -> &Vec<u8> {
        &self.owner
    }

    pub fn bucket_id(&self) -> &Vec<u8> {
        &self.bucket_id
    }

    pub fn location(&self) -> &Vec<u8> {
        &self.location
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    pub fn fingerprint(&self) -> &Fingerprint<H_LENGTH> {
        &self.fingerprint
    }

    pub fn file_key<T: sp_core::Hasher>(&self) -> T::Out {
        T::hash(self.encode().as_slice())
    }

    pub fn chunks_to_check(&self) -> u32 {
        // In here we downcast and saturate to u32, as we're going to saturate to MAX_CHUNKS_TO_CHECK anyway.
        let chunks = (self.file_size / SIZE_TO_CHALLENGES
            + (!self.file_size.is_multiple_of(SIZE_TO_CHALLENGES)) as u64)
            .saturated_into::<u32>();

        // Cap chunks to check at MAX_CHUNKS_TO_CHECK.
        // This maximum number of chunks is based on the issue raised in the audit https://github.com/Moonsong-Labs/internal-storage-hub-design-audit/issues/11.
        chunks.min(MAX_CHUNKS_TO_CHECK)
    }

    pub fn chunks_count(&self) -> u64 {
        self.file_size / CHUNK_SIZE + (!self.file_size.is_multiple_of(CHUNK_SIZE)) as u64
    }

    pub fn last_chunk_id(&self) -> ChunkId {
        // Chunks count should always be >= 1. This is assured by the checks in the constructor.
        let last_chunk_idx = self.chunks_count().saturating_sub(1);
        ChunkId::new(last_chunk_idx)
    }

    /// Calculates the size of a chunk at a given index.
    ///
    /// # Arguments
    /// - `chunk_idx` - The index of the chunk (0-based)
    ///
    /// # Returns
    /// A Result containing the size of the chunk in bytes, or an error if the chunk index is invalid
    ///
    /// This method handles the special case where the file size is an exact multiple
    /// of the chunk size, ensuring the last chunk is properly sized.
    ///
    /// In short:
    /// - For all chunks except the last one, it returns [`CHUNK_SIZE`]
    /// - For the last chunk, it returns the remainder of the file size modulo [`CHUNK_SIZE`],
    ///   or [`CHUNK_SIZE`] if the file size is an exact multiple of [`CHUNK_SIZE`].
    ///
    /// A `file_size` should never be 0. But if for whatever reason a [`FileMetadata`] is
    /// created with `file_size = 0`, this method will return that the expected chunk size
    /// is [`CHUNK_SIZE`], essentially making the verification fail. Which is ok, given that
    /// a `file_size = 0` is an invalid file.
    pub fn chunk_size_at(&self, chunk_idx: u64) -> Result<usize, ChunkSizeError> {
        // Validate chunk index is within range
        let chunks_count = self.chunks_count();
        if chunk_idx >= chunks_count {
            return Err(ChunkSizeError::OutOfRangeChunkIndex(
                chunk_idx,
                chunks_count,
            ));
        }

        let remaining_size = self.file_size % CHUNK_SIZE;
        let chunk_size = if remaining_size == 0 || chunk_idx != self.last_chunk_id().as_u64() {
            CHUNK_SIZE
        } else {
            remaining_size
        };

        Ok(chunk_size as usize)
    }

    /// Validates if a chunk's size is correct for its position
    ///
    /// # Arguments
    /// - `chunk_idx` - The index of the chunk (0-based)
    /// - `chunk_size` - The actual size of the chunk to validate
    ///
    /// # Returns
    /// true if the chunk size is valid, false otherwise
    pub fn is_valid_chunk_size(&self, chunk_idx: u64, chunk_size: usize) -> bool {
        match self.chunk_size_at(chunk_idx) {
            Ok(expected_size) => expected_size == chunk_size,
            Err(_) => false,
        }
    }
}

#[derive(Debug)]
pub enum FileMetadataError {
    InvalidOwner,
    InvalidBucketId,
    InvalidLocation,
    InvalidFileSize,
    InvalidFingerprint,
}

/// Interface for encoding and decoding FileMetadata, used by the runtime.
impl<const H_LENGTH: usize, const CHUNK_SIZE: u64, const SIZE_TO_CHALLENGES: u64>
    FileMetadataInterface for FileMetadata<H_LENGTH, CHUNK_SIZE, SIZE_TO_CHALLENGES>
{
    type Metadata = Self;
    type StorageDataUnit = u64;

    fn encode(metadata: &Self::Metadata) -> Vec<u8> {
        metadata.encode()
    }

    fn decode(data: &[u8]) -> Result<Self::Metadata, codec::Error> {
        <FileMetadata<H_LENGTH, CHUNK_SIZE, SIZE_TO_CHALLENGES> as Decode>::decode(&mut &data[..])
    }

    fn get_file_size(metadata: &Self::Metadata) -> Self::StorageDataUnit {
        metadata.file_size
    }

    fn owner(metadata: &Self::Metadata) -> &Vec<u8> {
        metadata.owner()
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

impl From<H256> for FileKey<32> {
    fn from(hash: H256) -> Self {
        let mut file_key = [0u8; 32];
        file_key.copy_from_slice(hash.as_bytes());
        Self(file_key)
    }
}

impl Into<H256> for FileKey<32> {
    fn into(self) -> H256 {
        H256::from_slice(&self.0)
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

impl<const H_LENGTH: usize> fmt::LowerHex for FileKey<H_LENGTH> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = self.0;

        write!(f, "0x{}", hex::encode(val))
    }
}

/// A fingerprint is something that uniquely identifies the content of a file.
/// In the context of this crate, a fingerprint is the root hash of a Merkle Patricia Trie
/// of the merklised file.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
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

impl<const H_LENGTH: usize> fmt::LowerHex for Fingerprint<H_LENGTH> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = self.0;

        write!(f, "0x{}", hex::encode(val))
    }
}

impl<const H_LENGTH: usize> PartialEq<[u8]> for Fingerprint<H_LENGTH> {
    fn eq(&self, other: &[u8]) -> bool {
        self.0 == other
    }
}

/// Typed u64 representing the index of a file [`Chunk`]. Indexed from 0.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, TypeInfo, Encode, Decode, Ord, PartialOrd, Hash,
)]
pub struct ChunkId(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChunkIdError {
    InvalidChunkId,
}

impl ChunkId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn from_challenge(challenge: &[u8], chunks_count: u64) -> Self {
        // Calculate the modulo of the challenge with the number of chunks in the file.
        // The challenge is a big endian 32 byte array.
        let challenged_chunk = BigUint::from_bytes_be(challenge) % chunks_count;
        ChunkId::new(challenged_chunk.try_into().expect(
            "This is impossible. The modulo of a number with a u64 should always fit in a u64.",
        ))
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

/// A chunk with its ID. This is the actual data stored in the Merkle tree for each chunk.
#[derive(Clone, Debug, Encode, Decode, PartialEq)]
pub struct ChunkWithId {
    pub chunk_id: ChunkId,
    pub data: Chunk,
}

impl ChunkWithId {
    pub fn new(chunk_id: ChunkId, data: Chunk) -> Self {
        Self { chunk_id, data }
    }
}

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

/// A hash type of arbitrary length `H_LENGTH`.
pub type Hash<const H_LENGTH: usize> = [u8; H_LENGTH];

/// Errors that can occur when calculating chunk sizes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChunkSizeError {
    /// The provided chunk index is out of range for this file
    OutOfRangeChunkIndex(u64, u64), // (provided_index, max_valid_index)
    /// The chunk size doesn't match what's expected
    UnexpectedChunkSize(usize, usize), // (expected_size, actual_size)
}

impl fmt::Display for ChunkSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChunkSizeError::OutOfRangeChunkIndex(idx, max) => {
                write!(
                    f,
                    "Chunk index {} is out of range (max valid index: {})",
                    idx,
                    max - 1
                )
            }
            ChunkSizeError::UnexpectedChunkSize(expected, actual) => {
                write!(
                    f,
                    "Unexpected chunk size: expected {} bytes, got {} bytes",
                    expected, actual
                )
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ChunkSizeError {}

#[cfg(test)]
mod tests {
    use super::*;
    const TEST_CHUNK_SIZE: u64 = 1024;

    #[test]
    fn test_chunk_size_calculations() {
        let metadata = FileMetadata::<32, TEST_CHUNK_SIZE, 1024> {
            file_size: 2500,
            fingerprint: Fingerprint::from([0u8; 32]),
            owner: vec![],
            location: vec![],
            bucket_id: vec![],
        };

        // Test regular chunks
        assert_eq!(metadata.chunk_size_at(0).unwrap(), TEST_CHUNK_SIZE as usize);
        assert_eq!(metadata.chunk_size_at(1).unwrap(), TEST_CHUNK_SIZE as usize);

        // Test last chunk
        assert_eq!(metadata.chunk_size_at(2).unwrap(), 452); // 2500 % 1024 = 452

        // Test validation
        assert!(metadata.is_valid_chunk_size(0, TEST_CHUNK_SIZE as usize));
        assert!(metadata.is_valid_chunk_size(2, 452));
        assert!(!metadata.is_valid_chunk_size(1, 500));
    }

    #[test]
    fn test_exact_multiple_chunks() {
        let metadata = FileMetadata::<32, TEST_CHUNK_SIZE, 1024> {
            file_size: TEST_CHUNK_SIZE * 2, // Exactly 2 chunks
            fingerprint: Fingerprint::from([0u8; 32]),
            owner: vec![],
            location: vec![],
            bucket_id: vec![],
        };

        // Both chunks should be full size since file_size is exact multiple of chunk_size
        assert_eq!(metadata.chunk_size_at(0).unwrap(), TEST_CHUNK_SIZE as usize);
        assert_eq!(metadata.chunk_size_at(1).unwrap(), TEST_CHUNK_SIZE as usize);
    }

    #[test]
    fn test_out_of_range_chunk() {
        let metadata = FileMetadata::<32, TEST_CHUNK_SIZE, 1024> {
            file_size: TEST_CHUNK_SIZE * 2, // Exactly 2 chunks
            fingerprint: Fingerprint::from([0u8; 32]),
            owner: vec![],
            location: vec![],
            bucket_id: vec![],
        };

        // Test out-of-range chunk access
        assert!(metadata.chunk_size_at(2).is_err());
        assert!(metadata.chunk_size_at(100).is_err());

        // Verify that is_valid_chunk_size rejects out-of-range indices
        assert!(!metadata.is_valid_chunk_size(2, TEST_CHUNK_SIZE as usize));
        assert!(!metadata.is_valid_chunk_size(100, TEST_CHUNK_SIZE as usize));
    }
}
