use std::{io, path::PathBuf, sync::Arc};

use hash_db::{AsHashDB, HashDB, Prefix};
use kvdb::{DBTransaction, KeyValueDB};
use kvdb_rocksdb::{Database, DatabaseConfig};
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

mod well_known_keys {
    pub const ROOT: &[u8] = b"forest_root";
}

pub(crate) fn other_io_error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

/// Open the database on disk, creating it if it doesn't exist.
fn open_creating_rocksdb() -> io::Result<Database> {
    let root = PathBuf::from("/tmp/");
    let path = root.join("storagehub").join("db");

    let db_config = DatabaseConfig::with_columns(1);

    let path_str = path
        .to_str()
        .ok_or_else(|| other_io_error(format!("Bad database path: {:?}", path)))?;

    std::fs::create_dir_all(&path_str)?;
    let db = Database::open(&db_config, &path_str)?;

    Ok(db)
}

/// Storage backend for RocksDB.
struct StorageDb<Hasher> {
    pub db: Arc<dyn KeyValueDB>,
    pub _phantom: std::marker::PhantomData<Hasher>,
}

impl<H: Hasher> Storage<H> for StorageDb<H> {
    fn get(&self, key: &H::Out, prefix: Prefix) -> Result<Option<DBValue>, String> {
        let prefixed_key = prefixed_key::<H>(key, prefix);
        self.db
            .get(0, &prefixed_key)
            .map_err(|e| format!("Database backend error: {:?}", e))
    }
}

/// Patricia trie-based pairs storage.
pub struct RocksDBForestStorage<T: TrieLayout> {
    storage: Arc<StorageDb<HashT<T>>>,
    // TODO: make sure this can only be accessed by a single write lock
    overlay: PrefixedMemoryDB<HashT<T>>,
    root: HasherOutT<T>,
    _phantom: std::marker::PhantomData<T>,
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
                warn!(target: "trie", "Failed to read from DB: {}", e);
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

impl<T: TrieLayout + Send + Sync> RocksDBForestStorage<T>
where
    <<T as TrieLayout>::Hash as Hasher>::Out: TryFrom<Vec<u8>>,
{
    /// Create a new `RocksDBForestStorage` instance.
    ///
    /// This will open the RocksDB database and read the root hash from it.
    /// If the root hash is not found, a new trie will be created and the root hash will be stored in the database.
    pub fn new() -> Result<Self, ForestStorageErrors> {
        let kvdb = Arc::new(open_creating_rocksdb().expect("Failed to open RocksDB"));
        let storage: Arc<StorageDb<<T as TrieLayout>::Hash>> = Arc::new(StorageDb::<HashT<T>> {
            db: kvdb,
            _phantom: Default::default(),
        });

        let maybe_root = storage
            .db
            .get(0, well_known_keys::ROOT)
            .map_err(|e| {
                warn!(target: "trie", "Failed to read root from DB: {}", e);
                ForestStorageErrors::FailedToReadStorage
            })
            .expect("This should not fail unless something is wrong with the DB");

        let rocksdb_forest_storage = match maybe_root {
            Some(root) => {
                let root = HasherOutT::<T>::try_from(root).map_err(|_| {
                    warn!(target: "trie", "Failed to parse root from DB");
                    ForestStorageErrors::FailedToParseRoot
                })?;

                RocksDBForestStorage {
                    storage,
                    overlay: Default::default(),
                    root,
                    _phantom: Default::default(),
                }
            }
            None => {
                let mut root = HasherOutT::<T>::default();

                let mut rocksdb_forest_storage = RocksDBForestStorage {
                    storage,
                    overlay: Default::default(),
                    root,
                    _phantom: Default::default(),
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
                    .db
                    .write(transaction)
                    .expect("Failed to write to RocksDB");

                rocksdb_forest_storage.root = root;

                rocksdb_forest_storage
            }
        };

        Ok(rocksdb_forest_storage)
    }

    /// Commit changes to the backend.
    ///
    /// This will write the changes including root in self to RocksDB.
    /// The `overlay` will be cleared.
    pub fn commit(&mut self) {
        let mut transaction = self.changes();

        // add transaction to update root
        transaction.put(0, well_known_keys::ROOT, self.root.as_ref());

        self.storage
            .db
            .write(transaction)
            .expect("Failed to write to RocksDB");
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

impl<T: TrieLayout + Send + Sync> ForestStorage for RocksDBForestStorage<T>
where
    <<T as TrieLayout>::Hash as Hasher>::Out: TryFrom<Vec<u8>>,
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

        let maybe_metadata = trie
            .get(file_key.as_ref())
            .map_err(|e| {
                warn!(target: "trie", "Failed to get file key: {:?}", e);
                ForestStorageErrors::FailedToGetFileKey
            })?
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
            .map(|file_key| prove::<T, Self>(&trie, file_key))
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
        let mut trie = TrieDBMutBuilder::<T>::new(self.as_hash_db_mut(), &mut root).build();

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
        self.commit();

        Ok(file_key)
    }

    fn delete_file_key(&mut self, file_key: &Self::LookupKey) -> Result<(), ForestStorageErrors> {
        let mut root = self.root.clone();
        let mut trie = TrieDBMutBuilder::<T>::new(self.as_hash_db_mut(), &mut root).build();

        // Remove the file key from the trie.
        let _ = trie
            .remove(file_key.as_ref())
            .map_err(|_| ForestStorageErrors::FailedToRemoveFileKey)?;

        // Drop trie to free `self`.
        drop(trie);

        // Update the root hash.
        self.root = root;

        // Commit the changes to disk.
        self.commit();

        Ok(())
    }
}
