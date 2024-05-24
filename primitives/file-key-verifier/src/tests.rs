use std::{
    fs::File,
    io::{Read, Write},
};

use codec::Encode;
use rand::Rng;
use serde::Serialize;
use sp_trie::{MemoryDB, TrieDBMutBuilder, TrieLayout, TrieMut};
use trie_db::Hasher;

/// The hash type of trie node keys
type HashT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;

const CHUNK_SIZE: usize = 2;
const FILES_BASE_PATH: &str = "./tmp/";

#[derive(Serialize, PartialEq, Debug)]
struct FileMetadata {
    owner: Vec<u8>,
    location: Vec<u8>,
    size: u64,
    /// The fingerprint will always be 32 bytes since we are using Keccak256, aka RefHasher.
    fingerprint: [u8; 32],
}

/// Build a Merkle Patricia Trie simulating a file split in chunks.
pub fn build_merkle_patricia_trie<T: TrieLayout>() -> (MemoryDB<T::Hash>, HashT<T>, FileMetadata) {
    let user_id = b"user_id";
    let bucket = b"bucket";
    let file_name = b"sample64b";
    let file_size = 2u64.pow(100);

    println!("Chunking file into 32 byte chunks and building Merkle Patricia Trie...");

    let file_path = format!(
        "{}-{}-{}.txt",
        String::from_utf8(user_id.to_vec()).unwrap(),
        String::from_utf8(bucket.to_vec()).unwrap(),
        String::from_utf8(file_name.to_vec()).unwrap()
    );

    let file = create_test_file(&file_path, file_size as usize);
    let (memdb, fingerprint) = merklise_file::<T>(&file_path);

    let metadata = FileMetadata {
        owner: user_id.to_vec(),
        location: file_path.as_bytes().to_vec(),
        size: file_size,
        fingerprint: fingerprint
            .as_ref()
            .try_into()
            .expect("slice with incorrect length"),
    };

    let file_key = T::Hash::hash(
        &[
            &metadata.owner.encode(),
            &metadata.location.encode(),
            &metadata.size.encode(),
            &metadata.fingerprint.encode(),
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<u8>>(),
    );

    (memdb, file_key, metadata)
}

/// Build a Merkle Patricia Trie with just one chunk.
pub fn build_merkle_patricia_trie_one_key<T: TrieLayout>(
) -> (MemoryDB<T::Hash>, HashT<T>, FileMetadata) {
    let user_id = b"user_id";
    let bucket = b"bucket";
    let file_name = b"sample64b";
    let file_size = 2u64.pow(100);

    println!("Chunking file into 32 byte chunks and building Merkle Patricia Trie...");

    let file_path = format!(
        "{}-{}-{}.txt",
        String::from_utf8(user_id.to_vec()).unwrap(),
        String::from_utf8(bucket.to_vec()).unwrap(),
        String::from_utf8(file_name.to_vec()).unwrap()
    );

    let file = create_test_file(&file_path, CHUNK_SIZE);
    let (memdb, fingerprint) = merklise_file::<T>(&file_path);

    let metadata = FileMetadata {
        owner: user_id.to_vec(),
        location: file_path.as_bytes().to_vec(),
        size: file_size,
        fingerprint: fingerprint
            .as_ref()
            .try_into()
            .expect("slice with incorrect length"),
    };

    let file_key = T::Hash::hash(
        &[
            &metadata.owner.encode(),
            &metadata.location.encode(),
            &metadata.size.encode(),
            &metadata.fingerprint.encode(),
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<u8>>(),
    );

    (memdb, file_key, metadata)
}

/// Chunk a file into [`CHUNK_SIZE`] byte chunks and store them in a Merkle Patricia Trie.
///
/// The trie is stored in a [`MemoryDB`] and the [`Root`] is returned.
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

pub fn create_test_file(filename: &str, size: usize) -> File {
    let mut file = File::create(filename).unwrap();
    let mut rng = rand::thread_rng();

    // Generate random content
    let content: Vec<u8> = (0..size).map(|_| rng.gen()).collect();

    file.write_all(&content).unwrap();

    file
}
