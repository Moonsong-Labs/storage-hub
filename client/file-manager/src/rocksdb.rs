use std::{collections::HashMap, io, path::PathBuf, sync::Arc};

use codec::Encode;
use hash_db::{AsHashDB, HashDB, Prefix, EMPTY_PREFIX};
use kvdb::{DBTransaction, KeyValueDB};
use kvdb_rocksdb::{Database, DatabaseConfig};
use log::{debug, error};
use shc_common::types::{
    Chunk, ChunkId, FileKeyProof, FileMetadata, FileProof, HashT, HasherOutT, Leaf, H_LENGTH,
};
use sp_state_machine::{warn, Storage};
use sp_trie::{
    prefixed_key, recorder::Recorder, PrefixedKey, PrefixedMemoryDB, TrieLayout, TrieMut,
};
use trie_db::{DBValue, Hasher, Trie, TrieDBBuilder, TrieDBMutBuilder};

use crate::{
    error::ErrorT,
    traits::{
        FileDataTrie, FileStorage, FileStorageError, FileStorageWriteError, FileStorageWriteOutcome,
    },
    LOG_TARGET,
};

// TODO: maybe extract common types used in Forest Manager impls.
// TODO: create error module for File Manager / refactor errors
// TODO: Add comments
// TODO: chunk_keys HashMap will not work because it's not persistent?
// TODO: Check TODOs
// TODO: use batch insert (write_chunks()). Needs API change

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
    pub _marker: std::marker::PhantomData<Hasher>,
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

pub trait Backend<T: TrieLayout>: Storage<HashT<T>>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    /// Write the transaction to the storage.
    fn write(&mut self, transaction: DBTransaction) -> Result<(), ErrorT<T>>;
}

impl<T: TrieLayout + Send + Sync> Backend<T> for StorageDb<HashT<T>>
where
    <T::Hash as Hasher>::Out: TryFrom<[u8; 32]>,
{
    fn write(&mut self, transaction: DBTransaction) -> Result<(), ErrorT<T>> {
        self.db.write(transaction).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to write to DB: {}", e);
            FileStorageError::FailedToWriteToStorage
        })?;

        Ok(())
    }
}

pub struct RocksDbFileDataTrie<T: TrieLayout> {
    // Persistent storage
    // TODO: Why Box not Arc?
    storage: Box<dyn Backend<T>>,
    overlay: PrefixedMemoryDB<HashT<T>>,
    // Root of the file Trie, which is a file key.
    root: HasherOutT<T>,
    // TODO: we also need to persist this somewhere.
    // idea: [owner/bucket/file_location/chunk_ids]: {chunk_hash: H, chunk_data: []}
    // Maintains relationship between external chunk key representation (chunk_id)
    // and internal chunk key representation (hash of value)
    chunk_keys: HashMap<ChunkId, HasherOutT<T>>,
}

// TODO: double check if `Default` is really necessary
impl<T: TrieLayout + Send + Sync + 'static> Default for RocksDbFileDataTrie<T>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    fn default() -> Self {
        let default_storage =
            Self::rocksdb_storage("/tmp".to_string()).expect("Failed to create RocksDB");

        Self::new(Box::new(default_storage))
    }
}

impl<T: TrieLayout + Send + Sync> RocksDbFileDataTrie<T>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    fn new(storage: Box<dyn Backend<T>>) -> Self {
        let mut root = HasherOutT::<T>::default();
        let mut rocksdb_file_data_trie = RocksDbFileDataTrie::<T> {
            storage,
            root,
            overlay: Default::default(),
            chunk_keys: HashMap::new(),
        };
        let db = rocksdb_file_data_trie.as_hash_db_mut();
        let trie = TrieDBMutBuilder::<T>::new(db, &mut root).build();

        drop(trie);

        rocksdb_file_data_trie.root = root;

        debug!(target: LOG_TARGET, "New root: {:?}", rocksdb_file_data_trie.root);

        rocksdb_file_data_trie
    }

    /// Persists the changes applied to the overlay.
    /// If the root has not changed, the commit will be skipped.
    /// The `overlay` will be cleared.
    pub fn commit(&mut self, new_root: HasherOutT<T>) -> Result<(), ErrorT<T>> {
        // Skip commit if the root has not changed.
        if self.root == new_root {
            warn!(target: LOG_TARGET, "Root has not changed, skipping commit");
            // println!(
            //     "CANNOT COMMIT received root: {:?}, internal root: {:?}",
            //     new_root, self.root
            // );
            return Ok(());
        }

        // println!(
        //     "COMMITING NOW received root: {:?}, internal root: {:?}",
        //     new_root, self.root
        // );

        // Aggregate changes from the overlay
        let transaction = self.changes();

        // Write the changes to storage
        self.storage.write(transaction)?;

        self.root = new_root;

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

    /// Open the RocksDB database at `dp_path` and return a new instance of [`StorageDb`].
    pub fn rocksdb_storage(dp_path: String) -> Result<StorageDb<HashT<T>>, ErrorT<T>> {
        let db = open_or_creating_rocksdb(dp_path).map_err(|e| {
            warn!(target: LOG_TARGET, "Failed to open RocksDB: {}", e);
            FileStorageError::FailedToReadStorage
        })?;

        Ok(StorageDb {
            db: Arc::new(db),
            _marker: Default::default(),
        })
    }
}

// Dropping the trie (either by calling `drop()` or by the end of the scope)
// automatically commits to the underlying db.
impl<T: TrieLayout + Send + Sync> FileDataTrie<T> for RocksDbFileDataTrie<T>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    fn get_root(&self) -> &HasherOutT<T> {
        &self.root
    }

    fn stored_chunks_count(&self) -> Result<u64, FileStorageError> {
        let db = self.as_hash_db();
        let trie = TrieDBBuilder::<T>::new(&db, &self.root).build();

        let mut iter = trie.iter().expect("Should be able to get iterator; qed.");
        let mut count = 0u64;
        while let Some(element) = iter.next() {
            // println!("{:?}", element);
            count += 1
        }

        Ok(count as u64)
    }

    fn generate_proof(&self, chunk_ids: &Vec<ChunkId>) -> Result<FileProof, FileStorageError> {
        let db = self.as_hash_db();
        let recorder: Recorder<T::Hash> = Recorder::default();

        // A `TrieRecorder` is needed to create a proof of the "visited" leafs, by the end of this process.
        let mut trie_recorder = recorder.as_trie_recorder(self.root);

        let trie = TrieDBBuilder::<T>::new(&db, &self.root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Read all the chunks to prove from the trie.
        let mut chunks = Vec::new();
        for chunk_id in chunk_ids {
            let chunk_key = self
                .chunk_keys
                .get(chunk_id)
                .ok_or(FileStorageError::FileChunkDoesNotExist)?;

            let chunk: Option<Vec<u8>> = trie
                .get(&chunk_key.as_ref())
                .map_err(|_| FileStorageError::FailedToGetFileChunk)?;

            let chunk = chunk.ok_or(FileStorageError::FileChunkDoesNotExist)?;
            chunks.push((*chunk_id, chunk));
        }

        // Drop the `trie_recorder` to release the `recorder`
        drop(trie_recorder);

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<T::Hash>(self.root)
            .map_err(|_| FileStorageError::FailedToGenerateCompactProof)?;

        Ok(FileProof {
            proof: proof.into(),
            fingerprint: self.get_root().as_ref().into(),
        })
    }

    // TODO: Return Result<Option> instead of Result only
    fn get_chunk(&self, chunk_id: &ChunkId) -> Result<Chunk, FileStorageError> {
        let db = self.as_hash_db();
        let trie = TrieDBBuilder::<T>::new(&db, &self.root).build();

        let chunk_key = self
            .chunk_keys
            .get(chunk_id)
            .ok_or(FileStorageError::FileChunkDoesNotExist)?;

        trie.get(&chunk_key.as_ref())
            .map_err(|_| FileStorageError::FailedToGetFileChunk)?
            .ok_or(FileStorageError::FileChunkDoesNotExist)
    }

    fn write_chunk(
        &mut self,
        chunk_id: &ChunkId,
        data: &Chunk,
    ) -> Result<(), FileStorageWriteError> {
        let chunk_key = HashT::<T>::hash(data);
        self.chunk_keys.insert(*chunk_id, chunk_key);

        let mut current_root = self.root;
        // println!("WRITE_CHUNK OLD ROOT: {:?}", current_root);
        let db = self.as_hash_db_mut();
        let mut trie = TrieDBMutBuilder::<T>::from_existing(db, &mut current_root).build();
        // println!("TRIE.ROOT {:?}", *trie.root());

        // Check that we don't have a chunk already stored.
        if trie.contains(chunk_key.as_ref()).map_err(|e| {
            error!(target: LOG_TARGET, "{}", e);
            // println!("WRITE_CHUNK CONTAINS {}", e);
            FileStorageWriteError::FailedToGetFileChunk
        })? {
            return Err(FileStorageWriteError::FileChunkAlreadyExists);
        }

        // Insert the chunk into the file trie.
        trie.insert(chunk_key.as_ref(), &data).map_err(|e| {
            error!(target: LOG_TARGET, "{}", e);
            // println!("WRITE_CHUNK INSERTS {}", e);
            FileStorageWriteError::FailedToInsertFileChunk
        })?;

        // get new root after trie modifications
        let new_root = *trie.root();
        // drop trie to commit to underlying db and release `self`
        drop(trie);

        // println!("WRITE_CHUNK NEW ROOT: {:?}", new_root);
        // TODO: improve error handling
        // Commit the changes to disk.
        self.commit(new_root).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to commit changes to persistent storage: {}", e);
            FileStorageWriteError::FailedToInsertFileChunk
        })?;

        Ok(())
    }

    // Deletes itself from the underlying db.
    fn delete(&mut self) -> Result<(), FileStorageError> {
        let mut root = self.root;
        // TODO: not good, bad performance
        let chunk_keys = self.chunk_keys.clone();

        // Need to clone because we cannot have a immutable borrow after mutably borrowing
        // in the next step.
        let trie_root_key = root.clone();
        let mut trie =
            TrieDBMutBuilder::<T>::from_existing(self.as_hash_db_mut(), &mut root).build();

        for (_, chunk_key) in chunk_keys {
            trie.remove(chunk_key.as_ref()).map_err(|e| {
                error!(target: LOG_TARGET, "Failed to delete chunk from RocksDb: {}", e);
                FileStorageError::FailedToWriteToStorage
            })?;
        }

        // Remove the root from the trie.
        trie.remove(trie_root_key.as_ref()).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to delete root from RocksDb: {}", e);
            FileStorageError::FailedToWriteToStorage
        })?;

        let new_root = *trie.root();

        drop(trie);

        // TODO: improve error handling
        // Commit the changes to disk.
        self.commit(trie_root_key).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to commit changes to persistent storage: {}", e);
            FileStorageError::FailedToWriteToStorage
        })?;

        // Set new internal root (no trie)
        self.root = new_root;

        Ok(())
    }
}

impl<T: TrieLayout + Send + Sync> AsHashDB<HashT<T>, DBValue> for RocksDbFileDataTrie<T>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    fn as_hash_db<'b>(&'b self) -> &'b (dyn HashDB<HashT<T>, DBValue> + 'b) {
        self
    }
    fn as_hash_db_mut<'b>(&'b mut self) -> &'b mut (dyn HashDB<HashT<T>, DBValue> + 'b) {
        &mut *self
    }
}

impl<T: TrieLayout + Send + Sync> hash_db::HashDB<HashT<T>, DBValue> for RocksDbFileDataTrie<T>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
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
        HashDB::contains(&self.overlay, key, prefix)
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

pub struct RocksDbFileStorage<T: TrieLayout + 'static>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    pub metadata: HashMap<HasherOutT<T>, FileMetadata>,
    pub file_data: HashMap<HasherOutT<T>, RocksDbFileDataTrie<T>>,
}

impl<T: TrieLayout + 'static> RocksDbFileStorage<T>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
            file_data: HashMap::new(),
        }
    }
}

impl<T: TrieLayout + 'static + Send + Sync> FileStorage<T> for RocksDbFileStorage<T>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    type FileDataTrie = RocksDbFileDataTrie<T>;

    fn get_chunk(
        &self,
        key: &HasherOutT<T>,
        chunk_id: &ChunkId,
    ) -> Result<Chunk, FileStorageError> {
        let file_data = self
            .file_data
            .get(key)
            .ok_or(FileStorageError::FileDoesNotExist)?;

        file_data.get_chunk(chunk_id)
    }

    fn get_metadata(&self, key: &HasherOutT<T>) -> Result<FileMetadata, FileStorageError> {
        self.metadata
            .get(key)
            .cloned()
            .ok_or(FileStorageError::FileDoesNotExist)
    }

    fn write_chunk(
        &mut self,
        key: &HasherOutT<T>,
        chunk_id: &ChunkId,
        data: &Chunk,
    ) -> Result<FileStorageWriteOutcome, FileStorageWriteError> {
        let file_data = self
            .file_data
            .get_mut(key)
            .ok_or(FileStorageWriteError::FileDoesNotExist)?;

        file_data.write_chunk(chunk_id, data)?;

        let metadata = self.metadata.get(key).expect(
                format!(
                    "Key {:?} already associated with File Trie, but not with File Metadata. Possible inconsistency between them.",                
                    key
            )
                .as_str(),
            );

        // Check if we have all the chunks for the file.
        let stored_chunks = file_data
            .stored_chunks_count()
            .map_err(|_| FileStorageWriteError::FailedToConstructTrieIter)?;
        if metadata.chunks_count() != stored_chunks {
            return Ok(FileStorageWriteOutcome::FileIncomplete);
        }

        // If we have all the chunks, check if the file metadata fingerprint and the file trie
        // root matches.
        if metadata.fingerprint != file_data.root.as_ref().into() {
            return Err(FileStorageWriteError::FingerprintAndStoredFileMismatch);
        }

        Ok(FileStorageWriteOutcome::FileComplete)
    }

    // TODO: check why this method is necessary and what is its use case.
    fn insert_file(
        &mut self,
        _key: HasherOutT<T>,
        _metadata: FileMetadata,
    ) -> Result<(), FileStorageError> {
        unimplemented!()
    }

    fn insert_file_with_data(
        &mut self,
        key: HasherOutT<T>,
        metadata: FileMetadata,
        file_data: Self::FileDataTrie,
    ) -> Result<(), FileStorageError> {
        if self.metadata.contains_key(&key) {
            return Err(FileStorageError::FileAlreadyExists);
        }
        self.metadata.insert(key, metadata);

        let maybe_file_data = self.file_data.insert(key, file_data);
        if maybe_file_data.is_some() {
            panic!("Key already associated with File Trie, but not with File Metadata. Possible inconsistency between them.");
        }

        Ok(())
    }

    fn generate_proof(
        &self,
        key: &HasherOutT<T>,
        chunk_ids: &Vec<ChunkId>,
    ) -> Result<FileKeyProof, FileStorageError> {
        let metadata = self
            .metadata
            .get(key)
            .ok_or(FileStorageError::FileDoesNotExist)?;

        // TODO: use better error
        let file_data = self.file_data.get(key).expect(
            format!("Key {:?} already associated with File Metadata, but not with File Trie. Possible inconsistency between them.",
                key
            )
            .as_str(),
        );

        let stored_chunks = file_data.stored_chunks_count()?;
        if metadata.chunks_count() != stored_chunks {
            return Err(FileStorageError::IncompleteFile);
        }

        if metadata.fingerprint
            != file_data
                .root
                .as_ref()
                .try_into()
                .expect("Hasher output mismatch")
        {
            return Err(FileStorageError::FingerprintAndStoredFileMismatch);
        }

        Ok(file_data
            .generate_proof(chunk_ids)?
            .to_file_key_proof(metadata.clone()))
    }

    fn delete_file(&mut self, key: &HasherOutT<T>) -> Result<(), FileStorageError> {
        let file_trie = self.file_data.get_mut(key).expect("No File data found");

        file_trie.delete()?;

        self.metadata.remove(key);
        self.file_data.remove(key);

        Ok(())
    }
}

// Utils
fn convert_raw_bytes_to_hasher_out<T: TrieLayout>(key: Vec<u8>) -> Result<HasherOutT<T>, ErrorT<T>>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    let key: [u8; 32] = key
        .try_into()
        .map_err(|_| FileStorageError::FailedToParseKey)?;

    let key = HasherOutT::<T>::try_from(key).map_err(|_| {
        warn!(target: LOG_TARGET, "Failed to parse root from DB");
        FileStorageError::FailedToParseKey
    })?;

    Ok(key)
}

mod tests {
    use sp_core::H256;
    use sp_runtime::traits::BlakeTwo256;
    use sp_runtime::AccountId32;
    use sp_trie::LayoutV1;
    use trie_db::TrieHash;

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
        HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
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
    }

    #[test]
    fn file_trie_creating_empty_works() {
        let storage = Box::new(MockStorageDb {
            data: Default::default(),
        });

        let file_trie = RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>>::new(storage);

        // expected hash is the root hash of an empty tree.
        let expected_hash = TrieHash::<LayoutV1<BlakeTwo256>>::try_from([
            3, 23, 10, 46, 117, 151, 183, 183, 227, 216, 76, 5, 57, 29, 19, 154, 98, 177, 87, 231,
            135, 134, 216, 192, 130, 242, 157, 207, 76, 17, 19, 20,
        ])
        .unwrap();

        assert_eq!(
            H256::from(*file_trie.get_root()),
            expected_hash,
            "Root should be initialized to default."
        );
    }

    #[test]
    fn file_trie_writing_chunk_works() {
        let storage = Box::new(MockStorageDb {
            data: Default::default(),
        });

        let mut file_trie = RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>>::new(storage);
        let old_root = file_trie.get_root().clone();
        file_trie
            .write_chunk(&ChunkId::from(0u64), &Chunk::from([1u8; 1024]))
            .unwrap();
        let new_root = file_trie.get_root();
        assert_ne!(&old_root, new_root);

        let chunk = file_trie.get_chunk(&ChunkId::from(0u64)).unwrap();
        assert_eq!(chunk.as_slice(), [1u8; 1024]);
    }

    #[test]
    fn file_trie_getting_chunk_works() {
        let storage = Box::new(MockStorageDb {
            data: Default::default(),
        });

        let mut file_trie = RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>>::new(storage);

        let chunk = Chunk::from([3u8; 1024]);
        let chunk_id: ChunkId = 3;
        file_trie.write_chunk(&chunk_id, &chunk).unwrap();
        let chunk = file_trie.get_chunk(&chunk_id).unwrap();
        assert_eq!(chunk.as_slice(), [3u8; 1024]);
    }

    #[test]
    fn file_trie_getting_stored_chunks_works() {
        let storage = Box::new(MockStorageDb {
            data: Default::default(),
        });
        let chunk_ids = vec![ChunkId::from(0u64), ChunkId::from(1u64)];
        let chunks = vec![Chunk::from([0u8; 1024]), Chunk::from([1u8; 1024])];
        let mut file_trie = RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>>::new(storage);

        file_trie.write_chunk(&chunk_ids[0], &chunks[0]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 1);
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_ok());

        file_trie.write_chunk(&chunk_ids[1], &chunks[1]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 2);
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_ok());
    }

    #[test]
    fn file_trie_generating_proof_works() {
        let storage = Box::new(MockStorageDb {
            data: Default::default(),
        });

        let chunk_ids = vec![
            ChunkId::from(0u64),
            ChunkId::from(1u64),
            ChunkId::from(2u64),
        ];

        let chunks = vec![
            Chunk::from([0u8; 1024]),
            Chunk::from([1u8; 1024]),
            Chunk::from([2u8; 1024]),
        ];

        let mut file_trie = RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>>::new(storage);

        file_trie.write_chunk(&chunk_ids[0], &chunks[0]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 1);
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_ok());

        file_trie.write_chunk(&chunk_ids[1], &chunks[1]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 2);
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_ok());

        file_trie.write_chunk(&chunk_ids[2], &chunks[2]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 3);
        assert!(file_trie.get_chunk(&chunk_ids[2]).is_ok());

        let file_proof = file_trie.generate_proof(&chunk_ids).unwrap();
        let proven_leaves = file_proof.proven;
        for (id, leaf) in proven_leaves.iter().enumerate() {
            assert_eq!(chunk_ids[id], leaf.key);
            assert_eq!(chunks[id], leaf.data);
        }
    }

    #[test]
    fn file_trie_deleting_works() {
        let storage = Box::new(MockStorageDb {
            data: Default::default(),
        });

        let chunk_ids = vec![
            ChunkId::from(0u64),
            ChunkId::from(1u64),
            ChunkId::from(2u64),
        ];

        let chunks = vec![
            Chunk::from([0u8; 1024]),
            Chunk::from([1u8; 1024]),
            Chunk::from([2u8; 1024]),
        ];

        let mut file_trie = RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>>::new(storage);

        file_trie.write_chunk(&chunk_ids[0], &chunks[0]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 1);
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_ok());

        file_trie.write_chunk(&chunk_ids[1], &chunks[1]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 2);
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_ok());

        file_trie.write_chunk(&chunk_ids[2], &chunks[2]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 3);
        assert!(file_trie.get_chunk(&chunk_ids[2]).is_ok());

        file_trie.delete().unwrap();
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_err());
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_err());
        assert!(file_trie.get_chunk(&chunk_ids[2]).is_err());

        assert_eq!(file_trie.stored_chunks_count(), 0);
    }

    #[test]
    fn file_storage_inserting_whole_file_works() {
        let storage = Box::new(MockStorageDb {
            data: Default::default(),
        });

        let chunks = vec![
            Chunk::from([5u8; 32]),
            Chunk::from([6u8; 32]),
            Chunk::from([7u8; 32]),
        ];

        let chunk_ids: Vec<ChunkId> = chunks
            .iter()
            .enumerate()
            .map(|(id, _)| ChunkId::from(id as u64))
            .collect();

        let mut file_trie = RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>>::new(storage);

        file_trie.write_chunk(&chunk_ids[0], &chunks[0]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 1);
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_ok());

        file_trie.write_chunk(&chunk_ids[1], &chunks[1]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 2);
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_ok());

        file_trie.write_chunk(&chunk_ids[2], &chunks[2]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 3);
        assert!(file_trie.get_chunk(&chunk_ids[2]).is_ok());

        let file_metadata = FileMetadata {
            size: 32u64 * chunks.len() as u64,
            fingerprint: file_trie.get_root().as_ref().into(),
            owner: <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            location: "location".to_string().into_bytes(),
        };

        let key = file_metadata.file_key::<BlakeTwo256>();
        let mut file_storage = RocksDbFileStorage::<LayoutV1<BlakeTwo256>>::new();
        file_storage
            .insert_file_with_data(key, file_metadata, file_trie)
            .unwrap();

        assert!(file_storage.file_data.contains_key(&key));
        assert!(file_storage.metadata.contains_key(&key));
    }

    #[test]
    fn file_storage_deleting_whole_file_works() {}

    #[test]
    fn file_storage_proof_generation_works() {
        let storage = Box::new(MockStorageDb {
            data: Default::default(),
        });

        let chunks = vec![
            Chunk::from([8u8; 1024]),
            Chunk::from([9u8; 1024]),
            Chunk::from([10u8; 1024]),
        ];

        let chunk_ids: Vec<ChunkId> = chunks
            .iter()
            .enumerate()
            .map(|(id, _)| ChunkId::from(id as u64))
            .collect();

        let mut file_trie = RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>>::new(storage);

        file_trie.write_chunk(&chunk_ids[0], &chunks[0]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 1);
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_ok());

        file_trie.write_chunk(&chunk_ids[1], &chunks[1]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 2);
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_ok());

        file_trie.write_chunk(&chunk_ids[2], &chunks[2]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 3);
        assert!(file_trie.get_chunk(&chunk_ids[2]).is_ok());

        let file_metadata = FileMetadata {
            size: 1024u64 * chunks.len() as u64,
            fingerprint: file_trie.get_root().as_ref().into(),
            owner: <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            location: "location".to_string().into_bytes(),
        };

        let key = file_metadata.file_key::<BlakeTwo256>();
        let mut file_storage = RocksDbFileStorage::<LayoutV1<BlakeTwo256>>::new();
        file_storage
            .insert_file_with_data(key, file_metadata, file_trie)
            .unwrap();

        assert!(file_storage.file_data.contains_key(&key));
        assert!(file_storage.metadata.contains_key(&key));

        let file_proof = file_storage.generate_proof(&key, &chunk_ids).unwrap();
        let proven_leaves = file_proof.proven;
        for (id, leaf) in proven_leaves.iter().enumerate() {
            assert_eq!(chunk_ids[id], leaf.key);
            assert_eq!(chunks[id], leaf.data);
        }
    }
}
