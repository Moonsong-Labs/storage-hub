#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::collections::BTreeSet;
use frame_support::sp_runtime::DispatchError;
use shp_file_metadata::ChunkId;
use shp_traits::CommitmentVerifier;
use sp_trie::{Trie, TrieDBBuilder, TrieLayout};
use types::FileKeyProof;

#[cfg(test)]
mod tests;

pub mod types;

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
    ) -> Result<BTreeSet<Self::Challenge>, DispatchError> {
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
        let expected_root: &[u8; H_LENGTH] = &proof.file_metadata.fingerprint().as_hash();
        let expected_root: Self::Commitment = expected_root
            .try_into()
            .map_err(|_| "Failed to convert fingerprint to a hasher output.")?;

        // Decode compact proof directly into memory DB without cloning.
        let mut memdb = sp_trie::MemoryDB::<T::Hash>::new(&[]);
        let root = sp_trie::decode_compact::<sp_trie::LayoutV1<T::Hash>, _, _>(
            &mut memdb,
            proof.proof.iter().map(|n| n.as_slice()),
            Some(&expected_root),
        )
        .map_err(|_| "Failed to convert proof to memory DB, root doesn't match with expected.")?;

        let trie = TrieDBBuilder::<T>::new(&memdb, &root).build();

        // Initialise vector of proven challenges. We use a `BTreeSet` to ensure that the items are unique.
        let mut proven_challenges = BTreeSet::new();
        let mut challenges_iter = challenges.iter();

        // Iterate over the challenges, compute the modulo of the challenged hashes with the number of chunks in the file,
        // and check if the resulting leaf is in the proof.
        while let Some(challenge) = challenges_iter.next() {
            // Calculate the chunks of the file based on its size.
            let chunks = proof.file_metadata.chunks_count();

            // Convert the challenge to a chunk ID.
            let challenged_chunk = ChunkId::from_challenge(challenge.as_ref(), chunks);

            // Check that the chunk is in the proof.
            let chunk = trie
                .get(&challenged_chunk.as_trie_key())
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

        Ok(proven_challenges)
    }
}
