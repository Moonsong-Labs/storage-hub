#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::sp_runtime::DispatchError;
use num_bigint::BigUint;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use shp_traits::AsCompact;
use sp_std::{collections::btree_set::BTreeSet, vec::Vec};
use sp_trie::{CompactProof, TrieDBBuilder, TrieLayout};
use storage_hub_traits::CommitmentVerifier;
use trie_db::Trie;

#[cfg(test)]
mod tests;

/// A struct that implements the `CommitmentVerifier` trait, where the commitment
/// is a Merkle Patricia Trie root hash and the response to a challenge is given
/// by taking the modulo of the challenged hash with the number of chunks in the file,
/// and interpreting the result as a chunk index.
pub struct FileKeyVerifier<
    T: TrieLayout,
    const H_LENGTH: usize,
    const CHUNK_SIZE: u64,
    const SIZE_TO_CHALLENGES: u64,
> where
    <T::Hash as sp_core::Hasher>::Out: for<'a> TryFrom<&'a [u8; H_LENGTH]>,
{
    pub _phantom: core::marker::PhantomData<T>,
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

#[derive(Clone, Debug, PartialEq, Eq, TypeInfo, Encode, Decode, Serialize, Deserialize)]
pub struct FileMetadata<const H_LENGTH: usize, const CHUNK_SIZE: u64, const SIZE_TO_CHALLENGES: u64>
{
    pub owner: Vec<u8>,
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
        location: Vec<u8>,
        size: u64,
        fingerprint: Fingerprint<H_LENGTH>,
    ) -> Self {
        Self {
            owner,
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

#[derive(Clone, Debug, PartialEq, Eq, TypeInfo, Encode, Decode)]
pub struct FileKeyProof<const H_LENGTH: usize, const CHUNK_SIZE: u64, const SIZE_TO_CHALLENGES: u64>
{
    pub file_metadata: FileMetadata<H_LENGTH, CHUNK_SIZE, SIZE_TO_CHALLENGES>,
    pub proof: CompactProof,
}

impl<const H_LENGTH: usize, const CHUNK_SIZE: u64, const SIZE_TO_CHALLENGES: u64>
    FileKeyProof<H_LENGTH, CHUNK_SIZE, SIZE_TO_CHALLENGES>
{
    pub fn new(
        owner: Vec<u8>,
        location: Vec<u8>,
        size: u64,
        fingerprint: Fingerprint<H_LENGTH>,
        proof: CompactProof,
    ) -> Self {
        Self {
            file_metadata: FileMetadata::new(owner, location, size, fingerprint),
            proof,
        }
    }
}

/// Implement the `CommitmentVerifier` trait for the `FileKeyVerifier` struct.
impl<
        T: TrieLayout,
        const H_LENGTH: usize,
        const CHUNK_SIZE: u64,
        const SIZE_TO_CHALLENGES: u64,
    > CommitmentVerifier for FileKeyVerifier<T, H_LENGTH, CHUNK_SIZE, SIZE_TO_CHALLENGES>
where
    <T::Hash as sp_core::Hasher>::Out: for<'a> TryFrom<&'a [u8; H_LENGTH]>,
{
    type Proof = FileKeyProof<H_LENGTH, CHUNK_SIZE, SIZE_TO_CHALLENGES>;
    type Commitment = <T::Hash as sp_core::Hasher>::Out;
    type Challenge = <T::Hash as sp_core::Hasher>::Out;

    /// Verifies a proof against a root (i.e. commitment) and a set of challenges.
    ///
    /// Iterates over the challenges, computes the modulo of the challenged hashes with the number of chunks in the file,
    /// and checks if the resulting leaf is in the proof.
    fn verify_proof(
        expected_file_key: &Self::Commitment,
        challenges: &[Self::Challenge],
        proof: &Self::Proof,
    ) -> Result<Vec<Self::Challenge>, DispatchError> {
        // Check that `challenges` is not empty.
        if challenges.is_empty() {
            return Err("No challenges provided.".into());
        }

        // Construct file key from the fields in the proof.
        let file_key = proof.file_metadata.file_key::<T::Hash>();

        // Check that the number of challenges is proportional to the size of the file.
        let chunks_to_check = proof.file_metadata.chunks_to_check();
        if challenges.len() != chunks_to_check as usize {
            return Err(
                "Number of challenges does not match the number of chunks that should have been challenged for a file of this size.".into(),
            );
        }

        // Check that the file key is equal to the root.
        if &file_key != expected_file_key {
            return Err(
                "File key provided should be equal to the file key constructed from the proof."
                    .into(),
            );
        };

        // Convert the fingerprint from the proof to the output of the hasher.
        let expected_root: &[u8; H_LENGTH] = &proof.file_metadata.fingerprint.into();
        let expected_root: Self::Commitment = expected_root
            .try_into()
            .map_err(|_| "Failed to convert fingerprint to a hasher output.")?;

        // This generates a partial trie based on the proof and checks that the root hash matches the `expected_root`.
        let (memdb, root) = proof
            .proof
            .to_memory_db(Some(&expected_root))
            .map_err(|_| {
                "Failed to convert proof to memory DB, root doesn't match with expected."
            })?;

        let trie = TrieDBBuilder::<T>::new(&memdb, &root).build();

        // Initialise vector of proven challenges. We use a `BTreeSet` to ensure that the items are unique.
        let mut proven_challenges = BTreeSet::new();
        let mut challenges_iter = challenges.iter();

        // Iterate over the challenges, compute the modulo of the challenged hashes with the number of chunks in the file,
        // and check if the resulting leaf is in the proof.
        while let Some(challenge) = challenges_iter.next() {
            // Calculate the chunks of the file based on its size.
            let chunks = proof.file_metadata.chunks_count();

            // Calculate the modulo of the challenge with the number of chunks in the file.
            // The challenge is a big endian 32 byte array.
            let challenged_chunk = BigUint::from_bytes_be(challenge.as_ref()) % chunks;
            let challenged_chunk: u64 = challenged_chunk.try_into().map_err(|_| {
                "This is impossible. The modulo of a number with a u64 should always fit in a u64."
            })?;

            // Check that the chunk is in the proof.
            let chunk = trie
                .get(&AsCompact(challenged_chunk).encode())
                .map_err(|_| "The proof is invalid. The challenge does not exist in the trie.")?;

            // The chunk should be Some(leaf) for the proof to be valid.
            if chunk.is_none() {
                return Err(
                    "The proof is invalid. The challenged chunk was not found in the trie, possibly because the challenged chunk has an index higher than the amount of chunks in the file. This should not be possible, provided that the size of the file (and therefore number of chunks) is correct.".into(),
                );
            }

            // Add the challenge to the proven challenges vector.
            proven_challenges.insert(*challenge);
        }

        return Ok(Vec::from_iter(proven_challenges));
    }
}
