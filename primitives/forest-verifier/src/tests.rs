use std::io::Read;

use serde::Serialize;
use shp_traits::{
    CommitmentVerifier, TrieAddMutation, TrieMutation, TrieProofDeltaApplier, TrieRemoveMutation,
};
use sp_core::H256;
use sp_runtime::traits::BlakeTwo256;
use sp_std::collections::btree_map::BTreeMap;
use sp_trie::{
    recorder::Recorder, CompactProof, LayoutV1, MemoryDB, Trie, TrieDBBuilder, TrieDBMutBuilder,
    TrieLayout, TrieMut,
};
use trie_db::{Hasher, TrieIterator};

use crate::ForestVerifier;

/// The hash type of trie node keys
type HashT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;

const CHUNK_SIZE: usize = 2;
const FILES_BASE_PATH: &str = "./tmp/";

#[derive(Serialize, PartialEq, Debug)]
struct FileMetadata<'a> {
    pub user_id: &'a [u8],
    pub bucket: &'a [u8],
    pub file_id: &'a [u8],
    pub size: u64,
    /// The fingerprint will always be 32 bytes since we are using BlakeTwo256.
    pub fingerprint: [u8; 32],
}

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
        b"01", b"02", b"03", b"04", b"05", b"06", b"07", b"08", b"09", b"10", b"11", b"12", b"13",
        b"14", b"15", b"16", b"17", b"18", b"19", b"20", b"21", b"22", b"23", b"24", b"25", b"26",
        b"27", b"28", b"29", b"30", b"31", b"32", b"33", b"34", b"35", b"36", b"37", b"38", b"39",
    ];
    let bucket = b"bucket";
    let file_name = b"file64b";

    let mut file_leaves = Vec::new();

    println!("Chunking file into 32 byte chunks and building Merkle Patricia Tries...");

    for user_id in user_ids {
        let file_path = format!(
            "{}-{}-{}.txt",
            String::from_utf8(user_id.to_vec()).unwrap(),
            String::from_utf8(bucket.to_vec()).unwrap(),
            String::from_utf8(file_name.to_vec()).unwrap()
        );

        std::fs::create_dir_all(FILES_BASE_PATH).unwrap();
        std::fs::File::create(FILES_BASE_PATH.to_owned() + &file_path).unwrap();

        let file = std::fs::File::open(FILES_BASE_PATH.to_owned() + &file_path).unwrap();
        let file_size = std::fs::File::metadata(&file).unwrap().len();
        let (_memdb, fingerprint) = merklise_file::<LayoutV1<BlakeTwo256>>(&file_path);

        let metadata = FileMetadata {
            user_id,
            bucket,
            file_id: file_name,
            size: file_size,
            fingerprint: fingerprint
                .as_ref()
                .try_into()
                .expect("slice with incorrect length"),
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

            file_keys.push(file.0);
        }

        println!(
            "Merkle Patricia Forest Trie root: {:?}",
            merkle_forest_trie.root()
        );
    }

    // Sorting file keys for deterministic proof generation
    file_keys.sort();

    (memdb, root, file_keys)
}

/// Build a Merkle Patricia Forest Trie with just one key.
///
/// The trie is built from the ground up, by each file into 32 byte chunks and storing them in a trie.
/// Each trie is then inserted into a new merkle patricia trie, which comprises the merkle forest.
pub fn build_merkle_patricia_forest_one_key<T: TrieLayout>() -> (
    MemoryDB<T::Hash>,
    HashT<T>,
    Vec<<<T as TrieLayout>::Hash as Hasher>::Out>,
) {
    let user_ids = vec![b"01"];
    let bucket = b"bucket";
    let file_name = b"sample64b";

    let mut file_leaves = Vec::new();

    println!("Chunking file into 32 byte chunks and building Merkle Patricia Tries...");

    for user_id in user_ids {
        let file_path = format!(
            "{}-{}-{}.txt",
            String::from_utf8(user_id.to_vec()).unwrap(),
            String::from_utf8(bucket.to_vec()).unwrap(),
            String::from_utf8(file_name.to_vec()).unwrap()
        );

        std::fs::create_dir_all(FILES_BASE_PATH).unwrap();
        std::fs::File::create(FILES_BASE_PATH.to_owned() + &file_path).unwrap();

        let file = std::fs::File::open(FILES_BASE_PATH.to_owned() + &file_path).unwrap();
        let file_size = std::fs::File::metadata(&file).unwrap().len();
        let (_memdb, fingerprint) = merklise_file::<LayoutV1<BlakeTwo256>>(&file_path);

        let metadata = FileMetadata {
            user_id,
            bucket,
            file_id: file_name,
            size: file_size,
            fingerprint: fingerprint
                .as_ref()
                .try_into()
                .expect("slice with incorrect length"),
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

            file_keys.push(file.0);
        }

        println!(
            "Merkle Patricia Forest Trie root: {:?}",
            merkle_forest_trie.root()
        );
    }

    // Sorting file keys for deterministic proof generation
    file_keys.sort();

    (memdb, root, file_keys)
}

/// Chunk a file into [`CHUNK_SIZE`] byte chunks and store them in a Merkle Patricia Trie.
///
/// The trie is stored in a [`MemoryDB`] and the [`Root`] is returned.
///
/// TODO: make this function fetch data from storage using Storage trait.
/// _It is assumed that the file is located in the same directory as the executable._
pub fn merklise_file<T: TrieLayout>(file_path: &str) -> (MemoryDB<T::Hash>, HashT<T>) {
    let file_path = FILES_BASE_PATH.to_owned() + file_path;
    let mut file = std::fs::File::open(file_path).unwrap();
    let mut buf = [0; CHUNK_SIZE];
    let mut chunks = Vec::new();

    // create list of key-value pairs consisting of chunk metadata and chunk data
    loop {
        let n = file.read(&mut buf).unwrap();
        if n == 0 {
            break;
        }

        let chunk_hash = T::Hash::hash(&buf);

        chunks.push((chunk_hash, buf.to_vec()));
    }

    let mut memdb = MemoryDB::<T::Hash>::default();
    let mut root = Default::default();
    {
        let mut t = TrieDBMutBuilder::<T>::new(&mut memdb, &mut root).build();
        for (k, v) in &chunks {
            t.insert(k.as_ref(), v).unwrap();
        }
    }

    (memdb, root)
}

mod verify_proof_tests {
    use super::*;

    #[test]
    fn commitment_verifier_challenge_exactly_first_key_success() {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        let challenge_key = leaf_keys.first().unwrap();

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            // Seek to the challenge key.
            iter.seek(&challenge_key.0).unwrap();

            // Read the leaf node.
            iter.next();
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        // Verify proof
        let proof_keys_with_values =
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &[*challenge_key],
                &proof,
            )
            .expect("Failed to verify proof");

        assert_eq!(
            proof_keys_with_values.keys().cloned().collect::<Vec<_>>(),
            vec![*challenge_key]
        );
    }

    #[test]
    fn commitment_verifier_challenge_key_in_between_success() {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        // Challenge key is the first key with the most significant bit incremented by 1.
        let mut challenge_key = leaf_keys[0];
        challenge_key.0[0] += 1;

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            // Seek to the challenge key.
            iter.seek(&challenge_key.0).unwrap();

            // Access the next leaf node.
            let next_leaf = iter.next();

            // Access the previous leaf node.
            let prev_leaf = iter.next_back();

            let challenged_key_vec = challenge_key.0.to_vec();

            // Assert that challenge_key is between `next_leaf` and `prev_leaf`
            assert!(
                prev_leaf.unwrap().unwrap().0 < challenged_key_vec
                    && challenged_key_vec < next_leaf.unwrap().unwrap().0
            );
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        // Verify proof
        let proof_keys_with_values =
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &[challenge_key],
                &proof,
            )
            .expect("Failed to verify proof");

        let mut expected_keys = vec![leaf_keys[0], leaf_keys[1]];
        expected_keys.sort();
        assert_eq!(
            proof_keys_with_values.keys().cloned().collect::<Vec<_>>(),
            expected_keys
        );
    }

    #[test]
    fn commitment_verifier_challenge_key_before_first_key_success() {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        // Challenge key is the first key with the most significant bit decremented by 1.
        let mut challenge_key = leaf_keys[0];
        challenge_key.0[0] -= 1;

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            // Seek to the challenge key.
            iter.seek(&challenge_key.0).unwrap();

            // Access the next leaf node.
            let next_leaf = iter.next();

            // Access the previous leaf node.
            let prev_leaf = iter.next_back();

            let challenged_key_vec = challenge_key.0.to_vec();

            // Assert that challenge_key is below next_leaf and that prev_leaf is None.
            assert!(prev_leaf.is_none() && challenged_key_vec < next_leaf.unwrap().unwrap().0);
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        // Verify proof
        let proof_keys_with_values =
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &[challenge_key],
                &proof,
            )
            .expect("Failed to verify proof");

        assert_eq!(
            proof_keys_with_values.keys().cloned().collect::<Vec<_>>(),
            vec![leaf_keys[0]]
        );
    }

    #[test]
    fn commitment_verifier_challenge_key_after_last_key_success() {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        let largest_key = leaf_keys
            .iter()
            .max()
            .map(|key| (*key, None::<TrieMutation>))
            .unwrap();

        // Challenge key is the largest key with the most significant bit incremented by 1.
        let mut challenge_key = largest_key.0;
        challenge_key.0[0] += 1;

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            // Seek to the challenge key.
            iter.seek(&challenge_key.0).unwrap();

            // Access the previous leaf node.
            let prev_leaf = iter.next_back();

            // Access the next leaf node.
            let next_leaf = iter.next();

            // Assert that challenge_key is greater than the last leaf node and that next_leaf is None.
            assert!(
                prev_leaf.unwrap().unwrap().0 < challenge_key.0.to_vec() && next_leaf.is_none()
            );
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        // Verify proof
        let proof_keys_with_values =
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &[challenge_key],
                &proof,
            )
            .expect("Failed to verify proof");

        assert_eq!(
            proof_keys_with_values.keys().cloned().collect::<Vec<_>>(),
            vec![largest_key.0]
        );
    }

    #[test]
    fn commitment_verifier_multiple_exact_challenge_keys_success() {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        let challenge_keys = [leaf_keys[0], leaf_keys[1], leaf_keys[2]];

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            for challenge_key in &challenge_keys {
                // Seek to the challenge key.
                iter.seek(&challenge_key.0).unwrap();

                // Access the next leaf node.
                iter.next();
            }
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        // Verify proof
        let proof_keys_with_values =
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &challenge_keys,
                &proof,
            )
            .expect("Failed to verify proof");

        let mut expected_keys: Vec<_> = challenge_keys.iter().cloned().collect();
        expected_keys.sort();
        assert_eq!(
            proof_keys_with_values.keys().cloned().collect::<Vec<_>>(),
            expected_keys
        );
    }

    #[test]
    fn commitment_verifier_multiple_in_between_challenge_keys_success() {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        let mut challenge_keys = [leaf_keys[0], leaf_keys[1], leaf_keys[2]];

        // Increment the most significant bit of every challenge key by 1.
        for key in &mut challenge_keys {
            key.0[0] += 1;
        }

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            for challenge_key in &challenge_keys {
                // Seek to the challenge key.
                iter.seek(&challenge_key.0).unwrap();

                // Access the next leaf node.
                iter.next();

                // Access the previous leaf node.
                iter.next_back();
            }
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        // Verify proof
        let proof_keys_with_values =
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &challenge_keys,
                &proof,
            )
            .expect("Failed to verify proof");

        let mut expected_keys = vec![leaf_keys[0], leaf_keys[1], leaf_keys[2], leaf_keys[3]];
        expected_keys.sort();
        assert_eq!(
            proof_keys_with_values.keys().cloned().collect::<Vec<_>>(),
            expected_keys
        );
    }

    #[test]
    fn commitment_verifier_multiple_in_between_challenge_keys_starting_before_first_key_success() {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        let mut challenge_keys = [leaf_keys[0], leaf_keys[1], leaf_keys[2]];

        // Decrement the most significant bit of every challenge key by 1.
        for key in &mut challenge_keys {
            key.0[0] -= 1;
        }

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            for challenge_key in &challenge_keys {
                // Seek to the challenge key.
                iter.seek(&challenge_key.0).unwrap();

                // Access the next leaf node.
                iter.next();

                // Access the previous leaf node.
                iter.next_back();
            }
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        // Verify proof
        let proof_keys_with_values =
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &challenge_keys,
                &proof,
            )
            .expect("Failed to verify proof");

        let mut expected_keys = vec![leaf_keys[0], leaf_keys[1], leaf_keys[2]];
        expected_keys.sort();
        assert_eq!(
            proof_keys_with_values.keys().cloned().collect::<Vec<_>>(),
            expected_keys
        );
    }

    #[test]
    fn commitment_verifier_multiple_in_between_challenge_keys_and_one_after_last_key_success() {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        let largest_key = leaf_keys.iter().max().unwrap();
        let mut challenge_keys = [
            leaf_keys[0],
            leaf_keys[1],
            leaf_keys[2],
            leaf_keys[3],
            *largest_key,
        ];

        // Increment the least significant byte of every challenge key by 1.
        for key in &mut challenge_keys {
            key.0[31] += 1;
        }

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            for challenge_key in &challenge_keys {
                // Seek to the challenge key.
                iter.seek(&challenge_key.0).unwrap();

                // Access the next leaf node.
                iter.next();

                // Access the previous leaf node.
                iter.next_back();
            }
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        // Verify proof
        let proof_keys_with_values =
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &challenge_keys,
                &proof,
            )
            .expect("Failed to verify proof");

        let mut expected_keys = vec![
            leaf_keys[0],
            leaf_keys[1],
            leaf_keys[2],
            leaf_keys[3],
            leaf_keys[4],
            *largest_key,
        ];
        expected_keys.sort();
        expected_keys.dedup();
        assert_eq!(
            proof_keys_with_values.keys().cloned().collect::<Vec<_>>(),
            expected_keys
        );
    }

    #[test]
    fn commitment_verifier_multiple_challenges_before_single_key_trie_success() {
        let (memdb, root, leaf_keys) =
            build_merkle_patricia_forest_one_key::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        let mut challenge_keys = [leaf_keys[0], leaf_keys[0], leaf_keys[0]];

        // Decrement the most significant bit of every challenge key by 1.
        let mut i = 0;
        for key in &mut challenge_keys {
            i += 2;
            key.0[0] -= i;
        }

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            for challenge_key in &challenge_keys {
                // Seek to the challenge key.
                iter.seek(&challenge_key.0).unwrap();

                // Access the next leaf node.
                iter.next();

                // Access the previous leaf node.
                iter.next_back();
            }
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        // Verify proof
        let proof_keys_with_values =
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &challenge_keys,
                &proof,
            )
            .expect("Failed to verify proof");

        assert_eq!(
            proof_keys_with_values.keys().cloned().collect::<Vec<_>>(),
            vec![leaf_keys[0]]
        );
    }

    #[test]
    fn commitment_verifier_multiple_challenges_after_single_key_trie_success() {
        let (memdb, root, leaf_keys) =
            build_merkle_patricia_forest_one_key::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        let mut challenge_keys = [leaf_keys[0], leaf_keys[0], leaf_keys[0]];

        // Decrement the most significant bit of every challenge key by 1.
        let mut i = 0;
        for key in &mut challenge_keys {
            i += 2;
            key.0[0] += i;
        }

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            for challenge_key in &challenge_keys {
                // Seek to the challenge key.
                iter.seek(&challenge_key.0).unwrap();

                // Access the next leaf node.
                iter.next();

                // Access the previous leaf node.
                iter.next_back();
            }
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        // Verify proof
        let proof_keys_with_values =
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &challenge_keys,
                &proof,
            )
            .expect("Failed to verify proof");

        assert_eq!(
            proof_keys_with_values.keys().cloned().collect::<Vec<_>>(),
            vec![leaf_keys[0]]
        );
    }

    #[test]
    fn commitment_verifier_multiple_challenges_single_key_trie_success() {
        let (memdb, root, leaf_keys) =
            build_merkle_patricia_forest_one_key::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        let mut challenge_keys = [leaf_keys[0], leaf_keys[0], leaf_keys[0]];

        // Decrement most significant byte of second challenge key by 1.
        challenge_keys[1].0[0] -= 1;
        // Increment most significant byte of third challenge key by 1.
        challenge_keys[2].0[0] += 1;

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            for challenge_key in &challenge_keys {
                // Seek to the challenge key.
                iter.seek(&challenge_key.0).unwrap();

                // Access the next leaf node.
                iter.next();

                // Access the previous leaf node.
                iter.next_back();
            }
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        // Verify proof
        let proof_keys_with_values =
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &challenge_keys,
                &proof,
            )
            .expect("Failed to verify proof");

        assert_eq!(
            proof_keys_with_values.keys().cloned().collect::<Vec<_>>(),
            vec![leaf_keys[0]]
        );
    }

    #[test]
    fn commitment_verifier_challenge_in_between_existing_leafs_shares_prefix_with_next_leaf() {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        let mut challenge_keys = [leaf_keys[1]];

        // Decrement the least significant byte of the challenge key by 1.
        challenge_keys[0].0[31] -= 1;

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            for challenge_key in &challenge_keys {
                // Seek to the challenge key.
                iter.seek(&challenge_key.0).unwrap();

                // Access the next leaf node.
                iter.next();

                // Access the previous leaf node.
                iter.next_back();
            }
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        // Verify proof
        let proof_keys_with_values =
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &challenge_keys,
                &proof,
            )
            .expect("Failed to verify proof");

        let mut expected_keys = vec![leaf_keys[0], leaf_keys[1]];
        expected_keys.sort();
        assert_eq!(
            proof_keys_with_values.keys().cloned().collect::<Vec<_>>(),
            expected_keys
        );
    }

    #[test]
    fn commitment_verifier_challenge_in_between_existing_leafs_shares_prefix_with_prev_leaf() {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        let mut challenge_keys = [leaf_keys[0]];

        // Increment the least significant byte of the challenge key by 1.
        challenge_keys[0].0[31] += 1;

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            for challenge_key in &challenge_keys {
                // Seek to the challenge key.
                iter.seek(&challenge_key.0).unwrap();

                // Access the next leaf node.
                iter.next();

                // Access the previous leaf node.
                iter.next_back();
            }
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        // Verify proof
        let proof_keys_with_values =
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &challenge_keys,
                &proof,
            )
            .expect("Failed to verify proof");

        let mut expected_keys = vec![leaf_keys[0], leaf_keys[1]];
        expected_keys.sort();
        assert_eq!(
            proof_keys_with_values.keys().cloned().collect::<Vec<_>>(),
            expected_keys
        );
    }

    #[test]
    fn commitment_verifier_empty_proof_and_root_failure() {
        let (_, _, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        let challenge_key = leaf_keys.first().unwrap();

        // Generate empty proof
        let empty_proof = CompactProof {
            encoded_nodes: vec![], // Empty proof
        };

        // Generate empty root
        let empty_root = Default::default();

        assert_eq!(
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &empty_root,
                &[*challenge_key],
                &empty_proof
            ),
            Err("Failed to convert proof to memory DB, root doesn't match with expected.".into())
        );
    }

    #[test]
    fn commitment_verifier_invalid_root_failure() {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        let challenge_key = leaf_keys.first().unwrap();

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            // Seek to the challenge key.
            iter.seek(&challenge_key.0).unwrap();
            iter.next().unwrap().unwrap();
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        let invalid_root = Default::default();

        assert_eq!(
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &invalid_root,
                &[*challenge_key],
                &proof
            ),
            Err("Failed to convert proof to memory DB, root doesn't match with expected.".into())
        );
    }

    #[test]
    fn commitment_verifier_invalid_proof_failure() {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        let challenge_key = leaf_keys.first().unwrap();

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            // Seek to the challenge key.
            iter.seek(&challenge_key.0).unwrap();
            iter.next().unwrap().unwrap();
        }

        // Generate proof
        let mut proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        // Modify the proof to make it invalid
        proof.encoded_nodes[0] = vec![0; 32];

        assert_eq!(
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &[*challenge_key],
                &proof
            ),
            Err("Failed to convert proof to memory DB, root doesn't match with expected.".into())
        );
    }

    #[test]
    fn commitment_verifier_empty_proof_failure() {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        let challenge_key = leaf_keys.first().unwrap();

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let _iter = trie.into_double_ended_iter().unwrap();

            // Not seeking any key, so that no leaf nodes are accessed.
        }

        // Generate proof
        let proof = CompactProof {
            encoded_nodes: vec![], // Empty proof
        };

        assert_eq!(
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &[*challenge_key],
                &proof
            ),
            Err("Failed to convert proof to memory DB, root doesn't match with expected.".into())
        );
    }

    #[test]
    fn commitment_verifier_no_challenges_failure() {
        let (memdb, root, _) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let _iter = trie.into_double_ended_iter().unwrap();

            // Not seeking any key, so that no leaf nodes are accessed.
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        assert_eq!(
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &[],
                &proof
            ),
            Err("No challenges provided.".into())
        );
    }

    #[test]
    fn commitment_verifier_no_leaves_in_proof_failure() {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        let challenge_key = leaf_keys.first().unwrap();

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let _iter = trie.into_double_ended_iter().unwrap();

            // Not seeking any key, so that no leaf nodes are accessed.
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        assert_eq!(
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &[*challenge_key],
                &proof
            ),
            Err("Failed to seek challenged key.".into())
        );
    }

    #[test]
    fn commitment_verifier_wrong_proof_answer_to_challenge_failure() {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        let challenge_key = &leaf_keys[0];
        let wrong_challenge_key = &leaf_keys[1];

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            // Seek to the wrong challenge key so that we can generate a valid proof for the wrong key.
            iter.seek(&wrong_challenge_key.0).unwrap();
            iter.next().unwrap().unwrap();
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        assert_eq!(
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &[*challenge_key],
                &proof
            ),
            Err("Failed to seek challenged key.".into())
        );
    }

    #[test]
    fn commitment_verifier_wrong_proof_next_and_prev_when_should_be_exact_failure() {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        // Existing key before the challenge.
        let prev_challenge_key = leaf_keys[0];

        // Actual challenge, which is an existing key in the trie.
        let challenge_key = leaf_keys[1];

        // Existing key after the challenge.
        let next_challenge_key = leaf_keys[2];

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            // Seek to the existing key before the challenge.
            iter.seek(&prev_challenge_key.0).unwrap();

            // Seek to the key after the challenge.
            iter.seek(&next_challenge_key.0).unwrap();
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        assert_eq!(
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &[challenge_key],
                &proof
            ),
            Err("Failed to seek challenged key.".into())
        );
    }

    #[test]
    fn commitment_verifier_wrong_proof_only_provide_prev_failure() {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        // Increment the most significant byte of the challenge key by 1.
        let mut challenge_key = leaf_keys[0];
        challenge_key.0[0] += 1;

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            // Seek to the challenge key, this will only look up the next leaf node.
            iter.seek(&challenge_key.0).unwrap();
            iter.next_back().unwrap().unwrap();
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        assert_eq!(
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &[challenge_key],
                &proof
            ),
            Err("Failed to get next leaf.".into())
        );
    }

    #[test]
    fn commitment_verifier_wrong_proof_only_provide_next_failure() {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        // Increment the most significant byte of the challenge key by 1.
        let mut challenge_key = leaf_keys[0];
        challenge_key.0[0] += 1;

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            // Seek to the challenge key, this will only look up the next leaf node.
            iter.seek(&challenge_key.0).unwrap();
            iter.next().unwrap().unwrap();
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        assert_eq!(
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &[challenge_key],
                &proof
            ),
            Err("Failed to get previous leaf.".into())
        );
    }

    #[test]
    fn commitment_verifier_wrong_proof_skip_actual_next_leaf_failure() {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        // Increment the most significant byte of the challenge key by 1.
        let mut challenge_key = leaf_keys[0];
        challenge_key.0[0] += 1;

        // Do the same for the next leaf key.
        let mut next_leaf_key = leaf_keys[1];
        next_leaf_key.0[0] += 1;

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            // Create an iterator over the leaf nodes.
            let mut iter = trie.into_double_ended_iter().unwrap();

            // Seek to the challenge key, this will only look up the next leaf node.
            iter.seek(&challenge_key.0).unwrap();
            iter.next_back().unwrap().unwrap();

            // Seek to two keys ahead of the challenge key.
            iter.seek(&next_leaf_key.0).unwrap();
            iter.next().unwrap().unwrap();
        }

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create compact proof from recorder");

        assert_eq!(
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &[challenge_key],
                &proof
            ),
            Err("Failed to get next leaf.".into())
        );
    }

    #[test]
    fn verify_proof_works_for_empty_proof() {
        let (memdb, root) = MemoryDB::<BlakeTwo256>::default_with_root();

        // This recorder is used to record accessed keys in the trie and later generate a proof for them.
        let recorder: Recorder<BlakeTwo256> = Recorder::default();

        {
            // Creating trie inside of closure to drop it before generating proof.
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let _trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();
        }

        // Generate empty proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(root)
            .expect("Failed to create empty compact proof from recorder");

        assert_eq!(
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                &[H256::random(), H256::random(), H256::random()],
                &proof
            ),
            Ok(BTreeMap::new())
        );
    }
}

mod mutate_root_tests {

    use super::*;
    use sp_core::H256;
    use sp_runtime::DispatchError;
    use std::{fs::File, io::Write};

    fn setup_trie_and_recorder() -> (
        MemoryDB<BlakeTwo256>,
        H256,
        Vec<H256>,
        Recorder<BlakeTwo256>,
    ) {
        let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<BlakeTwo256>>();
        let recorder: Recorder<BlakeTwo256> = Recorder::default();
        (memdb, root, leaf_keys, recorder)
    }

    fn generate_proof_and_verify(
        recorder: &mut Recorder<BlakeTwo256>,
        root: &H256,
        challenge_keys: &[H256],
    ) -> CompactProof {
        let proof = recorder
            .clone()
            .drain_storage_proof()
            .to_compact_proof::<BlakeTwo256>(*root)
            .expect("Failed to create compact proof from recorder");

        ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
            root,
            challenge_keys,
            &proof,
        )
        .expect("Failed to verify proof");

        proof
    }

    fn assert_key_in_trie(memdb: &MemoryDB<BlakeTwo256>, root: &H256, key: &H256) {
        let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(memdb, root).build();
        let mut iter = trie.iter().unwrap();
        iter.seek(&key.0).unwrap();
        assert_eq!(iter.next().unwrap().unwrap().0, key.0);
    }

    fn assert_key_not_in_trie(memdb: &MemoryDB<BlakeTwo256>, root: &H256, key: &H256) {
        let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(memdb, root).build();
        let mut iter = trie.iter().unwrap();

        // Seek to the key that shouldn't exist.
        iter.seek(&key.0).unwrap();

        // Get the next key. The acceptable scenarios are...
        let next_key = iter.next();
        // 1. `next_key` is None, which means that the key we searched for a key at the end of the trie, and there's nothing after it.
        if let Some(next_key) = next_key {
            // If the next key is Some, it means that there is something after the key we searched for.
            // 2. If that something is an error, it means that the trie knows that there is something, but that something is not
            // in `memdb`, so in fact the `key` searched is not in the trie.
            if let Ok(next_key) = next_key {
                // 3. If the next key is not an error, it means that there is something after the key we searched for, and that something is in `memdb`.
                // In this case, we have to check that that something is not the key we searched for.
                assert_ne!(next_key.0, key.0);
            }
        }
    }

    fn generate_unique_key(existing_keys: &Vec<H256>) -> H256 {
        let mut new_key = H256::random();
        while existing_keys.contains(&new_key) {
            new_key = H256::random();
        }
        new_key
    }

    #[test]
    fn mutate_root_add_key_success() {
        let (memdb, root, leaf_keys, mut recorder) = setup_trie_and_recorder();

        let challenge_key = generate_unique_key(&leaf_keys);
        let mutations: Vec<(H256, TrieMutation)> =
            vec![(challenge_key, TrieAddMutation::default().into())];

        {
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            let mut iter = trie.into_double_ended_iter().unwrap();
            iter.seek(&challenge_key.0).unwrap();

            // Access the next leaf node.
            iter.next();

            // Access the previous leaf node.
            iter.next_back();
        }

        let proof = generate_proof_and_verify(&mut recorder, &root, &[challenge_key]);

        let (memdb, new_root, mutated_keys_and_values) =
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::apply_delta(
                &root, &mutations, &proof,
            )
            .expect("Failed to mutate root");

        // Check that the key was added to the trie with an empty value
        assert!(mutated_keys_and_values.contains(&(challenge_key, Some(Vec::new()))));
        assert_key_in_trie(&memdb, &new_root, &challenge_key);
    }

    #[test]
    fn mutate_root_remove_key_success() {
        let (memdb, root, leaf_keys, mut recorder) = setup_trie_and_recorder();

        let challenge_key = *leaf_keys.first().unwrap();
        let mutations: Vec<(H256, TrieMutation)> =
            vec![(challenge_key, TrieRemoveMutation::default().into())];

        {
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            let mut iter = trie.into_double_ended_iter().unwrap();
            iter.seek(&challenge_key.0).unwrap();
            assert_eq!(iter.next().unwrap().unwrap().0, challenge_key.0);
        }

        let proof = generate_proof_and_verify(&mut recorder, &root, &[challenge_key]);

        let (memdb, new_root, _mutated_keys_and_values) =
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::apply_delta(
                &root, &mutations, &proof,
            )
            .expect("Failed to mutate root");

        assert_key_not_in_trie(&memdb, &new_root, &challenge_key);
    }

    #[test]
    fn mutate_root_add_multiple_keys_success() {
        let (memdb, root, leaf_keys, mut recorder) = setup_trie_and_recorder();

        let mut challenge_keys = vec![];
        for _ in 0..3 {
            challenge_keys.push(generate_unique_key(&leaf_keys));
        }

        let mutations: Vec<(H256, TrieMutation)> = challenge_keys
            .iter()
            .map(|key| (*key, TrieAddMutation::default().into()))
            .collect();

        {
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            for challenge_key in &challenge_keys {
                let mut iter = trie.into_double_ended_iter().unwrap();
                iter.seek(challenge_key.0.as_slice()).unwrap();

                // Access the next leaf node.
                iter.next();

                // Access the previous leaf node.
                iter.next_back();
            }
        }

        let proof = generate_proof_and_verify(&mut recorder, &root, &challenge_keys);

        let (memdb, new_root, mutated_keys_and_values) =
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::apply_delta(
                &root, &mutations, &proof,
            )
            .expect("Failed to mutate root");

        for challenge_key in &challenge_keys {
            assert!(mutated_keys_and_values.contains(&(*challenge_key, Some(Vec::new()))));
            assert_key_in_trie(&memdb, &new_root, &challenge_key);
        }
    }

    #[test]
    fn mutate_root_remove_multiple_keys_success() {
        let (memdb, root, leaf_keys, mut recorder) = setup_trie_and_recorder();
        let mut merkle_trie_file = File::create("tmp/merkle_trie.txt").unwrap();
        let merkle_trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root).build();
        write!(&mut merkle_trie_file, "{:#?}", merkle_trie).unwrap();
        let challenge_keys = leaf_keys.clone().into_iter().take(3).collect::<Vec<H256>>();
        let extra_needed_leaf_key = leaf_keys.get(3).unwrap();
        /* let third_child_key: H256 = H256([
            48, 107, 48, 245, 199, 176, 169, 107, 124, 91, 182, 231, 9, 210, 98, 97, 186, 176, 216,
            231, 145, 168, 8, 32, 60, 171, 240, 241, 139, 223, 163, 222,
        ]); */
        let mutations: Vec<(H256, TrieMutation)> = challenge_keys
            .iter()
            .map(|key| (*key, TrieRemoveMutation::default().into()))
            .collect();

        {
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            for challenge_key in &challenge_keys {
                let mut iter = trie.iter().unwrap();
                iter.seek(&challenge_key.0).unwrap();
                assert_eq!(iter.next().unwrap().unwrap().0, challenge_key.0);
            }
            // let node_value = trie.db().get(&third_child_key, (&[], Some(16))).unwrap();
            // assert_eq!(node_value, third_child_key.0.to_vec());
            // let full_trie: Vec<(Vec<u8>, Vec<u8>)> = iter.map(|element| element.unwrap()).collect();

            // Error: the proof generated by our recorder is valid to prove that a key exists in the trie,
            // but it is not useful to get a new root from it after applying mutations (since the reconstructed trie
            // is incomplete for that). To be able to use `apply_delta` to get the new root after the required
            // mutations, we have to make sure that we include in the proof not only the keys that we want to remove,
            // but also ALL their siblings, so that the trie can be reconstructed correctly.
            // In this case, the missing sibling is a branch node (prefix 20), so it's enough to include one of the leaf keys
            // for which the branch node is the parent.
            // We do this by simply seeking to it, as the recorder then records the path to it, which includes the branch node.
            let mut iter = trie.iter().unwrap();
            iter.seek(&extra_needed_leaf_key.0).unwrap();
            assert_eq!(iter.next().unwrap().unwrap().0, extra_needed_leaf_key.0);
            // Alternative way, a bit more clean (no need for hardcoding):
            let mut iter = trie.iter().unwrap();
            iter.seek(&challenge_keys.last().unwrap().0).unwrap();
            iter.next();
            assert_eq!(iter.next().unwrap().unwrap().0, extra_needed_leaf_key.0);
        }

        let proof = generate_proof_and_verify(&mut recorder, &root, challenge_keys.as_slice());
        /* let mut challenge_keys_with_values = challenge_keys
            .iter()
            .map(|key| (*key, Some(vec![])))
            .collect::<Vec<(H256, Option<Vec<u8>>)>>();
        challenge_keys_with_values.get_mut(0).unwrap().1 =
            Some(proof.encoded_nodes.get(2).unwrap().clone());
        challenge_keys_with_values.get_mut(1).unwrap().1 =
            Some(proof.encoded_nodes.get(5).unwrap().clone());
        challenge_keys_with_values.get_mut(2).unwrap().1 =
            Some(proof.encoded_nodes.get(7).unwrap().clone());
        let sp_trie_proof = sp_trie::generate_trie_proof::<
            LayoutV1<BlakeTwo256>,
            &Vec<sp_core::H256>,
            sp_core::H256,
            MemoryDB<BlakeTwo256>,
        >(&memdb, root, &challenge_keys)
        .unwrap();
        assert!(sp_trie::verify_trie_proof::<
            LayoutV1<BlakeTwo256>,
            &Vec<(sp_core::H256, std::option::Option<Vec<u8>>)>,
            sp_core::H256,
            Vec<u8>,
        >(&root, &sp_trie_proof, &challenge_keys_with_values)
        .is_ok());
        let storage_proof_sp_trie = sp_trie::StorageProof::new(sp_trie_proof);
        let mut memdb_sp_trie = storage_proof_sp_trie.to_memory_db::<BlakeTwo256>();
        let mut root_sp_trie = root.clone();
        /* let mut trie_sp_trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::from_existing(
            &mut memdb_sp_trie,
            &mut root_sp_trie,
        )
        .build(); */
        challenge_keys_with_values.get_mut(0).unwrap().1 = None;
        challenge_keys_with_values.get_mut(1).unwrap().1 = None;
        challenge_keys_with_values.get_mut(2).unwrap().1 = None;
        let challenge_keys_with_values = challenge_keys_with_values
            .iter()
            .map(|(key, value)| (key.0.to_vec(), value.clone()))
            .collect::<Vec<(Vec<u8>, Option<Vec<u8>>)>>();
        let new_root_after_deletion = sp_trie::delta_trie_root::<
            LayoutV1<BlakeTwo256>,
            Vec<(Vec<u8>, std::option::Option<Vec<u8>>)>,
            Vec<u8>,
            std::option::Option<Vec<u8>>,
            MemoryDB<BlakeTwo256>,
            Vec<u8>,
        >(
            &mut memdb_sp_trie,
            root_sp_trie,
            challenge_keys_with_values,
            None,
            None,
        );
        match new_root_after_deletion {
            Ok(_) => (),
            Err(err) => panic!("Error: {:?}", err),
        } */

        // Execute mutations to remove the selected keys and generate the new root.
        let (partial_trie_memdb, new_root, _mutated_keys_and_values) =
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::apply_delta(
                &root, &mutations, &proof,
            )
            .expect("Failed to mutate root");

        // Check that none of the deleted keys are still in the generated partial trie.
        for challenge_key in &challenge_keys {
            assert_key_not_in_trie(&partial_trie_memdb, &new_root, &challenge_key);
        }

        // Reconstruct the full trie, remove the selected keys, calculate the root and compare it with the one obtained using the `apply_delta` function.
        let mut full_memdb = memdb.clone();
        let mut full_root = root.clone();
        let mut full_trie = TrieDBMutBuilder::<LayoutV1<BlakeTwo256>>::from_existing(
            &mut full_memdb,
            &mut full_root,
        )
        .build();
        for challenge_key in &challenge_keys {
            // Remove the key from the trie.
            full_trie.remove(&challenge_key.0).unwrap();
        }

        // Calculate the root of the trie after removing the keys.
        let new_root_full_trie = full_trie.root();

        // Check that the roots obtained using the two methods are the same.
        assert_eq!(new_root, *new_root_full_trie);
    }

    #[test]
    fn mutate_root_add_remove_multiple_keys_success() {
        let (memdb, root, leaf_keys, mut recorder) = setup_trie_and_recorder();

        let mut leaf_to_add = *leaf_keys.first().unwrap();

        leaf_to_add.0[0] += 1;

        let add_mutation: (H256, TrieMutation) = (leaf_to_add, TrieAddMutation::default().into());

        let remove_mutation: (H256, TrieMutation) = (
            *leaf_keys.last().unwrap(),
            TrieRemoveMutation::default().into(),
        );

        let mutations = [add_mutation.clone(), remove_mutation.clone()];

        {
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            for challenge_key in &mutations {
                let mut iter = trie.into_double_ended_iter().unwrap();
                iter.seek(challenge_key.0 .0.as_slice()).unwrap();

                // Access the next leaf node.
                iter.next();

                // Access the previous leaf node only if it's an Add mutation.
                if let TrieMutation::Add(_) = challenge_key.1 {
                    iter.next_back();
                }
            }
        }

        let proof = generate_proof_and_verify(
            &mut recorder,
            &root,
            &mutations.iter().map(|(key, _)| *key).collect::<Vec<H256>>(),
        );

        let (memdb, new_root, _mutated_keys_and_values) =
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::apply_delta(
                &root, &mutations, &proof,
            )
            .expect("Failed to mutate root");

        for challenge_key in &mutations {
            if let TrieMutation::Add(_) = challenge_key.1 {
                assert_key_in_trie(&memdb, &new_root, &challenge_key.0);
            } else {
                assert_key_not_in_trie(&memdb, &new_root, &challenge_key.0);
            }
        }
    }

    #[test]
    fn mutate_root_no_mutations_failure() {
        let (memdb, root, leaf_keys, mut recorder) = setup_trie_and_recorder();

        let challenge_key = *leaf_keys.first().unwrap();
        let mutations: Vec<(H256, TrieMutation)> = vec![];

        {
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            let mut iter = trie.into_double_ended_iter().unwrap();
            iter.seek(&challenge_key.0).unwrap();

            // Access the next leaf node.
            iter.next();
        }

        let proof = generate_proof_and_verify(&mut recorder, &root, &[challenge_key]);

        let err =
            match ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::apply_delta(
                &root, &mutations, &proof,
            ) {
                Err(err) => err,
                Ok(_) => panic!("Expected error."),
            };

        assert_eq!(err, DispatchError::Other("No mutations provided."));
    }

    #[test]
    fn mutate_root_does_not_match_expected_fail() {
        let (memdb, root, leaf_keys, mut recorder) = setup_trie_and_recorder();

        let challenge_key = *leaf_keys.first().unwrap();
        let mutations: Vec<(H256, TrieMutation)> =
            vec![(challenge_key, TrieRemoveMutation::default().into())];

        {
            let mut trie_recorder = recorder.as_trie_recorder(root);
            let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
                .with_recorder(&mut trie_recorder)
                .build();

            let mut iter = trie.into_double_ended_iter().unwrap();
            iter.seek(&challenge_key.0).unwrap();

            // Access the next leaf node.
            iter.next();
        }

        let proof = generate_proof_and_verify(&mut recorder, &root, &[challenge_key]);

        let err =
            match ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::apply_delta(
                &H256([0; 32]),
                &mutations,
                &proof,
            ) {
                Err(err) => err,
                Ok(_) => panic!("Expected error."),
            };

        assert_eq!(
            err,
            DispatchError::Other(
                "Failed to convert proof to memory DB, root doesn't match with expected."
            )
        );
    }
}
