use hash_db::{AsHashDB, HashDB, Prefix};
use kvdb::{DBTransaction, KeyValueDB};
use log::debug;
use shc_common::types::{FileMetadata, ForestProof, HashT, HasherOutT};
use sp_state_machine::{warn, Storage};
use sp_trie::{
    prefixed_key, recorder::Recorder, PrefixedMemoryDB, TrieDBBuilder, TrieLayout, TrieMut,
};
use std::{io, path::PathBuf, sync::Arc};
use trie_db::{DBValue, Trie, TrieDBMutBuilder};

use crate::{
    error::{ErrorT, ForestStorageError},
    prove::prove,
    traits::ForestStorage,
    utils::convert_raw_bytes_to_hasher_out,
    LOG_TARGET,
};

mod well_known_keys {
    pub const ROOT: &[u8] = b":root";
}

pub(crate) fn other_io_error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

/// Open the database on disk, creating it if it doesn't exist.
fn open_or_creating_rocksdb(db_path: String) -> io::Result<kvdb_rocksdb::Database> {
    let mut path = PathBuf::new();
    path.push(db_path.as_str());
    path.push("storagehub/forest_storage/");

    let db_config = kvdb_rocksdb::DatabaseConfig::with_columns(1);

    let path_str = path
        .to_str()
        .ok_or_else(|| other_io_error(format!("Bad database path: {:?}", path)))?;

    std::fs::create_dir_all(&path_str)?;
    let db = kvdb_rocksdb::Database::open(&db_config, &path_str)?;

    Ok(db)
}

/// Storage backend for RocksDB.
pub struct StorageDb<T, DB> {
    pub db: Arc<DB>,
    pub _phantom: std::marker::PhantomData<T>,
}

impl<T, DB> StorageDb<T, DB>
where
    T: TrieLayout,
    DB: KeyValueDB,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    fn write(&mut self, transaction: DBTransaction) -> Result<(), ErrorT<T>> {
        self.db.write(transaction).map_err(|e| {
            warn!(target: LOG_TARGET, "Failed to write to DB: {}", e);
            ForestStorageError::FailedToWriteToStorage.into()
        })
    }

    fn storage_root(&self) -> Result<Option<HasherOutT<T>>, ErrorT<T>> {
        let maybe_root = self.db.get(0, well_known_keys::ROOT).map_err(|e| {
            warn!(target: LOG_TARGET, "Failed to read root from DB: {}", e);
            ForestStorageError::FailedToReadStorage
        })?;

        let root = maybe_root
            .map(|root| convert_raw_bytes_to_hasher_out::<T>(root))
            .transpose()?;

        Ok(root)
    }
}

impl<T, DB> Storage<HashT<T>> for StorageDb<T, DB>
where
    T: TrieLayout + Send + Sync,
    DB: KeyValueDB,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    fn get(&self, key: &HasherOutT<T>, prefix: Prefix) -> Result<Option<DBValue>, String> {
        let prefixed_key = prefixed_key::<HashT<T>>(key, prefix);
        self.db.get(0, &prefixed_key).map_err(|e| {
            warn!(target: LOG_TARGET, "Failed to read from DB: {}", e);
            format!("Failed to read from DB: {}", e)
        })
    }
}

/// RocksDB based [`ForestStorage`] implementation.
pub struct RocksDBForestStorage<T, DB>
where
    T: TrieLayout + 'static,
    DB: KeyValueDB + 'static,
{
    /// RocksDB storage backend.
    storage: StorageDb<T, DB>,
    /// In-memory overlay of the trie with changes not yet committed to the backend.
    ///
    /// Once all operations are done, the overlay will be committed to the storage by executing [`RocksDBForestStorage::commit`].
    overlay: PrefixedMemoryDB<HashT<T>>,
    root: HasherOutT<T>,
}

impl<T, DB> RocksDBForestStorage<T, DB>
where
    T: TrieLayout + Send + Sync,
    DB: KeyValueDB,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    /// This will open the RocksDB database and read the storage [`ROOT`](`well_known_keys::ROOT`) from it.
    /// If the root hash is not found in storage, a new trie will be created and the root hash will be stored in storage.
    pub fn new(storage: StorageDb<T, DB>) -> Result<Self, ErrorT<T>> {
        let maybe_root = storage.storage_root()?;

        let rocksdb_forest_storage = match maybe_root {
            Some(root) => {
                debug!(target: LOG_TARGET, "Found existing root in storage: {:?}\n Reusing trie", root);

                RocksDBForestStorage::<T, DB> {
                    storage,
                    overlay: Default::default(),
                    root,
                }
            }
            None => {
                debug!(target: LOG_TARGET, "No root found in storage, creating a new trie");

                let mut root = HasherOutT::<T>::default();

                let mut rocksdb_forest_storage = RocksDBForestStorage::<T, DB> {
                    storage,
                    overlay: Default::default(),
                    root,
                };

                // Create a new trie
                let trie =
                    TrieDBMutBuilder::<T>::new(rocksdb_forest_storage.as_hash_db_mut(), &mut root)
                        .build();

                // Drop the `trie` to free `rocksdb_forest_storage` and `root`.
                drop(trie);

                let mut transaction = DBTransaction::new();
                transaction.put(0, well_known_keys::ROOT, root.as_ref());

                // Add the root hash to storage at well-known key ROOT
                rocksdb_forest_storage.storage.write(transaction)?;

                rocksdb_forest_storage.root = root;

                debug!(target: LOG_TARGET, "New storage root: {:?}", rocksdb_forest_storage.root);

                rocksdb_forest_storage
            }
        };

        Ok(rocksdb_forest_storage)
    }

    /// Open the RocksDB database at `db_path` and return a new instance of [`StorageDb`].
    pub fn rocksdb_storage(
        db_path: String,
    ) -> Result<StorageDb<T, kvdb_rocksdb::Database>, ErrorT<T>> {
        let db = open_or_creating_rocksdb(db_path).map_err(|e| {
            warn!(target: LOG_TARGET, "Failed to open RocksDB: {}", e);
            ForestStorageError::FailedToReadStorage
        })?;

        Ok(StorageDb {
            db: Arc::new(db),
            _phantom: Default::default(),
        })
    }

    /// Commit [`overlay`](`RocksDBForestStorage::overlay`) to [`storage`](`RocksDBForestStorage::storage`)
    ///
    /// This will write the changes applied to the overlay, including the [`root`](`RocksDBForestStorage::root`). If the root has not changed, the commit will be skipped.
    /// The `overlay` will be cleared.
    pub fn commit(&mut self) -> Result<(), ErrorT<T>> {
        let root = &self
            .storage
            .storage_root()?
            .ok_or(ForestStorageError::ExpectingRootToBeInStorage)?;

        // Skip commit if the root has not changed.
        if &self.root == root {
            warn!(target: LOG_TARGET, "Root has not changed, skipping commit");
            return Ok(());
        }

        // Aggregate changes from the overlay
        let mut transaction = self.changes();

        // Update the root
        transaction.put(0, well_known_keys::ROOT, self.root.as_ref());

        // Write the changes to storage
        self.storage.write(transaction)?;

        debug!(target: LOG_TARGET, "Committed changes to storage, new root: {:?}", self.root);

        Ok(())
    }

    /// Build [`DBTransaction`] from the overlay and clear it.
    fn changes(&mut self) -> DBTransaction {
        let mut transaction = DBTransaction::new();

        for (key, (value, rc)) in self.overlay.drain() {
            if rc <= 0 {
                transaction.delete(0, &key);
            } else {
                transaction.put_vec(0, &key, value);
            }
        }

        transaction
    }
}

impl<T, DB> AsHashDB<HashT<T>, DBValue> for RocksDBForestStorage<T, DB>
where
    T: TrieLayout + Send + Sync,
    DB: KeyValueDB,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    fn as_hash_db<'b>(&'b self) -> &'b (dyn HashDB<HashT<T>, DBValue> + 'b) {
        self
    }
    fn as_hash_db_mut<'b>(&'b mut self) -> &'b mut (dyn HashDB<HashT<T>, DBValue> + 'b) {
        self
    }
}

impl<T, DB> hash_db::HashDB<HashT<T>, DBValue> for RocksDBForestStorage<T, DB>
where
    T: TrieLayout + Send + Sync,
    DB: KeyValueDB,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    fn get(&self, key: &HasherOutT<T>, prefix: Prefix) -> Option<DBValue> {
        HashDB::get(&self.overlay, key, prefix).or_else(|| {
            self.storage.get(key, prefix).unwrap_or_else(|e| {
                warn!(target: LOG_TARGET, "Failed to read from DB: {}", e);
                None
            })
        })
    }

    fn contains(&self, key: &HasherOutT<T>, prefix: Prefix) -> bool {
        HashDB::get(self, key, prefix).is_some()
    }

    fn insert(&mut self, prefix: Prefix, value: &[u8]) -> HasherOutT<T> {
        HashDB::insert(&mut self.overlay, prefix, value)
    }

    fn emplace(&mut self, key: HasherOutT<T>, prefix: Prefix, value: DBValue) {
        HashDB::emplace(&mut self.overlay, key, prefix, value)
    }

    fn remove(&mut self, key: &HasherOutT<T>, prefix: Prefix) {
        HashDB::remove(&mut self.overlay, key, prefix)
    }
}

impl<T, DB> ForestStorage<T> for RocksDBForestStorage<T, DB>
where
    T: TrieLayout + Send + Sync + 'static,
    DB: KeyValueDB + 'static,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    fn root(&self) -> HasherOutT<T> {
        self.root
    }

    fn contains_file_key(&self, file_key: &HasherOutT<T>) -> Result<bool, ErrorT<T>> {
        let db = self.as_hash_db();
        let trie = TrieDBBuilder::<T>::new(&db, &self.root).build();
        Ok(trie.contains(file_key.as_ref())?)
    }

    fn generate_proof(
        &self,
        challenged_file_keys: Vec<HasherOutT<T>>,
    ) -> Result<ForestProof<T>, ErrorT<T>> {
        let recorder: Recorder<T::Hash> = Recorder::default();

        // A `TrieRecorder` is needed to create a proof of the "visited" leafs, by the end of this process.
        let mut trie_recorder = recorder.as_trie_recorder(self.root);

        let db = self.as_hash_db();
        let trie = TrieDBBuilder::<T>::new(&db, &self.root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Get the proven leaves or leaf
        let proven = challenged_file_keys
            .iter()
            .map(|file_key| prove::<_>(&trie, file_key))
            .collect::<Result<Vec<_>, _>>()?;

        // Drop the `trie_recorder` to release the `self` and `recorder`
        drop(trie_recorder);

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<T::Hash>(self.root)
            .map_err(|_| ForestStorageError::FailedToGenerateCompactProof)?;

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
        let mut file_keys = Vec::with_capacity(files_metadata.len());

        // Pre-check for existing keys
        for metadata in files_metadata {
            let file_key = metadata.file_key::<T::Hash>();
            file_keys.push(file_key);
        }

        // Check if any of the new keys already exist in the trie
        for file_key in &file_keys {
            if self.contains_file_key(file_key)? {
                return Err(ForestStorageError::FileKeyAlreadyExists(*file_key).into());
            }
        }

        let mut root = self.root;

        let mut trie =
            TrieDBMutBuilder::<T>::from_existing(self.as_hash_db_mut(), &mut root).build();

        for file_key in &file_keys {
            trie.insert(file_key.as_ref(), b"")
                .map_err(|_| ForestStorageError::FailedToInsertFileKey(*file_key))?;
        }

        // Drop trie to free `self`.
        drop(trie);

        // Update the root and commit changes
        self.root = root;
        self.commit()?;

        Ok(file_keys)
    }

    fn delete_file_key(&mut self, file_key: &HasherOutT<T>) -> Result<(), ErrorT<T>> {
        let mut root = self.root;
        let mut trie =
            TrieDBMutBuilder::<T>::from_existing(self.as_hash_db_mut(), &mut root).build();

        // Remove the file key from the trie.
        let _ = trie.remove(file_key.as_ref())?;

        // Drop trie to free `self`.
        drop(trie);

        // Update the root hash.
        self.root = root;

        // Commit the changes to disk.
        self.commit()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::min;

    use crate::error::ErrorT;

    use super::*;
    use kvdb_memorydb::InMemory;
    use shc_common::types::{FileMetadata, Fingerprint, Proven};
    use sp_core::H256;
    use sp_runtime::traits::BlakeTwo256;
    use sp_trie::LayoutV1;
    use trie_db::Trie;

    // Reusable function to setup a new `StorageDb` and `RocksDBForestStorage`.
    fn setup_storage<T, DB>() -> Result<RocksDBForestStorage<T, InMemory>, ErrorT<T>>
    where
        T: TrieLayout + Send + Sync,
        DB: KeyValueDB,
        HasherOutT<T>: TryFrom<[u8; 32]>,
    {
        let storage = StorageDb {
            db: Arc::new(kvdb_memorydb::create(1)),
            _phantom: Default::default(),
        };
        RocksDBForestStorage::<T, InMemory>::new(storage)
    }

    #[test]
    fn test_initialization_with_no_existing_root() {
        let forest_storage = setup_storage::<LayoutV1<BlakeTwo256>, InMemory>().unwrap();
        let expected_hash = HasherOutT::<LayoutV1<BlakeTwo256>>::try_from([
            3, 23, 10, 46, 117, 151, 183, 183, 227, 216, 76, 5, 57, 29, 19, 154, 98, 177, 87, 231,
            135, 134, 216, 192, 130, 242, 157, 207, 76, 17, 19, 20,
        ])
        .unwrap();

        assert_eq!(
            forest_storage.root, expected_hash,
            "Root should be initialized to default on no existing ROOT key."
        );
    }

    #[test]
    fn test_write_read() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>, InMemory>().unwrap();

        let file_metadata = FileMetadata {
            bucket_id: "bucket".as_bytes().to_vec(),
            location: "location".as_bytes().to_vec(),
            owner: "Alice".as_bytes().to_vec(),
            file_size: 100,
            fingerprint: Fingerprint::default(),
        };

        let file_key = forest_storage
            .insert_files_metadata(&[file_metadata])
            .unwrap();

        assert!(forest_storage
            .contains_file_key(&file_key.first().unwrap())
            .unwrap());
    }

    #[test]
    fn test_remove_existing_file_key() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>, InMemory>().unwrap();

        let file_metadata = FileMetadata {
            bucket_id: "bucket".as_bytes().to_vec(),
            location: "location".as_bytes().to_vec(),
            owner: "Alice".as_bytes().to_vec(),
            file_size: 100,
            fingerprint: Fingerprint::default(),
        };

        let file_key = forest_storage
            .insert_files_metadata(&[file_metadata])
            .unwrap();

        let file_key = file_key.first().unwrap();

        assert!(forest_storage.delete_file_key(&file_key).is_ok());
        assert!(!forest_storage.contains_file_key(&file_key).unwrap());
    }

    #[test]
    fn test_remove_non_existent_file_key() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>, InMemory>().unwrap();
        assert!(forest_storage.delete_file_key(&[0u8; 32].into()).is_ok());
    }

    #[test]
    fn test_generate_proof_exact_key() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>, InMemory>().unwrap();

        let mut keys = Vec::new();
        for i in 0..50 {
            let file_metadata = FileMetadata {
                bucket_id: "bucket".as_bytes().to_vec(),
                location: "location".as_bytes().to_vec(),
                owner: "Alice".as_bytes().to_vec(),
                file_size: i,
                fingerprint: Fingerprint::default(),
            };

            let file_key = forest_storage
                .insert_files_metadata(&[file_metadata])
                .unwrap();

            keys.push(*file_key.first().unwrap());
        }

        let challenge = keys[0];

        let proof = forest_storage.generate_proof(vec![challenge]).unwrap();

        assert_eq!(proof.proven.len(), 1);
        assert!(
            matches!(proof.proven.first().expect("Proven leaves should have proven 1 challenge"), Proven::ExactKey(leaf) if leaf.key.as_ref() == challenge.as_bytes())
        );
    }

    #[test]
    fn test_generate_proof_neighbor_keys() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>, InMemory>().unwrap();

        let mut keys = Vec::new();
        for i in 0..50 {
            let file_metadata = FileMetadata {
                bucket_id: "bucket".as_bytes().to_vec(),
                location: "location".as_bytes().to_vec(),
                owner: "Alice".as_bytes().to_vec(),
                file_size: i,
                fingerprint: Fingerprint::default(),
            };

            let file_key = forest_storage
                .insert_files_metadata(&[file_metadata])
                .unwrap();

            keys.push(*file_key.first().unwrap());
        }

        let hash_db = forest_storage.as_hash_db();
        let root = forest_storage.root;
        let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&hash_db, &root).build();

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

        let proof = forest_storage.generate_proof(vec![challenge_hash]).unwrap();

        assert_eq!(proof.proven.len(), 1);
        assert!(
            matches!(proof.proven.first().expect("Proven leaves should have proven 1 challenge"), Proven::NeighbourKeys((Some(left_leaf), Some(right_leaf))) if left_leaf.key.as_ref() == first_key && right_leaf.key.as_ref() == second_key)
        );
    }

    #[test]
    fn test_generate_proof_challenge_before_first_leaf() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>, InMemory>().unwrap();

        let file_metadata_one = FileMetadata {
            bucket_id: "bucket".as_bytes().to_vec(),
            location: "location".as_bytes().to_vec(),
            owner: "Alice".as_bytes().to_vec(),
            file_size: 10,
            fingerprint: Fingerprint::default(),
        };

        let file_metadata_two = FileMetadata {
            bucket_id: "bucket".as_bytes().to_vec(),
            location: "location".as_bytes().to_vec(),
            owner: "Alice".as_bytes().to_vec(),
            file_size: 11,
            fingerprint: Fingerprint::default(),
        };

        let file_keys = forest_storage
            .insert_files_metadata(&[file_metadata_one, file_metadata_two])
            .unwrap();

        let smallest_key_challenge = min(file_keys[0], file_keys[1]);
        let mut challenge_bytes: H256 = smallest_key_challenge;
        let challenge_bytes = challenge_bytes.as_mut();
        challenge_bytes[31] = challenge_bytes[31] - 1;

        let challenge = H256::from_slice(challenge_bytes);

        let proof = forest_storage.generate_proof(vec![challenge]).unwrap();

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
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>, InMemory>().unwrap();

        let mut keys = Vec::new();
        for i in 0..50 {
            let file_metadata = FileMetadata {
                bucket_id: "bucket".as_bytes().to_vec(),
                location: "location".as_bytes().to_vec(),
                owner: "Alice".as_bytes().to_vec(),
                file_size: i,
                fingerprint: Fingerprint::default(),
            };

            let file_key = forest_storage
                .insert_files_metadata(&[file_metadata])
                .unwrap();

            keys.push(*file_key.first().unwrap());
        }

        let largest = keys.into_iter().max().unwrap();
        let mut challenge = largest;
        let challenge_bytes = challenge.as_mut();
        challenge_bytes[0] = challenge_bytes[0] + 1;

        let proof = forest_storage.generate_proof(vec![challenge]).unwrap();

        assert_eq!(proof.proven.len(), 1);
        assert!(
            matches!(proof.proven.first().expect("Proven leaves should have proven 1 challenge"), Proven::NeighbourKeys((Some(leaf), None)) if leaf.key.as_ref() == largest.as_bytes())
        );
    }

    #[test]
    fn test_trie_with_over_16_consecutive_leaves() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>, InMemory>().unwrap();

        let mut keys = Vec::new();
        for i in 0..50 {
            let file_metadata = FileMetadata {
                bucket_id: "bucket".as_bytes().to_vec(),
                location: "location".as_bytes().to_vec(),
                owner: "Alice".as_bytes().to_vec(),
                file_size: i,
                fingerprint: Fingerprint::default(),
            };

            let file_key = forest_storage
                .insert_files_metadata(&[file_metadata])
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
            assert!(forest_storage.delete_file_key(&key).is_ok());
        }

        // Test that the keys are removed
        for key in keys_to_remove {
            assert!(!forest_storage.contains_file_key(&key).unwrap());
        }
    }
}
