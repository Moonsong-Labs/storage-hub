use std::{collections::HashMap, io, path::PathBuf, sync::Arc};

use hash_db::{AsHashDB, HashDB, Prefix};
use kvdb::{DBTransaction, KeyValueDB};
use kvdb_rocksdb::{Database, DatabaseConfig};
use log::debug;
use shc_common::types::{
    Chunk, ChunkId, FileMetadata, FileProof, HashT, HasherOutT, Leaf, H_LENGTH,
};
use sp_state_machine::{warn, Storage};
use sp_trie::{
    prefixed_key, recorder::Recorder, PrefixedMemoryDB, TrieDBBuilder, TrieLayout, TrieMut,
};
use trie_db::{DBValue, Hasher, Trie, TrieDBMutBuilder};

use crate::{
    error::ErrorT,
    traits::{FileDataTrie, FileStorage, FileStorageError, FileStorageWriteError},
    LOG_TARGET,
};

// TODO: maybe extract common types used in Forest Manager impls.
// TODO: create error module for File Manager / refactor errors
// TODO: Add comments
// And filedatatrie is ephemeral and created for each file proven

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

struct RocksDbFileDataTrie<T: TrieLayout> {
    // Persistent storage
    pub storage: Box<dyn Backend<T>>,
    // For staging Trie modifications that will be persisted
    pub overlay: PrefixedMemoryDB<HashT<T>>,
    // Root of the file Trie, which is a file key.
    pub root: HasherOutT<T>,
}

// TODO: double check this default method,
// check if Default trait is really required (why is it required in the InMemory impl?)
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
        Self {
            root: HasherOutT::<T>::default(),
            overlay: Default::default(),
            storage,
        }
    }

    /// Commits [`overlay`](`RocksDbFileDataTrie::overlay`) to [`storage`](`RocksDbFileDataTrie::storage`)
    /// This will write the changes applied to the overlay.
    /// If the root has not changed, the commit will be skipped.
    /// The `overlay` is drained during the operation.
    pub fn commit(&mut self, file_key: HasherOutT<T>) -> Result<(), ErrorT<T>> {
        let root = &self
            .storage
            .file_root(file_key)?
            .ok_or(FileStorageError::ExpectingRootToBeInStorage)?;

        // Skip commit if the root has not changed.
        if &self.root == root {
            warn!(target: LOG_TARGET, "Root has not changed, skipping commit");
            return Ok(());
        }

        // Aggregate changes from the overlay
        let mut transaction = self.changes();

        // Update the root
        transaction.put(0, file_key.as_ref(), self.root.as_ref());

        // Write the changes to storage
        self.storage.write(transaction)?;

        debug!(target: LOG_TARGET, "Committed changes to storage, new File Trie root: {:?}", self.root);

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

// Our own trait for interacting with the File storage.
pub trait Backend<T: TrieLayout>: Storage<HashT<T>>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    /// Write the transaction to the storage.
    fn write(&mut self, transaction: DBTransaction) -> Result<(), ErrorT<T>>;
    /// Get the file trie root from the storage.
    fn file_root(&self, file_key: HasherOutT<T>) -> Result<Option<HasherOutT<T>>, ErrorT<T>>;
}

impl<T: TrieLayout + Send + Sync> Backend<T> for StorageDb<HashT<T>>
where
    <T::Hash as Hasher>::Out: TryFrom<[u8; 32]>,
{
    fn write(&mut self, transaction: DBTransaction) -> Result<(), ErrorT<T>> {
        self.db.write(transaction).map_err(|e| {
            warn!(target: LOG_TARGET, "Failed to write to DB: {}", e);
            FileStorageError::FailedToWriteToStorage
        })?;

        Ok(())
    }

    fn file_root(&self, file_key: HasherOutT<T>) -> Result<Option<HasherOutT<T>>, ErrorT<T>> {
        let maybe_root = self.db.get(0, file_key.as_ref().into()).map_err(|e| {
            warn!(target: LOG_TARGET, "Failed to read root from DB: {}", e);
            FileStorageError::FailedToReadStorage
        })?;

        let root = maybe_root
            .map(|root| convert_raw_bytes_to_hasher_out::<T>(root))
            .transpose()?;

        Ok(root)
    }
}

impl<T: TrieLayout + Send + Sync> FileDataTrie<T> for RocksDbFileDataTrie<T>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    fn get_root(&self) -> &HasherOutT<T> {
        &self.root
    }

    fn stored_chunks_count(&self) -> u64 {
        let hash_db = self.as_hash_db();
        let trie = TrieDBBuilder::<T>::new(&hash_db, &self.root).build();
        let stored_chunks = trie.key_iter().iter().count();
        stored_chunks as u64
    }

    fn generate_proof(&self, chunk_ids: &Vec<ChunkId>) -> Result<FileProof, FileStorageError> {
        let recorder: Recorder<T::Hash> = Recorder::default();

        // A `TrieRecorder` is needed to create a proof of the "visited" leafs, by the end of this process.
        let mut trie_recorder = recorder.as_trie_recorder(self.root);

        let hash_db = self.as_hash_db();
        let trie = TrieDBBuilder::<T>::new(&hash_db, &self.root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Read all the chunks to prove from the trie.
        let mut chunks = Vec::new();
        for chunk_id in chunk_ids {
            let chunk: Option<Vec<u8>> = trie
                .get(&chunk_id.to_be_bytes())
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

        // Convert the chunks to prove into `Leaf`s.
        let leaves = chunks
            .into_iter()
            .map(|(id, chunk)| Leaf {
                key: id,
                data: chunk,
            })
            .collect();

        Ok(FileProof {
            proven: leaves,
            proof: proof.into(),
            root: self.get_root().as_ref().into(),
        })
    }

    fn get_chunk(&self, chunk_id: &ChunkId) -> Result<Chunk, FileStorageError> {
        let hash_db = self.as_hash_db();
        let trie = TrieDBBuilder::<T>::new(&hash_db, &self.root).build();

        trie.get(&chunk_id.to_be_bytes())
            .map_err(|_| FileStorageError::FailedToGetFileChunk)?
            .ok_or(FileStorageError::FileChunkDoesNotExist)
    }

    fn write_chunk(
        &mut self,
        chunk_id: &ChunkId,
        data: &Chunk,
    ) -> Result<(), FileStorageWriteError> {
        let mut root = self.root;
        let hash_db = self.as_hash_db_mut();
        let mut trie = TrieDBMutBuilder::<T>::new(hash_db, &mut root).build();

        // Check that we don't have a chunk already stored.
        if trie
            .contains(&chunk_id.to_be_bytes())
            .map_err(|_| FileStorageWriteError::FailedToGetFileChunk)?
        {
            return Err(FileStorageWriteError::FileChunkAlreadyExists);
        }

        // Insert the chunk into the file trie.
        trie.insert(&chunk_id.to_be_bytes(), &data)
            .map_err(|_| FileStorageWriteError::FailedToInsertFileChunk)?;

        // Drop trie to free `self`.
        drop(trie);

        // Update the root hash.
        self.root = root;

        // Commit changes from the overlay to the persistent storage.
        self.commit(root)
            .map_err(|_| FileStorageWriteError::FailedToInsertFileChunk)?;

        Ok(())
    }
}

impl<T: TrieLayout + Send + Sync> AsHashDB<HashT<T>, DBValue> for RocksDbFileDataTrie<T> {
    fn as_hash_db<'b>(&'b self) -> &'b (dyn HashDB<HashT<T>, DBValue> + 'b) {
        self
    }
    fn as_hash_db_mut<'b>(&'b mut self) -> &'b mut (dyn HashDB<HashT<T>, DBValue> + 'b) {
        &mut *self
    }
}

impl<T: TrieLayout + Send + Sync> hash_db::HashDB<HashT<T>, DBValue> for RocksDbFileDataTrie<T> {
    fn get(&self, key: &HasherOutT<T>, prefix: Prefix) -> Option<DBValue> {
        // TODO: maybe we dont need to get from the overlay here, only from persistent storage, double check.
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

struct RocksDbFileStorage<T: TrieLayout + 'static>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    pub metadata: HashMap<HasherOutT<T>, FileMetadata>,
    pub file_data: HashMap<HasherOutT<T>, RocksDbFileDataTrie<T>>,
}

impl<T: TrieLayout + Send + Sync> FileStorage<T> for RocksDbFileStorage<T>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    type FileDataTrie = RocksDbFileDataTrie<T>;

    fn delete_file(&mut self, key: &HasherOutT<T>) {
        todo!()
    }
    fn generate_proof(
        &self,
        key: &HasherOutT<T>,
        chunk_id: &Vec<ChunkId>,
    ) -> Result<FileProof, FileStorageError> {
        todo!()
    }
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
    fn insert_file(
        &mut self,
        key: HasherOutT<T>,
        metadata: FileMetadata,
    ) -> Result<(), FileStorageError> {
        todo!()
    }
    fn insert_file_with_data(
        &mut self,
        key: HasherOutT<T>,
        metadata: FileMetadata,
        file_data: Self::FileDataTrie,
    ) -> Result<(), FileStorageError> {
        todo!()
    }
    fn write_chunk(
        &mut self,
        key: &HasherOutT<T>,
        chunk_id: &ChunkId,
        data: &Chunk,
    ) -> Result<crate::traits::FileStorageWriteOutcome, FileStorageWriteError> {
        todo!()
    }
}
