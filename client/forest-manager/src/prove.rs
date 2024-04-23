use storage_hub_infra::types::{Leaf, Proven};
use trie_db::{TrieIterator, TrieLayout};

use crate::{traits::ForestStorage, types::ForestStorageErrors, utils::deserialize_value};

/// Determines the presence and relationship of a challenged file key within a trie structure,
/// by attempting to find leaves that are exact matches or close neighbors to the challenged key.
///
/// # Arguments
/// * `trie` - A trie data structure from which an iterator is created.
/// * `challenged_file_key` - The key for which the proof is being constructed. This function
///   will seek this key within the trie to determine relational nodes.
///
/// # Returns
/// This function returns an `Ok` wrapping an `Option<Proven<F::RawKey, F::Value>>` which:
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
pub(crate) fn prove<T: TrieLayout, F: ForestStorage>(
    trie: &trie_db::TrieDB<'_, '_, T>,
    challenged_file_key: &F::LookupKey,
) -> Result<Proven<F::RawKey, F::Value>, ForestStorageErrors> {
    // Create an iterator over the leaf nodes.
    let mut iter = trie
        .into_double_ended_iter()
        .map_err(|_| ForestStorageErrors::FailedToCreateTrieIterator)?;

    // Position the iterator close to or on the challenged key.
    iter.seek(challenged_file_key.as_ref())
        .map_err(|_| ForestStorageErrors::FailedToSeek)?;

    let next = iter
        .next()
        .transpose()
        .map_err(|_| ForestStorageErrors::FailedToReadLeaf)?;
    let prev = iter
        .next_back()
        .transpose()
        .map_err(|_| ForestStorageErrors::FailedToReadLeaf)?;

    match (prev, next) {
        (_, Some((key, value))) if challenged_file_key.as_ref() == key => {
            // Scenario 1: Exact match
            Ok(Proven::new_exact_key(
                key.into(),
                deserialize_value(&value)?,
            ))
        }
        (Some((prev_key, prev_value)), Some((next_key, next_value))) => {
            // Scenario 2: Between two keys
            let prev_leaf = Leaf {
                key: prev_key.into(),
                data: deserialize_value(&prev_value)?,
            };
            let next_leaf = Leaf {
                key: next_key.into(),
                data: deserialize_value(&next_value)?,
            };
            Ok(Proven::new_neighbour_keys(Some(prev_leaf), Some(next_leaf)))
        }
        (Some((key, value)), None) if *challenged_file_key.as_ref() > *key => {
            // Scenario 3: After the last leaf
            let leaf = Leaf {
                key: key.into(),
                data: deserialize_value(&value)?,
            };
            Ok(Proven::new_neighbour_keys(Some(leaf), None))
        }
        (None, Some((key, value))) if *challenged_file_key.as_ref() < *key => {
            // Scenario 4: Before the first leaf
            let leaf = Leaf {
                key: key.into(),
                data: deserialize_value(&value)?,
            };
            Ok(Proven::new_neighbour_keys(None, Some(leaf)))
        }
        _ => Err(ForestStorageErrors::InvalidProvingScenario),
    }
}

#[cfg(test)]
mod tests {
    use crate::{in_memory::InMemoryForestStorage, types::HashT};

    use super::*;
    use reference_trie::RefHasher;
    use sp_core::H256;
    use sp_trie::{LayoutV1, MemoryDB};
    use storage_hub_infra::types::Metadata;
    use trie_db::{Hasher, TrieDBBuilder, TrieDBMutBuilder, TrieMut};

    /// Build a Merkle Patricia Forest Trie.
    ///
    /// The trie is built from the ground up, by each file into 32 byte chunks and storing them in a trie.
    /// Each trie is then inserted into a new merkle patricia trie, which comprises the merkle forest.
    pub fn build_merkle_patricia_forest<T: TrieLayout>() -> (
        MemoryDB<T::Hash>,
        HashT<T>,
        Vec<<<T as TrieLayout>::Hash as Hasher>::Out>,
    ) {
        let user_ids = vec![
            b"01", b"02", b"03", b"04", b"05", b"06", b"07", b"08", b"09", b"10", b"11", b"12",
            b"13", b"12", b"13", b"14", b"15", b"16", b"17", b"18", b"19", b"20", b"21", b"22",
            b"23", b"24", b"25", b"26", b"27", b"28", b"29", b"30", b"31", b"32",
        ];
        let bucket = b"bucket";
        let file_name = b"sample64b";

        let mut file_leaves = Vec::new();

        for user_id in user_ids {
            let file_path = format!(
                "{}-{}-{}.txt",
                String::from_utf8(user_id.to_vec()).unwrap(),
                String::from_utf8(bucket.to_vec()).unwrap(),
                String::from_utf8(file_name.to_vec()).unwrap()
            );

            let fingerprint = H256::from_slice(&[0; 32]);

            let metadata = Metadata {
                owner: String::from("owner"),
                location: file_path,
                size: 0,
                fingerprint,
            };

            let metadata = bincode::serialize(&metadata).unwrap();
            let metadata_hash = T::Hash::hash(&metadata);

            file_leaves.push((metadata_hash, metadata));
        }

        // Construct the Merkle Patricia Forest
        let mut memdb = MemoryDB::<T::Hash>::default();
        let mut root: HashT<T> = Default::default();

        let mut file_keys = Vec::new();
        {
            let mut merkle_forest_trie = TrieDBMutBuilder::<T>::new(&mut memdb, &mut root).build();

            // Insert file leaf and metadata into the Merkle Patricia Forest.
            for file in &file_leaves {
                merkle_forest_trie
                    .insert(file.0.as_ref(), file.1.as_ref())
                    .unwrap();

                file_keys.push(file.0.clone());
            }
        }
        (memdb, root, file_keys)
    }

    #[test]
    fn test_prove_challenge_exact_key_match() {
        let (memdb, root, keys) = build_merkle_patricia_forest::<LayoutV1<RefHasher>>();

        let trie = TrieDBBuilder::<LayoutV1<RefHasher>>::new(&memdb, &root).build();

        let challenge_key = keys[2];

        let result = prove::<LayoutV1<RefHasher>, InMemoryForestStorage<LayoutV1<RefHasher>>>(
            &trie,
            &challenge_key,
        );
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

        let result = prove::<LayoutV1<RefHasher>, InMemoryForestStorage<LayoutV1<RefHasher>>>(
            &trie,
            &challenge_key,
        );

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

        let result = prove::<LayoutV1<RefHasher>, InMemoryForestStorage<LayoutV1<RefHasher>>>(
            &trie,
            &challenge_key,
        );

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

        let result = prove::<LayoutV1<RefHasher>, InMemoryForestStorage<LayoutV1<RefHasher>>>(
            &trie,
            &challenge_key,
        );

        assert!(
            matches!(result, Ok(Proven::NeighbourKeys((None, Some(leaf)))) if leaf.key.as_ref() == smallest_key)
        );
    }
}
