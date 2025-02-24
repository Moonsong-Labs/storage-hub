use log::info;
use std::{collections::HashSet, io, path::PathBuf, sync::Arc};

use hash_db::{AsHashDB, HashDB, Prefix};
use kvdb::{DBTransaction, KeyValueDB};
use log::{debug, error};
use shc_common::types::{
    Chunk, ChunkId, ChunkWithId, FileKeyProof, FileMetadata, FileProof, HashT, HasherOutT, H_LENGTH,
};
use sp_state_machine::{warn, Storage};
use sp_trie::{prefixed_key, recorder::Recorder, PrefixedMemoryDB, TrieLayout, TrieMut};
use trie_db::{DBValue, Trie, TrieDBBuilder, TrieDBMutBuilder};

use crate::{
    error::{other_io_error, ErrorT},
    traits::{
        ExcludeType, FileDataTrie, FileStorage, FileStorageError, FileStorageWriteError,
        FileStorageWriteOutcome,
    },
    LOG_TARGET,
};
use codec::{Decode, Encode};
use strum::EnumCount;

#[derive(Debug, Clone, Copy, EnumCount)]
pub enum Column {
    /// Stores keys of 32 bytes representing the `file_key` with values being the serialized [`FileMetadata`].
    Metadata,
    /// Stores keys of 32 bytes representing the final `root` of the file based on the [`FileMetadata::fingerprint`] with values
    /// being the current `root` of the constructed file trie based on the chunks stored in the [`Column::Chunks`] for that `file_key`.
    ///
    /// Used for keeping track of the current root of the file Trie for each `file_key`.
    Roots,
    /// Stores keys of 32 bytes representing the `file_key`.
    ///
    /// Used for storing the chunks of the file.
    Chunks,
    /// Stores keys of 32 bytes representing the `file_key`.
    ///
    /// Used for counting the number of chunks currently stored for the `file_key`.
    ChunkCount,
    /// Stores keys of 64 bytes representing the concatenation of `bucket_id` and `file_key`.
    ///
    /// Used for deleting all files in a bucket efficiently.
    BucketPrefix,
    /// Exclude* columns stores keys of 32 bytes representing the `file_key` with empty values.
    ///
    /// These columns are used primarily to mark file keys as being excluded from certain operations.
    ExcludeFile,
    ExcludeUser,
    ExcludeBucket,
    ExcludeFingerprint,
}

impl Into<u32> for Column {
    fn into(self) -> u32 {
        self as u32
    }
}

// Replace NUMBER_OF_COLUMNS definition
const NUMBER_OF_COLUMNS: u32 = Column::COUNT as u32;

// Helper function to map ExcludeType enum to their matching rocksdb column.
fn get_exclude_type_db_column(exclude_type: ExcludeType) -> u32 {
    match exclude_type {
        ExcludeType::File => Column::ExcludeFile.into(),
        ExcludeType::User => Column::ExcludeUser.into(),
        ExcludeType::Bucket => Column::ExcludeBucket.into(),
        ExcludeType::Fingerprint => Column::ExcludeFingerprint.into(),
    }
}

/// Open the database on disk, creating it if it doesn't exist.
fn open_or_creating_rocksdb(db_path: String) -> io::Result<kvdb_rocksdb::Database> {
    let mut path = PathBuf::new();
    path.push(db_path.as_str());
    path.push("storagehub/file_storage/");

    let db_config = kvdb_rocksdb::DatabaseConfig::with_columns(NUMBER_OF_COLUMNS);

    let path_str = path
        .to_str()
        .ok_or_else(|| other_io_error(format!("Bad database path: {:?}", path)))?;

    std::fs::create_dir_all(&path_str)?;
    let db = kvdb_rocksdb::Database::open(&db_config, &path_str)?;

    Ok(db)
}

/// Storage backend implementation for RocksDB.
/// Provides low-level storage operations for the file system.
pub struct StorageDb<T, DB> {
    pub db: Arc<DB>,
    pub _marker: std::marker::PhantomData<T>,
}

impl<T, DB> StorageDb<T, DB>
where
    T: TrieLayout,
    DB: KeyValueDB,
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    /// Writes a transaction to the database.
    /// Returns an error if the write operation fails.
    fn write(&mut self, transaction: DBTransaction) -> Result<(), ErrorT<T>> {
        self.db.write(transaction).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to write to DB: {}", e);
            FileStorageError::FailedToWriteToStorage
        })?;

        Ok(())
    }

    /// Reads data from the specified column and key.
    /// Returns the value if found or None if the key doesn't exist.
    fn read(&self, column: u32, key: &[u8]) -> Result<Option<Vec<u8>>, ErrorT<T>> {
        let value = self.db.get(column, key.as_ref()).map_err(|e| {
            warn!(target: LOG_TARGET, "Failed to read from DB: {}", e);
            FileStorageError::FailedToReadStorage
        })?;

        Ok(value)
    }
}

impl<T, DB> Clone for StorageDb<T, DB> {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            _marker: self._marker,
        }
    }
}

impl<T: TrieLayout + Send + Sync, DB: KeyValueDB> Storage<HashT<T>> for StorageDb<T, DB> {
    fn get(&self, key: &HasherOutT<T>, prefix: Prefix) -> Result<Option<DBValue>, String> {
        let prefixed_key = prefixed_key::<HashT<T>>(key, prefix);
        self.db
            .get(Column::Chunks.into(), &prefixed_key)
            .map_err(|e| {
                warn!(target: LOG_TARGET, "Failed to read from DB: {}", e);
                format!("Failed to read from DB: {}", e)
            })
    }
}

/// Converts raw bytes into a [`HasherOutT<T>`].
fn convert_raw_bytes_to_hasher_out<T>(key: Vec<u8>) -> Result<HasherOutT<T>, ErrorT<T>>
where
    T: TrieLayout,
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    let key: [u8; 32] = key.try_into().map_err(|e| {
        error!(target: LOG_TARGET, "{:?}", e);
        FileStorageError::FailedToHasherOutput
    })?;

    let key = HasherOutT::<T>::try_from(key).map_err(|_| {
        error!(target: LOG_TARGET, "Failed to parse hasher output from DB");
        FileStorageError::FailedToHasherOutput
    })?;

    Ok(key)
}

/// File data trie implementation using RocksDB for persistent storage.
/// Manages file chunks and their proofs in a merkle trie structure.
pub struct RocksDbFileDataTrie<T: TrieLayout, DB> {
    // Persistent storage.
    storage: StorageDb<T, DB>,
    // In memory overlay used for Trie operations.
    overlay: PrefixedMemoryDB<HashT<T>>,
    // Root of the file Trie, which is the file fingerprint.
    root: HasherOutT<T>,
}

impl<T, DB> RocksDbFileDataTrie<T, DB>
where
    T: TrieLayout + Send + Sync,
    DB: KeyValueDB,
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    /// Creates a new empty file data trie.
    fn new(storage: StorageDb<T, DB>) -> Self {
        let (overlay, root) = PrefixedMemoryDB::<HashT<T>>::default_with_root();

        RocksDbFileDataTrie::<T, DB> {
            storage,
            root,
            overlay,
        }
    }

    /// Creates a file data trie from existing root and storage.
    fn from_existing(storage: StorageDb<T, DB>, root: &HasherOutT<T>) -> Self {
        RocksDbFileDataTrie::<T, DB> {
            root: *root,
            storage,
            overlay: Default::default(),
        }
    }

    /// Commits changes in the overlay to persistent storage.
    /// Skips if root hasn't changed. Clears the overlay after commit.
    pub fn commit(&mut self, new_root: HasherOutT<T>) -> Result<(), ErrorT<T>> {
        // Skip commit if the root has not changed.
        if self.root == new_root {
            warn!(target: LOG_TARGET, "Root has not changed, skipping commit");
            return Ok(());
        }

        // Aggregate changes from the overlay
        let transaction = self.changes();

        // Write the changes to storage
        self.storage.write(transaction)?;

        self.root = new_root;

        debug!(target: LOG_TARGET, "Committed changes to storage, new root: {:?}", self.root);

        Ok(())
    }

    /// Builds a database transaction from the overlay and clears it.
    fn changes(&mut self) -> DBTransaction {
        let mut transaction = DBTransaction::new();

        for (key, (value, rc)) in self.overlay.drain() {
            if rc <= 0 {
                transaction.delete(Column::Chunks.into(), &key);
            } else {
                transaction.put_vec(Column::Chunks.into(), &key, value);
            }
        }

        transaction
    }

    /// Open the RocksDB database at `db_path` and return a new instance of [`StorageDb`].
    pub fn rocksdb_storage(
        db_path: String,
    ) -> Result<StorageDb<T, kvdb_rocksdb::Database>, ErrorT<T>> {
        let db = open_or_creating_rocksdb(db_path).map_err(|e| {
            warn!(target: LOG_TARGET, "Failed to open RocksDB: {}", e);
            FileStorageError::FailedToReadStorage
        })?;

        Ok(StorageDb {
            db: Arc::new(db),
            _marker: Default::default(),
        })
    }
}

// As a reminder, dropping the trie (either by calling `drop()` or by the end of the scope)
// automatically commits to the underlying db.
impl<T, DB> FileDataTrie<T> for RocksDbFileDataTrie<T, DB>
where
    T: TrieLayout + Send + Sync,
    DB: KeyValueDB,
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    /// Returns the current root hash of the trie.
    fn get_root(&self) -> &HasherOutT<T> {
        &self.root
    }

    // Generates a [`FileProof`] for requested chunks.
    fn generate_proof(&self, chunk_ids: &HashSet<ChunkId>) -> Result<FileProof, FileStorageError> {
        let db = self.as_hash_db();
        let recorder: Recorder<T::Hash> = Recorder::default();

        // A `TrieRecorder` is needed to create a proof of the "visited" leafs, by the end of this process.
        let mut trie_recorder = recorder.as_trie_recorder(self.root);

        let trie = TrieDBBuilder::<T>::new(&db, &self.root)
            .with_recorder(&mut trie_recorder)
            .build();

        // We read all the chunks to prove from the trie.
        // This is step is required to actually record the proof.
        let mut chunks = Vec::new();
        for chunk_id in chunk_ids {
            // Get the encoded chunk from the trie.
            let encoded_chunk: Vec<u8> = trie
                .get(&chunk_id.as_trie_key())
                .map_err(|e| {
                    error!(target: LOG_TARGET, "Failed to find file chunk in File Trie {}", e);
                    FileStorageError::FailedToGetFileChunk
                })?
                .ok_or(FileStorageError::FileChunkDoesNotExist)?;

            // Decode it to its chunk ID and data.
            let decoded_chunk = ChunkWithId::decode(&mut encoded_chunk.as_slice())
                .map_err(|_| FileStorageError::FailedToParseChunkWithId)?;

            chunks.push((decoded_chunk.chunk_id, decoded_chunk.data));
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

    // TODO: make it accept a list of chunks to be retrieved
    /// Retrieves a chunk from the trie by its ID.
    /// Returns error if chunk doesn't exist or retrieval fails.
    fn get_chunk(&self, chunk_id: &ChunkId) -> Result<Chunk, FileStorageError> {
        let db = self.as_hash_db();
        let trie = TrieDBBuilder::<T>::new(&db, &self.root).build();

        // Get the encoded chunk from the trie.
        let encoded_chunk: Vec<u8> = trie
            .get(&chunk_id.as_trie_key())
            .map_err(|e| {
                error!(target: LOG_TARGET, "{}", e);
                FileStorageError::FailedToGetFileChunk
            })?
            .ok_or(FileStorageError::FileChunkDoesNotExist)?;

        // Decode it to its chunk ID and data.
        let decoded_chunk = ChunkWithId::decode(&mut encoded_chunk.as_slice())
            .map_err(|_| FileStorageError::FailedToParseChunkWithId)?;

        // Return the data.
        Ok(decoded_chunk.data)
    }

    // TODO: make it accept a list of chunks to be written
    /// Writes a chunk to the trie with its ID.
    /// Returns error if write fails or chunk already exists.
    fn write_chunk(
        &mut self,
        chunk_id: &ChunkId,
        data: &Chunk,
    ) -> Result<(), FileStorageWriteError> {
        let mut current_root = self.root;
        let db = self.as_hash_db_mut();
        let mut trie = TrieDBMutBuilder::<T>::from_existing(db, &mut current_root).build();

        // Check that we don't have a chunk already stored.
        if trie.contains(&chunk_id.as_trie_key()).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to fetch chunk: {}", e);
            FileStorageWriteError::FailedToGetFileChunk
        })? {
            return Err(FileStorageWriteError::FileChunkAlreadyExists);
        }

        // Insert the encoded chunk with its ID into the file trie.
        let decoded_chunk = ChunkWithId {
            chunk_id: *chunk_id,
            data: data.clone(),
        };
        let encoded_chunk = decoded_chunk.encode();
        trie.insert(&chunk_id.as_trie_key(), &encoded_chunk)
            .map_err(|e| {
                error!(target: LOG_TARGET, "{}", e);
                FileStorageWriteError::FailedToInsertFileChunk
            })?;

        // Get new root after trie modifications
        let new_root = *trie.root();

        // Drop trie to commit to underlying db and release `self`
        drop(trie);

        // TODO: improve error handling
        // Commit the changes to disk.
        self.commit(new_root).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to commit changes to persistent storage: {}", e);
            FileStorageWriteError::FailedToPersistChanges
        })?;

        Ok(())
    }

    /// Deletes all chunks and data associated with this file trie.
    fn delete(&mut self) -> Result<(), FileStorageWriteError> {
        let mut root = self.root;
        let db = self.as_hash_db_mut();
        let trie_root_key = root;
        let mut trie = TrieDBMutBuilder::<T>::from_existing(db, &mut root).build();

        let mut chunk_id = 0;
        loop {
            let chunk_id_struct = ChunkId::new(chunk_id as u64);
            if !trie.contains(&chunk_id_struct.as_trie_key()).map_err(|e| {
                error!(target: LOG_TARGET, "Failed to check if chunk exists: {}", e);
                FileStorageWriteError::FailedToDeleteChunk
            })? {
                break;
            }

            trie.remove(&chunk_id_struct.as_trie_key()).map_err(|e| {
                error!(target: LOG_TARGET, "Failed to delete chunk from RocksDb: {}", e);
                FileStorageWriteError::FailedToDeleteChunk
            })?;

            chunk_id += 1;
        }

        // Remove the root from the trie.
        trie.remove(trie_root_key.as_ref()).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to delete root from RocksDb: {}", e);
            FileStorageWriteError::FailedToDeleteRoot
        })?;

        let new_root = *trie.root();

        drop(trie);

        // TODO: improve error handling
        // Commit the changes to disk.
        self.commit(new_root).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to commit changes to persistent storage: {}", e);
            FileStorageWriteError::FailedToPersistChanges
        })?;

        // Set new internal root (empty trie root)
        self.root = new_root;

        Ok(())
    }
}

impl<T, DB> AsHashDB<HashT<T>, DBValue> for RocksDbFileDataTrie<T, DB>
where
    T: TrieLayout + Send + Sync,
    DB: KeyValueDB,
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    fn as_hash_db<'b>(&'b self) -> &'b (dyn HashDB<HashT<T>, DBValue> + 'b) {
        self
    }
    fn as_hash_db_mut<'b>(&'b mut self) -> &'b mut (dyn HashDB<HashT<T>, DBValue> + 'b) {
        &mut *self
    }
}

impl<T, DB> hash_db::HashDB<HashT<T>, DBValue> for RocksDbFileDataTrie<T, DB>
where
    T: TrieLayout + Send + Sync,
    DB: KeyValueDB,
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

/// Manages file metadata, chunks, and proofs using RocksDB as backend.
pub struct RocksDbFileStorage<T, DB>
where
    T: TrieLayout + 'static,
    DB: KeyValueDB,
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    storage: StorageDb<T, DB>,
}

impl<T: TrieLayout, DB> RocksDbFileStorage<T, DB>
where
    T: TrieLayout,
    DB: KeyValueDB,
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    /// Creates a new file storage instance with the given storage backend.
    pub fn new(storage: StorageDb<T, DB>) -> Self {
        Self { storage }
    }

    /// Open the RocksDB database at `db_path` and return a new instance of [`StorageDb`].
    pub fn rocksdb_storage(
        db_path: String,
    ) -> Result<StorageDb<T, kvdb_rocksdb::Database>, ErrorT<T>> {
        let db = open_or_creating_rocksdb(db_path).map_err(|e| {
            warn!(target: LOG_TARGET, "Failed to open RocksDB: {}", e);
            FileStorageError::FailedToReadStorage
        })?;

        Ok(StorageDb {
            db: Arc::new(db),
            _marker: Default::default(),
        })
    }

    /// Constructs a [`RocksDbFileDataTrie`] from the given [`FileMetadata`].
    ///
    /// Since files can be partially uploaded (i.e. not all chunks have been inserted to result in the root being the file metadata's fingerprint),
    /// the constructed trie is based on the current `partial_root` representing the current state of the file we are interested in.
    fn get_file_trie(
        &self,
        metadata: &FileMetadata,
    ) -> Result<RocksDbFileDataTrie<T, DB>, FileStorageError>
    where
        T: TrieLayout + Send + Sync + 'static,
        DB: KeyValueDB + 'static,
    {
        let b_fingerprint = metadata.fingerprint.as_ref();
        let h_fingerprint =
            convert_raw_bytes_to_hasher_out::<T>(b_fingerprint.to_vec()).map_err(|e| {
                error!(target: LOG_TARGET, "{:?}", e);
                FileStorageError::FailedToParseFingerprint
            })?;
        let raw_partial_root = self
            .storage
            .read(Column::Roots.into(), h_fingerprint.as_ref())
            .map_err(|e| {
                error!(target: LOG_TARGET, "{:?}", e);
                FileStorageError::FailedToReadStorage
            })?
            .expect("Failed to find partial root");
        let mut partial_root =
            convert_raw_bytes_to_hasher_out::<T>(raw_partial_root).map_err(|e| {
                error!(target: LOG_TARGET, "{:?}", e);
                FileStorageError::FailedToParsePartialRoot
            })?;
        let file_trie =
            RocksDbFileDataTrie::<T, DB>::from_existing(self.storage.clone(), &mut partial_root);
        Ok(file_trie)
    }
}

impl<T, DB> FileStorage<T> for RocksDbFileStorage<T, DB>
where
    T: TrieLayout + Send + Sync + 'static,
    DB: KeyValueDB + 'static,
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    type FileDataTrie = RocksDbFileDataTrie<T, DB>;

    /// Creates a new empty file data trie instance.
    fn new_file_data_trie(&self) -> Self::FileDataTrie {
        RocksDbFileDataTrie::new(self.storage.clone())
    }

    /// Retrieves a chunk by file key and chunk ID.
    fn get_chunk(
        &self,
        file_key: &HasherOutT<T>,
        chunk_id: &ChunkId,
    ) -> Result<Chunk, FileStorageError> {
        let metadata = self
            .get_metadata(file_key)?
            .ok_or(FileStorageError::FileDoesNotExist)?;

        let file_trie = self.get_file_trie(&metadata)?;

        file_trie.get_chunk(chunk_id)
    }

    /// Returns the number of chunks currently stored for a given file key tracked by [`CHUNK_COUNT_COLUMN`].
    fn stored_chunks_count(&self, file_key: &HasherOutT<T>) -> Result<u64, FileStorageError> {
        // Read from CHUNK_COUNT_COLUMN using the file key
        let current_count = self
            .storage
            .read(Column::ChunkCount.into(), file_key.as_ref())
            .map_err(|e| {
                error!(target: LOG_TARGET, "{:?}", e);
                FileStorageError::FailedToReadStorage
            })?
            .map(|bytes| u64::from_le_bytes(bytes.try_into().unwrap()))
            .unwrap_or(0);

        Ok(current_count)
    }

    /// Writes a chunk to storage with file key and chunk ID.
    ///
    /// Returns [`FileStorageWriteOutcome`] indicating if file is complete. This outcome is based on
    /// the current number of chunks stored (tracked by [`CHUNK_COUNT_COLUMN`]) and the file metadata's [`FileMetadata::chunks_count`].
    fn write_chunk(
        &mut self,
        file_key: &HasherOutT<T>,
        chunk_id: &ChunkId,
        data: &Chunk,
    ) -> Result<FileStorageWriteOutcome, FileStorageWriteError> {
        let metadata = self
            .get_metadata(file_key)
            .map_err(|_| FileStorageWriteError::FailedToParseFileMetadata)?
            .ok_or(FileStorageWriteError::FileDoesNotExist)?;

        let mut file_trie = self.get_file_trie(&metadata).map_err(|e| {
            error!(target: LOG_TARGET, "{:?}", e);
            FileStorageWriteError::FailedToContructFileTrie
        })?;

        file_trie.write_chunk(chunk_id, data).map_err(|e| {
            error!(target: LOG_TARGET, "{:?}", e);
            FileStorageWriteError::FailedToInsertFileChunk
        })?;

        // Update partial root.
        let new_partial_root = file_trie.get_root();
        let mut transaction = DBTransaction::new();
        transaction.put(
            Column::Roots.into(),
            metadata.fingerprint.as_ref(),
            new_partial_root.as_ref(),
        );

        // Get current chunk count or initialize to 0
        let current_count = self.stored_chunks_count(file_key).map_err(|e| {
            error!(target: LOG_TARGET, "{:?}", e);
            FileStorageWriteError::FailedToGetStoredChunksCount
        })?;

        // Increment chunk count.
        // This should never overflow unless there is a bug or we support file sizes as large as 16 exabytes.
        // Since this is executed within the context of a write lock in the layer above, we should not have any chunk count syncing issues.
        let new_count = current_count
            .checked_add(1)
            .ok_or(FileStorageWriteError::ChunkCountOverflow)?;
        transaction.put(
            Column::ChunkCount.into(),
            file_key.as_ref(),
            &new_count.to_le_bytes(),
        );

        self.storage.write(transaction).map_err(|e| {
            error!(target: LOG_TARGET,"{:?}", e);
            FileStorageWriteError::FailedToUpdatePartialRoot
        })?;

        // Check if we have all the chunks for the file using the count
        if metadata.chunks_count() != new_count {
            return Ok(FileStorageWriteOutcome::FileIncomplete);
        }

        let current_fingerprint = file_trie.get_root().as_ref().try_into().map_err(|_| {
            error!(target: LOG_TARGET, "Failed to convert root to fingerprint");
            FileStorageWriteError::FailedToParseFingerprint
        })?;

        // Verify that the final root matches the expected fingerprint
        if metadata.fingerprint != current_fingerprint {
            error!(
                target: LOG_TARGET,
                "Fingerprint mismatch. Expected: {:?}, got: {:?}",
                metadata.fingerprint,
                file_trie.get_root()
            );
            return Err(FileStorageWriteError::FingerprintAndStoredFileMismatch);
        }

        Ok(FileStorageWriteOutcome::FileComplete)
    }

    /// Checks if all chunks are stored for a given file key.
    fn is_file_complete(&self, file_key: &HasherOutT<T>) -> Result<bool, FileStorageError> {
        let metadata = self
            .get_metadata(file_key)?
            .ok_or(FileStorageError::FileDoesNotExist)?;

        let stored_chunks = self.stored_chunks_count(file_key)?;

        let file_trie = self.get_file_trie(&metadata)?;

        if metadata.fingerprint
            != file_trie
                .get_root()
                .as_ref()
                .try_into()
                .expect("Hasher output mismatch!")
        {
            return Ok(false);
        }

        Ok(metadata.chunks_count() == stored_chunks)
    }

    /// Stores file metadata with an empty root.
    /// Should be used before writing any chunks using [`Self::write_chunk`].
    fn insert_file(
        &mut self,
        file_key: HasherOutT<T>,
        metadata: FileMetadata,
    ) -> Result<(), FileStorageError> {
        let mut transaction = DBTransaction::new();
        let serialized_metadata = serde_json::to_vec(&metadata).map_err(|e| {
            error!(target: LOG_TARGET,"{:?}", e);
            FileStorageError::FailedToParseFileMetadata
        })?;

        let (_, empty_root) = PrefixedMemoryDB::<HashT<T>>::default_with_root();
        transaction.put(
            Column::Metadata.into(),
            file_key.as_ref(),
            &serialized_metadata,
        );
        // Stores an empty root to allow for later initialization of the trie.
        transaction.put(
            Column::Roots.into(),
            metadata.fingerprint.as_ref(),
            empty_root.as_ref(),
        );
        // Initialize chunk count to 0
        transaction.put(
            Column::ChunkCount.into(),
            file_key.as_ref(),
            &0u64.to_le_bytes(),
        );

        self.storage.write(transaction).map_err(|e| {
            error!(target: LOG_TARGET,"{:?}", e);
            FileStorageError::FailedToWriteToStorage
        })?;

        Ok(())
    }

    /// Stores file information with its (partial or final) root.
    /// Should be used if any chunks have already been written.
    /// Otherwise use [`Self::insert_file`].
    ///
    /// This is an expensive operation since it assumes that the file chunks were written
    /// via the [`RocksDbFileDataTrie::write_chunk`] method instead of [`Self::write_chunk`] and
    /// therefore iterates over all keys in `file_data` to count the number of chunks and update
    /// the chunk count in the [`CHUNK_COUNT_COLUMN`] column. This data is necessary to
    /// [`Self::generate_proof`]s for the file.
    fn insert_file_with_data(
        &mut self,
        file_key: HasherOutT<T>,
        metadata: FileMetadata,
        file_data: Self::FileDataTrie,
    ) -> Result<(), FileStorageError> {
        let raw_metadata = serde_json::to_vec(&metadata).map_err(|e| {
            error!(target: LOG_TARGET,"{:?}", e);
            FileStorageError::FailedToParseFileMetadata
        })?;

        let mut transaction = DBTransaction::new();

        transaction.put(Column::Metadata.into(), file_key.as_ref(), &raw_metadata);

        // Stores the current root of the trie.
        // if the file is complete, key and value will be equal.
        transaction.put(
            Column::Roots.into(),
            metadata.fingerprint.as_ref(),
            file_data.get_root().as_ref(),
        );

        let mem_db = file_data.as_hash_db();
        let trie = TrieDBBuilder::<T>::new(&mem_db, file_data.get_root()).build();

        let chunk_count = trie
            .iter()
            .map_err(|e| {
                error!(target: LOG_TARGET, "Failed to construct Trie iterator: {}", e);
                FileStorageError::FailedToConstructTrieIter
            })?
            .count();

        transaction.put(
            Column::ChunkCount.into(),
            file_key.as_ref(),
            &chunk_count.to_le_bytes(),
        );

        let bucket_prefixed_file_key = metadata
            .bucket_id
            .into_iter()
            .chain(file_key.as_ref().into_iter().cloned())
            .collect::<Vec<_>>();

        // Store the key prefixed by bucket id
        transaction.put(
            Column::BucketPrefix.into(),
            bucket_prefixed_file_key.as_ref(),
            &[],
        );

        self.storage.write(transaction).map_err(|e| {
            error!(target: LOG_TARGET,"{:?}", e);
            FileStorageError::FailedToWriteToStorage
        })?;

        Ok(())
    }

    /// Retrieves file metadata by file key.
    fn get_metadata(
        &self,
        file_key: &HasherOutT<T>,
    ) -> Result<Option<FileMetadata>, FileStorageError> {
        let raw_metadata = self
            .storage
            .read(Column::Metadata.into(), file_key.as_ref())
            .map_err(|e| {
                error!(target: LOG_TARGET,"{:?}", e);
                FileStorageError::FailedToReadStorage
            })?;
        match raw_metadata {
            None => return Ok(None),
            Some(metadata) => {
                let metadata: FileMetadata = serde_json::from_slice(&metadata).map_err(|e| {
                    error!(target: LOG_TARGET,"{:?}", e);
                    FileStorageError::FailedToParseFileMetadata
                })?;
                Ok(Some(metadata))
            }
        }
    }

    /// Generates a proof for specified chunks of a file.
    ///
    /// Returns error if file is incomplete or proof generation fails.
    fn generate_proof(
        &self,
        key: &HasherOutT<T>,
        chunk_ids: &HashSet<ChunkId>,
    ) -> Result<FileKeyProof, FileStorageError> {
        let metadata = self
            .get_metadata(key)?
            .ok_or(FileStorageError::FileDoesNotExist)?;

        let file_trie = self.get_file_trie(&metadata)?;

        let stored_chunks = self.stored_chunks_count(key)?;
        if metadata.chunks_count() != stored_chunks {
            return Err(FileStorageError::IncompleteFile);
        }

        if metadata.fingerprint
            != file_trie
                .get_root()
                .as_ref()
                .try_into()
                .expect("Hasher output mismatch!")
        {
            return Err(FileStorageError::FingerprintAndStoredFileMismatch);
        }

        Ok(file_trie
            .generate_proof(chunk_ids)?
            .to_file_key_proof(metadata.clone()))
    }

    /// Deletes a file and all its associated data.
    fn delete_file(&mut self, file_key: &HasherOutT<T>) -> Result<(), FileStorageError> {
        let metadata = self
            .get_metadata(file_key)?
            .ok_or(FileStorageError::FileDoesNotExist)?;

        let b_fingerprint = metadata.fingerprint.as_ref();
        let h_fingerprint =
            convert_raw_bytes_to_hasher_out::<T>(b_fingerprint.to_vec()).map_err(|e| {
                error!(target: LOG_TARGET, "{:?}", e);
                FileStorageError::FailedToParseFingerprint
            })?;

        let mut file_trie = self.get_file_trie(&metadata)?;

        file_trie.delete().map_err(|e| {
            error!(target: LOG_TARGET,"{:?}", e);
            FileStorageError::FailedToDeleteFileChunk
        })?;

        let mut transaction = DBTransaction::new();

        transaction.delete(Column::Metadata.into(), file_key.as_ref());
        transaction.delete(Column::Roots.into(), h_fingerprint.as_ref());
        transaction.delete(Column::ChunkCount.into(), file_key.as_ref());

        let bucket_prefixed_file_key = metadata
            .bucket_id
            .into_iter()
            .chain(file_key.as_ref().iter().cloned())
            .collect::<Vec<_>>();
        transaction.delete(
            Column::BucketPrefix.into(),
            bucket_prefixed_file_key.as_ref(),
        );

        self.storage.write(transaction).map_err(|e| {
            error!(target: LOG_TARGET,"{:?}", e);
            FileStorageError::FailedToWriteToStorage
        })?;

        Ok(())
    }

    /// Deletes all files with a matching bucket ID prefix.
    fn delete_files_with_prefix(
        &mut self,
        bucket_id_prefix: &[u8; 32],
    ) -> Result<(), FileStorageError> {
        let mut file_keys_to_delete = Vec::new();

        {
            let mut iter = self
                .storage
                .db
                .iter_with_prefix(Column::BucketPrefix.into(), bucket_id_prefix);

            while let Some(Ok((key, _))) = iter.next() {
                // Remove the prefix from the key.
                let file_key = key
                    .iter()
                    .skip(bucket_id_prefix.len())
                    .copied()
                    .collect::<Vec<u8>>();

                let h_file_key = convert_raw_bytes_to_hasher_out::<T>(file_key).map_err(|e| {
                    error!(target: LOG_TARGET, "{:?}", e);
                    FileStorageError::FailedToParseKey
                })?;

                file_keys_to_delete.push(h_file_key);
            }
        }

        for h_file_key in file_keys_to_delete {
            self.delete_file(&h_file_key)?;
        }

        Ok(())
    }

    /// Checks if a key is allowed based on the exclude type.
    fn is_allowed(
        &self,
        file_key: &HasherOutT<T>,
        exclude_type: ExcludeType,
    ) -> Result<bool, FileStorageError> {
        let exclude_column = get_exclude_type_db_column(exclude_type);
        let find = self
            .storage
            .db
            .get(exclude_column, file_key.as_ref())
            .map_err(|e| {
                error!(target: LOG_TARGET, "{:?}", e);
                FileStorageError::FailedToReadStorage
            })?;

        match find {
            Some(_) => return Ok(false),
            None => return Ok(true),
        }
    }

    /// Adds a key to the specified exclude list.
    fn add_to_exclude_list(
        &mut self,
        file_key: HasherOutT<T>,
        exclude_type: ExcludeType,
    ) -> Result<(), FileStorageError> {
        let exclude_column = get_exclude_type_db_column(exclude_type);

        let mut transaction = DBTransaction::new();
        transaction.put(exclude_column, file_key.as_ref(), &[]);

        self.storage.db.write(transaction).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to write to DB: {}", e);
            FileStorageError::FailedToWriteToStorage
        })?;

        info!("Key added to the exclude list : {:?}", file_key);
        Ok(())
    }

    /// Removes a key from the specified exclude list.
    fn remove_from_exclude_list(
        &mut self,
        file_key: &HasherOutT<T>,
        exclude_type: ExcludeType,
    ) -> Result<(), FileStorageError> {
        let exclude_column = get_exclude_type_db_column(exclude_type);

        let mut transaction = DBTransaction::new();
        transaction.delete(exclude_column, file_key.as_ref());

        self.storage.db.write(transaction).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to write to DB: {}", e);
            FileStorageError::FailedToWriteToStorage
        })?;

        info!("Key removed to the exclude list : {:?}", file_key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kvdb_memorydb::InMemory;
    use shc_common::types::{Fingerprint, FILE_CHUNK_SIZE};
    use sp_core::H256;
    use sp_runtime::traits::BlakeTwo256;
    use sp_runtime::AccountId32;
    use sp_trie::LayoutV1;

    fn stored_chunks_count(
        trie: &RocksDbFileDataTrie<LayoutV1<BlakeTwo256>, InMemory>,
    ) -> Result<u64, FileStorageError> {
        let db = trie.as_hash_db();
        let trie = TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&db, &trie.root).build();

        let count = trie
            .iter()
            .map_err(|e| {
                error!(target: LOG_TARGET, "Failed to construct Trie iterator: {}", e);
                FileStorageError::FailedToConstructTrieIter
            })?
            .count();

        Ok(count as u64)
    }

    #[test]
    fn file_trie_create_empty_works() {
        let storage = StorageDb {
            db: Arc::new(kvdb_memorydb::create(NUMBER_OF_COLUMNS)),
            _marker: Default::default(),
        };

        let file_trie = RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>, InMemory>::new(storage);

        // expected hash is the root hash of an empty tree.
        let expected_hash = HasherOutT::<LayoutV1<BlakeTwo256>>::try_from([
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
    fn file_trie_write_chunk_works() {
        let storage = StorageDb {
            db: Arc::new(kvdb_memorydb::create(NUMBER_OF_COLUMNS)),
            _marker: Default::default(),
        };

        let mut file_trie = RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>, InMemory>::new(storage);
        let old_root = *file_trie.get_root();
        file_trie
            .write_chunk(&ChunkId::new(0u64), &Chunk::from([1u8; 1024]))
            .unwrap();
        let new_root = file_trie.get_root();
        assert_ne!(&old_root, new_root);

        let chunk = file_trie.get_chunk(&ChunkId::new(0u64)).unwrap();
        assert_eq!(chunk.as_slice(), [1u8; 1024]);
    }

    #[test]
    fn file_trie_get_chunk_works() {
        let storage = StorageDb {
            db: Arc::new(kvdb_memorydb::create(NUMBER_OF_COLUMNS)),
            _marker: Default::default(),
        };

        let mut file_trie = RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>, InMemory>::new(storage);

        let chunk = Chunk::from([3u8; 32]);
        let chunk_id = ChunkId::new(3);
        file_trie.write_chunk(&chunk_id, &chunk).unwrap();
        let chunk = file_trie.get_chunk(&chunk_id).unwrap();
        assert_eq!(chunk.as_slice(), [3u8; 32]);
    }

    #[test]
    fn file_trie_stored_chunks_count_works() {
        let storage = StorageDb {
            db: Arc::new(kvdb_memorydb::create(NUMBER_OF_COLUMNS)),
            _marker: Default::default(),
        };

        let chunk_ids = vec![ChunkId::new(0u64), ChunkId::new(1u64)];
        let chunks = vec![Chunk::from([0u8; 1024]), Chunk::from([1u8; 1024])];

        let mut file_trie = RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>, InMemory>::new(storage);

        file_trie.write_chunk(&chunk_ids[0], &chunks[0]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 1);
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_ok());

        file_trie.write_chunk(&chunk_ids[1], &chunks[1]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 2);
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_ok());
    }

    #[test]
    fn file_trie_generate_proof_works() {
        let storage = StorageDb {
            db: Arc::new(kvdb_memorydb::create(NUMBER_OF_COLUMNS)),
            _marker: Default::default(),
        };

        let chunk_ids = vec![ChunkId::new(0u64), ChunkId::new(1u64), ChunkId::new(2u64)];
        let chunk_ids_set: HashSet<ChunkId> = chunk_ids.iter().cloned().collect();
        let chunks = vec![
            Chunk::from([0u8; 1024]),
            Chunk::from([1u8; 1024]),
            Chunk::from([2u8; 1024]),
        ];

        let mut file_trie = RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>, InMemory>::new(storage);

        file_trie.write_chunk(&chunk_ids[0], &chunks[0]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 1);
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_ok());

        file_trie.write_chunk(&chunk_ids[1], &chunks[1]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 2);
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_ok());

        file_trie.write_chunk(&chunk_ids[2], &chunks[2]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 3);
        assert!(file_trie.get_chunk(&chunk_ids[2]).is_ok());

        let file_proof = file_trie.generate_proof(&chunk_ids_set).unwrap();

        assert_eq!(
            file_proof.fingerprint.as_ref(),
            file_trie.get_root().as_ref()
        );
    }

    #[test]
    fn file_trie_delete_works() {
        let storage = StorageDb {
            db: Arc::new(kvdb_memorydb::create(NUMBER_OF_COLUMNS)),
            _marker: Default::default(),
        };

        let chunk_ids = vec![ChunkId::new(0u64), ChunkId::new(1u64), ChunkId::new(2u64)];

        let chunks = vec![
            Chunk::from([0u8; 1024]),
            Chunk::from([1u8; 1024]),
            Chunk::from([2u8; 1024]),
        ];

        let mut file_trie = RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>, InMemory>::new(storage);

        file_trie.write_chunk(&chunk_ids[0], &chunks[0]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 1);
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_ok());

        file_trie.write_chunk(&chunk_ids[1], &chunks[1]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 2);
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_ok());

        file_trie.write_chunk(&chunk_ids[2], &chunks[2]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 3);
        assert!(file_trie.get_chunk(&chunk_ids[2]).is_ok());

        file_trie.delete().unwrap();
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_err());
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_err());
        assert!(file_trie.get_chunk(&chunk_ids[2]).is_err());

        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 0);
    }

    #[test]
    fn file_storage_write_chunk_works() {
        let chunks = vec![
            Chunk::from([5u8; FILE_CHUNK_SIZE as usize]),
            Chunk::from([6u8; FILE_CHUNK_SIZE as usize]),
            Chunk::from([7u8; FILE_CHUNK_SIZE as usize]),
        ];

        let storage = StorageDb {
            db: Arc::new(kvdb_memorydb::create(NUMBER_OF_COLUMNS)),
            _marker: Default::default(),
        };

        let chunk_ids: Vec<ChunkId> = chunks
            .iter()
            .enumerate()
            .map(|(id, _)| ChunkId::new(id as u64))
            .collect();

        // Create a file trie to get the expected fingerprint
        let mut file_trie =
            RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>, InMemory>::new(storage.clone());

        for (chunk_id, chunk) in chunk_ids.iter().zip(chunks.iter()) {
            file_trie.write_chunk(chunk_id, chunk).unwrap();
        }

        let file_metadata = FileMetadata {
            file_size: FILE_CHUNK_SIZE * chunks.len() as u64,
            fingerprint: file_trie.get_root().as_ref().into(),
            owner: <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            location: "location".to_string().into_bytes(),
            bucket_id: [1u8; 32].to_vec(),
        };

        let key = file_metadata.file_key::<BlakeTwo256>();
        let mut file_storage = RocksDbFileStorage::<LayoutV1<BlakeTwo256>, InMemory>::new(storage);

        // Insert file metadata first
        file_storage.insert_file(key, file_metadata).unwrap();

        // Write chunks one by one and verify
        for (chunk_id, chunk) in chunk_ids.iter().zip(chunks.iter()) {
            let result = file_storage.write_chunk(&key, chunk_id, chunk);
            assert!(result.is_ok());
            assert!(file_storage.get_chunk(&key, chunk_id).is_ok());
        }

        // Verify final state
        assert!(file_storage.get_metadata(&key).is_ok());
        assert!(file_storage.get_chunk(&key, &chunk_ids[0]).is_ok());
        assert!(file_storage.get_chunk(&key, &chunk_ids[1]).is_ok());
        assert!(file_storage.get_chunk(&key, &chunk_ids[2]).is_ok());
    }

    #[test]
    fn file_storage_insert_file_works() {
        let storage = StorageDb {
            db: Arc::new(kvdb_memorydb::create(NUMBER_OF_COLUMNS)),
            _marker: Default::default(),
        };

        let chunks = vec![
            Chunk::from([5u8; 32]),
            Chunk::from([6u8; 32]),
            Chunk::from([7u8; 32]),
        ];

        let chunk_ids: Vec<ChunkId> = chunks
            .iter()
            .enumerate()
            .map(|(id, _)| ChunkId::new(id as u64))
            .collect();

        let mut file_trie =
            RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>, InMemory>::new(storage.clone());

        file_trie.write_chunk(&chunk_ids[0], &chunks[0]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 1);
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_ok());

        file_trie.write_chunk(&chunk_ids[1], &chunks[1]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 2);
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_ok());

        file_trie.write_chunk(&chunk_ids[2], &chunks[2]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 3);
        assert!(file_trie.get_chunk(&chunk_ids[2]).is_ok());

        let file_metadata = FileMetadata {
            file_size: 32u64 * chunks.len() as u64,
            fingerprint: file_trie.get_root().as_ref().into(),
            owner: <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            location: "location".to_string().into_bytes(),
            bucket_id: [1u8; 32].to_vec(),
        };

        let key = file_metadata.file_key::<BlakeTwo256>();
        let mut file_storage = RocksDbFileStorage::<LayoutV1<BlakeTwo256>, InMemory>::new(storage);
        file_storage
            .insert_file_with_data(key, file_metadata, file_trie)
            .unwrap();

        assert!(file_storage.get_metadata(&key).is_ok());
        assert!(file_storage.get_chunk(&key, &chunk_ids[0]).is_ok());
        assert!(file_storage.get_chunk(&key, &chunk_ids[1]).is_ok());
        assert!(file_storage.get_chunk(&key, &chunk_ids[2]).is_ok());
    }

    #[test]
    fn file_storage_delete_file_works() {
        let storage = StorageDb {
            db: Arc::new(kvdb_memorydb::create(NUMBER_OF_COLUMNS)),
            _marker: Default::default(),
        };

        let chunks = vec![
            Chunk::from([5u8; 32]),
            Chunk::from([6u8; 32]),
            Chunk::from([7u8; 32]),
        ];

        let chunk_ids: Vec<ChunkId> = chunks
            .iter()
            .enumerate()
            .map(|(id, _)| ChunkId::new(id as u64))
            .collect();

        let mut file_trie =
            RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>, InMemory>::new(storage.clone());
        file_trie.write_chunk(&chunk_ids[0], &chunks[0]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 1);
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_ok());

        file_trie.write_chunk(&chunk_ids[1], &chunks[1]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 2);
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_ok());

        file_trie.write_chunk(&chunk_ids[2], &chunks[2]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 3);
        assert!(file_trie.get_chunk(&chunk_ids[2]).is_ok());

        let file_metadata = FileMetadata {
            file_size: 32u64 * chunks.len() as u64,
            fingerprint: file_trie.get_root().as_ref().into(),
            owner: <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            location: "location".to_string().into_bytes(),
            bucket_id: [1u8; 32].to_vec(),
        };

        let key = file_metadata.file_key::<BlakeTwo256>();
        let mut file_storage = RocksDbFileStorage::<LayoutV1<BlakeTwo256>, InMemory>::new(storage);
        file_storage
            .insert_file_with_data(key, file_metadata, file_trie)
            .unwrap();
        assert!(file_storage.get_metadata(&key).is_ok());

        assert!(file_storage.delete_file(&key).is_ok());

        // Should get a None option here when trying to get File Metadata.
        assert!(file_storage
            .get_metadata(&key)
            .is_ok_and(|metadata| metadata.is_none()));
        assert!(file_storage.get_chunk(&key, &chunk_ids[0]).is_err());
        assert!(file_storage.get_chunk(&key, &chunk_ids[1]).is_err());
        assert!(file_storage.get_chunk(&key, &chunk_ids[2]).is_err());
    }

    #[test]
    fn file_storage_generate_proof_works() {
        let chunks = vec![
            Chunk::from([5u8; 32]),
            Chunk::from([6u8; 32]),
            Chunk::from([7u8; 32]),
        ];

        let storage = StorageDb {
            db: Arc::new(kvdb_memorydb::create(NUMBER_OF_COLUMNS)),
            _marker: Default::default(),
        };

        let user_storage = StorageDb {
            db: Arc::new(kvdb_memorydb::create(NUMBER_OF_COLUMNS)),
            _marker: Default::default(),
        };

        let mut user_file_trie =
            RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>, InMemory>::new(user_storage.clone());

        for (id, chunk) in chunks.iter().enumerate() {
            user_file_trie
                .write_chunk(&ChunkId::new(id as u64), chunk)
                .unwrap();
        }

        let fingerprint = Fingerprint::from(user_file_trie.get_root().as_ref());

        let chunk_ids: Vec<ChunkId> = chunks
            .iter()
            .enumerate()
            .map(|(id, _)| ChunkId::new(id as u64))
            .collect();

        let chunk_ids_set: HashSet<ChunkId> = chunk_ids.iter().cloned().collect();

        let file_metadata = FileMetadata {
            file_size: 1024u64 * chunks.len() as u64,
            fingerprint,
            owner: <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            location: "location".to_string().into_bytes(),
            bucket_id: [1u8; 32].to_vec(),
        };
        let key = file_metadata.file_key::<BlakeTwo256>();

        let mut file_storage =
            RocksDbFileStorage::<LayoutV1<BlakeTwo256>, InMemory>::new(storage.clone());
        file_storage.insert_file(key, file_metadata).unwrap();
        assert!(file_storage.get_metadata(&key).is_ok());

        file_storage
            .write_chunk(&key, &chunk_ids[0], &chunks[0])
            .unwrap();
        assert!(file_storage.get_chunk(&key, &chunk_ids[0]).is_ok());

        file_storage
            .write_chunk(&key, &chunk_ids[1], &chunks[1])
            .unwrap();
        assert!(file_storage.get_chunk(&key, &chunk_ids[1]).is_ok());

        file_storage
            .write_chunk(&key, &chunk_ids[2], &chunks[2])
            .unwrap();
        assert!(file_storage.get_chunk(&key, &chunk_ids[2]).is_ok());

        let file_proof = file_storage.generate_proof(&key, &chunk_ids_set).unwrap();
        let proven_leaves = file_proof.proven::<LayoutV1<BlakeTwo256>>().unwrap();
        for (id, leaf) in proven_leaves.iter().enumerate() {
            assert_eq!(chunk_ids[id], leaf.key);
            assert_eq!(chunks[id], leaf.data);
        }
    }

    #[test]
    fn same_chunk_id_with_different_data_produces_different_roots() {
        use sp_trie::MemoryDB;

        let mut memdb = MemoryDB::<BlakeTwo256>::default();
        let mut root1 = Default::default();
        let mut root2 = Default::default();
        let chunks1 = vec![0u8; 32];
        let chunks2 = vec![1u8; 32];

        {
            let mut t1 =
                TrieDBMutBuilder::<LayoutV1<BlakeTwo256>>::new(&mut memdb, &mut root1).build();
            t1.insert(&[0u8; 32], &chunks1).unwrap();
        }
        {
            let mut t2 =
                TrieDBMutBuilder::<LayoutV1<BlakeTwo256>>::new(&mut memdb, &mut root2).build();
            t2.insert(&[0u8; 32], &chunks2).unwrap();
        }

        assert_ne!(root1, root2)
    }

    #[test]
    fn delete_files_with_prefix_works() {
        let storage = StorageDb {
            db: Arc::new(kvdb_memorydb::create(NUMBER_OF_COLUMNS)),
            _marker: Default::default(),
        };

        fn create_file_and_metadata(
            storage: StorageDb<LayoutV1<BlakeTwo256>, InMemory>,
            chunks: Vec<Chunk>,
            bucket_id: [u8; 32],
            location: &str,
        ) -> (
            FileMetadata,
            H256,
            Vec<ChunkId>,
            RocksDbFileDataTrie<LayoutV1<BlakeTwo256>, InMemory>,
        ) {
            // Convert chunks into chunk IDs for referencing each chunk in the trie.
            let chunk_ids: Vec<ChunkId> = chunks
                .iter()
                .enumerate()
                .map(|(id, _)| ChunkId::new(id as u64))
                .collect();

            // Create a new file trie for storing file chunks.
            let mut file_trie =
                RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>, InMemory>::new(storage.clone());

            // Write each chunk into the trie.
            for (i, chunk) in chunks.iter().enumerate() {
                file_trie.write_chunk(&chunk_ids[i], chunk).unwrap();
            }

            // Create metadata for the file, including bucket ID, location, and owner.
            let file_metadata = FileMetadata {
                file_size: 32u64 * chunks.len() as u64,
                fingerprint: file_trie.get_root().as_ref().into(),
                owner: <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
                location: location.to_string().into_bytes(),
                bucket_id: bucket_id.to_vec(),
            };

            let key = file_metadata.file_key::<BlakeTwo256>();

            // Return the metadata, key, chunk IDs, and the trie.
            (file_metadata, key, chunk_ids, file_trie)
        }

        // Step 2: Define test data for three files.
        // These are the chunks of data that will be stored in the file trie.
        let chunks_1 = vec![
            Chunk::from([5u8; 32]),
            Chunk::from([6u8; 32]),
            Chunk::from([7u8; 32]),
        ];
        let chunks_2 = vec![
            Chunk::from([8u8; 32]),
            Chunk::from([9u8; 32]),
            Chunk::from([10u8; 32]),
        ];
        let chunks_3 = vec![
            Chunk::from([11u8; 32]),
            Chunk::from([12u8; 32]),
            Chunk::from([13u8; 32]),
        ];

        // Step 3: Create file metadata, keys, and file tries for each of the three files.
        let (file_metadata_1, key_1, chunk_ids_1, file_trie_1) =
            create_file_and_metadata(storage.clone(), chunks_1, [1u8; 32], "location");
        let (file_metadata_2, key_2, chunk_ids_2, file_trie_2) =
            create_file_and_metadata(storage.clone(), chunks_2, [2u8; 32], "location_2");
        let (file_metadata_3, key_3, chunk_ids_3, file_trie_3) =
            create_file_and_metadata(storage.clone(), chunks_3, [3u8; 32], "location_3");

        // Step 4: Create a file storage and insert all three files into the storage.
        let mut file_storage = RocksDbFileStorage::<LayoutV1<BlakeTwo256>, InMemory>::new(storage);

        file_storage
            .insert_file_with_data(key_1, file_metadata_1.clone(), file_trie_1)
            .unwrap();
        file_storage
            .insert_file_with_data(key_2, file_metadata_2.clone(), file_trie_2)
            .unwrap();
        file_storage
            .insert_file_with_data(key_3, file_metadata_3.clone(), file_trie_3)
            .unwrap();

        // Step 5: Verify that all files and their chunks are inserted properly.
        assert!(file_storage.get_metadata(&key_1).is_ok());
        assert!(file_storage.get_metadata(&key_2).is_ok());
        assert!(file_storage.get_metadata(&key_3).is_ok());

        // Step 6: Delete files with the prefix [1u8; 32], which corresponds to bucket ID 1.
        file_storage.delete_files_with_prefix(&[1u8; 32]).unwrap();

        // Step 7: Assert that files with bucket_id 1 are deleted.
        // We expect no metadata or chunks for the file with key_1 after deletion.
        assert!(file_storage
            .get_metadata(&key_1)
            .is_ok_and(|metadata| metadata.is_none()));
        assert!(file_storage.get_chunk(&key_1, &chunk_ids_1[0]).is_err());

        // Step 8: Assert that files with other bucket_ids (bucket_id 2 and 3) are not deleted.
        // Files with key_2 and key_3 should remain accessible.
        assert!(file_storage.get_metadata(&key_2).is_ok());
        assert!(file_storage.get_metadata(&key_3).is_ok());
        assert!(file_storage.get_chunk(&key_2, &chunk_ids_2[0]).is_ok());
        assert!(file_storage.get_chunk(&key_3, &chunk_ids_3[0]).is_ok());
    }
}
