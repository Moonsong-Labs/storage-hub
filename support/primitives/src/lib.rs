#![cfg_attr(not(feature = "std"), no_std)]

use sp_core::Hasher;
use sp_trie::{CompactProof, LayoutV1, TrieDBBuilder};
use storage_hub_traits::CommitmentVerifier;

use frame_support::dispatch::DispatchResult;
use trie_db::TrieIterator;

#[cfg(test)]
mod tests;

/// A struct that implements the `CommitmentVerifier` trait, where the commitment
/// is a Merkle Patricia Trie root hash.
pub struct TrieVerifier<H: Hasher> {
    pub _phantom: core::marker::PhantomData<H>,
}

/// Implement the `CommitmentVerifier` trait for the `TrieVerifier` struct.
impl<H: Hasher> CommitmentVerifier for TrieVerifier<H> {
    type Proof = CompactProof;
    type Key = H::Out;

    /// Verifies a proof against a root (i.e. commitment) and a set of challenges.
    ///
    /// Assumes that the challenges are ordered in ascending numerical order, and not repeated.
    fn verify_proof(
        root: &Self::Key,
        challenges: &[Self::Key],
        proof: &Self::Proof,
    ) -> DispatchResult {
        // This generates a partial trie based on the proof and checks that the root hash matches the `expected_root`.
        let (memdb, root) = proof.to_memory_db(Some(root.into())).map_err(|_| {
            "Failed to convert proof to memory DB, root doesn't match with expected."
        })?;

        let trie = TrieDBBuilder::<LayoutV1<H>>::new(&memdb, &root).build();

        // `TrieDBKeyDoubleEndedIterator` should always yield a `None` or `Some(leaf)` with a value.
        // `Some(leaf)` yields a `Result` and could therefore fail, so we still have to check it.
        let mut trie_de_iter = trie
            .into_key_double_ended_iter()
            .map_err(|_| "Failed to create trie iterator.")?;

        // Check if the iterator has at least one leaf.
        if trie_de_iter.next().is_none() {
            return Err("No leaves provided in proof.".into());
        }

        let mut challenges_iter = challenges.iter();

        // Iterate over the challenges and check if there is a leaf pair of consecutive
        // leaves that match the challenge, or an exact leaf that matches the challenge.
        while let Some(challenge) = challenges_iter.next() {
            trie_de_iter
                .seek(challenge.as_ref())
                .map_err(|_| "Failed to seek challenged key.")?;

            // Executing `next()` after a `seek()` should yield the challenged leaf or the next leaf after it (which could be `None`).
            let next_leaf = trie_de_iter.next();

            // Check if `Some(leaf)` yielded an internal error. Ignore `None`.
            if let Some(Err(_)) = next_leaf {
                return Err("Failed to get next leaf.".into());
            }

            let mut prev_leaf = trie_de_iter.next_back();

            // If the `prev_leaf` and `next_leaf` are the same and `next_leaf` yields a leaf, then it means we have to exectue
            // `next_back()` again to get the previous leaf. This is due to the behaviour of the Double Ended Iterator implemented in trie-db.
            //
            // When `next_leaf` is None, then `prev_leaf` will yield the last leaf automatically without having to call `next_back()` again.
            if prev_leaf == next_leaf && next_leaf.as_ref().is_some_and(|x| x.is_ok()) {
                // If the leaf is the same as the next leaf, then it means we
                prev_leaf = trie_de_iter.next_back();
            }

            // Check if there is a valid combination of leaves which validate the proof given the challenged key.
            match (prev_leaf, next_leaf) {
                // Scenario 1 (valid): `next_leaf` is the challenged leaf which is included in the proof.
                // The challenge is the leaf itself (i.e. the challenge exists in the trie).
                (_, Some(Ok(next_leaf))) if next_leaf == challenge.as_ref().to_vec() => continue,
                // Scenario 2 (valid): `prev_leaf` and `next_leaf` are consecutive leaves.
                // The challenge is between the two leaves (i.e. the challenge exists in the trie).
                (Some(Ok(prev_leaf)), Some(Ok(next_leaf)))
                    if prev_leaf < challenge.as_ref().to_vec()
                        && challenge.as_ref().to_vec() < next_leaf =>
                {
                    continue
                }
                // Scenario 3 (valid): `prev_leaf` is the last leaf since `next_leaf` is `None`.
                // The challenge is after the last leaf (i.e. the challenge does not exist in the trie).
                (Some(Ok(_prev_leaf)), None) => continue,
                // Scenario 4 (valid): `next_leaf` is the first leaf since `prev_leaf` is `None`.
                // The challenge is before the first leaf (i.e. the challenge does not exist in the trie).
                (None, Some(Ok(_next_leaf))) => continue,
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
                    return Err("Proof is invalid.".into());
                }
            }
        }

        return Ok(());
    }
}
