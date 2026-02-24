use std::{
    collections::HashSet,
    io,
    num::NonZeroUsize,
    sync::Arc,
    time::{Duration, Instant},
};

use hash_db::{AsHashDB, HashDB, Prefix};
use kvdb::{DBTransaction, KeyValueDB};
use log::{debug, error, info};
use lru::LruCache;
use shc_common::types::{
    Chunk, ChunkId, ChunkWithId, FileKeyProof, FileMetadata, FileProof, HashT, HasherOutT, H_LENGTH,
};
use sp_state_machine::{warn, Storage};
use sp_trie::{prefixed_key, recorder::Recorder, PrefixedMemoryDB, TrieLayout, TrieMut};
use trie_db::{DBValue, Trie, TrieDBBuilder, TrieDBMutBuilder};

use crate::{
    error::ErrorT,
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
    /// Stores keys of 32 bytes representing the `fingerprint` with values being a `u64` refcount.
    ///
    /// Used to ensure the underlying trie (Chunks/Roots) is deleted only when the last reference
    /// to this fingerprint is removed.
    FingerprintRefCount,
}

impl Into<u32> for Column {
    fn into(self) -> u32 {
        self as u32
    }
}

// Replace NUMBER_OF_COLUMNS definition
const NUMBER_OF_COLUMNS: u32 = Column::COUNT as u32;
const BATCH_CACHE_TTL: Duration = Duration::from_secs(15 * 60);
const BATCH_CACHE_MAX_ENTRIES: usize = 4096;

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
    let db_config = kvdb_rocksdb::DatabaseConfig::with_columns(NUMBER_OF_COLUMNS);

    std::fs::create_dir_all(&db_path)?;
    let db = kvdb_rocksdb::Database::open(&db_config, &db_path)?;

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

    /// Batched write fast-path: insert many chunks into the trie overlay.
    ///
    /// This method intentionally does not persist to RocksDB on its own.
    /// The caller is expected to drain overlay changes and commit them together with
    /// related metadata updates (e.g. roots + chunk counters) in one transaction.
    fn insert_chunks_batched(
        &mut self,
        chunks: Vec<(ChunkId, Chunk)>,
    ) -> Result<HasherOutT<T>, FileStorageWriteError> {
        if chunks.is_empty() {
            return Ok(self.root);
        }

        let mut current_root = self.root;
        let db = self.as_hash_db_mut();
        let mut trie = TrieDBMutBuilder::<T>::from_existing(db, &mut current_root).build();

        for (chunk_id, data) in chunks {
            let decoded_chunk = ChunkWithId { chunk_id, data };
            let encoded_chunk = decoded_chunk.encode();
            trie.insert(&chunk_id.as_trie_key(), &encoded_chunk)
                .map_err(|_| FileStorageWriteError::FailedToInsertFileChunk)?;
        }

        let new_root = *trie.root();
        drop(trie);

        Ok(new_root)
    }

    /// Builds a database transaction from the overlay and clears it.
    fn changes(&mut self) -> DBTransaction {
        let mut transaction = DBTransaction::new();

        self.drain_overlay_into_transaction(&mut transaction);
        transaction
    }

    /// Drains overlay changes into the provided transaction.
    ///
    /// This is useful for composing a single RocksDB write that includes both trie node updates
    /// and other metadata updates (e.g. roots + chunk counters) for batch writes.
    fn drain_overlay_into_transaction(&mut self, transaction: &mut DBTransaction) {
        for (key, (value, rc)) in self.overlay.drain() {
            if rc <= 0 {
                transaction.delete(Column::Chunks.into(), &key);
            } else {
                transaction.put_vec(Column::Chunks.into(), &key, value);
            }
        }
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
struct BatchWriteCacheState<T: TrieLayout> {
    metadata: FileMetadata,
    partial_root: HasherOutT<T>,
    chunk_count: u64,
}

struct BatchCacheEntry<T: TrieLayout> {
    state: Option<BatchWriteCacheState<T>>,
    last_touched: Instant,
}

impl<T: TrieLayout> BatchCacheEntry<T> {
    fn new(now: Instant) -> Self {
        Self {
            state: None,
            last_touched: now,
        }
    }
}

pub struct RocksDbFileStorage<T, DB>
where
    T: TrieLayout + 'static,
    DB: KeyValueDB,
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    storage: StorageDb<T, DB>,
    batch_states: LruCache<Vec<u8>, BatchCacheEntry<T>>,
    batch_cache_ttl: Duration,
}

impl<T: TrieLayout, DB> RocksDbFileStorage<T, DB>
where
    T: TrieLayout,
    DB: KeyValueDB,
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    /// Helper to increment the fingerprint refcount within the provided transaction.
    /// Returns the new refcount value.
    fn increment_fingerprint_refcount(
        &self,
        fingerprint: &[u8],
        transaction: &mut DBTransaction,
    ) -> Result<u64, FileStorageError> {
        let current = self
            .storage
            .read(Column::FingerprintRefCount.into(), fingerprint)
            .map_err(|e| {
                error!(target: LOG_TARGET, "{:?}", e);
                FileStorageError::FailedToReadStorage
            })?
            .map(|bytes| u64::from_le_bytes(bytes.try_into().unwrap()))
            .unwrap_or(0);

        let new_count = current
            .checked_add(1)
            .ok_or(FileStorageError::FailedToWriteToStorage)?;

        transaction.put(
            Column::FingerprintRefCount.into(),
            fingerprint,
            &new_count.to_le_bytes(),
        );

        Ok(new_count)
    }

    /// Helper to decrement the fingerprint refcount within the provided transaction.
    /// Returns the new refcount value (saturating at 0).
    fn decrement_fingerprint_refcount(
        &self,
        fingerprint: &[u8],
        transaction: &mut DBTransaction,
    ) -> Result<u64, FileStorageError> {
        let current = self
            .storage
            .read(Column::FingerprintRefCount.into(), fingerprint)
            .map_err(|e| {
                error!(target: LOG_TARGET, "{:?}", e);
                FileStorageError::FailedToReadStorage
            })?
            .map(|bytes| u64::from_le_bytes(bytes.try_into().unwrap()))
            .unwrap_or(0);

        let new_count = current.saturating_sub(1);

        transaction.put(
            Column::FingerprintRefCount.into(),
            fingerprint,
            &new_count.to_le_bytes(),
        );

        Ok(new_count)
    }
    /// Helper to build the bucket-prefixed file key used for efficient prefix scans.
    ///
    /// This is used, for instance, to delete all files in a bucket efficiently.
    fn build_bucket_prefixed_file_key(
        metadata: &FileMetadata,
        file_key: &HasherOutT<T>,
    ) -> Vec<u8> {
        metadata
            .bucket_id()
            .iter()
            .copied()
            .chain(file_key.as_ref().iter().copied())
            .collect::<Vec<_>>()
    }

    /// Creates a new file storage instance with the given storage backend.
    pub fn new(storage: StorageDb<T, DB>) -> Self {
        Self {
            storage,
            batch_states: LruCache::new(
                NonZeroUsize::new(BATCH_CACHE_MAX_ENTRIES)
                    .expect("BATCH_CACHE_MAX_ENTRIES must be greater than zero"),
            ),
            batch_cache_ttl: BATCH_CACHE_TTL,
        }
    }

    fn prune_batch_cache(&mut self, now: Instant) {
        // Drop stale upload states to avoid unbounded growth when uploads stall.
        let stale_keys: Vec<Vec<u8>> = self
            .batch_states
            .iter()
            .filter_map(|(key, entry)| {
                (now.saturating_duration_since(entry.last_touched) > self.batch_cache_ttl)
                    .then(|| key.clone())
            })
            .collect();

        for key in stale_keys {
            let _ = self.batch_states.pop(&key);
        }
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
        let b_fingerprint = metadata.fingerprint().as_ref();
        let h_fingerprint =
            convert_raw_bytes_to_hasher_out::<T>(b_fingerprint.to_vec()).map_err(|e| {
                error!(target: LOG_TARGET, "{:?}", e);
                FileStorageError::FailedToParseFingerprint
            })?;

        debug!(target: LOG_TARGET, "Reading partial root for fingerprint {:?}", h_fingerprint);

        // We call this root "partial root" because a file trie can exist while not all
        // chunks have been written to it. When all chunks are written, this root is
        // in fact the final root of the file trie.
        let raw_partial_root = self
            .storage
            .read(Column::Roots.into(), h_fingerprint.as_ref())
            .map_err(|e| {
                error!(target: LOG_TARGET, "Failed to read partial root for fingerprint {:?}: {:?}", h_fingerprint, e);
                FileStorageError::FailedToReadStorage
            })?.ok_or_else(|| {
                error!(target: LOG_TARGET, "Partial root returned None for fingerprint {:?}", h_fingerprint);
                FileStorageError::PartialRootNotFound
            })?;

        let partial_root = convert_raw_bytes_to_hasher_out::<T>(raw_partial_root).map_err(|e| {
            error!(target: LOG_TARGET, "{:?}", e);
            FileStorageError::FailedToParsePartialRoot
        })?;

        debug!(
            target: LOG_TARGET,
            "Constructing file trie from partial root {:?}",
            partial_root
        );

        Ok(RocksDbFileDataTrie::<T, DB>::from_existing(
            self.storage.clone(),
            &partial_root,
        ))
    }
}

impl<T, DB> RocksDbFileStorage<T, DB>
where
    T: TrieLayout + Send + Sync + 'static,
    DB: KeyValueDB + 'static,
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    /// Ensure the per-file batch cache entry is fully initialized.
    ///
    /// The cache state is all-or-nothing: metadata, partial root, and chunk count are loaded
    /// together on first use and then reused across subsequent batches for the same file key.
    fn ensure_batch_state_initialized(
        &self,
        file_key: &HasherOutT<T>,
        cache_entry: &mut BatchCacheEntry<T>,
    ) -> Result<(), FileStorageWriteError> {
        if cache_entry.state.is_some() {
            return Ok(());
        }

        let metadata = self
            .get_metadata(file_key)
            .map_err(|_| FileStorageWriteError::FailedToParseFileMetadata)?
            .ok_or(FileStorageWriteError::FileDoesNotExist)?;
        let file_trie = self.get_file_trie(&metadata).map_err(|e| {
            error!(target: LOG_TARGET, "{:?}", e);
            FileStorageWriteError::FailedToConstructFileTrie
        })?;
        let partial_root = *file_trie.get_root();
        let chunk_count = self.stored_chunks_count(file_key).map_err(|e| {
            error!(target: LOG_TARGET, "{:?}", e);
            FileStorageWriteError::FailedToGetStoredChunksCount
        })?;

        cache_entry.state = Some(BatchWriteCacheState {
            metadata,
            partial_root,
            chunk_count,
        });

        Ok(())
    }

    /// Apply a chunk batch using an already initialized per-file cache state.
    ///
    /// This method:
    /// - inserts batch chunks into trie overlay
    /// - commits trie changes + root + chunk count in one DB transaction
    /// - updates the cached root/chunk-count
    /// - returns whether the file is complete after this batch
    fn apply_chunks_batch_with_state(
        &mut self,
        file_key: &HasherOutT<T>,
        chunks: Vec<(ChunkId, Chunk)>,
        state: &mut BatchWriteCacheState<T>,
    ) -> Result<FileStorageWriteOutcome, FileStorageWriteError> {
        // Number of chunks in this batch; used to update cached chunk count.
        let delta =
            u64::try_from(chunks.len()).map_err(|_| FileStorageWriteError::ChunkCountOverflow)?;

        let mut file_trie =
            RocksDbFileDataTrie::<T, DB>::from_existing(self.storage.clone(), &state.partial_root);

        // Insert trie nodes into overlay. Persistence happens below in one transaction.
        let new_partial_root = file_trie.insert_chunks_batched(chunks)?;
        let new_count = state
            .chunk_count
            .checked_add(delta)
            .ok_or(FileStorageWriteError::ChunkCountOverflow)?;

        // Prevent never-ending uploads from writing more chunks than declared in metadata.
        if new_count > state.metadata.chunks_count() {
            warn!(
                target: LOG_TARGET,
                "Batch write exceeded metadata chunk count for file {:?}: new_count={}, declared={}",
                file_key,
                new_count,
                state.metadata.chunks_count()
            );
            return Err(FileStorageWriteError::ChunkCountOverflow);
        }

        // Persist trie mutations + root + chunk count in a single RocksDB transaction.
        let mut transaction = file_trie.changes();
        transaction.put(
            Column::Roots.into(),
            state.metadata.fingerprint().as_ref(),
            new_partial_root.as_ref(),
        );
        transaction.put(
            Column::ChunkCount.into(),
            file_key.as_ref(),
            &new_count.to_le_bytes(),
        );

        self.storage.write(transaction).map_err(|e| {
            error!(target: LOG_TARGET, "{:?}", e);
            FileStorageWriteError::FailedToUpdatePartialRoot
        })?;

        // Keep cache in sync so the next batch avoids extra DB reads.
        state.partial_root = new_partial_root;
        state.chunk_count = new_count;

        // Mirror `is_file_complete` semantics without re-reading from DB/trie.
        let file_complete = state.metadata.fingerprint() == new_partial_root.as_ref()
            && state.metadata.chunks_count() == new_count;

        if file_complete {
            Ok(FileStorageWriteOutcome::FileComplete)
        } else {
            Ok(FileStorageWriteOutcome::FileIncomplete)
        }
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
            FileStorageWriteError::FailedToConstructFileTrie
        })?;

        match file_trie.write_chunk(chunk_id, data) {
            Ok(()) => {
                // Chunk was successfully inserted into shared trie - need to update root
                debug!(target: LOG_TARGET, "Chunk {:?} successfully written to shared trie for file key {:?}", chunk_id, file_key);
            }
            Err(FileStorageWriteError::FileChunkAlreadyExists) => {
                // Chunk already exists in shared trie - no need to update root
                debug!(target: LOG_TARGET, "Chunk {:?} already exists in shared trie for file key {:?}, incrementing count for progress tracking", chunk_id, file_key);
            }
            Err(other) => {
                error!(target: LOG_TARGET, "Error while writing chunk {:?} of file key {:?}: {:?}", chunk_id, file_key, other);
                return Err(FileStorageWriteError::FailedToInsertFileChunk);
            }
        };

        // Update the root of the file trie.
        let new_partial_root = file_trie.get_root();
        let mut transaction = DBTransaction::new();
        transaction.put(
            Column::Roots.into(),
            metadata.fingerprint().as_ref(),
            new_partial_root.as_ref(),
        );

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

        // Update the chunk count.
        transaction.put(
            Column::ChunkCount.into(),
            file_key.as_ref(),
            &new_count.to_le_bytes(),
        );

        self.storage.write(transaction).map_err(|e| {
            error!(target: LOG_TARGET,"{:?}", e);
            FileStorageWriteError::FailedToUpdatePartialRoot
        })?;

        // Check if file is complete using the helper method (only once at the end)
        match self.is_file_complete(file_key) {
            Ok(true) => Ok(FileStorageWriteOutcome::FileComplete),
            Ok(false) => Ok(FileStorageWriteOutcome::FileIncomplete),
            Err(e) => {
                error!(target: LOG_TARGET, "Failed to check file completion status for file key {:?}: {:?}", file_key, e);
                Err(FileStorageWriteError::FailedToCheckFileCompletion(e))
            }
        }
    }

    fn write_chunks_batched(
        &mut self,
        file_key: &HasherOutT<T>,
        chunks: Vec<(ChunkId, Chunk)>,
    ) -> Result<FileStorageWriteOutcome, FileStorageWriteError> {
        if chunks.is_empty() {
            return Ok(FileStorageWriteOutcome::FileIncomplete);
        }

        let now = Instant::now();
        self.prune_batch_cache(now);

        let cache_key = file_key.as_ref().to_vec();
        let mut cache_entry = self
            .batch_states
            .pop(&cache_key)
            .unwrap_or_else(|| BatchCacheEntry::new(now));
        cache_entry.last_touched = now;
        self.ensure_batch_state_initialized(file_key, &mut cache_entry)?;

        let outcome = {
            let state = cache_entry
                .state
                .as_mut()
                .ok_or(FileStorageWriteError::FailedToParseFileMetadata)?;
            self.apply_chunks_batch_with_state(file_key, chunks, state)?
        };

        if matches!(outcome, FileStorageWriteOutcome::FileComplete) {
            // Cache entry is intentionally not reinserted when upload completes.
            Ok(FileStorageWriteOutcome::FileComplete)
        } else {
            // Upload is still in progress; keep updated state for next batch.
            cache_entry.last_touched = Instant::now();
            self.batch_states.put(cache_key, cache_entry);
            Ok(FileStorageWriteOutcome::FileIncomplete)
        }
    }

    /// Checks if all chunks are stored for a given file key.
    fn is_file_complete(&self, file_key: &HasherOutT<T>) -> Result<bool, FileStorageError> {
        let metadata = self
            .get_metadata(file_key)?
            .ok_or(FileStorageError::FileDoesNotExist)?;

        let stored_chunks = self.stored_chunks_count(file_key)?;

        let file_trie = self.get_file_trie(&metadata)?;

        if metadata.fingerprint() != file_trie.get_root().as_ref() {
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

        // Ensure a partial root exists for this fingerprint, but do not overwrite if it already exists
        let existing_partial_root = self
            .storage
            .read(Column::Roots.into(), metadata.fingerprint().as_ref())
            .map_err(|e| {
                error!(target: LOG_TARGET, "{:?}", e);
                FileStorageError::FailedToReadStorage
            })?;
        if existing_partial_root.is_none() {
            // Stores an empty root to allow for later initialization of the trie.
            transaction.put(
                Column::Roots.into(),
                metadata.fingerprint().as_ref(),
                empty_root.as_ref(),
            );
        }
        // Initialize chunk count to 0
        transaction.put(
            Column::ChunkCount.into(),
            file_key.as_ref(),
            &0u64.to_le_bytes(),
        );

        // Also store the bucket-prefixed key to support efficient deletions by bucket prefix.
        let bucket_prefixed_file_key = Self::build_bucket_prefixed_file_key(&metadata, &file_key);
        transaction.put(
            Column::BucketPrefix.into(),
            bucket_prefixed_file_key.as_ref(),
            &[],
        );

        // Increment fingerprint refcount
        self.increment_fingerprint_refcount(metadata.fingerprint().as_ref(), &mut transaction)?;

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
            metadata.fingerprint().as_ref(),
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

        let bucket_prefixed_file_key = Self::build_bucket_prefixed_file_key(&metadata, &file_key);

        // Store the key prefixed by bucket id
        transaction.put(
            Column::BucketPrefix.into(),
            bucket_prefixed_file_key.as_ref(),
            &[],
        );

        // Increment fingerprint refcount
        self.increment_fingerprint_refcount(metadata.fingerprint().as_ref(), &mut transaction)?;

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

        if metadata.fingerprint() != file_trie.get_root().as_ref() {
            return Err(FileStorageError::FingerprintAndStoredFileMismatch);
        }

        file_trie
            .generate_proof(chunk_ids)?
            .to_file_key_proof(metadata.clone())
            .map_err(|e| {
                error!(target: LOG_TARGET, "{:?}", e);
                FileStorageError::FailedToConstructFileKeyProof
            })
    }

    /// Deletes a file and all its associated data.
    fn delete_file(&mut self, file_key: &HasherOutT<T>) -> Result<(), FileStorageError> {
        let Some(metadata) = self.get_metadata(file_key)? else {
            // Idempotent: if already deleted, nothing to do
            warn!(target: LOG_TARGET, "File key {:?} already deleted", file_key);
            return Ok(());
        };

        let b_fingerprint = metadata.fingerprint().as_ref();
        let h_fingerprint =
            convert_raw_bytes_to_hasher_out::<T>(b_fingerprint.to_vec()).map_err(|e| {
                error!(target: LOG_TARGET, "{:?}", e);
                FileStorageError::FailedToParseFingerprint
            })?;

        // Transaction 1: remove per-file metadata and decrement refcount
        let mut txn1 = DBTransaction::new();
        txn1.delete(Column::Metadata.into(), file_key.as_ref());
        txn1.delete(Column::ChunkCount.into(), file_key.as_ref());
        let bucket_prefixed_file_key = Self::build_bucket_prefixed_file_key(&metadata, file_key);
        txn1.delete(
            Column::BucketPrefix.into(),
            bucket_prefixed_file_key.as_ref(),
        );

        let new_refcount = self.decrement_fingerprint_refcount(b_fingerprint, &mut txn1)?;
        self.storage.write(txn1).map_err(|e| {
            error!(target: LOG_TARGET, "{:?}", e);
            FileStorageError::FailedToWriteToStorage
        })?;

        if new_refcount > 0 {
            // Other references still exist; do not touch trie or roots
            info!(target: LOG_TARGET, "File key {:?} has other references, skipping trie deletion", file_key);
            return Ok(());
        }

        // Last reference: try to delete trie. If partial root is missing, treat as already deleted
        let maybe_trie = self.get_file_trie(&metadata);
        match maybe_trie {
            Ok(mut file_trie) => {
                if let Err(e) = file_trie.delete() {
                    error!(target: LOG_TARGET, "Failed to delete file trie for file key {:?} with fingerprint {:?}: {:?}", file_key, metadata.fingerprint(), e);
                    // Keep refcount at 0 but leave roots; caller may retry
                    return Err(FileStorageError::FailedToDeleteFileChunk);
                }
            }
            Err(e) => {
                warn!(target: LOG_TARGET, "Failed to get file trie for file key {:?} with fingerprint {:?}: {:?}\n\nSkipping trie deletion, file may already be deleted.", file_key, metadata.fingerprint(), e);
            }
        }

        // Transaction 2: delete roots and refcount entry (idempotent)
        let mut txn2 = DBTransaction::new();
        txn2.delete(Column::Roots.into(), h_fingerprint.as_ref());
        txn2.delete(Column::FingerprintRefCount.into(), b_fingerprint);
        self.storage.write(txn2).map_err(|e| {
            error!(target: LOG_TARGET, "{:?}", e);
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

        info!(target: LOG_TARGET, "Deleting {} file keys with prefix {:?}", file_keys_to_delete.len(), bucket_id_prefix);

        for h_file_key in file_keys_to_delete {
            debug!(target: LOG_TARGET, "Deleting file key {:?}", h_file_key);
            let result = self.delete_file(&h_file_key);
            if let Err(e) = result {
                // If metadata is already gone or partial root is missing, skip as idempotent behaviour
                match e {
                    FileStorageError::FileDoesNotExist | FileStorageError::PartialRootNotFound => {
                        warn!(target: LOG_TARGET, "Skipping already-deleted file key {:?}", h_file_key);
                        continue;
                    }
                    _ => {
                        error!(target: LOG_TARGET, "Failed to delete file key {:?}: {:?}", h_file_key, e);
                        return Err(e);
                    }
                }
            }

            debug!(target: LOG_TARGET, "Successfully deleted file key {:?}", h_file_key);
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
    use kvdb_rocksdb::Database as RocksDbDatabase;
    use shc_common::types::{Fingerprint, FILE_CHUNK_SIZE, H_LENGTH};
    use sp_core::H256;
    use sp_runtime::traits::BlakeTwo256;
    use sp_runtime::AccountId32;
    use sp_trie::LayoutV1;
    use std::path::PathBuf;
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    };
    use std::time::{Instant, SystemTime};

    static BENCHMARK_DB_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);
    static BENCHMARK_TEST_LOCK: Mutex<()> = Mutex::new(());

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

        let file_metadata = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            [1u8; 32].to_vec(),
            "location".to_string().into_bytes(),
            FILE_CHUNK_SIZE * chunks.len() as u64,
            file_trie.get_root().as_ref().into(),
        )
        .unwrap();

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

        let file_metadata = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            [1u8; 32].to_vec(),
            "location".to_string().into_bytes(),
            32u64 * chunks.len() as u64,
            file_trie.get_root().as_ref().into(),
        )
        .unwrap();

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

        let file_metadata = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            [1u8; 32].to_vec(),
            "location".to_string().into_bytes(),
            32u64 * chunks.len() as u64,
            file_trie.get_root().as_ref().into(),
        )
        .unwrap();

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

        let file_metadata = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            [1u8; 32].to_vec(),
            "location".to_string().into_bytes(),
            1024u64 * chunks.len() as u64,
            fingerprint,
        )
        .unwrap();

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
    fn file_storage_write_chunks_batched_works() {
        let chunks = vec![Chunk::from([0u8; 1024]), Chunk::from([1u8; 1024])];
        let chunk_ids: Vec<ChunkId> = chunks
            .iter()
            .enumerate()
            .map(|(id, _)| ChunkId::new(id as u64))
            .collect();

        let fingerprint_storage = StorageDb {
            db: Arc::new(kvdb_memorydb::create(NUMBER_OF_COLUMNS)),
            _marker: Default::default(),
        };
        let mut expected_trie =
            RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>, InMemory>::new(fingerprint_storage);
        for (chunk_id, chunk) in chunk_ids.iter().zip(chunks.iter()) {
            expected_trie.write_chunk(chunk_id, chunk).unwrap();
        }

        let file_metadata = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            [1u8; 32].to_vec(),
            "batched-write".to_string().into_bytes(),
            1024u64 * chunks.len() as u64,
            Fingerprint::from(expected_trie.get_root().as_ref()),
        )
        .unwrap();
        let key = file_metadata.file_key::<BlakeTwo256>();

        let storage = StorageDb {
            db: Arc::new(kvdb_memorydb::create(NUMBER_OF_COLUMNS)),
            _marker: Default::default(),
        };
        let mut file_storage = RocksDbFileStorage::<LayoutV1<BlakeTwo256>, InMemory>::new(storage);
        file_storage.insert_file(key, file_metadata).unwrap();

        let empty = file_storage.write_chunks_batched(&key, Vec::new()).unwrap();
        assert!(matches!(empty, FileStorageWriteOutcome::FileIncomplete));

        let first = file_storage
            .write_chunks_batched(&key, vec![(chunk_ids[0], chunks[0].clone())])
            .unwrap();
        assert!(matches!(first, FileStorageWriteOutcome::FileIncomplete));

        let second = file_storage
            .write_chunks_batched(&key, vec![(chunk_ids[1], chunks[1].clone())])
            .unwrap();
        assert!(matches!(second, FileStorageWriteOutcome::FileComplete));

        assert_eq!(file_storage.stored_chunks_count(&key).unwrap(), 2);
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
            let file_metadata = FileMetadata::new(
                <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
                bucket_id.to_vec(),
                location.to_string().into_bytes(),
                32u64 * chunks.len() as u64,
                file_trie.get_root().as_ref().into(),
            )
            .unwrap();

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

    #[test]
    fn delete_files_with_prefix_after_insert_file_and_writes_works() {
        let storage = StorageDb {
            db: Arc::new(kvdb_memorydb::create(NUMBER_OF_COLUMNS)),
            _marker: Default::default(),
        };

        let mut file_storage = RocksDbFileStorage::<LayoutV1<BlakeTwo256>, InMemory>::new(storage);

        // Create a file via insert_file (which now also writes BucketPrefix) and then write chunks.
        let mut tmp_trie = RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>, InMemory>::new(
            file_storage.storage.clone(),
        );
        // Write 3 chunks to derive a fingerprint for the metadata
        for i in 0..3u64 {
            tmp_trie
                .write_chunk(
                    &ChunkId::new(i),
                    &Chunk::from([i as u8; FILE_CHUNK_SIZE as usize]),
                )
                .unwrap();
        }
        let fingerprint = Fingerprint::from(tmp_trie.get_root().as_ref());

        let file_metadata = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            [7u8; 32].to_vec(),
            "location_after_insert_file".to_string().into_bytes(),
            FILE_CHUNK_SIZE * 3,
            fingerprint,
        )
        .unwrap();
        let file_key = file_metadata.file_key::<BlakeTwo256>();

        // Insert only metadata (path under test)
        file_storage.insert_file(file_key, file_metadata).unwrap();

        // Then write chunks using the storage API
        for i in 0..3u64 {
            file_storage
                .write_chunk(
                    &file_key,
                    &ChunkId::new(i),
                    &Chunk::from([i as u8; FILE_CHUNK_SIZE as usize]),
                )
                .unwrap();
        }

        // Sanity: metadata and chunk are accessible
        assert!(file_storage.get_metadata(&file_key).is_ok());
        assert!(file_storage
            .get_chunk(&file_key, &ChunkId::new(0u64))
            .is_ok());

        // Now delete by bucket prefix and ensure it is removed
        file_storage.delete_files_with_prefix(&[7u8; 32]).unwrap();

        assert!(file_storage
            .get_metadata(&file_key)
            .is_ok_and(|m| m.is_none()));
        assert!(file_storage
            .get_chunk(&file_key, &ChunkId::new(0u64))
            .is_err());
    }

    #[test]
    fn multi_bucket_same_fingerprint_refcount_and_idempotent_delete() {
        let storage = StorageDb {
            db: Arc::new(kvdb_memorydb::create(NUMBER_OF_COLUMNS)),
            _marker: Default::default(),
        };

        let mut tmp_trie =
            RocksDbFileDataTrie::<LayoutV1<BlakeTwo256>, InMemory>::new(storage.clone());
        // Build a common fingerprint
        for i in 0..2u64 {
            tmp_trie
                .write_chunk(
                    &ChunkId::new(i),
                    &Chunk::from([i as u8; FILE_CHUNK_SIZE as usize]),
                )
                .unwrap();
        }
        let fingerprint = Fingerprint::from(tmp_trie.get_root().as_ref());

        // Create two metadata objects pointing to same fingerprint but different buckets
        let meta_a = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            [10u8; 32].to_vec(),
            "loc_a".as_bytes().to_vec(),
            FILE_CHUNK_SIZE * 2,
            fingerprint.clone(),
        )
        .unwrap();
        let key_a = meta_a.file_key::<BlakeTwo256>();

        let meta_b = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            [11u8; 32].to_vec(),
            "loc_b".as_bytes().to_vec(),
            FILE_CHUNK_SIZE * 2,
            fingerprint,
        )
        .unwrap();
        let key_b = meta_b.file_key::<BlakeTwo256>();

        let mut file_storage = RocksDbFileStorage::<LayoutV1<BlakeTwo256>, InMemory>::new(storage);

        // Insert both files via insert_file and then write chunks
        file_storage.insert_file(key_a, meta_a).unwrap();
        file_storage.insert_file(key_b, meta_b).unwrap();

        for i in 0..2u64 {
            let ch = Chunk::from([i as u8; FILE_CHUNK_SIZE as usize]);
            file_storage
                .write_chunk(&key_a, &ChunkId::new(i), &ch)
                .unwrap();
            file_storage
                .write_chunk(&key_b, &ChunkId::new(i), &ch)
                .unwrap();
        }

        // Delete bucket A only
        file_storage.delete_files_with_prefix(&[10u8; 32]).unwrap();

        // File under bucket B should still be accessible
        assert!(file_storage.get_metadata(&key_b).is_ok());
        assert!(file_storage.get_chunk(&key_b, &ChunkId::new(0)).is_ok());

        // Deleting bucket B should clean up everything
        file_storage.delete_files_with_prefix(&[11u8; 32]).unwrap();
        assert!(file_storage.get_metadata(&key_b).is_ok_and(|m| m.is_none()));

        // Idempotent: deleting bucket B again does nothing and should not error
        assert!(file_storage.delete_files_with_prefix(&[11u8; 32]).is_ok());
    }

    const MIB: u64 = 1024 * 1024;
    const BATCH_SIZE_BYTES: usize = 2 * 1024 * 1024;

    fn unique_temp_rocksdb_path(label: &str) -> PathBuf {
        let nanos = SystemTime::UNIX_EPOCH
            .elapsed()
            .expect("System time should be after UNIX epoch")
            .as_nanos();
        let seq = BENCHMARK_DB_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);

        std::env::temp_dir().join(format!(
            "sh-file-manager-rocksdb-{label}-pid{}-{nanos}-{seq}",
            std::process::id(),
        ))
    }

    fn setup_disk_backed_storage_for_size(
        size_mb: u64,
    ) -> (
        PathBuf,
        RocksDbFileStorage<LayoutV1<BlakeTwo256>, RocksDbDatabase>,
        H256,
        u64,
    ) {
        let db_path = unique_temp_rocksdb_path(&format!("{size_mb}mb"));
        let db_path_str = db_path.to_string_lossy().into_owned();

        let storage =
            RocksDbFileStorage::<LayoutV1<BlakeTwo256>, RocksDbDatabase>::rocksdb_storage(
                db_path_str,
            )
            .expect("Should create disk-backed RocksDB storage for benchmark");

        let mut file_storage =
            RocksDbFileStorage::<LayoutV1<BlakeTwo256>, RocksDbDatabase>::new(storage);

        let file_size_bytes = size_mb
            .checked_mul(MIB)
            .expect("File size in bytes should not overflow");
        let chunk_count =
            file_size_bytes / FILE_CHUNK_SIZE + (file_size_bytes % FILE_CHUNK_SIZE != 0) as u64;

        let mut fingerprint_bytes = [0u8; H_LENGTH];
        fingerprint_bytes[..8].copy_from_slice(&size_mb.to_le_bytes());

        let metadata = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            [size_mb as u8; 32].to_vec(),
            format!("rocksdb-write-bench-{size_mb}mb").into_bytes(),
            file_size_bytes,
            Fingerprint::from(fingerprint_bytes),
        )
        .expect("Metadata should be valid for benchmark file");

        let file_key = metadata.file_key::<BlakeTwo256>();
        file_storage
            .insert_file(file_key, metadata)
            .expect("Should insert benchmark file metadata");

        (db_path, file_storage, file_key, chunk_count)
    }

    fn run_disk_write_benchmark(size_mb: u64) {
        let (db_path, mut file_storage, file_key, chunk_count) =
            setup_disk_backed_storage_for_size(size_mb);
        let batch_size_chunks = usize::max(1, BATCH_SIZE_BYTES / (FILE_CHUNK_SIZE as usize));

        let mut next_chunk_id: u64 = 0;
        let mut total_batches: u64 = 0;
        let mut write_only_time = std::time::Duration::ZERO;
        let benchmark_start = Instant::now();

        while next_chunk_id < chunk_count {
            let remaining_chunks = chunk_count - next_chunk_id;
            let chunks_in_batch = usize::min(batch_size_chunks, remaining_chunks as usize);

            let mut batch = Vec::with_capacity(chunks_in_batch);
            for _ in 0..chunks_in_batch {
                let mut chunk = vec![0u8; FILE_CHUNK_SIZE as usize];
                chunk[..8].copy_from_slice(&next_chunk_id.to_le_bytes());
                batch.push((ChunkId::new(next_chunk_id), chunk));
                next_chunk_id += 1;
            }

            let write_start = Instant::now();
            let _ = file_storage
                .write_chunks_batched(&file_key, batch)
                .expect("Batch write should succeed");
            write_only_time = write_only_time.saturating_add(write_start.elapsed());
            total_batches = total_batches.saturating_add(1);
        }

        let total_time = benchmark_start.elapsed();
        let stored_chunks = file_storage
            .stored_chunks_count(&file_key)
            .expect("Should read stored chunk count");
        assert_eq!(
            stored_chunks, chunk_count,
            "Stored chunk count should match the number of written chunks",
        );

        let throughput_mb_s = size_mb as f64 / total_time.as_secs_f64();
        eprintln!(
            "ROCKSDB WRITE BENCH: size_mb={size_mb} chunks={chunk_count} batches={total_batches} total_ms={} write_ms={} throughput_mb_s={throughput_mb_s:.2}",
            total_time.as_millis(),
            write_only_time.as_millis(),
        );

        drop(file_storage);
        let _ = std::fs::remove_dir_all(db_path);
    }

    fn run_disk_write_benchmark_regular(size_mb: u64) {
        let (db_path, mut file_storage, file_key, chunk_count) =
            setup_disk_backed_storage_for_size(size_mb);
        let benchmark_start = Instant::now();
        let mut write_only_time = std::time::Duration::ZERO;

        for chunk_index in 0..chunk_count {
            let mut chunk = vec![0u8; FILE_CHUNK_SIZE as usize];
            chunk[..8].copy_from_slice(&chunk_index.to_le_bytes());

            let write_start = Instant::now();
            let _ = file_storage
                .write_chunk(&file_key, &ChunkId::new(chunk_index), &chunk)
                .expect("Regular write_chunk should succeed");
            write_only_time = write_only_time.saturating_add(write_start.elapsed());
        }

        let total_time = benchmark_start.elapsed();
        let stored_chunks = file_storage
            .stored_chunks_count(&file_key)
            .expect("Should read stored chunk count");
        assert_eq!(
            stored_chunks, chunk_count,
            "Stored chunk count should match the number of written chunks",
        );

        let throughput_mb_s = size_mb as f64 / total_time.as_secs_f64();
        eprintln!(
            "ROCKSDB WRITE BENCH REGULAR: size_mb={size_mb} chunks={chunk_count} total_ms={} write_ms={} throughput_mb_s={throughput_mb_s:.2}",
            total_time.as_millis(),
            write_only_time.as_millis(),
        );

        drop(file_storage);
        let _ = std::fs::remove_dir_all(db_path);
    }

    #[test]
    #[ignore = "Disk benchmark for RocksDB write path. Run with: cargo test -p shc-file-manager rocksdb_write_benchmark_batch_sizes -- --ignored --nocapture"]
    fn rocksdb_write_benchmark_batch_sizes() {
        let _guard = BENCHMARK_TEST_LOCK
            .lock()
            .expect("Benchmark lock should not be poisoned");
        for size_mb in [1u64, 10, 50] {
            run_disk_write_benchmark(size_mb);
        }
    }

    #[test]
    #[ignore = "Disk benchmark for regular RocksDB write_chunk path. Run with: cargo test -p shc-file-manager rocksdb_write_benchmark_regular_chunk_sizes -- --ignored --nocapture"]
    fn rocksdb_write_benchmark_regular_chunk_sizes() {
        let _guard = BENCHMARK_TEST_LOCK
            .lock()
            .expect("Benchmark lock should not be poisoned");
        for size_mb in [1u64, 10, 50] {
            run_disk_write_benchmark_regular(size_mb);
        }
    }
}
