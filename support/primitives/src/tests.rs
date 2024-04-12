use std::io::Read;

use frame_support::assert_ok;
use reference_trie::RefHasher;
use serde::Serialize;
use sp_trie::{
    recorder::Recorder, LayoutV1, MemoryDB, Trie, TrieDBBuilder, TrieDBMutBuilder, TrieLayout,
    TrieMut,
};
use storage_hub_traits::CommitmentVerifier;
use trie_db::Hasher;

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
fn e2e_test() {
    let (memdb, root, _leaf_keys) = build_merkle_patricia_forest::<LayoutV1<RefHasher>>();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<RefHasher> = Recorder::default();

    // Generate a previous and next challenge hash.
    // This is because trie-db does not implement DoubleEndedIterator for the TrieDBNodeIterator,
    // so what we do here instead is hardcode the previous challenge as well as the actual
    // challenge hashes, by looking at the leaf node keys printed below inside the closure.
    let prev_challenge_hash = [
        132, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];
    let challenge_hash = [
        138, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];

    // ************* PROVER (STORAGE PROVIDER) *************
    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<RefHasher>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.iter().unwrap();

        // Seek to the previous and next challenge hashes.
        iter.seek(&prev_challenge_hash).unwrap();
        let prev_key = iter.next().unwrap().unwrap().0;
        println!("Prev key: {:?}", prev_key);

        iter.seek(&challenge_hash).unwrap();
        let next_key = iter.next().unwrap().unwrap().0;
        println!("Next key: {:?}", next_key);

        // Accessing the previous and next challenge hashes.
        let _ = trie.get(&prev_key).unwrap().unwrap();
        let _ = trie.get(&next_key).unwrap().unwrap();
    }

    // Generate proof for the previous and next challenge hashes.
    let proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<RefHasher>(root)
        .unwrap();

    assert_ok!(TrieVerifier::<RefHasher>::verify_proof(
        &root,
        &[prev_challenge_hash, challenge_hash],
        &proof
    ));
}
