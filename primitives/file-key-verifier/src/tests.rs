use std::io::Read;

use reference_trie::RefHasher;
use serde::Serialize;
use sp_trie::{
    recorder::Recorder, CompactProof, LayoutV1, MemoryDB, TrieDBBuilder, TrieDBMutBuilder,
    TrieLayout, TrieMut,
};
use storage_hub_traits::CommitmentVerifier;
use trie_db::{Hasher, TrieIterator};

use crate::FileKeyVerifier;

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
