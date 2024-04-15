use std::io::Read;

use frame_support::assert_ok;
use reference_trie::RefHasher;
use serde::Serialize;
use sp_trie::{
    recorder::Recorder, LayoutV1, MemoryDB, TrieDBBuilder, TrieDBMutBuilder, TrieLayout, TrieMut,
};
use storage_hub_traits::CommitmentVerifier;
use trie_db::{Hasher, TrieIterator};

use crate::TrieVerifier;

/// The hash type of trie node keys
type HashT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;

const CHUNK_SIZE: usize = 2;
const FILES_BASE_PATH: &str = "./tmp/";

#[derive(Serialize, PartialEq, Debug)]
struct FileMetadata<'a> {
    user_id: &'a [u8],
    bucket: &'a [u8],
    file_id: &'a [u8],
    size: u64,
    /// The fingerprint will always be 32 bytes since we are using Keccak256, aka RefHasher.
    fingerprint: [u8; 32],
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
        b"12", b"13", b"14", b"15", b"16", b"17", b"18", b"19", b"20", b"21", b"22", b"23", b"24",
        b"25", b"26", b"27", b"28", b"29", b"30", b"31", b"32",
    ];
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
        let (_memdb, fingerprint) = merklise_file::<LayoutV1<RefHasher>>(&file_path);

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

            file_keys.push(file.0.clone());
        }

        println!(
            "Merkle Patricia Forest Trie root: {:?}",
            merkle_forest_trie.root()
        );
    }
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

#[test]
fn commitment_verifier_scenario_1_success() {
    let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<RefHasher>>();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<RefHasher> = Recorder::default();

    let challenge_key = leaf_keys.first().unwrap();

    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<RefHasher>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.into_double_ended_iter().unwrap();

        // Seek to the challenge key.
        // This already accesses and records the the node. There is no need to call `next()`.
        iter.seek(challenge_key).unwrap();
    }

    // Generate proof
    let proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<RefHasher>(root)
        .expect("Failed to create compact proof from recorder");

    assert_ok!(TrieVerifier::<RefHasher>::verify_proof(
        &root,
        &[*challenge_key],
        &proof
    ));
}

#[test]
fn commitment_verifier_scenario_2_success() {
    let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<RefHasher>>();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<RefHasher> = Recorder::default();

    // challenged_key xor leaf key 1
    let challenge_key = leaf_keys[0]
        .iter()
        .zip(leaf_keys[1].iter())
        .map(|(a, b)| a ^ b)
        .collect::<Vec<u8>>();

    let challenge_key: [u8; 32] = challenge_key
        .as_slice()
        .try_into()
        .expect("slice with incorrect length");

    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<RefHasher>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.into_double_ended_iter().unwrap();

        // Seek to the challenge key.
        iter.seek(&challenge_key).unwrap();

        // Access the next leaf node.
        let next_leaf = iter.next();

        // Access the previous leaf node.
        let prev_leaf = iter.next_back();

        let challenged_key_vec = challenge_key.to_vec();
        // assert that challenge_key is between `next_leaf` and `prev_leaf`
        assert!(
            prev_leaf.unwrap().unwrap().0 < challenged_key_vec
                && challenged_key_vec < next_leaf.unwrap().unwrap().0
        );
    }

    // Generate proof
    let proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<RefHasher>(root)
        .expect("Failed to create compact proof from recorder");

    assert_ok!(TrieVerifier::<RefHasher>::verify_proof(
        &root,
        &[challenge_key],
        &proof
    ));
}

#[test]
fn commitment_verifier_scenario_3_success() {
    let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<RefHasher>>();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<RefHasher> = Recorder::default();

    let largest_key = leaf_keys.iter().max().unwrap();

    // Challenge a key that exceeds the largest key in the trie
    let challenge_key: Vec<u8> = largest_key.iter().map(|&b| b.saturating_add(1)).collect();
    let challenge_key: [u8; 32] = challenge_key
        .as_slice()
        .try_into()
        .expect("slice with incorrect length");

    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<RefHasher>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.into_double_ended_iter().unwrap();

        // Seek to the challenge key.
        iter.seek(&challenge_key).unwrap();

        // Access the previous leaf node.
        let prev_leaf = iter.next_back();

        // assert that challenge_key is greater than the last leaf node
        assert!(prev_leaf.unwrap().unwrap().0 < challenge_key.to_vec());
    }

    // Generate proof
    let proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<RefHasher>(root)
        .expect("Failed to create compact proof from recorder");

    assert_ok!(TrieVerifier::<RefHasher>::verify_proof(
        &root,
        &[challenge_key],
        &proof
    ));
}

#[test]
fn commitment_verifier_scenario_4_success() {
    let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<RefHasher>>();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<RefHasher> = Recorder::default();

    let smallest_key = leaf_keys.iter().min().unwrap();

    // Challenge a key that is less than the smallest key in the trie
    let challenge_key: Vec<u8> = smallest_key.iter().map(|&b| b.saturating_sub(1)).collect();
    let challenge_key: [u8; 32] = challenge_key
        .as_slice()
        .try_into()
        .expect("slice with incorrect length");

    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<RefHasher>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.into_double_ended_iter().unwrap();

        // Seek to the challenge key.
        iter.seek(&challenge_key).unwrap();

        // Access the next leaf node.
        let next_leaf = iter.next();

        // assert that challenge_key is less than the first leaf node
        assert!(next_leaf.unwrap().unwrap().0 > challenge_key.to_vec());
    }

    // Generate proof
    let proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<RefHasher>(root)
        .expect("Failed to create compact proof from recorder");

    assert_ok!(TrieVerifier::<RefHasher>::verify_proof(
        &root,
        &[challenge_key],
        &proof
    ));
}

#[test]
fn commitment_verifier_multiple_challenge_keys_success() {
    let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<RefHasher>>();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<RefHasher> = Recorder::default();

    let challenge_keys = [leaf_keys[0], leaf_keys[1], leaf_keys[2]];

    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<RefHasher>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.into_double_ended_iter().unwrap();

        for challenge_key in &challenge_keys {
            // Seek to the challenge key.
            iter.seek(challenge_key).unwrap();
        }
    }

    // Generate proof
    let proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<RefHasher>(root)
        .expect("Failed to create compact proof from recorder");

    assert_ok!(TrieVerifier::<RefHasher>::verify_proof(
        &root,
        &challenge_keys,
        &proof
    ));
}

#[test]
fn commitment_verifier_invalid_proof_failure() {
    let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<RefHasher>>();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<RefHasher> = Recorder::default();

    let challenge_key = leaf_keys.first().unwrap();

    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<RefHasher>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.into_double_ended_iter().unwrap();

        // Seek to the challenge key.
        // This already accesses and records the the node. There is no need to call `next()`.
        iter.seek(challenge_key).unwrap();
    }

    // Generate proof
    let mut proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<RefHasher>(root)
        .expect("Failed to create compact proof from recorder");

    // Modify the proof to make it invalid
    proof.encoded_nodes[0] = vec![0; 32];

    assert!(TrieVerifier::<RefHasher>::verify_proof(&root, &[*challenge_key], &proof).is_err());
}

#[test]
fn commitment_verifier_invalid_root_failure() {
    let (memdb, root, leaf_keys) = build_merkle_patricia_forest::<LayoutV1<RefHasher>>();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<RefHasher> = Recorder::default();

    let challenge_key = leaf_keys.first().unwrap();

    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<RefHasher>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.into_double_ended_iter().unwrap();

        // Seek to the challenge key.
        // This already accesses and records the the node. There is no need to call `next()`.
        iter.seek(challenge_key).unwrap();
    }

    // Generate proof
    let proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<RefHasher>(root)
        .expect("Failed to create compact proof from recorder");

    let invalid_root = Default::default();

    assert!(
        TrieVerifier::<RefHasher>::verify_proof(&invalid_root, &[*challenge_key], &proof).is_err()
    );
}
