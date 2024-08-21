#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Compact, Decode, Encode};
use core::fmt::Debug;
use num_bigint::BigUint;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use shp_traits::AsCompact;
use sp_arithmetic::traits::SaturatedConversion;
use sp_core::H256;
use sp_std::vec::Vec;

/// A struct containing all the information about a file in StorageHub.
///
/// It also provides utility functions like calculating the number of chunks in a file,
/// the last chunk ID, and generating a file key for a given file metadata.
#[derive(Clone, Debug, PartialEq, Eq, TypeInfo, Encode, Decode, Serialize, Deserialize)]
pub struct FileMetadata<const H_LENGTH: usize, const CHUNK_SIZE: u64, const SIZE_TO_CHALLENGES: u64>
{
    pub owner: Vec<u8>,
    pub bucket_id: Vec<u8>,
    pub location: Vec<u8>,
    #[codec(compact)]
    pub file_size: u64,
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
            file_size: size,
            fingerprint,
        }
    }

    pub fn file_key<T: sp_core::Hasher>(&self) -> T::Out {
        T::hash(self.encode().as_slice())
    }

    pub fn chunks_to_check(&self) -> u32 {
        // In here we downcast and saturate to u32, as we consider u32::MAX to be an already large
        // enough number of challenges to be generated.
        (self.file_size / SIZE_TO_CHALLENGES + (self.file_size % SIZE_TO_CHALLENGES != 0) as u64)
            .saturated_into::<u32>()
    }

    pub fn chunks_count(&self) -> u64 {
        self.file_size / CHUNK_SIZE + (self.file_size % CHUNK_SIZE != 0) as u64
    }

    pub fn last_chunk_id(&self) -> ChunkId {
        ChunkId::new(self.chunks_count() - 1)
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

/// A fingerprint is something that uniquely identifies the content of a file.
/// In the context of this crate, a fingerprint is the root hash of a Merkle Patricia Trie
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, TypeInfo, Encode, Decode, Ord, PartialOrd, Hash)]
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
        let challenged_chunk = BigUint::from_bytes_be(challenge.as_ref()) % chunks_count;
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
