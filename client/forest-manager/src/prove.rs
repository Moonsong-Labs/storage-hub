use shc_common::types::{HasherOutT, Leaf, Proven};
use trie_db::{TrieIterator, TrieLayout};

use crate::{
    error::{ErrorT, ForestStorageError},
    utils::convert_raw_bytes_to_hasher_out,
};

/// Determines the presence and relationship of a challenged file key within a trie structure,
/// by attempting to find leaves that are exact matches or close neighbors to the challenged key.
///
/// # Arguments
/// * `trie` - A trie data structure from which an iterator is created.
/// * `challenged_file_key` - The key for which the proof is being constructed. This function
///   will seek this key within the trie to determine relational nodes.
///
/// # Returns
/// This function returns an `Ok` wrapping an `Proven<HasherOutT<T>, Metadata>` which:
/// - `None` indicates that no relevant keys were found (unevaluable situation).
/// - An instance of `Proven`, which depending on the located keys, could be:
///   1. An exact match.
///   2. Neighboring leaves (previous and next to the challenged key).
///   3. The leaf before the challenged key (if the challenged key is greater than the largest key in the trie).
///   4. The leaf after the challenged key (if the challenged key is smaller than the smallest key in the trie).
///
/// # Errors
/// This function can return an error in cases where it fails to read or seek within the trie,
/// or when deserialization of a leaf's value fails.
pub(crate) fn prove<T: TrieLayout>(
    trie: &trie_db::TrieDB<'_, '_, T>,
    challenged_file_key: &HasherOutT<T>,
) -> Result<Proven<HasherOutT<T>, ()>, ErrorT<T>>
where
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    // Create an iterator over the leaf nodes.
    let mut iter = trie
        .into_double_ended_iter()
        .map_err(|_| ForestStorageError::FailedToCreateTrieIterator)?;

    // Position the iterator close to or on the challenged key.
    iter.seek(challenged_file_key.as_ref())?;

    let next = iter.next().transpose()?;
    let prev = iter.next_back().transpose()?;

    match (prev, next) {
        // Scenario 1: Exact match
        (_, Some((key, _))) if challenged_file_key.as_ref() == key => {
            // This is just done to allow the `apply_delta` function to remove files from the trie without failing because of an incomplete database.
            iter.next().transpose()?;
            iter.next_back().transpose()?;
            Ok(Proven::new_exact_key(
                convert_raw_bytes_to_hasher_out::<T>(key)?,
                (),
            ))
        }
        // Scenario 2: Between two keys
        (Some((prev_key, _)), Some((next_key, _)))
            if prev_key < challenged_file_key.as_ref().to_vec()
                && next_key > challenged_file_key.as_ref().to_vec() =>
        {
            let prev_leaf = Leaf::new(convert_raw_bytes_to_hasher_out::<T>(prev_key)?, ());
            let next_leaf = Leaf::new(convert_raw_bytes_to_hasher_out::<T>(next_key)?, ());

            Ok(Proven::new_neighbour_keys(Some(prev_leaf), Some(next_leaf))
                .map_err(|_| ForestStorageError::FailedToConstructProvenLeaves)?)
        }
        // Scenario 3: Before the first leaf
        (None, Some((key, _))) if *challenged_file_key.as_ref() < *key => {
            let leaf = Leaf::new(convert_raw_bytes_to_hasher_out::<T>(key)?, ());

            Ok(Proven::new_neighbour_keys(None, Some(leaf))
                .map_err(|_| ForestStorageError::FailedToConstructProvenLeaves)?)
        }
        // Scenario 4: After the last leaf
        (Some((key, _)), None) if *challenged_file_key.as_ref() > *key => {
            let leaf = Leaf::new(convert_raw_bytes_to_hasher_out::<T>(key)?, ());

            Ok(Proven::new_neighbour_keys(Some(leaf), None)
                .map_err(|_| ForestStorageError::FailedToConstructProvenLeaves)?)
        }
        _ => Ok(Proven::Empty),
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils::build_merkle_patricia_forest;

    use super::*;
    use reference_trie::RefHasher;
    use sp_trie::LayoutV1;
    use trie_db::TrieDBBuilder;

    #[test]
    fn test_prove_challenge_exact_key_match() {
        let (memdb, root, keys) = build_merkle_patricia_forest::<LayoutV1<RefHasher>>();

        let trie = TrieDBBuilder::<LayoutV1<RefHasher>>::new(&memdb, &root).build();

        let challenge_key = keys[2];

        let result = prove::<LayoutV1<RefHasher>>(&trie, &challenge_key);
        assert!(matches!(result, Ok(Proven::ExactKey(leaf)) if leaf.key.as_ref() == challenge_key));
    }

    #[test]
    fn test_prove_challenge_between_two_keys() {
        let (memdb, root, keys) = build_merkle_patricia_forest::<LayoutV1<RefHasher>>();
        let trie = TrieDBBuilder::<LayoutV1<RefHasher>>::new(&memdb, &root).build();

        // challenged_key xor leaf key 1
        let challenge_key = keys[0]
            .iter()
            .zip(keys[1].iter())
            .map(|(a, b)| a ^ b)
            .collect::<Vec<u8>>();

        let challenge_key: [u8; 32] = challenge_key
            .as_slice()
            .try_into()
            .expect("slice with incorrect length");

        let result = prove::<LayoutV1<RefHasher>>(&trie, &challenge_key);

        assert!(
            matches!(result, Ok(Proven::NeighbourKeys((Some(leaf1), Some(leaf2)))) if leaf1.key.as_ref() < challenge_key.as_slice() && leaf2.key.as_ref() > challenge_key.as_slice())
        );
    }

    #[test]
    fn test_prove_challenge_after_last_key() {
        let (memdb, root, keys) = build_merkle_patricia_forest::<LayoutV1<RefHasher>>();
        let trie = TrieDBBuilder::<LayoutV1<RefHasher>>::new(&memdb, &root).build();

        let largest_key = keys.iter().max().unwrap();

        // Challenge a key that exceeds the largest key in the trie
        let challenge_key: Vec<u8> = largest_key.iter().map(|&b| b.saturating_add(1)).collect();
        let challenge_key: [u8; 32] = challenge_key
            .as_slice()
            .try_into()
            .expect("slice with incorrect length");

        let result = prove::<LayoutV1<RefHasher>>(&trie, &challenge_key);

        assert!(
            matches!(result, Ok(Proven::NeighbourKeys((Some(leaf), None))) if leaf.key.as_ref() == largest_key)
        );
    }

    #[test]
    fn test_prove_challenge_before_first_key() {
        let (memdb, root, keys) = build_merkle_patricia_forest::<LayoutV1<RefHasher>>();
        let trie = TrieDBBuilder::<LayoutV1<RefHasher>>::new(&memdb, &root).build();

        let smallest_key = keys.iter().min().unwrap();

        // Challenge a key that is less than the smallest key in the trie
        let challenge_key: Vec<u8> = smallest_key.iter().map(|&b| b.saturating_sub(1)).collect();
        let challenge_key: [u8; 32] = challenge_key
            .as_slice()
            .try_into()
            .expect("slice with incorrect length");

        let result = prove::<LayoutV1<RefHasher>>(&trie, &challenge_key);

        assert!(
            matches!(result, Ok(Proven::NeighbourKeys((None, Some(leaf)))) if leaf.key.as_ref() == smallest_key)
        );
    }
}
