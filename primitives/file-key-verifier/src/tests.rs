use codec::Encode;
use num_bigint::BigUint;
use rand::Rng;
use shp_file_metadata::ChunkId;
use shp_file_metadata::FileMetadata;
use shp_file_metadata::Fingerprint;
use shp_traits::{AsCompact, CommitmentVerifier};
use sp_runtime::traits::{BlakeTwo256, Keccak256};
use sp_trie::{
    recorder::Recorder, CompactProof, LayoutV1, MemoryDB, TrieDBBuilder, TrieDBMutBuilder,
    TrieLayout, TrieMut,
};
use trie_db::{Hasher, Trie, TrieIterator};

use crate::{FileKeyProof, FileKeyVerifier};

/// The hash type of trie node keys
type HashT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;

const H_LENGTH: usize = 32;
const CHUNK_SIZE: u64 = 2;
const FILE_SIZE: u64 = 2u64.pow(11);
const SIZE_TO_CHALLENGES: u64 = FILE_SIZE / 10;

/// Build a Merkle Patricia Trie simulating a file split in chunks.
fn build_merkle_patricia_trie<T: TrieLayout>(
    random: bool,
    file_size: u64,
) -> (
    MemoryDB<T::Hash>,
    HashT<T>,
    FileMetadata<H_LENGTH, CHUNK_SIZE, SIZE_TO_CHALLENGES>,
)
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

    let data = if random {
        create_random_test_data(file_size)
    } else {
        create_sequential_test_data(file_size)
    };

    let (memdb, fingerprint) = merklise_data::<T>(&data);

    let file_path = format!(
        "{}-{}-{}.txt",
        String::from_utf8(user_id.to_vec()).unwrap(),
        String::from_utf8(bucket.to_vec()).unwrap(),
        String::from_utf8(file_name.to_vec()).unwrap()
    );

    let file_metadata = FileMetadata::new(
        user_id.to_vec(),
        bucket.to_vec(),
        file_path.as_bytes().to_vec(),
        file_size,
        fingerprint
            .as_ref()
            .try_into()
            .expect("slice with incorrect length"),
    )
    .expect("Failed to create file metadata");

    let file_key = file_metadata.file_key::<T::Hash>();

    (memdb, file_key, file_metadata)
}

/// Chunk data into [`CHUNK_SIZE`] byte chunks and store them in a Merkle Patricia Trie.
///
/// The trie is stored in a [`MemoryDB`] and the [`Root`] is returned.
pub fn merklise_data<T: TrieLayout>(data: &[u8]) -> (MemoryDB<T::Hash>, HashT<T>) {
    let mut buf = [0; CHUNK_SIZE as usize];
    let mut chunks = Vec::new();

    // create list of key-value pairs consisting of chunk metadata and chunk data
    let mut chunk_i = 0u64;
    let mut offset = 0;

    while offset < data.len() {
        // Determine the end of the current chunk
        let end = std::cmp::min(offset + CHUNK_SIZE as usize, data.len());

        // Copy the current chunk from data into the buffer
        buf[..end - offset].copy_from_slice(&data[offset..end]);

        // Store the current chunk as a vector in the chunks vector
        chunks.push((chunk_i, buf[..end - offset].to_vec()));

        // Increment the chunk index
        chunk_i += 1;

        // Move the offset forward by CHUNK_SIZE
        offset += CHUNK_SIZE as usize;
    }

    let mut memdb = MemoryDB::<T::Hash>::default();
    let mut root = Default::default();
    {
        let mut t = TrieDBMutBuilder::<T>::new(&mut memdb, &mut root).build();
        for (k, v) in &chunks {
            t.insert(&ChunkId::new(*k).as_trie_key(), v).unwrap();
        }
    }

    println!("Data fingerprint (root): {:?}", root);

    (memdb, root)
}

/// Generate sequential test data of a given size.
pub fn create_sequential_test_data(size: u64) -> Vec<u8> {
    let mut i = 0u8;
    let content: Vec<u8> = (0..size)
        .map(|_| {
            i = i % (u8::MAX - 1);
            i += 1;
            i
        })
        .collect();
    content
}

/// Generate random test data of a given size.
pub fn create_random_test_data(size: u64) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let content: Vec<u8> = (0..size).map(|_| rng.r#gen()).collect();
    content
}

fn generate_challenges<T: TrieLayout>(
    challenges_count: u32,
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
    let (memdb, _file_key, file_metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);
    let root = file_metadata.fingerprint().as_hash().into();

    let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&memdb, &root).build();

    println!("Trie root: {:?}", trie.root());

    // Count the number of leaves in the trie.
    // This should be the same as the number of chunks in the file.
    let mut leaves_count = 0;
    let mut trie_iter = trie.iter().unwrap();
    while let Some(_) = trie_iter.next() {
        leaves_count += 1;
    }

    let chunks_count = file_metadata.chunks_count();

    assert_eq!(leaves_count, chunks_count);

    println!("Number of leaves: {:?}", leaves_count);
}

#[test]
fn commitment_verifier_many_challenges_success() {
    let (memdb, file_key, file_metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);
    let root = file_metadata.fingerprint().as_hash().into();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let chunks_count = file_metadata.chunks_count();
    let challenges_count = file_metadata.chunks_to_check();

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
        file_metadata,
        proof,
    };

    // Verify proof
    let proven_challenges = FileKeyVerifier::<
        LayoutV1<BlakeTwo256>,
        { BlakeTwo256::LENGTH },
        { CHUNK_SIZE },
        { SIZE_TO_CHALLENGES },
    >::verify_proof(&file_key, &challenges, &file_key_proof)
    .expect("Failed to verify proof");

    assert_eq!(
        proven_challenges.into_iter().collect::<Vec<_>>().sort(),
        challenges.sort()
    );
}

#[test]
fn commitment_verifier_many_challenges_random_file_success() {
    let (memdb, file_key, file_metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(true, FILE_SIZE);
    let root = file_metadata.fingerprint().as_hash().into();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let chunks_count = file_metadata.chunks_count();
    let challenges_count = file_metadata.chunks_to_check();

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
        file_metadata,
        proof,
    };

    // Verify proof
    let proven_challenges = FileKeyVerifier::<
        LayoutV1<BlakeTwo256>,
        { BlakeTwo256::LENGTH },
        { CHUNK_SIZE },
        { SIZE_TO_CHALLENGES },
    >::verify_proof(&file_key, &challenges, &file_key_proof)
    .expect("Failed to verify proof");

    assert_eq!(
        proven_challenges.into_iter().collect::<Vec<_>>().sort(),
        challenges.sort()
    );
}

#[test]
fn commitment_verifier_many_challenges_keccak_success() {
    let (memdb, file_key, file_metadata) =
        build_merkle_patricia_trie::<LayoutV1<Keccak256>>(false, FILE_SIZE);
    let root = file_metadata.fingerprint().as_hash().into();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<Keccak256> = Recorder::default();

    let chunks_count = file_metadata.chunks_count();
    let challenges_count = file_metadata.chunks_to_check();

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
        file_metadata,
        proof,
    };

    // Verify proof
    let proven_challenges = FileKeyVerifier::<
        LayoutV1<Keccak256>,
        H_LENGTH,
        CHUNK_SIZE,
        SIZE_TO_CHALLENGES,
    >::verify_proof(&file_key, &challenges, &file_key_proof)
    .expect("Failed to verify proof");

    assert_eq!(
        proven_challenges.into_iter().collect::<Vec<_>>().sort(),
        challenges.sort()
    );
}

#[test]
fn commitment_verifier_many_challenges_one_chunk_success() {
    let (memdb, file_key, file_metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, CHUNK_SIZE);
    let root = file_metadata.fingerprint().as_hash().into();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let chunks_count = file_metadata.chunks_count();
    let challenges_count = file_metadata.chunks_to_check();

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
        file_metadata,
        proof,
    };

    // Verify proof
    let proven_challenges = FileKeyVerifier::<
        LayoutV1<BlakeTwo256>,
        H_LENGTH,
        CHUNK_SIZE,
        SIZE_TO_CHALLENGES,
    >::verify_proof(&file_key, &challenges, &file_key_proof)
    .expect("Failed to verify proof");

    assert_eq!(
        proven_challenges.into_iter().collect::<Vec<_>>().sort(),
        challenges.sort()
    );
}

#[test]
fn commitment_verifier_many_challenges_two_chunks_success() {
    let (memdb, file_key, file_metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, CHUNK_SIZE + 1);
    let root = file_metadata.fingerprint().as_hash().into();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let chunks_count = file_metadata.chunks_count();
    let challenges_count = file_metadata.chunks_to_check();

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
        file_metadata,
        proof,
    };

    // Verify proof
    let proven_challenges = FileKeyVerifier::<
        LayoutV1<BlakeTwo256>,
        H_LENGTH,
        CHUNK_SIZE,
        SIZE_TO_CHALLENGES,
    >::verify_proof(&file_key, &challenges, &file_key_proof)
    .expect("Failed to verify proof");

    assert_eq!(
        proven_challenges.into_iter().collect::<Vec<_>>().sort(),
        challenges.sort()
    );
}

#[test]
fn commitment_verifier_no_challenges_failure() {
    let (memdb, file_key, file_metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);
    let root = file_metadata.fingerprint().as_hash().into();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let chunks_count = file_metadata.chunks_count();
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
        file_metadata,
        proof,
    };

    // Verify proof
    assert_eq!(
        FileKeyVerifier::<
            LayoutV1<BlakeTwo256>,
            H_LENGTH,
            CHUNK_SIZE,
            SIZE_TO_CHALLENGES,
        >::verify_proof(&file_key, &challenges, &file_key_proof),
        Err("No challenges provided.".into())
    );
}

#[test]
fn commitment_verifier_wrong_number_of_challenges_failure() {
    let (memdb, file_key, file_metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);
    let root = file_metadata.fingerprint().as_hash().into();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let chunks_count = file_metadata.chunks_count();
    let challenges_count = file_metadata.chunks_to_check();

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
        file_metadata,
        proof,
    };

    // Verify proof
    assert_eq!(
        FileKeyVerifier::<
            LayoutV1<BlakeTwo256>,
            H_LENGTH,
            CHUNK_SIZE,
            SIZE_TO_CHALLENGES,
        >::verify_proof(&file_key, &challenges, &file_key_proof),
        Err("Number of challenges does not match the number of chunks that should have been challenged for a file of this size.".into())
    );
}

#[test]
fn commitment_verifier_wrong_file_key_failure() {
    let (memdb, _file_key, file_metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);
    let root = file_metadata.fingerprint().as_hash().into();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let chunks_count = file_metadata.chunks_count();
    let challenges_count = file_metadata.chunks_to_check();

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
        file_metadata,
        proof,
    };

    // Verify proof
    assert_eq!(
        FileKeyVerifier::<
            LayoutV1<BlakeTwo256>,
            H_LENGTH,
            CHUNK_SIZE,
            SIZE_TO_CHALLENGES,
        >::verify_proof(&Default::default(), &challenges, &file_key_proof),
        Err("File key provided should be equal to the file key constructed from the proof.".into())
    );
}

#[test]
fn commitment_verifier_wrong_file_key_no_compact_encoding_failure() {
    let (memdb, _file_key, file_metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);
    let root = file_metadata.fingerprint().as_hash().into();

    let file_key = BlakeTwo256::hash(
        &[
            &file_metadata.owner().encode(),
            &file_metadata.location().encode(),
            &file_metadata.file_size().encode(),
            &file_metadata.fingerprint().encode(),
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<u8>>(),
    );

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let chunks_count = file_metadata.chunks_count();
    let challenges_count = file_metadata.chunks_to_check();

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
        file_metadata,
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
    let (memdb, _file_key, file_metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);
    let root = file_metadata.fingerprint().as_hash().into();

    let file_key = BlakeTwo256::hash(
        &[
            &file_metadata.owner().encode(),
            &file_metadata.location().encode(),
            &AsCompact(file_metadata.file_size()).encode(),
            &file_metadata.fingerprint().as_hash().to_vec().encode(),
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<u8>>(),
    );

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let chunks_count = file_metadata.chunks_count();
    let challenges_count = file_metadata.chunks_to_check();
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
        file_metadata,
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
fn commitment_verifier_wrong_file_key_encoding_as_bytes_failure() {
    let (memdb, _file_key, file_metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);
    let root = file_metadata.fingerprint().as_hash().into();

    let file_key = BlakeTwo256::hash(
        &[
            &file_metadata.owner(),
            &file_metadata.location(),
            &file_metadata.file_size().to_be_bytes().to_vec(),
            &file_metadata.fingerprint().as_hash().to_vec(),
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<u8>>(),
    );

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let chunks_count = file_metadata.chunks_count();
    let challenges_count = file_metadata.chunks_to_check();

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
        file_metadata,
        proof,
    };

    // Verify proof
    assert_eq!(
        FileKeyVerifier::<
            LayoutV1<BlakeTwo256>,
            H_LENGTH,
            CHUNK_SIZE,
            SIZE_TO_CHALLENGES,
        >::verify_proof(&file_key, &challenges, &file_key_proof),
        Err("File key provided should be equal to the file key constructed from the proof.".into())
    );
}

#[test]
fn commitment_verifier_empty_proof_failure() {
    let (_memdb, _file_key, file_metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);

    let file_key = file_metadata.file_key::<BlakeTwo256>();

    let chunks_count = file_metadata.chunks_count();
    let challenges_count = file_metadata.chunks_to_check();

    let (challenges, _) =
        generate_challenges::<LayoutV1<BlakeTwo256>>(challenges_count, chunks_count);

    // Generate proof
    let proof = CompactProof {
        encoded_nodes: vec![],
    };
    let file_key_proof = FileKeyProof {
        file_metadata,
        proof,
    };

    // Verify proof
    assert_eq!(
        FileKeyVerifier::<
            LayoutV1<BlakeTwo256>,
            H_LENGTH,
            CHUNK_SIZE,
            SIZE_TO_CHALLENGES,
        >::verify_proof(&file_key, &challenges, &file_key_proof),
        Err("Failed to convert proof to memory DB, root doesn't match with expected.".into())
    );
}

#[test]
fn commitment_verifier_empty_fingerprint_failure() {
    let (_memdb, _file_key, file_metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);

    let file_metadata = FileMetadata::new(
        file_metadata.owner().clone(),
        file_metadata.bucket_id().clone(),
        file_metadata.location().clone(),
        file_metadata.file_size(),
        Fingerprint::default(),
    )
    .unwrap();

    let file_key = file_metadata.file_key::<BlakeTwo256>();

    let chunks_count = file_metadata.chunks_count();
    let challenges_count = file_metadata.chunks_to_check();

    let (challenges, _) =
        generate_challenges::<LayoutV1<BlakeTwo256>>(challenges_count, chunks_count);

    // Generate proof
    let proof = CompactProof {
        encoded_nodes: vec![],
    };

    let file_key_proof = FileKeyProof {
        file_metadata,
        proof,
    };

    // Verify proof
    assert_eq!(
        FileKeyVerifier::<
            LayoutV1<BlakeTwo256>,
            H_LENGTH,
            CHUNK_SIZE,
            SIZE_TO_CHALLENGES,
        >::verify_proof(&file_key, &challenges, &file_key_proof),
        Err("Failed to convert proof to memory DB, root doesn't match with expected.".into())
    );
}

#[test]
fn commitment_verifier_challenge_missing_from_proof_failure() {
    let (memdb, file_key, file_metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, FILE_SIZE);
    let root = file_metadata.fingerprint().as_hash().into();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let chunks_count = file_metadata.chunks_count();
    let challenges_count = file_metadata.chunks_to_check();

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
        file_metadata,
        proof,
    };

    // Change one challenge so that the proof is invalid.
    challenges[0] = BlakeTwo256::hash("invalid_challenge".as_bytes());

    // Verify proof
    assert_eq!(
        FileKeyVerifier::<
            LayoutV1<BlakeTwo256>,
            H_LENGTH,
            CHUNK_SIZE,
            SIZE_TO_CHALLENGES,
        >::verify_proof(&file_key, &challenges, &file_key_proof),
        Err("The proof is invalid. The challenge does not exist in the trie.".into())
    );
}

#[test]
fn commitment_verifier_challenge_with_none_value_failure() {
    let (memdb, _file_key, file_metadata) =
        build_merkle_patricia_trie::<LayoutV1<BlakeTwo256>>(false, 2 * CHUNK_SIZE);
    let root = file_metadata.fingerprint().as_hash().into();

    let file_metadata = FileMetadata::new(
        file_metadata.owner().clone(),
        file_metadata.bucket_id().clone(),
        file_metadata.location().clone(),
        FILE_SIZE,
        file_metadata.fingerprint().clone(),
    )
    .unwrap();

    let file_key = file_metadata.file_key::<BlakeTwo256>();

    // This recorder is used to record accessed keys in the trie and later generate a proof for them.
    let recorder: Recorder<BlakeTwo256> = Recorder::default();

    let chunks_count = file_metadata.chunks_count();
    let challenges_count = file_metadata.chunks_to_check();

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
    // Using wrong file size (larger than it actually is)
    let file_key_proof = FileKeyProof {
        file_metadata,
        proof,
    };

    // Verify proof
    assert_eq!(
        FileKeyVerifier::<
            LayoutV1<BlakeTwo256>,
            H_LENGTH,
            CHUNK_SIZE,
            SIZE_TO_CHALLENGES,
        >::verify_proof(&file_key, &challenges, &file_key_proof),
        Err("The proof is invalid. The challenged chunk was not found in the trie, possibly because the challenged chunk has an index higher than the amount of chunks in the file. This should not be possible, provided that the size of the file (and therefore number of chunks) is correct.".into())
    );
}

#[test]
fn chunk_id_convert_to_and_from_trie_key() {
    let chunk_id = ChunkId::new(0x12345678u64);
    let chunk_id_bytes = chunk_id.as_trie_key();
    let chunk_id_decoded = ChunkId::from_trie_key(&chunk_id_bytes).unwrap();
    assert_eq!(chunk_id, chunk_id_decoded);
}
