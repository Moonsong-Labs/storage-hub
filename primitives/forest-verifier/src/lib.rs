#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};
use frame_support::sp_runtime::DispatchError;
use shp_traits::{
    CommitmentVerifier, TrieMutation, TrieProofDeltaApplier, TrieRemoveMutation,
};
use sp_trie::{CompactProof, MemoryDB, StorageProof, TrieDBBuilder, TrieDBMutBuilder, TrieLayout, TrieMut};
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
    fn verify_proof(
        root: &Self::Commitment,
        challenges: &[Self::Challenge],
        proof: &Self::Proof,
    ) -> Result<BTreeSet<Self::Challenge>, DispatchError> {
        // Check that `challenges` is not empty.
        if challenges.is_empty() {
            return Err("No challenges provided.".into());
        }

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
            // If there are no leaves, and still we reached this point, it is because this is a proof of an empty forest.
            // In this case, we return an empty set of proven keys, meaning that this is a valid proof of having an empty forest.
            return Ok(BTreeSet::new());
        }

        // Initialise vector of proven keys. We use a `BTreeSet` to ensure that the keys are unique.
        let mut proven_keys = BTreeSet::new();
        let mut challenges_iter = challenges.iter();

        // Iterate over the challenges and check if there is a pair of consecutive
        // leaves that match the challenge, or an exact leaf that matches the challenge.
        while let Some(challenge) = challenges_iter.next() {
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
                // Scenario 5 (unreachable): The trie is empty. While it is possible to have an empty
                // trie, this case should not be reached as the check for an empty trie is done before
                // iterating through the challenges.
                (None, None) => {
                    #[cfg(test)]
                    unreachable!(
                        "This should not happen. Empty trie scenario should be handled before reaching this point."
                    );

                    #[allow(unreachable_code)]
                    {
                        return Err("Proof is invalid.".into());
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

        return Ok(proven_keys);
    }
}

impl<T: TrieLayout, const H_LENGTH: usize> TrieProofDeltaApplier<T::Hash>
    for ForestVerifier<T, H_LENGTH>
where
    <T::Hash as sp_core::Hasher>::Out: for<'a> TryFrom<&'a [u8; H_LENGTH]>,
{
    type Proof = CompactProof;
    type Key = <T::Hash as sp_core::Hasher>::Out;

    fn apply_delta(
        root: &Self::Key,
        mutations: &[(Self::Key, TrieMutation)],
        proof: &Self::Proof,
    ) -> Result<
        (
            MemoryDB<T::Hash>,
            Self::Key,
            BTreeMap<Self::Key, TrieMutation>,
        ),
        DispatchError,
    > {
        // Check if the mutations are empty
        if mutations.is_empty() {
            return Err("No mutations provided.".into());
        }

        // Check if the root is empty
        if root.as_ref().is_empty() {
            return Err("Root is empty.".into());
        }

        // TODO: Understand why `CompactProof` cannot be used directly to construct memdb and modify a partial trie. (it fails with error IncompleteDatabase)
        // Convert compact proof to `sp_trie::StorageProof` in order to access the trie nodes.
        let (storage_proof, mut root) = proof
            .to_storage_proof::<T::Hash>(Some(root.into()))
            .map_err(|_| {
                "Failed to convert proof to memory DB, root doesn't match with expected."
            })?;

        let mut memdb = storage_proof.to_memory_db();

        let mut trie = TrieDBMutBuilder::<T>::from_existing(&mut memdb, &mut root).build();

        // Apply mutations to the trie
        let mut mutated_keys_and_values: BTreeMap<Self::Key, TrieMutation> = BTreeMap::new();
        for mutation in mutations.iter() {
            match mutation {
                (key, TrieMutation::Add(mutation)) => {
                    trie.insert(key.as_ref(), &mutation.value)
                        .map_err(|_| "Failed to insert key into trie.")?;
                    mutated_keys_and_values.insert(*key, mutation.clone().into());
                }
                (key, TrieMutation::Remove(_)) => {
                    let previous_value = trie
                        .get(key.as_ref())
                        .map_err(|_| "Failed to get value from trie.")?;
                    trie.remove(key.as_ref())
                        .map_err(|_| "Failed to remove key from trie.")?;
                    mutated_keys_and_values.insert(
                        *key,
                        TrieRemoveMutation::with_maybe_value(previous_value).into(),
                    );
                }
            }
        }

        let new_root = *trie.root();

        drop(trie);

        Ok((memdb, new_root, mutated_keys_and_values))
    }
}
