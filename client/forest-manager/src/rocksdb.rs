use std::{io, path::PathBuf, sync::Arc};

use hash_db::{AsHashDB, HashDB, Prefix};
use kvdb::{DBTransaction, KeyValueDB};
use kvdb_rocksdb::{Database, DatabaseConfig};
use log::debug;
use sp_state_machine::{warn, Storage};
use sp_trie::{
    prefixed_key, recorder::Recorder, PrefixedMemoryDB, Trie, TrieDBBuilder, TrieLayout, TrieMut,
};
use storage_hub_infra::types::{ForestProof, Metadata};
use trie_db::{DBValue, Hasher, TrieDBMutBuilder};

use crate::{
    prove::prove,
    traits::ForestStorage,
    types::{ForestStorageErrors, HashT, HasherOutT, RawKey},
    utils::{deserialize_value, serialize_value},
};

const LOG_TARGET: &str = "forest_storage";

mod well_known_keys {
    pub const ROOT: &[u8] = b":root";
}

pub(crate) fn other_io_error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

/// Open the database on disk, creating it if it doesn't exist.
fn open_creating_rocksdb(db_path: String) -> io::Result<Database> {
    let root = PathBuf::from("/tmp/");
    let path = root.join("storagehub").join(db_path);

    let db_config = DatabaseConfig::with_columns(1);

    let path_str = path
        .to_str()
        .ok_or_else(|| other_io_error(format!("Bad database path: {:?}", path)))?;

    std::fs::create_dir_all(&path_str)?;
    let db = Database::open(&db_config, &path_str)?;

    Ok(db)
}

/// Storage backend for RocksDB.
pub struct StorageDb<Hasher> {
    pub db: Arc<dyn KeyValueDB>,
    pub _phantom: std::marker::PhantomData<Hasher>,
}

impl<H: Hasher> Storage<H> for StorageDb<H> {
    fn get(&self, key: &H::Out, prefix: Prefix) -> Result<Option<DBValue>, String> {
        let prefixed_key = prefixed_key::<H>(key, prefix);
        self.db.get(0, &prefixed_key).map_err(|e| {
            warn!(target: LOG_TARGET, "Failed to read from DB: {}", e);
            format!("Failed to read from DB: {}", e)
        })
    }
}

/// Backend storage used by the [`RocksDBForestStorage`].
pub trait Backend<T: TrieLayout>: Storage<HashT<T>>
where
    <T::Hash as Hasher>::Out: TryFrom<Vec<u8>>,
{
    /// Write the transaction to the storage.
    fn write(&mut self, transaction: DBTransaction) -> Result<(), ForestStorageErrors>;
    /// Get the [`ROOT`](`well_known_keys::ROOT`) from storage.
    fn storage_root(&self) -> Result<Option<HasherOutT<T>>, ForestStorageErrors>;
}

impl<T: TrieLayout> Backend<T> for StorageDb<HashT<T>>
where
    <T::Hash as Hasher>::Out: TryFrom<Vec<u8>>,
{
    fn write(&mut self, transaction: DBTransaction) -> Result<(), ForestStorageErrors> {
        self.db.write(transaction).map_err(|e| {
            warn!(target: LOG_TARGET, "Failed to write to DB: {}", e);
            ForestStorageErrors::FailedToWriteToStorage
        })
    }

    fn storage_root(&self) -> Result<Option<HasherOutT<T>>, ForestStorageErrors> {
        let maybe_root = self.db.get(0, well_known_keys::ROOT).map_err(|e| {
            warn!(target: LOG_TARGET, "Failed to read root from DB: {}", e);
            ForestStorageErrors::FailedToReadStorage
        })?;

        let root = maybe_root
            .map(|root| {
                HasherOutT::<T>::try_from(root).map_err(|_| {
                    warn!(target: LOG_TARGET, "Failed to parse root from DB");
                    ForestStorageErrors::FailedToParseRoot
                })
            })
            .transpose()?;

        Ok(root)
    }
}

/// Forest storage backend using RocksDB.
pub struct RocksDBForestStorage<T: TrieLayout> {
    /// RocksDB storage backend.
    storage: Box<dyn Backend<T>>,
    /// In-memory overlay of the trie with changes not yet committed to the backend.
    ///
    /// Once all operations are done, the overlay will be committed to the storage by executing [`RocksDBForestStorage::commit`].
    overlay: PrefixedMemoryDB<HashT<T>>,
    root: HasherOutT<T>,
}

impl<T: TrieLayout + Send + Sync> RocksDBForestStorage<T>
where
    <T::Hash as Hasher>::Out: TryFrom<Vec<u8>>,
{
    /// This will open the RocksDB database and read the storage [`ROOT`](`well_known_keys::ROOT`) from it.
    /// If the root hash is not found in storage, a new trie will be created and the root hash will be stored in storage.
    pub fn new(storage: Box<dyn Backend<T>>) -> Result<Self, ForestStorageErrors> {
        let maybe_root = storage.storage_root()?;

        let rocksdb_forest_storage = match maybe_root {
            Some(root) => {
                debug!(target: LOG_TARGET, "Found existing root in storage: {:?}\n Reusing trie", root);

                RocksDBForestStorage::<T> {
                    storage,
                    overlay: Default::default(),
                    root,
                }
            }
            None => {
                debug!(target: LOG_TARGET, "No root found in storage, creating a new trie");

                let mut root = HasherOutT::<T>::default();

                let mut rocksdb_forest_storage = RocksDBForestStorage::<T> {
                    storage,
                    overlay: Default::default(),
                    root,
                };

                // Create a new trie
                let trie =
                    TrieDBMutBuilder::<T>::new(rocksdb_forest_storage.as_hash_db_mut(), &mut root)
                        .build();

                // Drop the `trie` to free `rockdb_forest_storage` and `root`.
                drop(trie);

                let mut transaction = DBTransaction::new();
                transaction.put(0, well_known_keys::ROOT, root.as_ref());

                // Add the root hash to storage at well-known key ROOT
                rocksdb_forest_storage
                    .storage
                    .write(transaction)
                    .expect("Failed to write to RocksDB");

                rocksdb_forest_storage.root = root;

                debug!(target: LOG_TARGET, "New storage root: {:?}", rocksdb_forest_storage.root);

                rocksdb_forest_storage
            }
        };

        Ok(rocksdb_forest_storage)
    }

    /// Open the RocksDB database at `dp_path` and return a new instance of [`StorageDb`].
    pub fn rocksdb_storage(dp_path: String) -> Result<StorageDb<HashT<T>>, ForestStorageErrors> {
        let db = open_creating_rocksdb(dp_path).map_err(|e| {
            warn!(target: LOG_TARGET, "Failed to open RocksDB: {}", e);
            ForestStorageErrors::FailedToReadStorage
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
    pub fn commit(&mut self) -> Result<(), ForestStorageErrors> {
        let root = &self
            .storage
            .storage_root()?
            .ok_or(ForestStorageErrors::ExpectingRootToBeInStorage)?;

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
        self.storage
            .write(transaction)
            .expect("Failed to write to RocksDB");

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

impl<T: TrieLayout + Send + Sync> AsHashDB<HashT<T>, DBValue> for RocksDBForestStorage<T> {
    fn as_hash_db<'b>(&'b self) -> &'b (dyn HashDB<HashT<T>, DBValue> + 'b) {
        self
    }
    fn as_hash_db_mut<'b>(&'b mut self) -> &'b mut (dyn HashDB<HashT<T>, DBValue> + 'b) {
        self
    }
}

impl<T: TrieLayout + Send + Sync> hash_db::HashDB<HashT<T>, DBValue> for RocksDBForestStorage<T> {
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

impl<T: TrieLayout + Send + Sync> ForestStorage for RocksDBForestStorage<T>
where
    <T::Hash as Hasher>::Out: TryFrom<Vec<u8>>,
{
    type LookupKey = HasherOutT<T>;
    type RawKey = RawKey<T>;
    type Value = Metadata;

    fn get_file_key(
        &self,
        file_key: &Self::LookupKey,
    ) -> Result<Option<Self::Value>, ForestStorageErrors> {
        let db = self.as_hash_db();
        let trie = TrieDBBuilder::<T>::new(&db, &self.root).build();

        let maybe_metadata = trie.get(file_key.as_ref()).map_err(|e| {
            warn!(target: LOG_TARGET, "Failed to get file key: {:?}", e);
            ForestStorageErrors::FailedToGetFileKey
        })?;

        let maybe_metadata = maybe_metadata
            .map(|raw_metadata| deserialize_value(&raw_metadata))
            .transpose()?;

        Ok(maybe_metadata)
    }

    fn generate_proof(
        &self,
        challenged_file_keys: &Vec<Self::LookupKey>,
    ) -> Result<ForestProof<Self::RawKey>, ForestStorageErrors> {
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
            .map(|file_key| prove::<_, Self>(&trie, file_key))
            .collect::<Result<Vec<_>, _>>()?;

        // Drop the `trie_recorder` to release the `self` and `recorder`
        drop(trie_recorder);

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<T::Hash>(self.root)
            .map_err(|_| ForestStorageErrors::FailedToGenerateCompactProof)?;

        Ok(ForestProof {
            proven,
            proof,
            root: self.root.as_ref().to_vec().into(),
        })
    }

    fn insert_file_key(
        &mut self,
        raw_file_key: &Self::RawKey,
        metadata: &Self::Value,
    ) -> Result<Self::LookupKey, ForestStorageErrors> {
        let file_key = <T::Hash as Hasher>::hash(&raw_file_key.key);

        if self.get_file_key(&file_key)?.is_some() {
            return Err(ForestStorageErrors::FileKeyAlreadyExists);
        }

        let mut root = self.root.clone();
        let mut trie =
            TrieDBMutBuilder::<T>::from_existing(self.as_hash_db_mut(), &mut root).build();

        // Serialize the metadata.
        let raw_metadata = serialize_value(metadata)?;

        // Insert the file key and metadata into the trie.
        trie.insert(file_key.as_ref(), &raw_metadata)
            .map_err(|_| ForestStorageErrors::FailedToInsertFileKey)?;

        // Drop trie to free `self`.
        drop(trie);

        // Update the root hash.
        self.root = root;

        // Commit the changes to disk.
        self.commit()?;

        Ok(file_key)
    }

    fn delete_file_key(&mut self, file_key: &Self::LookupKey) -> Result<(), ForestStorageErrors> {
        let mut root = self.root.clone();
        let mut trie =
            TrieDBMutBuilder::<T>::from_existing(self.as_hash_db_mut(), &mut root).build();

        // Remove the file key from the trie.
        let _ = trie
            .remove(file_key.as_ref())
            .map_err(|_| ForestStorageErrors::FailedToRemoveFileKey)?;

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
    use reference_trie::RefHasher;
    use sp_trie::LayoutV1;
    use storage_hub_infra::types::Proven;

    use super::*;

    /// Mock that simulates the backend for testing purposes.
    struct MockStorageDb {
        pub data: std::collections::HashMap<Vec<u8>, Vec<u8>>,
    }

    impl<H: Hasher> Storage<H> for MockStorageDb {
        fn get(&self, key: &H::Out, prefix: Prefix) -> Result<Option<DBValue>, String> {
            let prefixed_key = prefixed_key::<H>(key, prefix);
            Ok(self.data.get(&prefixed_key).cloned())
        }
    }

    impl<T: TrieLayout> Backend<T> for MockStorageDb
    where
        <T::Hash as Hasher>::Out: TryFrom<Vec<u8>>,
    {
        fn write(&mut self, transaction: DBTransaction) -> Result<(), ForestStorageErrors> {
            for op in transaction.ops {
                match op {
                    kvdb::DBOp::Insert {
                        col: _col,
                        key,
                        value,
                    } => {
                        self.data.insert(key.to_vec(), value);
                        // TODO delete old root key
                    }
                    kvdb::DBOp::Delete { col: _col, key } => {
                        self.data.remove(&key.to_vec());
                    }
                    kvdb::DBOp::DeletePrefix { col: _col, prefix } => {
                        self.data.retain(|k, _| !k.starts_with(&prefix));
                    }
                };
            }

            Ok(())
        }

        fn storage_root(&self) -> Result<Option<HasherOutT<T>>, ForestStorageErrors> {
            Ok(self
                .data
                .get(well_known_keys::ROOT)
                .map(|root| {
                    HasherOutT::<T>::try_from(root.clone()).map_err(|_| {
                        warn!(target: LOG_TARGET, "Failed to parse root from DB");
                        ForestStorageErrors::FailedToParseRoot
                    })
                })
                .transpose()?)
        }
    }

    #[test]
    fn test_read_write() {
        let storage = Box::new(MockStorageDb {
            data: Default::default(),
        });

        let mut forest_storage =
            RocksDBForestStorage::<sp_trie::LayoutV1<RefHasher>>::new(storage).unwrap();

        // Create and insert metadata
        let metadata = Metadata {
            owner: "Bob".to_string(),
            location: vec![7, 8, 9],
            size: 200,
            fingerprint: Default::default(),
        };

        let raw_key = RawKey::<_>::new(vec![7, 8, 9]);
        let lookup_key = forest_storage.insert_file_key(&raw_key, &metadata).unwrap();

        // Retrieve and verify metadata
        let retrieved_metadata = forest_storage.get_file_key(&lookup_key).unwrap().unwrap();
        assert_eq!(retrieved_metadata, metadata);
    }

    #[test]
    fn test_remove_non_existent_file_key() {
        let storage = Box::new(MockStorageDb {
            data: Default::default(),
        });

        let mut forest_storage =
            RocksDBForestStorage::<sp_trie::LayoutV1<RefHasher>>::new(storage).unwrap();

        // Attempt to delete nonexistent key which should not yield an error.
        let non_existent_key = HasherOutT::<sp_trie::LayoutV1<RefHasher>>::default();
        assert!(forest_storage.delete_file_key(&non_existent_key).is_ok());
    }

    #[test]
    fn test_initialization_with_no_existing_root() {
        let storage = Box::new(MockStorageDb {
            data: std::collections::HashMap::new(),
        });

        let forest_storage = RocksDBForestStorage::<LayoutV1<RefHasher>>::new(storage).unwrap();

        assert_eq!(
            forest_storage.root,
            HasherOutT::<LayoutV1<RefHasher>>::try_from([
                188, 54, 120, 158, 122, 30, 40, 20, 54, 70, 66, 41, 130, 143, 129, 125, 102, 18,
                247, 180, 119, 214, 101, 145, 255, 150, 169, 224, 100, 188, 201, 138
            ])
            .unwrap(),
            "Root should be initialized to default on no existing ROOT key."
        );
    }

    #[test]
    fn test_generate_proof_exact_key() {
        let storage = Box::new(MockStorageDb {
            data: std::collections::HashMap::new(),
        });

        let mut forest_storage = RocksDBForestStorage::<LayoutV1<RefHasher>>::new(storage).unwrap();

        let metadata = Metadata {
            owner: "Alice".to_string(),
            location: vec![1, 2, 3],
            size: 100,
            fingerprint: Default::default(),
        };

        let raw_key = RawKey::<_>::new(vec![1, 2, 3]);
        let lookup_key = forest_storage.insert_file_key(&raw_key, &metadata).unwrap();

        // Generate proof for the file key
        let proof = forest_storage
            .generate_proof(&vec![lookup_key])
            .expect("Failed to generate proof");

        // Verify the proof
        assert_eq!(proof.proven.len(), 1);
        assert!(match proof.proven[0] {
            Proven::ExactKey(ref leaf) => leaf.key.as_ref() == lookup_key.as_ref(),
            _ => false,
        });
    }

    #[test]
    fn test_generate_proof_exact_keys() {
        let storage = Box::new(MockStorageDb {
            data: std::collections::HashMap::new(),
        });

        let mut forest_storage = RocksDBForestStorage::<LayoutV1<RefHasher>>::new(storage).unwrap();

        let metadata1 = Metadata {
            owner: "Alice".to_string(),
            location: vec![1, 2, 3],
            size: 100,
            fingerprint: Default::default(),
        };

        let metadata2 = Metadata {
            owner: "Bob".to_string(),
            location: vec![4, 5, 6],
            size: 150,
            fingerprint: Default::default(),
        };

        let raw_key2 = RawKey::<_>::new(vec![4, 5, 6]);
        let lookup_key2 = forest_storage
            .insert_file_key(&raw_key2, &metadata2)
            .unwrap();

        let raw_key1 = RawKey::<_>::new(vec![1, 2, 3]);
        let lookup_key1 = forest_storage
            .insert_file_key(&raw_key1, &metadata1)
            .unwrap();

        match forest_storage.get_file_key(&lookup_key1).unwrap() {
            Some(metadata) => assert_eq!(metadata, metadata1),
            None => panic!("Key not found"),
        }
        match forest_storage.get_file_key(&lookup_key2).unwrap() {
            Some(metadata) => assert_eq!(metadata, metadata2),
            None => panic!("Key not found"),
        }

        let proof = forest_storage
            .generate_proof(&vec![lookup_key1, lookup_key2])
            .expect("Failed to generate proof");

        // Verify the proof
        assert_eq!(proof.proven.len(), 2);
        assert!(match proof.proven[0] {
            Proven::ExactKey(ref leaf) => leaf.key.as_ref() == lookup_key1.as_ref(),
            _ => false,
        });
        assert!(match proof.proven[1] {
            Proven::ExactKey(ref leaf) => leaf.key.as_ref() == lookup_key2.as_ref(),
            _ => false,
        });
    }

    #[test]
    fn test_key_alread_exists() {
        let storage = Box::new(MockStorageDb {
            data: std::collections::HashMap::new(),
        });

        let mut forest_storage = RocksDBForestStorage::<LayoutV1<RefHasher>>::new(storage).unwrap();

        let metadata = Metadata {
            owner: "Alice".to_string(),
            location: vec![1, 2, 3],
            size: 100,
            fingerprint: Default::default(),
        };

        let raw_key = RawKey::<_>::new(vec![1, 2, 3]);
        let lookup_key = forest_storage.insert_file_key(&raw_key, &metadata).unwrap();

        // Attempt to insert the same key again
        assert_eq!(
            forest_storage.insert_file_key(&raw_key, &metadata),
            Err(ForestStorageErrors::FileKeyAlreadyExists)
        );

        // Verify the key is still in storage
        let retrieved_metadata = forest_storage.get_file_key(&lookup_key).unwrap().unwrap();
        assert_eq!(retrieved_metadata, metadata);
    }
}
