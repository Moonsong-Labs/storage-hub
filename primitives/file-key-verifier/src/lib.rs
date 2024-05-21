#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::sp_runtime::DispatchError;
use num_bigint::BigUint;
use scale_info::TypeInfo;
use sp_core::Hasher;
use sp_std::{collections::btree_set::BTreeSet, vec::Vec};
use sp_trie::{CompactProof, TrieDBBuilder, TrieLayout};
use storage_hub_traits::CommitmentVerifier;
use trie_db::Trie;

#[cfg(test)]
mod tests;

/// A struct that implements the `CommitmentVerifier` trait, where the commitment
/// is a Merkle Patricia Trie root hash and the response to a challenge is given
/// by either the exact key or the next and previous keys in the trie.
pub struct FileKeyVerifier<T: TrieLayout, const H_LENGTH: usize, const CHUNK_SIZE: u64>
where
    <T::Hash as sp_core::Hasher>::Out: for<'a> TryFrom<&'a [u8; H_LENGTH]>,
{
    pub _phantom: core::marker::PhantomData<T>,
}

#[derive(Clone, Debug, PartialEq, Eq, TypeInfo, Encode, Decode)]
pub struct FileKeyProof {
    pub owner: Vec<u8>,
    pub location: Vec<u8>,
    pub size: u64,
    pub fingerprint: Vec<u8>,
    pub proof: CompactProof,
}

/// Implement the `CommitmentVerifier` trait for the `FileKeyVerifier` struct.
impl<T: TrieLayout, const H_LENGTH: usize, const CHUNK_SIZE: u64> CommitmentVerifier
    for FileKeyVerifier<T, H_LENGTH, CHUNK_SIZE>
where
    <T::Hash as sp_core::Hasher>::Out: for<'a> TryFrom<&'a [u8; H_LENGTH]>,
{
    type Proof = FileKeyProof;
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
        let file_key = T::Hash::hash(
            &[
                &proof.owner.encode(),
                &proof.location.encode(),
                &proof.size.encode(),
                &proof.fingerprint.encode(),
            ]
            .into_iter()
            .flatten()
            .cloned()
            .collect::<Vec<u8>>(),
        );

        // Check that the file key is equal to the root.
        if &file_key != expected_file_key {
            return Err(
                "Root provided should be equal to the file key constructed from the proof.".into(),
            );
        };

        // Convert the fingerprint from the proof to the output of the hasher.
        let expected_root: &[u8; H_LENGTH] = proof
            .fingerprint
            .as_slice()
            .try_into()
            .map_err(|_| "Failed to convert fingerprint to a fixed size array.")?;
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

        // Initialise vector of proven challenges. We use a `BTreeSet` to ensure that the keys are unique.
        let mut proven_challenges = BTreeSet::new();
        let mut challenges_iter = challenges.iter();

        // Iterate over the challenges compute the modulo of the challenged hashes with the number of chunks in the file,
        // and check if the resulting leaf is in the proof.
        while let Some(challenge) = challenges_iter.next() {
            // Calculate the chunks of the file based on its size.
            let mut chunks = proof.size / CHUNK_SIZE;
            if proof.size % CHUNK_SIZE != 0 {
                chunks += 1;
            }

            // Calculate the modulo of the challenge with the number of chunks in the file.
            // The challenge is a big endian 32 byte array.
            let challenged_chunk = BigUint::from_bytes_be(challenge.as_ref()) % chunks;
            let challenged_chunk: u64 = challenged_chunk.try_into().map_err(|_| {
                "This is impossible. The modulo of a number with a u64 should always fit in a u64."
            })?;

            // Check that the chunk is in the proof.
            let chunk = trie
                .get(&challenged_chunk.to_be_bytes())
                .map_err(|_| "The proof is invalid. The challenge does not exist in the trie.")?;

            // The chunk should be Some(leaf) for the proof to be valid.
            if chunk.is_none() {
                return Err(
                    "The proof is invalid. The challenge does not exist in the trie.".into(),
                );
            }

            // Convert the challenged chunk to a key (now that we know it is a proven key).
            let challenged_chunk_bytes = challenged_chunk.to_be_bytes();
            let proven_challenge: &[u8; H_LENGTH] = challenged_chunk_bytes
                .as_slice()
                .try_into()
                .map_err(|_| "Failed to convert a challenged chunk (u64) to a fixed size array.")?;
            let proven_challenge: Self::Challenge = proven_challenge.try_into().map_err(|_| {
                "Failed to convert a challenged chunk as an array of bytes to a key."
            })?;
            // Add the challenge to the proven keys.
            proven_challenges.insert(proven_challenge);
        }

        return Ok(Vec::from_iter(proven_challenges));
    }
}
