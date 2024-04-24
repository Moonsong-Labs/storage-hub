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

            // We check before the loop if the iterator has at least one leaf.
            // Therefore, if `prev_leaf` is `None` here, it means that the behaviour of the double ended iterator has changed.
            // This is because of an inconsistency in the behaviour of the iterator. If there is no leaf lower than the challenge,
            // the iterator will return the same for both `next()` and `next_back()`. This is why we check if `prev_leaf` is `None`,
            // because it shouldn't be, even in that case.
            if prev_leaf.is_none() {
                #[cfg(test)]
                unreachable!(
                    "This should not happen. We check if the iterator has at least one leaf."
                );

                #[allow(unreachable_code)]
                {
                    return Err(
                        "Unexpected double ended iterator behaviour: no previous leaf.".into(),
                    );
                }
            }

            // Check if there is a valid combination of leaves which validate the proof given the challenged key.
            match (prev_leaf, next_leaf) {
                // Scenario 1 (valid): `next_leaf` is the challenged leaf which is included in the proof.
                // The challenge is the leaf itself (i.e. the challenge exists in the trie).
                (_, Some((next_key, _))) if next_key == challenge.as_ref().to_vec() => continue,
                // Scenario 2 (valid): `prev_leaf` and `next_leaf` are consecutive leaves.
                // The challenge is between the two leaves (i.e. the challenge exists in the trie).
                (Some((prev_key, _)), Some((next_key, _)))
                    if prev_key < challenge.as_ref().to_vec()
                        && challenge.as_ref().to_vec() < next_key =>
                {
                    continue
                }
                // Scenario 3 (valid): `next_leaf` is the first leaf since the next previous leaf is `None`.
                // The challenge is before the first leaf (i.e. the challenge does not exist in the trie).
                (Some((prev_key, _)), Some((next_key, _)))
                    if prev_key == next_key && trie_de_iter.next_back().is_none() =>
                {
                    continue;
                }
                // Scenario 4 (valid): `prev_leaf` is the last leaf since `next_leaf` is `None`.
                // The challenge is after the last leaf (i.e. the challenge does not exist in the trie).
                (Some(_prev_leaf), None) => continue,
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
                        "This should not happen. We check if the iterator has at least one leaf."
                    );

                    #[allow(unreachable_code)]
                    {
                        return Err("Proof is invalid.".into());
                    }
                }
            }
        }

        return Ok(());
    }
}
