#![cfg_attr(not(feature = "std"), no_std)]

use sp_core::Hasher;
use sp_trie::{CompactProof, LayoutV1, Trie, TrieDBBuilder};
use storage_hub_traits::CommitmentVerifier;

use frame_support::dispatch::DispatchResult;

/// A struct that implements the `CommitmentVerifier` trait, where the commitment
/// is a Merkle Patricia Trie root hash.
pub struct TrieVerifier<T> {
    pub _phantom: core::marker::PhantomData<T>,
}

/// Implement the `CommitmentVerifier` trait for the `TrieVerifier` struct.
impl<H: Hasher> CommitmentVerifier for TrieVerifier<H> {
    type Proof = CompactProof;
    type Commitment = H::Out;
    type Challenge = H::Out;

    /// Verifies a proof against a commitment and a set of challenges.
    ///
    /// Assumes that the challenges are ordered in ascending numerical order, and not repeated.
    /// TODO: Optimise loops and iterations.
    fn verify_proof(
        commitment: &Self::Commitment,
        challenges: &[Self::Challenge],
        proof: &Self::Proof,
    ) -> DispatchResult {
        // This generates a partial trie based on the proof and checks that the root hash matches the `expected_root`.
        let (memdb, root) = match proof.to_memory_db(Some(commitment.into())) {
            Ok((memdb, root)) => (memdb, root),
            Err(_) => {
                return Err(
                    "Failed to convert proof to memory DB, root doesn't match with expected."
                        .into(),
                )
            }
        };

        // Create an iterator over the leaf nodes.
        let trie = TrieDBBuilder::<LayoutV1<H>>::new(&memdb, &root).build();
        let trie_iter = match trie.iter() {
            Ok(iter) => iter,
            Err(_) => return Err("Failed to create trie iterator.".into()),
        };

        // Filter out the trie elements that are not leaves.
        // i.e. keep only the ones who return `Ok`.
        let mut leaves = trie_iter.filter_map(|element| element.ok());

        // TODO: Handle case where trie is made up of only one leaf node.

        // Setting up variables for the iteration.
        let mut prev_leaf = None;
        let mut next_leaf = leaves.next();
        let mut trie_iter = match trie.iter() {
            Ok(iter) => iter,
            Err(_) => return Err("Failed to create trie iterator.".into()),
        };
        let mut challenges_iter = challenges.iter();

        // Iterate over the challenges and check if there is a leaf pair of consecutive
        // leaves that match the challenge, or an exact leaf that matches the challenge.
        while let Some(challenge) = challenges_iter.next() {
            // Advance leaves until we find the leafs that match the challenge.
            while next_leaf.is_some()
                && next_leaf
                    .clone()
                    .expect("Just checked that next_leaf is Some")
                    .0
                    < challenge.as_ref().to_vec()
            {
                prev_leaf = next_leaf.clone();
                next_leaf = leaves.next();
            }

            match (prev_leaf, next_leaf) {
                (None, Some(next_leaf)) if next_leaf.0 < challenge.as_ref().to_vec() => {
                    // If prev_leaf is None, then the challenge is still less than the first leaf.
                    // TODO: Check that next_leaf is the first leaf of the trie.
                    todo!("Check that next_leaf is the first leaf of the trie.")
                }
                (_, Some(next_leaf)) if next_leaf.0 == challenge.as_ref().to_vec() => {
                    // TODO: Provide a closure argument to execute in this case.
                    todo!("Provide a closure argument to execute in this case.");
                }
                (Some(prev_leaf), Some(next_leaf)) => {
                    // If next_leaf is not equal to the challenge, but next_leaf is still
                    // Some, then by this point it is known that the challenge should be
                    // between prev_leaf and next_leaf.
                    // TODO: Check that they are consecutive leaves in the trie, i.e. that there
                    // TODO: is no empty leaf in between them.
                    todo!("Check that they are consecutive leaves in the trie.")
                }
                (Some(prev_leaf), None) => {
                    // If next_leaf is None, then the challenge is greater than the last leaf.
                    // TODO: Check that prev_leaf is the last leaf of the trie.
                    todo!("Check that prev_leaf is the last leaf of the trie.")
                }
                (None, None) => {
                    // If both prev_leaf and next_leaf are None, then there were no leaves present.
                    return Err("No leaves provided in proof.".into());
                }
                _ => {
                    #[cfg(test)]
                    unreachable!("This should not happen. Impossible combination of prev_leaf and next_leaf.");

                    #[allow(unreachable_code)]
                    {
                        return Err("Impossible combination of prev_leaf and next_leaf.".into());
                    }
                }
            }
        }

        return Ok(());
    }
}
