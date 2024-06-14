use std::{collections::HashMap, io, path::PathBuf, sync::Arc};

use hash_db::{AsHashDB, HashDB, Prefix};
use kvdb::{DBTransaction, KeyValueDB};
use kvdb_rocksdb::{Database, DatabaseConfig};
use log::{debug, error};
use shc_common::types::{
    Chunk, ChunkId, FileMetadata, FileProof, HashT, HasherOutT, Leaf, H_LENGTH,
};
use sp_state_machine::{warn, Storage};
use sp_trie::{
    prefixed_key, recorder::Recorder, PrefixedKey, PrefixedMemoryDB, TrieDBBuilder, TrieLayout,
    TrieMut,
};
use trie_db::{DBValue, Hasher, Trie, TrieDBMutBuilder};

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
// TODO: maybe use different columns in RocksDB for each file data trie?
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
    // Root of the file Trie, which is a file key.
    root: HasherOutT<T>,
    // Maintains relationship between external chunk key representation (integer index)
    // and internal chunk key representation (hash of value)
    inner_chunk_keys: HashMap<ChunkId, HasherOutT<T>>,
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
        Self {
            root: HasherOutT::<T>::default(),
            storage,
            inner_chunk_keys: HashMap::new(),
        }
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
// automatically commits to the underlaying `db`
impl<T: TrieLayout + Send + Sync> FileDataTrie<T> for RocksDbFileDataTrie<T>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    fn get_root(&self) -> &HasherOutT<T> {
        &self.root
    }

    fn stored_chunks_count(&self) -> u64 {
        let db: &dyn HashDB<<T as TrieLayout>::Hash, Vec<u8>> = self.as_hash_db();
        let trie = TrieDBBuilder::<T>::new(&db, &self.root).build();
        let stored_chunks = trie.key_iter().iter().count();
        stored_chunks as u64
    }

    fn generate_proof(&self, chunk_ids: &Vec<ChunkId>) -> Result<FileProof, FileStorageError> {
        let db: &dyn HashDB<<T as TrieLayout>::Hash, Vec<u8>> = self.as_hash_db();
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
                .inner_chunk_keys
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
        let db = self.as_hash_db();
        let trie = TrieDBBuilder::<T>::new(&db, &self.root).build();

        let chunk_key = self
            .inner_chunk_keys
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
        let chunk_key = T::Hash::hash(data);
        self.inner_chunk_keys.insert(*chunk_id, chunk_key);

        let mut root = self.root;
        let db = self.as_hash_db_mut();

        let mut trie = TrieDBMutBuilder::<T>::new(db, &mut root).build();

        // Check that we don't have a chunk already stored.
        if trie
            .contains(&chunk_key.as_ref())
            .map_err(|_| FileStorageWriteError::FailedToGetFileChunk)?
        {
            return Err(FileStorageWriteError::FileChunkAlreadyExists);
        }

        // Insert the chunk into the file trie.
        trie.insert(&chunk_key.as_ref(), &data)
            .map_err(|_| FileStorageWriteError::FailedToInsertFileChunk)?;

        // get new root so we can update it internally
        let new_root = *trie.root();

        // drop trie to commit to underlying db and release `self`
        drop(trie);

        // update internal root
        self.root = new_root;

        Ok(())
    }

    // Deletes the Trie from the underlying Db.
    fn delete(&mut self) -> Result<(), FileStorageError> {
        let mut root = self.root;
        // Need to clone because we cannot have a immutable borrow after mutably borrowing
        // in the next step.
        let trie_root_key = root.clone();
        let mut trie =
            TrieDBMutBuilder::<T>::from_existing(self.as_hash_db_mut(), &mut root).build();

        // Remove the file key from the trie.
        trie.remove(trie_root_key.as_ref()).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to delete File Trie from RocksDb: {}", e);
            FileStorageError::FailedToWriteToStorage
        })?;

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
        self.storage.get(key, prefix).unwrap_or_else(|e| {
            warn!(target: LOG_TARGET, "Failed to read from DB: {}", e);
            None
        })
    }

    fn contains(&self, key: &HasherOutT<T>, prefix: Prefix) -> bool {
        self.get(key, prefix).is_some()
    }

    fn insert(&mut self, _prefix: Prefix, value: &[u8]) -> HasherOutT<T> {
        let mut transaction = DBTransaction::new();
        let key: HasherOutT<T> = T::Hash::hash(value);
        transaction.put(0, key.as_ref(), value);

        // bubble up error from `write()` method
        if let Err(e) = self.storage.write(transaction) {
            panic!("{}", e)
        };

        key
    }

    fn emplace(&mut self, key: HasherOutT<T>, _prefix: Prefix, value: DBValue) {
        let mut transaction = DBTransaction::new();
        transaction.put(0, key.as_ref(), &value);

        // bubble up error from `write()` method
        if let Err(e) = self.storage.write(transaction) {
            panic!("{}", e)
        };
    }

    fn remove(&mut self, key: &HasherOutT<T>, _prefix: Prefix) {
        let mut transaction = DBTransaction::new();
        transaction.delete(0, key.as_ref());

        // bubble up error from `write()` method
        if let Err(e) = self.storage.write(transaction) {
            panic!("{}", e)
        };
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
                "Invariant broken! Metadata for file key {:?} not found but associated trie is present",
                key
            )
                .as_str(),
            );

        // Check if we have all the chunks for the file.
        if metadata.chunks_count() != file_data.stored_chunks_count() {
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
        chunk_id: &Vec<ChunkId>,
    ) -> Result<FileProof, FileStorageError> {
        let metadata = self
            .metadata
            .get(key)
            .ok_or(FileStorageError::FileDoesNotExist)?;

        // TODO: use better error
        let file_data = self.file_data.get(key).expect(
            format!("Key {:?} already associated with File Metadata, but no File Trie. Possible inconsistency between them.",
                key
            )
            .as_str(),
        );

        if metadata.chunks_count() != file_data.stored_chunks_count() {
            return Err(FileStorageError::IncompleteFile);
        }

        if metadata.fingerprint
            != file_data
                .root
                .as_ref()
                .try_into()
                .expect("Hasher output mismatch!")
        {
            return Err(FileStorageError::FingerprintAndStoredFileMismatch);
        }

        file_data.generate_proof(chunk_id)
    }

    fn delete_file(&mut self, key: &HasherOutT<T>) -> Result<(), FileStorageError> {
        // TODO: should FileStorage also have an access point to RocksDb,
        // or should we create a delete() method for each FileDataTrie?
        // In the first case, seems cleaner, but then we would lose the separation
        // between the file data trie and file storage structures.
        let file_data = self.file_data.get_mut(key).expect("No File data found");

        file_data.delete()?;

        self.metadata.remove(key);
        self.file_data.remove(key);

        Ok(())
    }
}

// Utils
// TODO: maybe unify errors from FOrest and File Storages in one enum
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
    #[test]
    fn it_works() {}
}
