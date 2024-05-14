use std::{io, path::PathBuf, sync::Arc};

use shc_common::types::{ForestProof, HashT, HasherOutT, Metadata};
use hash_db::{AsHashDB, HashDB, Prefix};
use kvdb::{DBTransaction, KeyValueDB};
use kvdb_rocksdb::{Database, DatabaseConfig};
use log::debug;
use sp_state_machine::{warn, Storage};
use sp_trie::{
    prefixed_key, recorder::Recorder, PrefixedMemoryDB, TrieDBBuilder, TrieLayout, TrieMut,
};
use trie_db::{DBValue, Hasher, Trie, TrieDBMutBuilder};

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
fn open_or_creating_rocksdb(db_path: String) -> io::Result<Database> {
    // TODO: add a configuration option for the base path
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

/// Trait that [`RocksDBForestStorage`] requires to interact with the storage backend.
pub trait Backend<T: TrieLayout>: Storage<HashT<T>>
where
    <T::Hash as Hasher>::Out: TryFrom<[u8; 32]>,
{
    /// Write the transaction to the storage.
    fn write(&mut self, transaction: DBTransaction) -> Result<(), ErrorT<T>>;
    /// Get the [`ROOT`](`well_known_keys::ROOT`) from storage.
    fn storage_root(&self) -> Result<Option<HasherOutT<T>>, ErrorT<T>>;
}

impl<T: TrieLayout> Backend<T> for StorageDb<HashT<T>>
where
    <T::Hash as Hasher>::Out: TryFrom<[u8; 32]>,
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

/// RocksDB based [`ForestStorage`] implementation.
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
    <T::Hash as Hasher>::Out: TryFrom<[u8; 32]>,
{
    /// This will open the RocksDB database and read the storage [`ROOT`](`well_known_keys::ROOT`) from it.
    /// If the root hash is not found in storage, a new trie will be created and the root hash will be stored in storage.
    pub fn new(storage: Box<dyn Backend<T>>) -> Result<Self, ErrorT<T>> {
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

    /// Open the RocksDB database at `dp_path` and return a new instance of [`StorageDb`].
    pub fn rocksdb_storage(dp_path: String) -> Result<StorageDb<HashT<T>>, ErrorT<T>> {
        let db = open_or_creating_rocksdb(dp_path).map_err(|e| {
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

impl<T: TrieLayout + Send + Sync> ForestStorage<T> for RocksDBForestStorage<T>
where
    <T::Hash as Hasher>::Out: TryFrom<[u8; 32]>,
{
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

    fn insert_metadata(&mut self, metadata: &Metadata) -> Result<HasherOutT<T>, ErrorT<T>> {
        let file_key = metadata.key::<T::Hash>();

        if self.contains_file_key(&file_key)? {
            return Err(ForestStorageError::FileKeyAlreadyExists(file_key).into());
        }

        let mut root = self.root.clone();
        let mut trie =
            TrieDBMutBuilder::<T>::from_existing(self.as_hash_db_mut(), &mut root).build();

        // Insert the file key with a dummy value to make it a leaf node.
        // We only need a set of `file_key`s, not a map.
        trie.insert(file_key.as_ref(), b"")
            .map_err(|_| ForestStorageError::FailedToInsertFileKey(file_key))?;

        // Drop trie to free `self`.
        drop(trie);

        // Update the root hash.
        self.root = root;

        // Commit the changes to disk.
        self.commit()?;

        Ok(file_key)
    }

    fn delete_file_key(&mut self, file_key: &HasherOutT<T>) -> Result<(), ErrorT<T>> {
        let mut root = self.root.clone();
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
    use crate::error::ErrorT;

    use super::*;
    use shc_common::types::{Fingerprint, Metadata, Proven};
    use sp_core::H256;
    use sp_runtime::traits::BlakeTwo256;
    use sp_trie::LayoutV1;
    use trie_db::Trie;

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
        <T::Hash as Hasher>::Out: TryFrom<[u8; 32]>,
    {
        fn write(&mut self, transaction: DBTransaction) -> Result<(), ErrorT<T>> {
            for op in transaction.ops {
                match op {
                    kvdb::DBOp::Insert {
                        col: _col,
                        key,
                        value,
                    } => {
                        self.data.insert(key.to_vec(), value);
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

        fn storage_root(&self) -> Result<Option<HasherOutT<T>>, ErrorT<T>> {
            Ok(self
                .data
                .get(well_known_keys::ROOT)
                .map(|root| convert_raw_bytes_to_hasher_out::<T>(root.to_owned()))
                .transpose()?)
        }
    }

    // Reusable function to setup a new `MockStorageDb` and `RocksDBForestStorage`.
    fn setup_storage<T>() -> Result<RocksDBForestStorage<T>, ErrorT<T>>
    where
        T: TrieLayout + Send + Sync,
        <T::Hash as Hasher>::Out: TryFrom<[u8; 32]>,
    {
        let storage = Box::new(MockStorageDb {
            data: Default::default(),
        });
        RocksDBForestStorage::<T>::new(storage)
    }

    // Reused function to create metadata with variable parameters.
    fn create_metadata(owner: &str, location: Vec<u8>, size: u64) -> Metadata {
        Metadata {
            owner: owner.to_string(),
            location,
            size,
            fingerprint: Fingerprint::default(),
        }
    }

    /// Reusable function to create metadata, insert it into the storage and return the lookup key and metadata.
    fn create_and_insert_metadata<T>(
        forest_storage: &mut RocksDBForestStorage<T>,
        owner: &str,
        location: Vec<u8>,
        size: u64,
    ) -> HasherOutT<T>
    where
        T: TrieLayout + Send + Sync,
        HasherOutT<T>: TryFrom<[u8; 32]>,
    {
        let metadata = create_metadata(owner, location.clone(), size);
        let file_key = forest_storage.insert_metadata(&metadata).unwrap();
        file_key
    }

    #[test]
    fn test_initialization_with_no_existing_root() {
        let forest_storage = setup_storage::<LayoutV1<BlakeTwo256>>().unwrap();
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
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>>().unwrap();

        let file_key = create_and_insert_metadata(&mut forest_storage, "Bob", vec![7, 8, 9], 200);

        assert!(forest_storage.contains_file_key(&file_key).unwrap());
    }

    #[test]
    fn test_remove_existing_file_key() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>>().unwrap();

        let file_key = create_and_insert_metadata(&mut forest_storage, "Bob", vec![7, 8, 9], 200);

        assert!(forest_storage.delete_file_key(&file_key).is_ok());
        assert!(!forest_storage.contains_file_key(&file_key).unwrap());
    }

    #[test]
    fn test_remove_non_existent_file_key() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>>().unwrap();
        assert!(forest_storage.delete_file_key(&[0u8; 32].into()).is_ok());
    }

    #[test]
    fn test_generate_proof_exact_key() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>>().unwrap();

        let mut keys = Vec::new();
        for i in 0..50 {
            let file_key = create_and_insert_metadata(&mut forest_storage, "Alice", vec![i], 200);
            keys.push(file_key);
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
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>>().unwrap();

        let mut keys = Vec::new();
        for i in 0..50 {
            let file_key = create_and_insert_metadata(&mut forest_storage, "Alice", vec![i], 200);
            keys.push(file_key);
        }

        let hash_db = forest_storage.as_hash_db();
        let root = forest_storage.root.clone();
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

    // TODO: Fix this test
    #[test]
    #[ignore = "double ended iterator has inconsistent behaviour"]
    fn test_generate_proof_challenge_before_first_leaf() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>>().unwrap();

        let file_key1 = create_and_insert_metadata(&mut forest_storage, "Alice", vec![10], 200);
        let _file_key2 = create_and_insert_metadata(&mut forest_storage, "Alice", vec![11], 200);

        let mut challenge = file_key1.clone();
        let challenge_bytes = challenge.as_mut();
        challenge_bytes[0] = challenge_bytes[0] - 1;

        let proof = forest_storage.generate_proof(vec![challenge]).unwrap();

        assert_eq!(proof.proven.len(), 1);
        assert!(
            matches!(proof.proven.first().expect("Proven leaves should have proven 1 challenge"), Proven::NeighbourKeys((None, Some(leaf))) if leaf.key.as_ref() == file_key1.as_bytes())
        );
    }

    #[test]
    fn test_generate_proof_challenge_after_last_leaf() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>>().unwrap();

        let mut keys = Vec::new();
        for i in 0..50 {
            let file_key = create_and_insert_metadata(&mut forest_storage, "Alice", vec![i], 200);
            keys.push(file_key);
        }

        let largest = keys.iter().max().unwrap();
        let mut challenge = largest.clone();
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
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>>().unwrap();

        let mut keys = Vec::new();
        for i in 0..50 {
            let file_key = create_and_insert_metadata(&mut forest_storage, "Alice", vec![i], 200);
            keys.push(file_key);
        }

        // Remove specific keys
        let keys_to_remove = keys
            .iter()
            .enumerate()
            .filter(|(i, _)| i % 2 == 0)
            .map(|(_, key)| key.clone())
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
