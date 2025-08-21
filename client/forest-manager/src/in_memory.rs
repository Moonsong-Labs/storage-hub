use codec::{Decode, Encode};
use hash_db::Hasher;
use shc_common::{
    traits::StorageEnableRuntime,
    types::{FileMetadata, HasherOutT},
};
use sp_trie::{recorder::Recorder, MemoryDB, TrieDBBuilder, TrieLayout, TrieMut};
use trie_db::{Trie, TrieDBMutBuilder};

use shc_common::types::ForestProof;

use crate::{
    error::{ErrorT, ForestStorageError},
    prove::prove,
    traits::ForestStorage,
};

pub struct InMemoryForestStorage<T: TrieLayout + 'static> {
    pub root: HasherOutT<T>,
    pub memdb: MemoryDB<T::Hash>,
}

impl<T: TrieLayout> InMemoryForestStorage<T> {
    pub fn new() -> Self {
        let (memdb, root) = MemoryDB::default_with_root();

        Self { root, memdb }
    }
}

impl<T: TrieLayout> Clone for InMemoryForestStorage<T> {
    fn clone(&self) -> Self {
        Self {
            root: self.root,
            memdb: self.memdb.clone(),
        }
    }
}

impl<T: TrieLayout, Runtime: StorageEnableRuntime> ForestStorage<T, Runtime>
    for InMemoryForestStorage<T>
where
    <T::Hash as Hasher>::Out: TryFrom<[u8; 32]>,
{
    fn root(&self) -> HasherOutT<T> {
        self.root
    }

    fn contains_file_key(&self, file_key: &HasherOutT<T>) -> Result<bool, ErrorT<T>> {
        let trie = TrieDBBuilder::<T>::new(&self.memdb, &self.root).build();
        Ok(trie.contains(file_key.as_ref())?)
    }

    fn get_file_metadata(
        &self,
        file_key: &HasherOutT<T>,
    ) -> Result<Option<FileMetadata>, ErrorT<T>> {
        let trie = TrieDBBuilder::<T>::new(&self.memdb, &self.root).build();
        let encoded_metadata = trie.get(file_key.as_ref())?;
        match encoded_metadata {
            Some(data) => {
                let decoded_metadata = FileMetadata::decode(&mut &data[..])?;
                Ok(Some(decoded_metadata))
            }
            None => Ok(None),
        }
    }

    fn generate_proof(
        &self,
        challenged_file_keys: Vec<HasherOutT<T>>,
    ) -> Result<ForestProof<T>, ErrorT<T>> {
        let recorder: Recorder<T::Hash> = Recorder::default();

        // A `TrieRecorder` is needed to create a proof of the "visited" leafs, by the end of this process.
        let mut trie_recorder = recorder.as_trie_recorder(self.root);

        let trie = TrieDBBuilder::<T>::new(&self.memdb, &self.root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Get the proven leaves or leaf
        let proven = challenged_file_keys
            .iter()
            .map(|file_key| prove::<T>(&trie, file_key))
            .collect::<Result<Vec<_>, _>>()?;

        // Drop the `trie_recorder` to release the `self` and `recorder`
        drop(trie_recorder);

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<T::Hash>(self.root)?;

        Ok(ForestProof {
            proven,
            proof,
            root: self.root,
        })
    }

    fn insert_files_metadata(
        &mut self,
        files_metadata: &[FileMetadata],
    ) -> Result<Vec<HasherOutT<T>>, ErrorT<T>> {
        if files_metadata.is_empty() {
            return Ok(Vec::new());
        }

        // First collect all file keys and check for existence
        let mut file_keys = Vec::with_capacity(files_metadata.len());
        let trie_read = TrieDBBuilder::<T>::new(&self.memdb, &self.root).build();

        for metadata in files_metadata {
            let file_key = metadata.file_key::<T::Hash>();
            if trie_read.contains(file_key.as_ref())? {
                return Err(ForestStorageError::FileKeyAlreadyExists(file_key).into());
            }
            file_keys.push(file_key);
        }

        let mut trie =
            TrieDBMutBuilder::<T>::from_existing(&mut self.memdb, &mut self.root).build();

        for (i, file_metadata) in files_metadata.iter().enumerate() {
            let file_key = &file_keys[i];
            trie.insert(file_key.as_ref(), file_metadata.encode().as_slice())
                .map_err(|_| ForestStorageError::FailedToInsertFileKey(*file_key))?;
        }

        Ok(file_keys)
    }

    fn delete_file_key(&mut self, file_key: &HasherOutT<T>) -> Result<(), ErrorT<T>> {
        let mut trie =
            TrieDBMutBuilder::<T>::from_existing(&mut self.memdb, &mut self.root).build();

        // Remove the file key from the trie.
        trie.remove(file_key.as_ref())?;

        Ok(())
    }

    fn get_files_by_user(
        &self,
        user: &Runtime::AccountId,
    ) -> Result<Vec<(HasherOutT<T>, FileMetadata)>, ErrorT<T>> {
        let trie = TrieDBBuilder::<T>::new(&self.memdb, &self.root).build();
        let mut files = Vec::new();
        let mut trie_iter = trie
            .iter()
            .map_err(|_| ForestStorageError::FailedToCreateTrieIterator)?;

        let encoded_user = user.encode();

        while let Some((_, value)) = trie_iter.next().transpose()? {
            let metadata = FileMetadata::decode(&mut &value[..])?;
            let file_key = metadata.file_key::<T::Hash>();
            if metadata.owner() == &encoded_user {
                files.push((file_key, metadata));
            }
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use core::cmp::min;
    use shc_common::types::{Fingerprint, Proven, StorageProofsMerkleTrieLayout};
    use sp_core::H256;

    #[test]
    fn test_initialization_with_no_existing_root() {
        let forest_storage = InMemoryForestStorage::<StorageProofsMerkleTrieLayout>::new();
        let expected_hash = HasherOutT::<StorageProofsMerkleTrieLayout>::try_from([
            3, 23, 10, 46, 117, 151, 183, 183, 227, 216, 76, 5, 57, 29, 19, 154, 98, 177, 87, 231,
            135, 134, 216, 192, 130, 242, 157, 207, 76, 17, 19, 20,
        ])
        .unwrap();

        let root =
            ForestStorage::<StorageProofsMerkleTrieLayout, sh_runtime_parachain::Runtime>::root(
                &forest_storage,
            );
        assert_eq!(root, expected_hash);
    }

    #[test]
    fn test_write_read() {
        let mut forest_storage = InMemoryForestStorage::<StorageProofsMerkleTrieLayout>::new();

        let file_metadata = FileMetadata::new(
            "Alice".as_bytes().to_vec(),
            "bucket".as_bytes().to_vec(),
            "location".as_bytes().to_vec(),
            100,
            Fingerprint::default(),
        )
        .unwrap();

        let file_key = ForestStorage::<StorageProofsMerkleTrieLayout, sh_runtime_parachain::Runtime>::insert_files_metadata(
            &mut forest_storage,
            &[file_metadata],
        )
        .unwrap();

        assert!(ForestStorage::<
            StorageProofsMerkleTrieLayout,
            sh_runtime_parachain::Runtime
        >::contains_file_key(
            &forest_storage,
            &file_key.first().unwrap()
        )
        .unwrap());
    }

    #[test]
    fn test_remove_existing_file_key() {
        let mut forest_storage = InMemoryForestStorage::<StorageProofsMerkleTrieLayout>::new();

        let file_metadata = FileMetadata::new(
            "Alice".as_bytes().to_vec(),
            "bucket".as_bytes().to_vec(),
            "location".as_bytes().to_vec(),
            100,
            Fingerprint::default(),
        )
        .unwrap();

        let file_key = ForestStorage::<
            StorageProofsMerkleTrieLayout,
            sh_runtime_parachain::Runtime
        >::insert_files_metadata(
            &mut forest_storage,
            &[file_metadata],
        )
        .unwrap();

        let file_key = file_key.first().unwrap();

        assert!(ForestStorage::<
            StorageProofsMerkleTrieLayout,
            sh_runtime_parachain::Runtime
        >::delete_file_key(&mut forest_storage, &file_key)
            .is_ok()
        );
        assert!(!ForestStorage::<
            StorageProofsMerkleTrieLayout,
            sh_runtime_parachain::Runtime
        >::contains_file_key(&forest_storage, &file_key)
            .unwrap()
        );
    }

    #[test]
    fn test_remove_non_existent_file_key() {
        let mut forest_storage = InMemoryForestStorage::<StorageProofsMerkleTrieLayout>::new();
        assert!(ForestStorage::<
            StorageProofsMerkleTrieLayout,
            sh_runtime_parachain::Runtime
        >::delete_file_key(&mut forest_storage, &[0u8; 32].into())
            .is_ok()
        );
    }

    #[test]
    fn test_get_file_metadata() {
        let mut forest_storage = InMemoryForestStorage::<StorageProofsMerkleTrieLayout>::new();

        let mut keys = Vec::new();
        for i in 1..=50 {
            let file_metadata = FileMetadata::new(
                "Alice".as_bytes().to_vec(),
                "bucket".as_bytes().to_vec(),
                "location".as_bytes().to_vec(),
                i,
                Fingerprint::default(),
            )
            .unwrap();

            let file_key = ForestStorage::<
                StorageProofsMerkleTrieLayout,
                sh_runtime_parachain::Runtime,
            >::insert_files_metadata(
                &mut forest_storage, &[file_metadata]
            )
            .unwrap();

            keys.push(*file_key.first().unwrap());
        }

        let file_metadata = ForestStorage::<
            StorageProofsMerkleTrieLayout,
            sh_runtime_parachain::Runtime,
        >::get_file_metadata(&forest_storage, &keys[0])
        .unwrap()
        .unwrap();
        assert_eq!(file_metadata.file_size(), 1);
        assert_eq!(file_metadata.bucket_id(), "bucket".as_bytes());
        assert_eq!(file_metadata.location(), "location".as_bytes());
        assert_eq!(file_metadata.owner(), "Alice".as_bytes());
        assert_eq!(file_metadata.fingerprint(), &Fingerprint::default());
    }

    #[test]
    fn test_generate_proof_exact_key() {
        let mut forest_storage = InMemoryForestStorage::<StorageProofsMerkleTrieLayout>::new();

        let mut keys = Vec::new();
        for i in 1..=50 {
            let file_metadata = FileMetadata::new(
                "Alice".as_bytes().to_vec(),
                "bucket".as_bytes().to_vec(),
                "location".as_bytes().to_vec(),
                i,
                Fingerprint::default(),
            )
            .unwrap();

            let file_key = ForestStorage::<
                StorageProofsMerkleTrieLayout,
                sh_runtime_parachain::Runtime,
            >::insert_files_metadata(
                &mut forest_storage, &[file_metadata]
            )
            .unwrap();

            keys.push(*file_key.first().unwrap());
        }

        let challenge = keys[0];

        let proof = ForestStorage::<
            StorageProofsMerkleTrieLayout,
            sh_runtime_parachain::Runtime,
        >::generate_proof(&forest_storage, vec![challenge])
            .unwrap();

        assert_eq!(proof.proven.len(), 1);
        assert!(
            matches!(proof.proven.first().expect("Proven leaves should have proven 1 challenge"), Proven::ExactKey(leaf) if leaf.key.as_ref() == challenge.as_bytes())
        );
    }

    #[test]
    fn test_generate_proof_neighbor_keys() {
        let mut forest_storage = InMemoryForestStorage::<StorageProofsMerkleTrieLayout>::new();
        let mut keys = Vec::new();
        for i in 1..=50 {
            let file_metadata = FileMetadata::new(
                "Alice".as_bytes().to_vec(),
                "bucket".as_bytes().to_vec(),
                "location".as_bytes().to_vec(),
                i,
                Fingerprint::default(),
            )
            .unwrap();

            let file_key = ForestStorage::<
                StorageProofsMerkleTrieLayout,
                sh_runtime_parachain::Runtime,
            >::insert_files_metadata(
                &mut forest_storage, &[file_metadata]
            )
            .unwrap();

            keys.push(*file_key.first().unwrap());
        }

        let memdb = forest_storage.memdb.clone();
        let root = forest_storage.root;
        let trie = TrieDBBuilder::<StorageProofsMerkleTrieLayout>::new(&memdb, &root).build();

        let mut iter = trie.iter().unwrap();
        let first_key = iter.next().unwrap().unwrap().0;
        let second_key = iter.next().unwrap().unwrap().0;

        // increment last byte by 1
        let challenge = first_key[0..31]
            .iter()
            .chain(std::iter::once(&(first_key[31] + 1)))
            .copied()
            .collect::<Vec<u8>>();
        let challenge_hash = H256::from_slice(&challenge);

        let proof = ForestStorage::<
            StorageProofsMerkleTrieLayout,
            sh_runtime_parachain::Runtime,
        >::generate_proof(&forest_storage, vec![challenge_hash])
            .unwrap();

        assert_eq!(proof.proven.len(), 1);
        assert!(
            matches!(proof.proven.first().expect("Proven leaves should have proven 1 challenge"), Proven::NeighbourKeys((Some(left_leaf), Some(right_leaf))) if left_leaf.key.as_ref() == first_key && right_leaf.key.as_ref() == second_key)
        );
    }

    #[test]
    fn test_generate_proof_challenge_before_first_leaf() {
        let mut forest_storage = InMemoryForestStorage::<StorageProofsMerkleTrieLayout>::new();

        let file_metadata_one = FileMetadata::new(
            "Alice".as_bytes().to_vec(),
            "bucket".as_bytes().to_vec(),
            "location".as_bytes().to_vec(),
            10,
            Fingerprint::default(),
        )
        .unwrap();

        let file_metadata_two = FileMetadata::new(
            "Alice".as_bytes().to_vec(),
            "bucket".as_bytes().to_vec(),
            "location".as_bytes().to_vec(),
            11,
            Fingerprint::default(),
        )
        .unwrap();

        let file_keys = ForestStorage::<
            StorageProofsMerkleTrieLayout,
            sh_runtime_parachain::Runtime,
        >::insert_files_metadata(
            &mut forest_storage,
            &[file_metadata_one, file_metadata_two],
        )
        .unwrap();

        let smallest_key_challenge = min(file_keys[0], file_keys[1]);
        let mut challenge_bytes: H256 = smallest_key_challenge;
        let challenge_bytes = challenge_bytes.as_mut();
        challenge_bytes[31] = challenge_bytes[31] - 1;

        let challenge = H256::from_slice(challenge_bytes);

        let proof = ForestStorage::<
            StorageProofsMerkleTrieLayout,
            sh_runtime_parachain::Runtime,
        >::generate_proof(&forest_storage, vec![challenge])
            .unwrap();

        let proven = proof
            .proven
            .first()
            .expect("Proven leaves should have proven 1 challenge");

        assert_eq!(proof.proven.len(), 1);
        assert!(
            matches!(proven, Proven::NeighbourKeys((None, Some(leaf))) if leaf.key.as_ref() == smallest_key_challenge.as_bytes())
        );
    }

    #[test]
    fn test_generate_proof_challenge_after_last_leaf() {
        let mut forest_storage = InMemoryForestStorage::<StorageProofsMerkleTrieLayout>::new();

        let mut keys = Vec::new();
        for i in 1..=50 {
            let file_metadata = FileMetadata::new(
                "Alice".as_bytes().to_vec(),
                "bucket".as_bytes().to_vec(),
                "location".as_bytes().to_vec(),
                i,
                Fingerprint::default(),
            )
            .unwrap();

            let file_key = ForestStorage::<
                StorageProofsMerkleTrieLayout,
                sh_runtime_parachain::Runtime,
            >::insert_files_metadata(
                &mut forest_storage, &[file_metadata]
            )
            .unwrap();

            keys.push(*file_key.first().unwrap());
        }

        let largest = keys.into_iter().max().unwrap();
        let mut challenge = largest;
        let challenge_bytes = challenge.as_mut();
        challenge_bytes[0] = challenge_bytes[0] + 1;

        let proof = ForestStorage::<
            StorageProofsMerkleTrieLayout,
            sh_runtime_parachain::Runtime,
        >::generate_proof(&forest_storage, vec![challenge])
            .unwrap();

        assert_eq!(proof.proven.len(), 1);
        assert!(
            matches!(proof.proven.first().expect("Proven leaves should have proven 1 challenge"), Proven::NeighbourKeys((Some(leaf), None)) if leaf.key.as_ref() == largest.as_bytes())
        );
    }

    #[test]
    fn test_trie_with_over_16_consecutive_leaves() {
        let mut forest_storage = InMemoryForestStorage::<StorageProofsMerkleTrieLayout>::new();

        let mut keys = Vec::new();
        for i in 1..=50 {
            let file_metadata = FileMetadata::new(
                "Alice".as_bytes().to_vec(),
                "bucket".as_bytes().to_vec(),
                "location".as_bytes().to_vec(),
                i,
                Fingerprint::default(),
            )
            .unwrap();

            let file_key = ForestStorage::<
                StorageProofsMerkleTrieLayout,
                sh_runtime_parachain::Runtime,
            >::insert_files_metadata(
                &mut forest_storage, &[file_metadata]
            )
            .unwrap();

            keys.push(*file_key.first().unwrap());
        }

        // Remove specific keys
        let keys_to_remove = keys
            .iter()
            .enumerate()
            .filter(|(i, _)| i % 2 == 0)
            .map(|(_, key)| *key)
            .collect::<Vec<_>>();

        for key in &keys_to_remove {
            assert!(ForestStorage::<
                StorageProofsMerkleTrieLayout,
                sh_runtime_parachain::Runtime
            >::delete_file_key(&mut forest_storage, &key)
                .is_ok()
            );
        }

        // Test that the keys are removed
        for key in keys_to_remove {
            assert!(!ForestStorage::<
                StorageProofsMerkleTrieLayout,
                sh_runtime_parachain::Runtime
            >::contains_file_key(&forest_storage, &key)
                .unwrap()
            );
        }
    }
}
