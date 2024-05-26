use std::{
    fs::{create_dir_all, File},
    io::{Read, Write},
};

use codec::Encode;
use num_bigint::BigUint;
use rand::Rng;
use shp_traits::AsCompact;
use sp_core::H256;
use sp_runtime::traits::{BlakeTwo256, Keccak256};
use sp_trie::{
    recorder::Recorder, CompactProof, LayoutV1, MemoryDB, TrieDBBuilder, TrieDBMutBuilder,
    TrieLayout, TrieMut,
};
use storage_hub_traits::CommitmentVerifier;
use trie_db::{Hasher, Trie, TrieIterator};

use crate::{FileKeyProof, FileKeyVerifier};

/// The hash type of trie node keys
type HashT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;

const CHUNK_SIZE: u64 = 2;
const FILES_BASE_PATH: &str = "./tmp/";
const FILE_SIZE: u64 = 2u64.pow(11);
const SIZE_TO_CHALLENGES: u64 = FILE_SIZE / 10;

#[derive(PartialEq, Debug)]
struct FileMetadata {
    owner: Vec<u8>,
    location: Vec<u8>,
    size: u64,
    fingerprint: [u8; 32],
}

/// Build a Merkle Patricia Trie simulating a file split in chunks.
fn build_merkle_patricia_trie<T: TrieLayout>(
    random: bool,
    file_size: u64,
) -> (MemoryDB<T::Hash>, HashT<T>, FileMetadata)
where
    <T::Hash as sp_core::Hasher>::Out: for<'a> TryFrom<&'a [u8; 32]>,
{
    let user_id = b"user_id";
    let bucket = b"bucket";
    let file_name = if random {
        String::from("random_file") + &file_size.to_string()
    } else {
        String::from("large_file") + &file_size.to_string()
    };
    let file_name = file_name.as_bytes();

    println!("Chunking file into 32 byte chunks and building Merkle Patricia Trie...");

    let file_path = format!(
        "{}-{}-{}.txt",
        String::from_utf8(user_id.to_vec()).unwrap(),
        String::from_utf8(bucket.to_vec()).unwrap(),
        String::from_utf8(file_name.to_vec()).unwrap()
    );

    if random {
        create_random_test_file(&file_path, file_size)
    } else {
        create_test_file(&file_path, file_size)
    };
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
            &AsCompact(metadata.size).encode(),
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
    let mut buf = [0; CHUNK_SIZE as usize];
    let mut chunks = Vec::new();

    // create list of key-value pairs consisting of chunk metadata and chunk data
    let mut chunk_i = 0u64;
    loop {
        let n = file.read(&mut buf).unwrap();
        if n == 0 {
            break;
        }

        chunks.push((chunk_i, buf.to_vec()));
        chunk_i += 1;
    }

    let mut memdb = MemoryDB::<T::Hash>::default();
    let mut root = Default::default();
    {
        let mut t = TrieDBMutBuilder::<T>::new(&mut memdb, &mut root).build();
        for (k, v) in &chunks {
            t.insert(&AsCompact(*k).encode(), v).unwrap();
        }
    }

    println!("File fingerprint (root): {:?}", root);

    (memdb, root)
}

/// Create a local file for testing, for a given size, and with a given file name.
pub fn create_test_file(filename: &str, size: u64) -> File {
    create_dir_all(FILES_BASE_PATH).unwrap();
    let mut file = File::create(FILES_BASE_PATH.to_owned() + filename).unwrap();

    // Generate random content
    let mut i = 0;
    let content: Vec<u8> = (0..size)
        .map(|_| {
            i = i % (u8::MAX - 1);
            i += 1;
            i
        })
        .collect();

    file.write_all(&content).unwrap();

    file
}

/// Create a local file for testing, for a given size, with a given file name,
/// and with random content.
pub fn create_random_test_file(filename: &str, size: u64) -> File {
    create_dir_all(FILES_BASE_PATH).unwrap();
    let mut file = File::create(FILES_BASE_PATH.to_owned() + filename).unwrap();
    let mut rng = rand::thread_rng();

    // Generate random content
    let content: Vec<u8> = (0..size).map(|_| rng.gen()).collect();

    file.write_all(&content).unwrap();

    file
}

fn generate_challenges<T: TrieLayout>(
    challenges_count: u64,
    chunks_count: u64,
) -> (Vec<HashT<T>>, Vec<Vec<u8>>) {
    let mut challenges = Vec::new();
    let mut chunks_challenged = Vec::new();

    for i in 0..challenges_count {
        // Generate challenge as a hash.
        let hash_arg = "chunk".to_string() + i.to_string().as_str();
        let challenge = T::Hash::hash(hash_arg.as_bytes());
        challenges.push(challenge);

        // Calculate the modulo of the challenge with the number of chunks in the file.
        // The challenge is a big endian 32 byte array.
        let challenged_chunk = BigUint::from_bytes_be(challenge.as_ref()) % chunks_count;
        let challenged_chunk: u64 = challenged_chunk.try_into().expect(
            "This is impossible. The modulo of a number with a u64 should always fit in a u64.",
        );

        chunks_challenged.push(AsCompact(challenged_chunk).encode());
    }

    (challenges, chunks_challenged)
}

#[test]
fn generate_trie_works() {
    let (memdb, _file_key, metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);
    let root = metadata.fingerprint.try_into().unwrap();

    let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root).build();

    println!("Trie root: {:?}", trie.root());

    // Count the number of leaves in the trie.
    // This should be the same as the number of chunks in the file.
    let mut leaves_count = 0;
    let mut trie_iter = trie.iter().unwrap();
    while let Some(_) = trie_iter.next() {
        leaves_count += 1;
    }
    let mut chunks_count = FILE_SIZE / CHUNK_SIZE;
    if FILE_SIZE % CHUNK_SIZE != 0 {
        chunks_count += 1;
    }
    assert_eq!(leaves_count, chunks_count);

    println!("Number of leaves: {:?}", leaves_count);
}

#[test]
fn commitment_verifier_many_challenges_success() {
    let (memdb, file_key, metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);
    let root = metadata.fingerprint.try_into().unwrap();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let mut chunks_count = FILE_SIZE / CHUNK_SIZE;
    if FILE_SIZE % CHUNK_SIZE != 0 {
        chunks_count += 1;
    }
    let mut challenges_count = FILE_SIZE / SIZE_TO_CHALLENGES;
    if FILE_SIZE % SIZE_TO_CHALLENGES != 0 {
        challenges_count += 1;
    }
    let (mut challenges, chunks_challenged) =
        generate_challenges::<LayoutV1<BlakeTwo256>>(challenges_count, chunks_count);

    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.into_double_ended_iter().unwrap();

        for challenged_chunk in chunks_challenged {
            // Seek to the challenge key.
            iter.seek(&challenged_chunk).unwrap();

            // Read the leaf node.
            iter.next();
        }
    }

    // Generate proof
    let proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<BlakeTwo256>(root)
        .expect("Failed to create compact proof from recorder");
    let file_key_proof = FileKeyProof {
        owner: metadata.owner.clone(),
        location: metadata.location.clone(),
        size: metadata.size,
        fingerprint: metadata.fingerprint.clone(),
        proof,
    };

    // Verify proof
    let mut proven_challenges = FileKeyVerifier::<
        LayoutV1<BlakeTwo256>,
        { BlakeTwo256::LENGTH },
        { CHUNK_SIZE },
        { SIZE_TO_CHALLENGES },
    >::verify_proof(&file_key, &challenges, &file_key_proof)
    .expect("Failed to verify proof");

    assert_eq!(proven_challenges.sort(), challenges.sort());
}

#[test]
fn commitment_verifier_many_challenges_random_file_success() {
    let (memdb, file_key, metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(true, FILE_SIZE);
    let root = metadata.fingerprint.try_into().unwrap();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let mut chunks_count = FILE_SIZE / CHUNK_SIZE;
    if FILE_SIZE % CHUNK_SIZE != 0 {
        chunks_count += 1;
    }
    let mut challenges_count = FILE_SIZE / SIZE_TO_CHALLENGES;
    if FILE_SIZE % SIZE_TO_CHALLENGES != 0 {
        challenges_count += 1;
    }
    let (mut challenges, chunks_challenged) =
        generate_challenges::<LayoutV1<BlakeTwo256>>(challenges_count, chunks_count);

    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.into_double_ended_iter().unwrap();

        for challenged_chunk in chunks_challenged {
            // Seek to the challenge key.
            iter.seek(&challenged_chunk).unwrap();

            // Read the leaf node.
            iter.next();
        }
    }

    // Generate proof
    let proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<BlakeTwo256>(root)
        .expect("Failed to create compact proof from recorder");
    let file_key_proof = FileKeyProof {
        owner: metadata.owner.clone(),
        location: metadata.location.clone(),
        size: metadata.size,
        fingerprint: metadata.fingerprint.clone(),
        proof,
    };

    // Verify proof
    let mut proven_challenges = FileKeyVerifier::<
        LayoutV1<BlakeTwo256>,
        { BlakeTwo256::LENGTH },
        { CHUNK_SIZE },
        { SIZE_TO_CHALLENGES },
    >::verify_proof(&file_key, &challenges, &file_key_proof)
    .expect("Failed to verify proof");

    assert_eq!(proven_challenges.sort(), challenges.sort());
}

#[test]
fn commitment_verifier_many_challenges_keccak_success() {
    let (memdb, file_key, metadata) =
        build_merkle_patricia_trie::<LayoutV1<Keccak256>>(false, FILE_SIZE);
    let root = metadata.fingerprint.try_into().unwrap();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<Keccak256> = Recorder::default();

    let mut chunks_count = FILE_SIZE / CHUNK_SIZE;
    if FILE_SIZE % CHUNK_SIZE != 0 {
        chunks_count += 1;
    }
    let mut challenges_count = FILE_SIZE / SIZE_TO_CHALLENGES;
    if FILE_SIZE % SIZE_TO_CHALLENGES != 0 {
        challenges_count += 1;
    }
    let (mut challenges, chunks_challenged) =
        generate_challenges::<LayoutV1<Keccak256>>(challenges_count, chunks_count);

    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<Keccak256>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.into_double_ended_iter().unwrap();

        for challenged_chunk in chunks_challenged {
            // Seek to the challenge key.
            iter.seek(&challenged_chunk).unwrap();

            // Read the leaf node.
            iter.next();
        }
    }

    // Generate proof
    let proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<Keccak256>(root)
        .expect("Failed to create compact proof from recorder");
    let file_key_proof = FileKeyProof {
        owner: metadata.owner.clone(),
        location: metadata.location.clone(),
        size: metadata.size,
        fingerprint: metadata.fingerprint.clone(),
        proof,
    };

    // Verify proof
    let mut proven_challenges = FileKeyVerifier::<
        LayoutV1<Keccak256>,
        { Keccak256::LENGTH },
        { CHUNK_SIZE },
        { SIZE_TO_CHALLENGES },
    >::verify_proof(&file_key, &challenges, &file_key_proof)
    .expect("Failed to verify proof");

    assert_eq!(proven_challenges.sort(), challenges.sort());
}

#[test]
fn commitment_verifier_many_challenges_one_chunk_success() {
    let (memdb, file_key, metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, CHUNK_SIZE);
    let root = metadata.fingerprint.try_into().unwrap();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let file_size = metadata.size;
    let mut chunks_count = file_size / CHUNK_SIZE;
    if file_size % CHUNK_SIZE != 0 {
        chunks_count += 1;
    }
    let mut challenges_count = file_size / SIZE_TO_CHALLENGES;
    if file_size % SIZE_TO_CHALLENGES != 0 {
        challenges_count += 1;
    }
    let (mut challenges, chunks_challenged) =
        generate_challenges::<LayoutV1<BlakeTwo256>>(challenges_count, chunks_count);
    assert!(chunks_count == 1);

    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.into_double_ended_iter().unwrap();

        for challenged_chunk in chunks_challenged {
            // Seek to the challenge key.
            iter.seek(&challenged_chunk).unwrap();

            // Read the leaf node.
            iter.next();
        }
    }

    // Generate proof
    let proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<BlakeTwo256>(root)
        .expect("Failed to create compact proof from recorder");
    let file_key_proof = FileKeyProof {
        owner: metadata.owner.clone(),
        location: metadata.location.clone(),
        size: metadata.size,
        fingerprint: metadata.fingerprint.clone(),
        proof,
    };

    // Verify proof
    let mut proven_challenges = FileKeyVerifier::<
        LayoutV1<BlakeTwo256>,
        { BlakeTwo256::LENGTH },
        { CHUNK_SIZE },
        { SIZE_TO_CHALLENGES },
    >::verify_proof(&file_key, &challenges, &file_key_proof)
    .expect("Failed to verify proof");

    assert_eq!(proven_challenges.sort(), challenges.sort());
}

#[test]
fn commitment_verifier_many_challenges_two_chunks_success() {
    let (memdb, file_key, metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, CHUNK_SIZE + 1);
    let root = metadata.fingerprint.try_into().unwrap();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let file_size = metadata.size;
    let mut chunks_count = file_size / CHUNK_SIZE;
    if file_size % CHUNK_SIZE != 0 {
        chunks_count += 1;
    }
    let mut challenges_count = file_size / SIZE_TO_CHALLENGES;
    if file_size % SIZE_TO_CHALLENGES != 0 {
        challenges_count += 1;
    }
    let (mut challenges, chunks_challenged) =
        generate_challenges::<LayoutV1<BlakeTwo256>>(challenges_count, chunks_count);
    assert!(chunks_count == 2);

    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.into_double_ended_iter().unwrap();

        for challenged_chunk in chunks_challenged {
            // Seek to the challenge key.
            iter.seek(&challenged_chunk).unwrap();

            // Read the leaf node.
            iter.next();
        }
    }

    // Generate proof
    let proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<BlakeTwo256>(root)
        .expect("Failed to create compact proof from recorder");
    let file_key_proof = FileKeyProof {
        owner: metadata.owner.clone(),
        location: metadata.location.clone(),
        size: metadata.size,
        fingerprint: metadata.fingerprint.clone(),
        proof,
    };

    // Verify proof
    let mut proven_challenges = FileKeyVerifier::<
        LayoutV1<BlakeTwo256>,
        { BlakeTwo256::LENGTH },
        { CHUNK_SIZE },
        { SIZE_TO_CHALLENGES },
    >::verify_proof(&file_key, &challenges, &file_key_proof)
    .expect("Failed to verify proof");

    assert_eq!(proven_challenges.sort(), challenges.sort());
}

#[test]
fn commitment_verifier_no_challenges_failure() {
    let (memdb, file_key, metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);
    let root = metadata.fingerprint.try_into().unwrap();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let file_size = metadata.size;
    let mut chunks_count = file_size / CHUNK_SIZE;
    if file_size % CHUNK_SIZE != 0 {
        chunks_count += 1;
    }
    let (challenges, chunks_challenged) =
        generate_challenges::<LayoutV1<BlakeTwo256>>(0, chunks_count);

    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.into_double_ended_iter().unwrap();

        for challenged_chunk in chunks_challenged {
            // Seek to the challenge key.
            iter.seek(&challenged_chunk).unwrap();

            // Read the leaf node.
            iter.next();
        }
    }

    // Generate proof
    let proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<BlakeTwo256>(root)
        .expect("Failed to create compact proof from recorder");
    let file_key_proof = FileKeyProof {
        owner: metadata.owner.clone(),
        location: metadata.location.clone(),
        size: metadata.size,
        fingerprint: metadata.fingerprint.clone(),
        proof,
    };

    // Verify proof
    assert_eq!(
        FileKeyVerifier::<
            LayoutV1<BlakeTwo256>,
            { BlakeTwo256::LENGTH },
            { CHUNK_SIZE },
            { SIZE_TO_CHALLENGES },
        >::verify_proof(&file_key, &challenges, &file_key_proof),
        Err("No challenges provided.".into())
    );
}

#[test]
fn commitment_verifier_wrong_number_of_challenges_failure() {
    let (memdb, file_key, metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);
    let root = metadata.fingerprint.try_into().unwrap();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let mut chunks_count = FILE_SIZE / CHUNK_SIZE;
    if FILE_SIZE % CHUNK_SIZE != 0 {
        chunks_count += 1;
    }
    let mut challenges_count = FILE_SIZE / SIZE_TO_CHALLENGES;
    if FILE_SIZE % SIZE_TO_CHALLENGES != 0 {
        challenges_count += 1;
    }
    let (challenges, chunks_challenged) =
        generate_challenges::<LayoutV1<BlakeTwo256>>(challenges_count - 1, chunks_count);

    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.into_double_ended_iter().unwrap();

        for challenged_chunk in chunks_challenged {
            // Seek to the challenge key.
            iter.seek(&challenged_chunk).unwrap();

            // Read the leaf node.
            iter.next();
        }
    }

    // Generate proof
    let proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<BlakeTwo256>(root)
        .expect("Failed to create compact proof from recorder");
    let file_key_proof = FileKeyProof {
        owner: metadata.owner.clone(),
        location: metadata.location.clone(),
        size: metadata.size,
        fingerprint: metadata.fingerprint.clone(),
        proof,
    };

    // Verify proof
    assert_eq!(
        FileKeyVerifier::<
            LayoutV1<BlakeTwo256>,
            { BlakeTwo256::LENGTH },
            { CHUNK_SIZE },
            { SIZE_TO_CHALLENGES },
        >::verify_proof(&file_key, &challenges, &file_key_proof),
        Err("Number of challenges does not match the number of chunks that should have been challenged for a file of this size.".into())
    );
}

#[test]
fn commitment_verifier_wrong_file_key_failure() {
    let (memdb, _file_key, metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);
    let root = metadata.fingerprint.try_into().unwrap();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let mut chunks_count = FILE_SIZE / CHUNK_SIZE;
    if FILE_SIZE % CHUNK_SIZE != 0 {
        chunks_count += 1;
    }
    let mut challenges_count = FILE_SIZE / SIZE_TO_CHALLENGES;
    if FILE_SIZE % SIZE_TO_CHALLENGES != 0 {
        challenges_count += 1;
    }
    let (challenges, chunks_challenged) =
        generate_challenges::<LayoutV1<BlakeTwo256>>(challenges_count, chunks_count);

    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.into_double_ended_iter().unwrap();

        for challenged_chunk in chunks_challenged {
            // Seek to the challenge key.
            iter.seek(&challenged_chunk).unwrap();

            // Read the leaf node.
            iter.next();
        }
    }

    // Generate proof
    let proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<BlakeTwo256>(root)
        .expect("Failed to create compact proof from recorder");
    let file_key_proof = FileKeyProof {
        owner: metadata.owner.clone(),
        location: metadata.location.clone(),
        size: metadata.size,
        fingerprint: metadata.fingerprint.clone(),
        proof,
    };

    // Verify proof
    assert_eq!(
        FileKeyVerifier::<
            LayoutV1<BlakeTwo256>,
            { BlakeTwo256::LENGTH },
            { CHUNK_SIZE },
            { SIZE_TO_CHALLENGES },
        >::verify_proof(&Default::default(), &challenges, &file_key_proof),
        Err("File key provided should be equal to the file key constructed from the proof.".into())
    );
}

#[test]
fn commitment_verifier_wrong_file_key_encoding_as_bytes_failure() {
    let (memdb, _file_key, metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);
    let root = metadata.fingerprint.try_into().unwrap();

    let file_key = BlakeTwo256::hash(
        &[
            &metadata.owner,
            &metadata.location,
            &metadata.size.to_be_bytes().to_vec(),
            &metadata.fingerprint.to_vec(),
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<u8>>(),
    );

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let mut chunks_count = FILE_SIZE / CHUNK_SIZE;
    if FILE_SIZE % CHUNK_SIZE != 0 {
        chunks_count += 1;
    }
    let mut challenges_count = FILE_SIZE / SIZE_TO_CHALLENGES;
    if FILE_SIZE % SIZE_TO_CHALLENGES != 0 {
        challenges_count += 1;
    }
    let (challenges, chunks_challenged) =
        generate_challenges::<LayoutV1<BlakeTwo256>>(challenges_count, chunks_count);

    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.into_double_ended_iter().unwrap();

        for challenged_chunk in chunks_challenged {
            // Seek to the challenge key.
            iter.seek(&challenged_chunk).unwrap();

            // Read the leaf node.
            iter.next();
        }
    }

    // Generate proof
    let proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<BlakeTwo256>(root)
        .expect("Failed to create compact proof from recorder");
    let file_key_proof = FileKeyProof {
        owner: metadata.owner.clone(),
        location: metadata.location.clone(),
        size: metadata.size,
        fingerprint: metadata.fingerprint.clone(),
        proof,
    };

    // Verify proof
    assert_eq!(
        FileKeyVerifier::<
            LayoutV1<BlakeTwo256>,
            { BlakeTwo256::LENGTH },
            { CHUNK_SIZE },
            { SIZE_TO_CHALLENGES },
        >::verify_proof(&file_key, &challenges, &file_key_proof),
        Err("File key provided should be equal to the file key constructed from the proof.".into())
    );
}

#[test]
fn commitment_verifier_wrong_file_key_no_compact_encoding_failure() {
    let (memdb, _file_key, metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);
    let root = metadata.fingerprint.try_into().unwrap();

    let file_key = BlakeTwo256::hash(
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

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let mut chunks_count = FILE_SIZE / CHUNK_SIZE;
    if FILE_SIZE % CHUNK_SIZE != 0 {
        chunks_count += 1;
    }
    let mut challenges_count = FILE_SIZE / SIZE_TO_CHALLENGES;
    if FILE_SIZE % SIZE_TO_CHALLENGES != 0 {
        challenges_count += 1;
    }
    let (challenges, chunks_challenged) =
        generate_challenges::<LayoutV1<BlakeTwo256>>(challenges_count, chunks_count);

    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.into_double_ended_iter().unwrap();

        for challenged_chunk in chunks_challenged {
            // Seek to the challenge key.
            iter.seek(&challenged_chunk).unwrap();

            // Read the leaf node.
            iter.next();
        }
    }

    // Generate proof
    let proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<BlakeTwo256>(root)
        .expect("Failed to create compact proof from recorder");
    let file_key_proof = FileKeyProof {
        owner: metadata.owner.clone(),
        location: metadata.location.clone(),
        size: metadata.size,
        fingerprint: metadata.fingerprint.clone(),
        proof,
    };

    // Verify proof
    assert_eq!(
        FileKeyVerifier::<
            LayoutV1<BlakeTwo256>,
            { BlakeTwo256::LENGTH },
            { CHUNK_SIZE },
            { SIZE_TO_CHALLENGES },
        >::verify_proof(&file_key, &challenges, &file_key_proof),
        Err("File key provided should be equal to the file key constructed from the proof.".into())
    );
}

#[test]
fn commitment_verifier_wrong_file_key_vec_fingerprint_failure() {
    let (memdb, _file_key, metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);
    let root = metadata.fingerprint.try_into().unwrap();

    let file_key = BlakeTwo256::hash(
        &[
            &metadata.owner.encode(),
            &metadata.location.encode(),
            &AsCompact(metadata.size).encode(),
            &metadata.fingerprint.to_vec().encode(),
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<u8>>(),
    );

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let mut chunks_count = FILE_SIZE / CHUNK_SIZE;
    if FILE_SIZE % CHUNK_SIZE != 0 {
        chunks_count += 1;
    }
    let mut challenges_count = FILE_SIZE / SIZE_TO_CHALLENGES;
    if FILE_SIZE % SIZE_TO_CHALLENGES != 0 {
        challenges_count += 1;
    }
    let (challenges, chunks_challenged) =
        generate_challenges::<LayoutV1<BlakeTwo256>>(challenges_count, chunks_count);

    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.into_double_ended_iter().unwrap();

        for challenged_chunk in chunks_challenged {
            // Seek to the challenge key.
            iter.seek(&challenged_chunk).unwrap();

            // Read the leaf node.
            iter.next();
        }
    }

    // Generate proof
    let proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<BlakeTwo256>(root)
        .expect("Failed to create compact proof from recorder");
    let file_key_proof = FileKeyProof {
        owner: metadata.owner.clone(),
        location: metadata.location.clone(),
        size: metadata.size,
        fingerprint: metadata.fingerprint.clone(),
        proof,
    };

    // Verify proof
    assert_eq!(
        FileKeyVerifier::<
            LayoutV1<BlakeTwo256>,
            { BlakeTwo256::LENGTH },
            { CHUNK_SIZE },
            { SIZE_TO_CHALLENGES },
        >::verify_proof(&file_key, &challenges, &file_key_proof),
        Err("File key provided should be equal to the file key constructed from the proof.".into())
    );
}

#[test]
fn commitment_verifier_empty_proof_failure() {
    let (_memdb, _file_key, metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);

    let file_key = BlakeTwo256::hash(
        &[
            &metadata.owner.encode(),
            &metadata.location.encode(),
            &AsCompact(metadata.size).encode(),
            &metadata.fingerprint.encode(),
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<u8>>(),
    );

    let mut chunks_count = FILE_SIZE / CHUNK_SIZE;
    if FILE_SIZE % CHUNK_SIZE != 0 {
        chunks_count += 1;
    }
    let mut challenges_count = FILE_SIZE / SIZE_TO_CHALLENGES;
    if FILE_SIZE % SIZE_TO_CHALLENGES != 0 {
        challenges_count += 1;
    }
    let (challenges, _) =
        generate_challenges::<LayoutV1<BlakeTwo256>>(challenges_count, chunks_count);

    // Generate proof
    let proof = CompactProof {
        encoded_nodes: vec![],
    };
    let file_key_proof = FileKeyProof {
        owner: metadata.owner.clone(),
        location: metadata.location.clone(),
        size: metadata.size,
        fingerprint: metadata.fingerprint.clone(),
        proof,
    };

    // Verify proof
    assert_eq!(
        FileKeyVerifier::<
            LayoutV1<BlakeTwo256>,
            { BlakeTwo256::LENGTH },
            { CHUNK_SIZE },
            { SIZE_TO_CHALLENGES },
        >::verify_proof(&file_key, &challenges, &file_key_proof),
        Err("Failed to convert proof to memory DB, root doesn't match with expected.".into())
    );
}

#[test]
fn commitment_verifier_empty_fingerprint_failure() {
    let (_memdb, _file_key, metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);

    let fingerprint = H256::zero();

    let file_key = BlakeTwo256::hash(
        &[
            &metadata.owner.encode(),
            &metadata.location.encode(),
            &AsCompact(metadata.size).encode(),
            &fingerprint.encode(),
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<u8>>(),
    );

    let mut chunks_count = FILE_SIZE / CHUNK_SIZE;
    if FILE_SIZE % CHUNK_SIZE != 0 {
        chunks_count += 1;
    }
    let mut challenges_count = FILE_SIZE / SIZE_TO_CHALLENGES;
    if FILE_SIZE % SIZE_TO_CHALLENGES != 0 {
        challenges_count += 1;
    }
    let (challenges, _) =
        generate_challenges::<LayoutV1<BlakeTwo256>>(challenges_count, chunks_count);

    // Generate proof
    let proof = CompactProof {
        encoded_nodes: vec![],
    };
    let file_key_proof = FileKeyProof {
        owner: metadata.owner.clone(),
        location: metadata.location.clone(),
        size: metadata.size,
        fingerprint: fingerprint.clone().into(),
        proof,
    };

    // Verify proof
    assert_eq!(
        FileKeyVerifier::<
            LayoutV1<BlakeTwo256>,
            { BlakeTwo256::LENGTH },
            { CHUNK_SIZE },
            { SIZE_TO_CHALLENGES },
        >::verify_proof(&file_key, &challenges, &file_key_proof),
        Err("Failed to convert proof to memory DB, root doesn't match with expected.".into())
    );
}

#[test]
fn commitment_verifier_challenge_missing_from_proof_failure() {
    let (memdb, file_key, metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);
    let root = metadata.fingerprint.try_into().unwrap();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let mut chunks_count = FILE_SIZE / CHUNK_SIZE;
    if FILE_SIZE % CHUNK_SIZE != 0 {
        chunks_count += 1;
    }
    let mut challenges_count = FILE_SIZE / SIZE_TO_CHALLENGES;
    if FILE_SIZE % SIZE_TO_CHALLENGES != 0 {
        challenges_count += 1;
    }
    let (mut challenges, chunks_challenged) =
        generate_challenges::<LayoutV1<BlakeTwo256>>(challenges_count, chunks_count);

    {
        // Creating trie inside of closure to drop it before generating proof.
        let mut trie_recorder = recorder.as_trie_recorder(root);
        let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Create an iterator over the leaf nodes.
        let mut iter = trie.into_double_ended_iter().unwrap();

        for challenged_chunk in chunks_challenged {
            // Seek to the challenge key.
            iter.seek(&challenged_chunk).unwrap();

            // Read the leaf node.
            iter.next();
        }
    }

    // Generate proof
    let proof = recorder
        .drain_storage_proof()
        .to_compact_proof::<BlakeTwo256>(root)
        .expect("Failed to create compact proof from recorder");
    let file_key_proof = FileKeyProof {
        owner: metadata.owner.clone(),
        location: metadata.location.clone(),
        size: metadata.size,
        fingerprint: metadata.fingerprint.clone(),
        proof,
    };

    // Change one challenge so that the proof is invalid.
    challenges[0] = BlakeTwo256::hash("invalid_challenge".as_bytes());

    // Verify proof
    assert_eq!(
        FileKeyVerifier::<
            LayoutV1<BlakeTwo256>,
            { BlakeTwo256::LENGTH },
            { CHUNK_SIZE },
            { SIZE_TO_CHALLENGES },
        >::verify_proof(&file_key, &challenges, &file_key_proof),
        Err("The proof is invalid. The challenge does not exist in the trie.".into())
    );
}
