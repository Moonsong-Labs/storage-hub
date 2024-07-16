use shc_common::types::{FileMetadata, Fingerprint, HashT, HasherOutT};

use codec::Encode;
use sp_trie::MemoryDB;
use trie_db::{TrieDBMutBuilder, TrieLayout, TrieMut};

/// Build a Merkle Patricia Forest Trie.
///
/// The trie is built from the ground up, by each file into 32 byte chunks and storing them in a trie.
/// Each trie is then inserted into a new merkle patricia trie, which comprises the merkle forest.
pub fn build_merkle_patricia_forest<T: TrieLayout>(
) -> (MemoryDB<T::Hash>, HasherOutT<T>, Vec<HasherOutT<T>>) {
    let user_ids = vec![
        b"01", b"02", b"03", b"04", b"05", b"06", b"07", b"08", b"09", b"10", b"11", b"12", b"13",
        b"12", b"13", b"14", b"15", b"16", b"17", b"18", b"19", b"20", b"21", b"22", b"23", b"24",
        b"25", b"26", b"27", b"28", b"29", b"30", b"31", b"32",
    ];
    let bucket = b"bucket";
    let file_name = b"sample64b";

    let mut file_leaves: Vec<(HasherOutT<T>, Vec<u8>)> = Vec::new();

    for user_id in user_ids {
        let mut file_path = Vec::new();
        file_path.append(&mut user_id.to_vec());
        file_path.append(&mut bucket.to_vec());
        file_path.append(&mut file_name.to_vec());

        let metadata = FileMetadata {
            owner: "owner".as_bytes().to_vec(),
            bucket_id: bucket.to_vec(),
            location: file_path,
            file_size: 0,
            fingerprint: Fingerprint::default(),
        };

        file_leaves.push((metadata.file_key::<HashT<T>>(), metadata.encode()));
    }
    // Construct the Merkle Patricia Forest
    let mut memdb = MemoryDB::<T::Hash>::default();
    let mut root = Default::default();

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
    }
    (memdb, root, file_keys)
}
