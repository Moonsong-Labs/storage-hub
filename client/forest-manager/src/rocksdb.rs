use codec::{Decode, Encode};
use hash_db::{AsHashDB, HashDB, Prefix};
use kvdb::{DBTransaction, KeyValueDB};
use log::debug;
use shc_common::{
    traits::StorageEnableRuntime,
    types::{FileMetadata, ForestProof, HashT, HasherOutT},
};
use sp_state_machine::{warn, Storage};
use sp_trie::{
    prefixed_key, recorder::Recorder, PrefixedMemoryDB, TrieDBBuilder, TrieLayout, TrieMut,
};
use std::{fs, io, path::Path, sync::Arc};
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

/// Open an existing RocksDB database at `db_path` and return a new instance of [`StorageDb`].
///
/// Returns an error if `db_path` does not exist on disk.
pub fn open_db<T>(db_path: String) -> Result<StorageDb<T, kvdb_rocksdb::Database>, ErrorT<T>>
where
    T: TrieLayout,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    let path = Path::new(&db_path);
    if !path.exists() {
        warn!(target: LOG_TARGET, "RocksDB path does not exist: {}", db_path);
        return Err(ForestStorageError::FailedToReadStorage.into());
    }

    let mut db_config = kvdb_rocksdb::DatabaseConfig::with_columns(1);
    db_config.create_if_missing = false;
    let db = kvdb_rocksdb::Database::open(&db_config, &db_path).map_err(|e| {
        warn!(target: LOG_TARGET, "Failed to open RocksDB: {}", e);
        ForestStorageError::FailedToReadStorage
    })?;

    Ok(StorageDb {
        db: Arc::new(db),
        _phantom: Default::default(),
    })
}

/// Create or open the RocksDB database at `db_path` and return a new instance of [`StorageDb`].
///
/// Creates the database directory if it does not exist.
pub fn create_db<T>(db_path: String) -> Result<StorageDb<T, kvdb_rocksdb::Database>, ErrorT<T>>
where
    T: TrieLayout,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    let db = open_or_creating_rocksdb(db_path).map_err(|e| {
        warn!(target: LOG_TARGET, "Failed to open RocksDB: {}", e);
        ForestStorageError::FailedToReadStorage
    })?;

    Ok(StorageDb {
        db: Arc::new(db),
        _phantom: Default::default(),
    })
}

pub fn copy_db<T>(
    src: String,
    dest: String,
) -> Result<StorageDb<T, kvdb_rocksdb::Database>, ErrorT<T>>
where
    T: TrieLayout,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    let src_path = Path::new(&src);
    let dest_path = Path::new(&dest);

    // Copying all the files from the source directory to the destination directory.
    copy_dir_all(src_path, dest_path).map_err(|e| {
        warn!(target: LOG_TARGET, "Failed to copy RocksDB: {}", e);
        ForestStorageError::FailedToCopyRocksDB
    })?;

    // Opening the directory with the copied files and returning a new instance of [`StorageDb`].
    create_db(dest)
}

/// Open the database on disk, creating it if it doesn't exist.
fn open_or_creating_rocksdb(db_path: String) -> io::Result<kvdb_rocksdb::Database> {
    let db_config = kvdb_rocksdb::DatabaseConfig::with_columns(1);

    std::fs::create_dir_all(&db_path)?;
    let db = kvdb_rocksdb::Database::open(&db_config, &db_path)?;

    Ok(db)
}

fn copy_dir_all(src: &Path, dest: &Path) -> io::Result<()> {
    if !dest.exists() {
        fs::create_dir_all(dest)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_all(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
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
    /// Root hash of the forest.
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

impl<T, DB, Runtime> ForestStorage<T, Runtime> for RocksDBForestStorage<T, DB>
where
    T: TrieLayout + Send + Sync + 'static,
    DB: KeyValueDB + 'static,
    Runtime: StorageEnableRuntime,
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

    fn get_file_metadata(
        &self,
        file_key: &HasherOutT<T>,
    ) -> Result<Option<FileMetadata>, ErrorT<T>> {
        let db = self.as_hash_db();
        let trie = TrieDBBuilder::<T>::new(&db, &self.root).build();
        let encoded_metadata = trie.get(file_key.as_ref())?;
        match encoded_metadata {
            Some(data) => {
                let decoded_metadata = FileMetadata::decode(&mut &data[..])?;
                Ok(Some(decoded_metadata))
            }
            None => Ok(None),
        }
    }

    fn get_all_files(&self) -> Result<Vec<(HasherOutT<T>, FileMetadata)>, ErrorT<T>> {
        let db = self.as_hash_db();
        let trie = TrieDBBuilder::<T>::new(&db, &self.root).build();
        let mut files = Vec::new();
        let mut trie_iter = trie
            .iter()
            .map_err(|_| ForestStorageError::FailedToCreateTrieIterator)?;

        while let Some((_, value)) = trie_iter.next().transpose()? {
            let metadata = FileMetadata::decode(&mut &value[..])?;
            let file_key = metadata.file_key::<T::Hash>();
            files.push((file_key, metadata));
        }

        Ok(files)
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
        if files_metadata.is_empty() {
            return Ok(Vec::new());
        }

        let mut file_keys = Vec::with_capacity(files_metadata.len());

        // Pre-check for existing keys
        for metadata in files_metadata {
            let file_key = metadata.file_key::<T::Hash>();

            let contains =
                <RocksDBForestStorage<T, DB> as ForestStorage<T, Runtime>>::contains_file_key(
                    self, &file_key,
                )?;
            if contains {
                return Err(ForestStorageError::FileKeyAlreadyExists(file_key).into());
            }

            file_keys.push(file_key);
        }

        let mut root = self.root;

        let mut trie =
            TrieDBMutBuilder::<T>::from_existing(self.as_hash_db_mut(), &mut root).build();

        // Batch insert all keys
        for file_metadata in files_metadata {
            let file_key = file_metadata.file_key::<T::Hash>();
            trie.insert(file_key.as_ref(), file_metadata.encode().as_slice())
                .map_err(|_| ForestStorageError::FailedToInsertFileKey(file_key))?;
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

    fn get_files_by_user(
        &self,
        user: &Runtime::AccountId,
    ) -> Result<Vec<(HasherOutT<T>, FileMetadata)>, ErrorT<T>> {
        let db = self.as_hash_db();
        let trie = TrieDBBuilder::<T>::new(&db, &self.root).build();
        let mut files = Vec::new();
        let mut trie_iter = trie
            .iter()
            .map_err(|_| ForestStorageError::FailedToCreateTrieIterator)?;

        while let Some((_, value)) = trie_iter.next().transpose()? {
            let metadata = FileMetadata::decode(&mut &value[..])?;
            let file_key = metadata.file_key::<T::Hash>();
            if metadata.owner() == &user.encode() {
                files.push((file_key, metadata));
            }
        }

        Ok(files)
    }

    fn list_all_file_keys(&self) -> Result<Vec<HasherOutT<T>>, ErrorT<T>> {
        let db = self.as_hash_db();
        let trie = TrieDBBuilder::<T>::new(&db, &self.root).build();
        let mut file_keys = Vec::new();
        let mut trie_iter = trie
            .iter()
            .map_err(|_| ForestStorageError::FailedToCreateTrieIterator)?;

        while let Some((key, _)) = trie_iter.next().transpose()? {
            let file_key = convert_raw_bytes_to_hasher_out::<T>(key)?;
            file_keys.push(file_key);
        }

        Ok(file_keys)
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::min;

    use crate::error::ErrorT;

    use super::*;
    use kvdb_memorydb::InMemory;
    use shc_common::types::StorageProofsMerkleTrieLayout;
    use shc_common::types::{FileMetadata, Fingerprint, Proven, TrieMutation, TrieRemoveMutation};
    use shp_forest_verifier::ForestVerifier;
    use shp_traits::{CommitmentVerifier, TrieProofDeltaApplier};
    use sp_core::Hasher;
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
            sh_parachain_runtime::Runtime,
        >::insert_files_metadata(
            &mut forest_storage,
            &[file_metadata],
        )
        .unwrap();

        assert!(ForestStorage::<
            StorageProofsMerkleTrieLayout,
            sh_parachain_runtime::Runtime,
        >::contains_file_key(&forest_storage, &file_key.first().unwrap())
            .unwrap()
        );
    }

    #[test]
    fn test_remove_existing_file_key() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>, InMemory>().unwrap();

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
            sh_parachain_runtime::Runtime,
        >::insert_files_metadata(
            &mut forest_storage,
            &[file_metadata],
        )
        .unwrap();

        let file_key = file_key.first().unwrap();

        assert!(ForestStorage::<
            StorageProofsMerkleTrieLayout,
            sh_parachain_runtime::Runtime,
        >::delete_file_key(&mut forest_storage, &file_key)
            .is_ok()
        );
        assert!(!ForestStorage::<
            StorageProofsMerkleTrieLayout,
            sh_parachain_runtime::Runtime,
        >::contains_file_key(&forest_storage, &file_key)
            .unwrap()
        );
    }

    #[test]
    fn test_remove_non_existent_file_key() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>, InMemory>().unwrap();
        assert!(ForestStorage::<
            StorageProofsMerkleTrieLayout,
            sh_parachain_runtime::Runtime,
        >::delete_file_key(&mut forest_storage, &[0u8; 32].into())
            .is_ok()
        );
    }

    #[test]
    fn test_get_file_metadata() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>, InMemory>().unwrap();

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
                sh_parachain_runtime::Runtime,
            >::insert_files_metadata(
                &mut forest_storage, &[file_metadata]
            )
            .unwrap();

            keys.push(*file_key.first().unwrap());
        }

        let file_metadata = ForestStorage::<
            StorageProofsMerkleTrieLayout,
            sh_parachain_runtime::Runtime,
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
    fn test_get_all_files() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>, InMemory>().unwrap();

        let metadata_1 = FileMetadata::new(
            "Alice".as_bytes().to_vec(),
            "bucket".as_bytes().to_vec(),
            "location_1".as_bytes().to_vec(),
            100,
            Fingerprint::default(),
        )
        .unwrap();
        let metadata_2 = FileMetadata::new(
            "Bob".as_bytes().to_vec(),
            "bucket".as_bytes().to_vec(),
            "location_2".as_bytes().to_vec(),
            200,
            Fingerprint::default(),
        )
        .unwrap();

        ForestStorage::<StorageProofsMerkleTrieLayout, sh_parachain_runtime::Runtime>::insert_files_metadata(
            &mut forest_storage,
            &[metadata_1, metadata_2],
        )
        .unwrap();

        let all_files =
            ForestStorage::<StorageProofsMerkleTrieLayout, sh_parachain_runtime::Runtime>::get_all_files(
                &forest_storage,
            )
            .unwrap();

        assert_eq!(all_files.len(), 2);

        let mut sizes = all_files
            .iter()
            .map(|(_, m)| m.file_size())
            .collect::<Vec<_>>();
        sizes.sort_unstable();
        assert_eq!(sizes, vec![100, 200]);

        for (_, metadata) in all_files {
            assert_eq!(metadata.bucket_id(), "bucket".as_bytes());
        }
    }

    #[test]
    fn test_generate_proof_exact_key() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>, InMemory>().unwrap();

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
                sh_parachain_runtime::Runtime,
            >::insert_files_metadata(
                &mut forest_storage, &[file_metadata]
            )
            .unwrap();

            keys.push(*file_key.first().unwrap());
        }

        let challenge = keys[0];

        let proof =
            ForestStorage::<StorageProofsMerkleTrieLayout, sh_parachain_runtime::Runtime>::generate_proof(
                &forest_storage,
                vec![challenge],
            )
            .unwrap();

        assert_eq!(proof.proven.len(), 1);
        assert!(
            matches!(proof.proven.first().expect("Proven leaves should have proven 1 challenge"), Proven::ExactKey(leaf) if leaf.key.as_ref() == challenge.as_bytes())
        );
    }

    #[test]
    fn test_generate_proof_includes_neighbor_keys() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>, InMemory>().unwrap();

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
                sh_parachain_runtime::Runtime,
            >::insert_files_metadata(
                &mut forest_storage, &[file_metadata]
            )
            .unwrap();

            keys.push(*file_key.first().unwrap());
        }
        keys.sort();

        let challenge = keys[1];
        let root = forest_storage.root;

        let proof =
            ForestStorage::<StorageProofsMerkleTrieLayout, sh_parachain_runtime::Runtime>::generate_proof(
                &forest_storage,
                vec![challenge],
            )
            .unwrap();
        let included_keys = vec![keys[0], keys[1], keys[2]];
        assert!(
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                included_keys.as_slice(),
                &proof.proof
            )
            .is_ok()
        );

        let new_challenges = vec![keys[10], keys[40]];
        let proof =
            ForestStorage::<StorageProofsMerkleTrieLayout, sh_parachain_runtime::Runtime>::generate_proof(
                &forest_storage,
                new_challenges,
            )
            .unwrap();
        let included_keys = vec![keys[9], keys[10], keys[11], keys[39], keys[40], keys[41]];
        assert!(
            ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::verify_proof(
                &root,
                included_keys.as_slice(),
                &proof.proof
            )
            .is_ok()
        );

        // Probabilistically, two of the 50 generated keys should share the same prefix and as such should be neighbors.
        // So, we test that any generated proof is able to be used to remove any key from the trie.
        // Spoiler alert: with the current parameters, the first two keys are neighbors.
        for key in keys.iter() {
            println!("Trying to remove key: {:?}", key.as_bytes());
            let proof = ForestStorage::<
                StorageProofsMerkleTrieLayout,
                sh_parachain_runtime::Runtime,
            >::generate_proof(&forest_storage, vec![*key])
                .unwrap();
            let proof = proof.proof;
            let mutations: Vec<(H256, TrieMutation)> =
                vec![(*key, TrieRemoveMutation::default().into())];

            let apply_delta_result =
                ForestVerifier::<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>::apply_delta(
                    &root, &mutations, &proof,
                );
            assert!(apply_delta_result.is_ok());
            assert!(apply_delta_result
                .unwrap()
                .2
                .into_iter()
                .map(|(key, _)| key)
                .collect::<Vec<H256>>()
                .contains(key));
        }
    }

    #[test]
    fn test_generate_proof_neighbor_keys() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>, InMemory>().unwrap();

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
                sh_parachain_runtime::Runtime,
            >::insert_files_metadata(
                &mut forest_storage, &[file_metadata]
            )
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

        let proof =
            ForestStorage::<StorageProofsMerkleTrieLayout, sh_parachain_runtime::Runtime>::generate_proof(
                &forest_storage,
                vec![challenge_hash],
            )
            .unwrap();

        assert_eq!(proof.proven.len(), 1);
        assert!(
            matches!(proof.proven.first().expect("Proven leaves should have proven 1 challenge"), Proven::NeighbourKeys((Some(left_leaf), Some(right_leaf))) if left_leaf.key.as_ref() == first_key && right_leaf.key.as_ref() == second_key)
        );
    }

    #[test]
    fn test_generate_proof_challenge_before_first_leaf() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>, InMemory>().unwrap();

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
            sh_parachain_runtime::Runtime,
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

        let proof =
            ForestStorage::<StorageProofsMerkleTrieLayout, sh_parachain_runtime::Runtime>::generate_proof(
                &forest_storage,
                vec![challenge],
            )
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
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>, InMemory>().unwrap();

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
                sh_parachain_runtime::Runtime,
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

        let proof =
            ForestStorage::<StorageProofsMerkleTrieLayout, sh_parachain_runtime::Runtime>::generate_proof(
                &forest_storage,
                vec![challenge],
            )
            .unwrap();

        assert_eq!(proof.proven.len(), 1);
        assert!(
            matches!(proof.proven.first().expect("Proven leaves should have proven 1 challenge"), Proven::NeighbourKeys((Some(leaf), None)) if leaf.key.as_ref() == largest.as_bytes())
        );
    }

    #[test]
    fn test_trie_with_over_16_consecutive_leaves() {
        let mut forest_storage = setup_storage::<LayoutV1<BlakeTwo256>, InMemory>().unwrap();

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
                sh_parachain_runtime::Runtime,
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
                sh_parachain_runtime::Runtime,
            >::delete_file_key(&mut forest_storage, &key)
                .is_ok()
            );
        }

        // Test that the keys are removed
        for key in keys_to_remove {
            assert!(
                !ForestStorage::<
                    StorageProofsMerkleTrieLayout,
                    sh_parachain_runtime::Runtime,
                >::contains_file_key(&forest_storage, &key)
                    .unwrap()
            );
        }
    }
}
