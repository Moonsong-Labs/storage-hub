#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::sp_runtime::DispatchError;
use shp_traits::{ChallengeKeyInclusion, CommitmentVerifier, Mutation, ProofDeltaApplier};
use sp_std::{collections::btree_set::BTreeSet, vec::Vec};
use sp_trie::{CompactProof, MemoryDB, TrieDBBuilder, TrieDBMutBuilder, TrieLayout, TrieMut};
use trie_db::TrieIterator;

#[cfg(test)]
mod tests;

/// A struct that implements the `CommitmentVerifier` trait, where the commitment
/// is a Merkle Patricia Trie root hash and the response to a challenge is given
/// by either the exact key or the next and previous keys in the trie.
pub struct ForestVerifier<T: TrieLayout, const H_LENGTH: usize>(core::marker::PhantomData<T>)
where
    <T::Hash as sp_core::Hasher>::Out: for<'a> TryFrom<&'a [u8; H_LENGTH]>;

/// Implement the `CommitmentVerifier` trait for the `ForestVerifier` struct.
impl<T: TrieLayout, const H_LENGTH: usize> CommitmentVerifier for ForestVerifier<T, H_LENGTH>
where
    <T::Hash as sp_core::Hasher>::Out: for<'a> TryFrom<&'a [u8; H_LENGTH]>,
{
    type Proof = CompactProof;
    type Commitment = <T::Hash as sp_core::Hasher>::Out;
    type Challenge = <T::Hash as sp_core::Hasher>::Out;

    /// Verifies a proof against a root (i.e. commitment) and a set of challenges.
    ///
    /// Iterates over the challenges and checks if there is a pair of consecutive
    /// leaves that match the challenge, or an exact leaf that matches the challenge.
    ///
    /// Callers could optionally provide for each challenge a `ChallengeKeyInclusion` which
    /// indicates whether the challenge is expected to be included in the proof or not. In some cases it
    /// is not possible to determine if a challenge should be included in the proof or not, in which case
    /// the `ChallengeKeyInclusion` should be `None`.
    fn verify_proof(
        root: &Self::Commitment,
        challenges: &[(Self::Challenge, Option<ChallengeKeyInclusion>)],
        proof: &Self::Proof,
    ) -> Result<Vec<Self::Challenge>, DispatchError> {
        // This generates a partial trie based on the proof and checks that the root hash matches the `expected_root`.
        let (memdb, root) = proof.to_memory_db(Some(root.into())).map_err(|_| {
            "Failed to convert proof to memory DB, root doesn't match with expected."
        })?;

        let trie = TrieDBBuilder::<T>::new(&memdb, &root).build();

        // `TrieDBKeyDoubleEndedIterator` should always yield a `None` or `Some(leaf)` with a value.
        // `Some(leaf)` yields a `Result` and could therefore fail, so we still have to check it.
        let mut trie_de_iter = trie
            .into_double_ended_iter()
            .map_err(|_| "Failed to create trie iterator.")?;

        // Check if the iterator has at least one leaf.
        if trie_de_iter.next().is_none() {
            #[cfg(test)]
            unreachable!(
                "This should not happen. A trie with no leafs wouldn't be possible to create."
            );

            #[allow(unreachable_code)]
            {
                return Err("No leaves provided in proof.".into());
            }
        }

        // Check that `challenges` is not empty.
        if challenges.is_empty() {
            return Err("No challenges provided.".into());
        }

        // Initialise vector of proven keys. We use a `BTreeSet` to ensure that the keys are unique.
        let mut proven_keys = BTreeSet::new();
        let mut challenges_iter = challenges.iter();

        // Iterate over the challenges and check if there is a pair of consecutive
        // leaves that match the challenge, or an exact leaf that matches the challenge.
        while let Some((challenge, expected_inclusion)) = challenges_iter.next() {
            trie_de_iter
                .seek(challenge.as_ref())
                .map_err(|_| "Failed to seek challenged key.")?;

            // Executing `next()` after `seek()` should yield the challenged leaf or the next leaf after it (which could be `None`).
            let next_leaf = trie_de_iter
                .next()
                .transpose()
                .map_err(|_| "Failed to get next leaf.")?;

            // Executing `next_back()` after `seek()` should always yield `Some(leaf)` based on the double ended iterator behaviour.
            let prev_leaf = trie_de_iter
                .next_back()
                .transpose()
                .map_err(|_| "Failed to get previous leaf.")?;

            // Check if there is a valid combination of leaves which validate the proof given the challenged key.
            match (prev_leaf, next_leaf) {
                // Scenario 1 (valid): `next_leaf` is the challenged leaf which is included in the proof.
                // The challenge is the leaf itself (i.e. the challenge exists in the trie).
                (_, Some((next_key, _))) if next_key == challenge.as_ref().to_vec() => {
                    if let Some(ChallengeKeyInclusion::NotIncluded) = expected_inclusion {
                        return Err(
                            "Challenge key is not expected to be included in the proof.".into()
                        );
                    }

                    // Converting the key to a slice and then to a fixed size array.
                    let next_key: &[u8; H_LENGTH] = next_key
                        .as_slice()
                        .try_into()
                        .map_err(|_| "Failed to convert proven key to a fixed size array.")?;

                    // Converting the fixed size array to the key type.
                    let next_key = next_key
                        .try_into()
                        .map_err(|_| "Failed to convert proven key.")?;

                    proven_keys.insert(next_key);
                    continue;
                }
                // Scenario 2 (valid): `prev_leaf` and `next_leaf` are consecutive leaves.
                // The challenge is between the two leaves (i.e. the challenge exists in the trie).
                (Some((prev_key, _)), Some((next_key, _)))
                    if prev_key < challenge.as_ref().to_vec()
                        && challenge.as_ref().to_vec() < next_key =>
                {
                    if let Some(ChallengeKeyInclusion::Included) = expected_inclusion {
                        return Err("Challenge key is expected to be included in the proof.".into());
                    }

                    // Converting the key to a slice and then to a fixed size array.
                    let prev_key: &[u8; H_LENGTH] = prev_key
                        .as_slice()
                        .try_into()
                        .map_err(|_| "Failed to convert proven key to a fixed size array.")?;

                    // Converting the fixed size array to the key type.
                    let prev_key = prev_key
                        .try_into()
                        .map_err(|_| "Failed to convert proven key.")?;

                    proven_keys.insert(prev_key);

                    // Converting the key to a slice and then to a fixed size array.
                    let next_key: &[u8; H_LENGTH] = next_key
                        .as_slice()
                        .try_into()
                        .map_err(|_| "Failed to convert proven key to a fixed size array.")?;

                    // Converting the fixed size array to the key type.
                    let next_key = next_key
                        .try_into()
                        .map_err(|_| "Failed to convert proven key.")?;

                    proven_keys.insert(next_key);

                    continue;
                }
                // Scenario 3 (valid): `next_leaf` is the first leaf since the next previous leaf is `None`.
                // The challenge is before the first leaf (i.e. the challenge does not exist in the trie).
                (None, Some((next_key, _))) => {
                    if let Some(ChallengeKeyInclusion::Included) = expected_inclusion {
                        return Err("Challenge key is expected to be included in the proof.".into());
                    }

                    // Converting the key to a slice and then to a fixed size array.
                    let next_key: &[u8; H_LENGTH] = next_key
                        .as_slice()
                        .try_into()
                        .map_err(|_| "Failed to convert proven key to a fixed size array.")?;

                    // Converting the fixed size array to the key type.
                    let next_key = next_key
                        .try_into()
                        .map_err(|_| "Failed to convert proven key.")?;

                    proven_keys.insert(next_key);

                    continue;
                }
                // Scenario 4 (valid): `prev_leaf` is the last leaf since `next_leaf` is `None`.
                // The challenge is after the last leaf (i.e. the challenge does not exist in the trie).
                (Some(prev_leaf), None) => {
                    if let Some(ChallengeKeyInclusion::Included) = expected_inclusion {
                        return Err("Challenge key is expected to be included in the proof.".into());
                    }

                    // Converting the key to a slice and then to a fixed size array.
                    let prev_key: &[u8; H_LENGTH] = prev_leaf
                        .0
                        .as_slice()
                        .try_into()
                        .map_err(|_| "Failed to convert proven key to a fixed size array.")?;

                    // Converting the fixed size array to the key type.
                    let prev_key = prev_key
                        .try_into()
                        .map_err(|_| "Failed to convert proven key.")?;

                    proven_keys.insert(prev_key);

                    continue;
                }
                // Invalid
                (None, None) => {
                    #[cfg(test)]
                    unreachable!(
                        "This should not happen. We check if the iterator has at least one leaf."
                    );

                    #[allow(unreachable_code)]
                    {
                        return Err("No leaves provided in proof.".into());
                    }
                }
                _ => {
                    #[cfg(test)]
                    unreachable!(
                        "This should not happen. Unexpected scenario when iterating through proofs."
                    );

                    #[allow(unreachable_code)]
                    {
                        return Err("Proof is invalid.".into());
                    }
                }
            }
        }

        return Ok(Vec::from_iter(proven_keys));
    }
}

impl<T: TrieLayout, const H_LENGTH: usize> ProofDeltaApplier<T::Hash>
    for ForestVerifier<T, H_LENGTH>
where
    <T::Hash as sp_core::Hasher>::Out: for<'a> TryFrom<&'a [u8; H_LENGTH]>,
{
    type Proof = CompactProof;
    type Commitment = <T::Hash as sp_core::Hasher>::Out;
    type Challenge = <T::Hash as sp_core::Hasher>::Out;

    fn apply_delta(
        commitment: &Self::Commitment,
        mutations: &[Mutation<Self::Challenge>],
        proof: &Self::Proof,
    ) -> Result<(MemoryDB<T::Hash>, Self::Commitment), DispatchError> {
        // Check if the mutations are empty
        if mutations.is_empty() {
            return Err("No mutations provided.".into());
        }

        // Check if the commitment is empty
        if commitment.as_ref().is_empty() {
            return Err("Commitment is empty.".into());
        }

        // Convert compact proof to `sp_trie::StorageProof` in order to access the trie nodes.
        // TODO: Understand why `CompactProof` cannot be used directly to construct memdb and modify a partial trie. (it fails with error IncompleteDatabase)
        let (storage_proof, mut root) = proof
            .to_storage_proof::<T::Hash>(Some(commitment.into()))
            .map_err(|_| {
                "Failed to convert proof to memory DB, root doesn't match with expected."
            })?;

        let mut memdb = storage_proof.to_memory_db();

        let mut trie = TrieDBMutBuilder::<T>::new(&mut memdb, &mut root).build();

        // Apply mutations to the trie
        for mutation in mutations.iter() {
            match mutation {
                Mutation::Add(key) => {
                    trie.insert(key.as_ref(), &[])
                        .map_err(|_| "Failed to insert key into trie.")?;
                }
                Mutation::Remove(key) => {
                    trie.remove(key.as_ref())
                        .map_err(|_| "Failed to remove key from trie.")?;
                }
            }
        }

        let new_root = *trie.root();

        drop(trie);

        Ok((memdb, new_root))
    }
}
