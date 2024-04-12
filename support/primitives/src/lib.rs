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

    /// Verifies a proof against a commitment and a set of challenges.
    ///
    /// Assumes that the challenges are ordered in ascending numerical order, and not repeated.
    fn verify_proof(
        commitment: &Self::Key,
        challenges: &[Self::Key],
        proof: &Self::Proof,
    ) -> DispatchResult {
        // This generates a partial trie based on the proof and checks that the root hash matches the `expected_root`.
        let (memdb, root) = proof.to_memory_db(Some(commitment.into())).map_err(|_| {
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
            if let Some(Err(e)) = next_leaf {
                println!("Error: {:?}", e);
                return Err("Failed to get next leaf.".into());
            }

            // Executing `next_back()` after a `seek()` should yield the same leaf as `next()`.
            let mut prev_leaf = trie_de_iter.next_back();

            println!("challenge: {:?}", challenge);
            println!("next_leaf: {:?}", next_leaf);
            println!("prev_leaf: {:?}", prev_leaf);

            // Ensure that the initial `next()` and `next_back()` are equal after a `seek()`.
            // if next_leaf != prev_leaf {
            //     return Err("Unexpected trie double ended iterator behaviour.".into());
            // }

            // Actually iterate back to the leaf before the challenge (which could be `None`).
            prev_leaf = trie_de_iter.next_back();

            println!("prev_leaf: {:?}", prev_leaf);

            // Check if `Some(leaf)` yielded an internal error. Ignore `None`.
            if let Some(Err(e)) = prev_leaf {
                println!("Error: {:?}", e);
                return Err("Failed to get prev leaf.".into());
            }

            // Check if there is a valid combination of leaves which validate the proof given the challenged key.
            match (prev_leaf, next_leaf) {
                // Valid: `next_leaf` is the challenged leaf which is included in the proof.
                (_, Some(Ok(next_leaf))) if next_leaf == challenge.as_ref().to_vec() => continue,
                // Valid: `prev_leaf` and `next_leaf` are consecutive leaves and the challenge is between them.
                (Some(Ok(prev_leaf)), Some(Ok(next_leaf)))
                    if prev_leaf < challenge.as_ref().to_vec()
                        && challenge.as_ref().to_vec() < next_leaf =>
                {
                    continue
                }
                // Valid: `prev_leaf` is the last leaf since `next_leaf` is `None`.
                // Since `seek()` is already placing the cursor at the nearest leaf (which is `None` initially in this scenario),
                // executing `next_back()` yields the last leaf.
                (Some(Ok(_prev_leaf)), None) => continue,
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
                // Invalid
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
