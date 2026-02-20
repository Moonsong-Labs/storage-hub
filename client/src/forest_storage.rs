use std::{
    collections::HashSet,
    fmt::{Debug, Display},
    hash::Hash,
    marker::PhantomData,
    num::NonZeroUsize,
    path::PathBuf,
    sync::Arc,
};

use anyhow::Result;
use async_trait::async_trait;
use log::{debug, error, info, warn};
use lru::LruCache;
use shc_common::{traits::StorageEnableRuntime, types::StorageProofsMerkleTrieLayout};
use shc_forest_manager::{
    in_memory::InMemoryForestStorage,
    rocksdb::{self, RocksDBForestStorage},
    traits::{ForestStorage, ForestStorageHandler},
};
use tokio::sync::RwLock;

use crate::types::FOREST_STORAGE_PATH;

const LOG_TARGET: &str = "forest-storage-handler";

/// Forest storage handler that manages a single forest storage instance.
#[derive(Debug)]
pub struct ForestStorageSingle<FS, Runtime>
where
    FS: ForestStorage<StorageProofsMerkleTrieLayout, Runtime> + Send + Sync,
    Runtime: StorageEnableRuntime,
{
    storage_path: Option<String>,
    fs_instance: Arc<RwLock<FS>>,
    _runtime: PhantomData<Runtime>,
}

impl<FS, Runtime> Clone for ForestStorageSingle<FS, Runtime>
where
    FS: ForestStorage<StorageProofsMerkleTrieLayout, Runtime> + Send + Sync,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> Self {
        Self {
            storage_path: self.storage_path.clone(),
            fs_instance: self.fs_instance.clone(),
            _runtime: PhantomData,
        }
    }
}

impl<Runtime> ForestStorageSingle<InMemoryForestStorage<StorageProofsMerkleTrieLayout>, Runtime>
where
    Runtime: StorageEnableRuntime,
{
    pub fn new() -> Self {
        Self {
            storage_path: None,
            fs_instance: Arc::new(RwLock::new(InMemoryForestStorage::new())),
            _runtime: PhantomData,
        }
    }
}

impl<Runtime>
    ForestStorageSingle<
        RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>,
        Runtime,
    >
where
    Runtime: StorageEnableRuntime,
{
    #[allow(dead_code)]
    pub fn new(storage_path: String) -> Self {
        let mut path = PathBuf::new();
        path.push(storage_path.clone());
        path.push(FOREST_STORAGE_PATH);

        let path_str = path.to_string_lossy().to_string();
        debug!(target: LOG_TARGET, "Creating RocksDB at path: {}", path_str);

        let fs = rocksdb::create_db::<StorageProofsMerkleTrieLayout>(path_str)
            .expect("Failed to create RocksDB");

        let fs = RocksDBForestStorage::new(fs).expect("Failed to create Forest Storage");

        Self {
            storage_path: Some(storage_path),
            fs_instance: Arc::new(RwLock::new(fs)),
            _runtime: PhantomData,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NoKey;

impl From<Vec<u8>> for NoKey {
    fn from(_: Vec<u8>) -> Self {
        NoKey
    }
}

impl AsRef<[u8]> for NoKey {
    fn as_ref(&self) -> &[u8] {
        &[]
    }
}

#[async_trait]
impl<Runtime> ForestStorageHandler<Runtime>
    for ForestStorageSingle<InMemoryForestStorage<StorageProofsMerkleTrieLayout>, Runtime>
where
    Runtime: StorageEnableRuntime,
{
    type Key = NoKey;
    type FS = InMemoryForestStorage<StorageProofsMerkleTrieLayout>;

    async fn get(&self, _key: &Self::Key) -> Option<Arc<RwLock<Self::FS>>> {
        Some(self.fs_instance.clone())
    }

    async fn create(&mut self, _key: &Self::Key) -> Result<Arc<RwLock<Self::FS>>> {
        let fs: InMemoryForestStorage<StorageProofsMerkleTrieLayout> = InMemoryForestStorage::new();

        let fs = Arc::new(RwLock::new(fs));
        self.fs_instance = fs.clone();
        Ok(fs)
    }

    async fn remove_forest_storage(&mut self, _key: &Self::Key) {}

    async fn is_forest_storage_present(&self, _key: &Self::Key) -> bool {
        true
    }

    async fn snapshot(
        &self,
        _key: &Self::Key,
        _key_for_copy: &Self::Key,
    ) -> Option<Arc<RwLock<Self::FS>>> {
        None
    }
}

#[async_trait]
impl<Runtime> ForestStorageHandler<Runtime>
    for ForestStorageSingle<
        RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>,
        Runtime,
    >
where
    Runtime: StorageEnableRuntime,
{
    type Key = NoKey;
    type FS = RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>;

    async fn get(&self, _key: &Self::Key) -> Option<Arc<RwLock<Self::FS>>> {
        Some(self.fs_instance.clone())
    }

    async fn create(&mut self, _key: &Self::Key) -> Result<Arc<RwLock<Self::FS>>> {
        let mut path = PathBuf::new();
        path.push(
            self.storage_path
                .clone()
                .expect("Storage path should be set for RocksDB implementation"),
        );
        path.push(FOREST_STORAGE_PATH);

        let path_str = path.to_string_lossy().to_string();
        debug!(target: LOG_TARGET, "Creating RocksDB at path: {}", path_str);

        let fs = rocksdb::create_db::<StorageProofsMerkleTrieLayout>(path_str)
            .map_err(|e| anyhow::anyhow!("Failed to create RocksDB: {:?}", e))?;

        let fs = RocksDBForestStorage::new(fs)
            .map_err(|e| anyhow::anyhow!("Failed to create Forest Storage: {:?}", e))?;

        let fs = Arc::new(RwLock::new(fs));
        self.fs_instance = fs.clone();
        Ok(fs)
    }

    async fn remove_forest_storage(&mut self, _key: &Self::Key) {}

    async fn is_forest_storage_present(&self, _key: &Self::Key) -> bool {
        true
    }

    async fn snapshot(
        &self,
        _key: &Self::Key,
        _key_for_copy: &Self::Key,
    ) -> Option<Arc<RwLock<Self::FS>>> {
        None
    }
}

/// Forest storage handler that manages multiple forest storage instances with lazy loading.
///
/// Uses an LRU cache to limit the number of simultaneously open RocksDB instances,
/// preventing file descriptor exhaustion when managing many forests.
/// Forests are opened on-demand and evicted when the cache reaches capacity.
pub struct ForestStorageCaching<K, FS, Runtime>
where
    K: Eq + Hash + Send + Sync,
    FS: ForestStorage<StorageProofsMerkleTrieLayout, Runtime> + Send + Sync,
    Runtime: StorageEnableRuntime,
{
    /// Path to the storage directory.
    storage_path: Option<String>,
    /// LRU cache of currently-open forest instances.
    ///
    /// ! IMPORTANT: If you need to get a write lock on `open_forests` and
    /// `write_forests` together, you must acquire the write lock on `known_forests` first.
    /// This is necessary to prevent race conditions resulting in deadlocks.
    open_forests: Arc<RwLock<LruCache<K, Arc<RwLock<FS>>>>>,
    /// Set of all known forest keys that exist on disk.
    ///
    /// ! IMPORTANT: If you need to get a write lock on `open_forests` and
    /// `write_forests` together, you must acquire the write lock on `known_forests` first.
    /// This is necessary to prevent race conditions resulting in deadlocks.
    known_forests: Arc<RwLock<HashSet<K>>>,
    _runtime: PhantomData<Runtime>,
}

impl<K, FS, Runtime> Clone for ForestStorageCaching<K, FS, Runtime>
where
    K: Eq + Hash + Send + Sync,
    FS: ForestStorage<StorageProofsMerkleTrieLayout, Runtime> + Send + Sync,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> Self {
        Self {
            storage_path: self.storage_path.clone(),
            open_forests: self.open_forests.clone(),
            known_forests: self.known_forests.clone(),
            _runtime: PhantomData,
        }
    }
}

impl<K, FS, Runtime> std::fmt::Debug for ForestStorageCaching<K, FS, Runtime>
where
    K: Eq + Hash + Send + Sync,
    FS: ForestStorage<StorageProofsMerkleTrieLayout, Runtime> + Send + Sync,
    Runtime: StorageEnableRuntime,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForestStorageCaching")
            .field("storage_path", &self.storage_path)
            .finish_non_exhaustive()
    }
}

impl<K, Runtime>
    ForestStorageCaching<K, InMemoryForestStorage<StorageProofsMerkleTrieLayout>, Runtime>
where
    K: Eq + Hash + From<Vec<u8>> + Clone + Debug + Display + Send + Sync,
    Runtime: StorageEnableRuntime,
{
    /// Creates a new in-memory ForestStorageCaching.
    ///
    /// # Arguments
    /// * `max_open_forests` - Maximum number of forests to keep open simultaneously.
    pub fn new() -> Self {
        Self {
            storage_path: None,
            open_forests: Arc::new(RwLock::new(LruCache::unbounded())),
            known_forests: Arc::new(RwLock::new(HashSet::new())),
            _runtime: PhantomData,
        }
    }
}

impl<K, Runtime>
    ForestStorageCaching<
        K,
        RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>,
        Runtime,
    >
where
    K: Eq + Hash + From<Vec<u8>> + Clone + Debug + Display + Send + Sync,
    Runtime: StorageEnableRuntime,
{
    /// Creates a new ForestStorageCaching with lazy loading.
    ///
    /// Scans the storage directory to discover existing forests but does NOT open them.
    /// Forests are opened on-demand when accessed via `get()` or `create()`.
    ///
    /// # Arguments
    /// * `storage_path` - Path to the storage directory.
    /// * `max_open_forests` - Maximum number of forests to keep open simultaneously.
    pub fn new(storage_path: String, max_open_forests: usize) -> Self {
        let mut known = HashSet::new();

        // Compute the base folder where per-bucket RocksDB instances live
        let mut base = PathBuf::new();
        base.push(&storage_path);
        base.push(FOREST_STORAGE_PATH);

        // Scan disk to discover existing forests
        match std::fs::read_dir(&base) {
            Ok(entries) => {
                for entry_result in entries {
                    let Ok(entry) = entry_result else { continue };
                    let path = entry.path();
                    if !path.is_dir() {
                        continue;
                    }
                    let name_os = match path.file_name() {
                        Some(n) => n,
                        None => continue,
                    };
                    let name = name_os.to_string_lossy();
                    // Expect directory names to be hex string keys, typically formatted via Display as "0x<hex>"
                    let hex_str = name.strip_prefix("0x").unwrap_or(&name);
                    // Decode to bytes; skip if malformed
                    let key_bytes = match hex::decode(hex_str) {
                        Ok(b) => b,
                        Err(e) => {
                            warn!(target: LOG_TARGET, "Skipping invalid forest dir name '{}': {}", name, e);
                            continue;
                        }
                    };

                    // Register the key as known
                    let key: K = K::from(key_bytes);
                    known.insert(key);
                }

                if !known.is_empty() {
                    info!(
                        target: LOG_TARGET,
                        "🌳 Discovered {} forest(s) on disk (lazy loading enabled, max open: {})",
                        known.len(),
                        max_open_forests
                    );
                } else {
                    debug!(
                        target: LOG_TARGET,
                        "No existing forests found at {}",
                        base.to_string_lossy()
                    );
                }
            }
            Err(e) => {
                error!(
                    target: LOG_TARGET,
                    "Forest base path not found or unreadable ({}): {}",
                    base.to_string_lossy(),
                    e
                );
            }
        }

        Self {
            storage_path: Some(storage_path),
            open_forests: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(max_open_forests).expect("max_open_forests must be > 0"),
            ))),
            known_forests: Arc::new(RwLock::new(known)),
            _runtime: PhantomData,
        }
    }

    /// Opens a forest from disk and returns it.
    /// Returns an error if the forest cannot be opened.
    fn open_forest_from_disk(
        &self,
        key: &K,
    ) -> Result<
        Arc<RwLock<RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>>>,
    > {
        let storage_path = self
            .storage_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Storage path not set"))?;

        let mut db_path = PathBuf::new();
        db_path.push(storage_path);
        db_path.push(FOREST_STORAGE_PATH);
        db_path.push(key.to_string());

        let db_path_str = db_path.to_string_lossy().to_string();
        debug!(target: LOG_TARGET, "Lazy-loading forest from disk: {}", db_path_str);

        let storage_db = rocksdb::create_db::<StorageProofsMerkleTrieLayout>(db_path_str)
            .map_err(|e| anyhow::anyhow!("Failed to open RocksDB for forest [{}]: {:?}", key, e))?;

        let fs = RocksDBForestStorage::new(storage_db)
            .map_err(|e| anyhow::anyhow!("Failed to initialise forest [{}]: {:?}", key, e))?;

        Ok(Arc::new(RwLock::new(fs)))
    }

    /// Creates a new forest on disk and returns it.
    fn create_new_forest_on_disk(
        &self,
        key: &K,
    ) -> Result<
        Arc<RwLock<RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>>>,
    > {
        let storage_path = self
            .storage_path
            .as_ref()
            .expect("Storage path should be set for RocksDB implementation");

        let mut path = PathBuf::new();
        path.push(storage_path);
        path.push(FOREST_STORAGE_PATH);
        path.push(key.to_string());

        let path_str = path.to_string_lossy().to_string();
        debug!(target: LOG_TARGET, "Creating new forest at: {}", path_str);

        let underlying_db =
            rocksdb::create_db::<StorageProofsMerkleTrieLayout>(path_str).map_err(|e| {
                anyhow::anyhow!("Failed to create RocksDB for forest [{}]: {:?}", key, e)
            })?;

        let forest_storage = RocksDBForestStorage::new(underlying_db).map_err(|e| {
            anyhow::anyhow!(
                "Failed to create Forest Storage for forest [{}]: {:?}",
                key,
                e
            )
        })?;

        Ok(Arc::new(RwLock::new(forest_storage)))
    }
}

#[async_trait]
impl<K, Runtime> ForestStorageHandler<Runtime>
    for ForestStorageCaching<K, InMemoryForestStorage<StorageProofsMerkleTrieLayout>, Runtime>
where
    K: Eq + Hash + From<Vec<u8>> + AsRef<[u8]> + Clone + Debug + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    type Key = K;
    type FS = InMemoryForestStorage<StorageProofsMerkleTrieLayout>;

    async fn get(&self, key: &Self::Key) -> Option<Arc<RwLock<Self::FS>>> {
        // Check if the forest is known
        let known = self.known_forests.read().await;
        if !known.contains(key) {
            return None;
        }
        drop(known);

        // Get the forest from the cache (only place it can be in the InMemoryForestStorage implementation)
        let mut cache = self.open_forests.write().await;
        cache.get(key).cloned()
    }

    async fn create(&mut self, key: &Self::Key) -> Result<Arc<RwLock<Self::FS>>> {
        // Acquire the known forests write lock to prevent concurrent threads trying to create the same forest.
        // ! IMPORTANT: We will later have to acquire the `open_forests` write lock, but first we acquire the
        // ! `known_forests` write lock to prevent race conditions resulting in deadlocks.
        let mut known_forest_write_lock = self.known_forests.write().await;

        // Now that we have the write lock, check if the forest is already known.
        // This would be the case if there were more than one thread trying to create the same forest
        // at the same time, and this thread got the write lock only after another thread had already
        // created the forest.
        if known_forest_write_lock.contains(key) {
            // Hold the cache write lock across the entire check-and-open operation.
            // This prevents multiple threads from trying to create the same in-memory forest simultaneously.
            let mut cache = self.open_forests.write().await;

            // Check if the forest is already in the cache
            if let Some(fs) = cache.get(key) {
                return Ok(fs.clone());
            }

            // The forest is known, but missing from the cache (should be rare).
            // Re-create it in memory and insert it into the cache.
            let fs = Arc::new(RwLock::new(InMemoryForestStorage::new()));

            // Add the forest to the LRU cache
            cache.put(key.clone(), fs.clone());

            return Ok(fs);
        }

        // Now we know this thread is the first and only one trying to create the forest.
        // Create a new in-memory forest
        let fs = Arc::new(RwLock::new(InMemoryForestStorage::new()));

        // Register the forest as known and add to the cache
        known_forest_write_lock.insert(key.clone());
        self.open_forests.write().await.put(key.clone(), fs.clone());

        Ok(fs)
    }

    async fn remove_forest_storage(&mut self, key: &Self::Key) {
        self.known_forests.write().await.remove(key);
        self.open_forests.write().await.pop(key);
    }

    async fn is_forest_storage_present(&self, key: &Self::Key) -> bool {
        self.known_forests.read().await.contains(key)
    }

    async fn snapshot(
        &self,
        src_key: &Self::Key,
        dest_key: &Self::Key,
    ) -> Option<Arc<RwLock<Self::FS>>> {
        // Return potentially existing instance since we have to wait for the lock.
        // This is for the case where many threads called `snapshot` at the same time with the same `dest_key`.
        if let Some(fs) = self.get(dest_key).await {
            return Some(fs);
        }

        // Get the source forest from the cache
        let forest_storage_src = self.get(src_key).await?;

        // Create a copy of the Forest Storage
        let forest_storage_dest = forest_storage_src.read().await.clone();
        let forest_storage_dest = Arc::new(RwLock::new(forest_storage_dest));

        // Register the destination forest as known and add to the cache
        self.known_forests.write().await.insert(dest_key.clone());
        self.open_forests
            .write()
            .await
            .put(dest_key.clone(), forest_storage_dest.clone());

        Some(forest_storage_dest)
    }
}

#[async_trait]
impl<K, Runtime> ForestStorageHandler<Runtime>
    for ForestStorageCaching<
        K,
        RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>,
        Runtime,
    >
where
    K: Eq + Hash + From<Vec<u8>> + AsRef<[u8]> + Clone + Debug + Display + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    type Key = K;
    type FS = RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>;

    async fn get(&self, key: &Self::Key) -> Option<Arc<RwLock<Self::FS>>> {
        // Check if the forest exists on disk
        {
            let known = self.known_forests.read().await;
            if !known.contains(key) {
                return None;
            }
        }

        // Hold the cache write lock across the entire check-and-open operation.
        // This prevents multiple threads from trying to open the same RocksDB file simultaneously.
        let mut cache = self.open_forests.write().await;

        // Check if the forest is already in the cache
        if let Some(fs) = cache.get(key) {
            return Some(fs.clone());
        }

        // Open the forest from disk
        let fs = match self.open_forest_from_disk(key) {
            Ok(fs) => fs,
            Err(e) => {
                error!(target: LOG_TARGET, "Failed to open forest [{}]: {}", key, e);
                return None;
            }
        };

        // Add the forest to the LRU cache
        cache.put(key.clone(), fs.clone());

        Some(fs)
    }

    async fn create(&mut self, key: &Self::Key) -> Result<Arc<RwLock<Self::FS>>> {
        // Acquire the known forests write lock to prevent concurrent threads trying to create the same forest.
        // ! IMPORTANT: We will later have to acquire the `open_forests` write lock, but first we acquire the
        // ! `known_forests` write lock to prevent race conditions resulting in deadlocks.
        let mut known_forest_write_lock = self.known_forests.write().await;

        // Now that we have the write lock, check if the forest is already known.
        // This would be the case if there were more than one thread trying to create the same forest
        // at the same time, and this thread got the write lock only after another thread had already
        // created the forest.
        if known_forest_write_lock.contains(key) {
            // Hold the cache write lock across the entire check-and-open operation.
            // This prevents multiple threads from trying to open the same RocksDB file simultaneously.
            let mut cache = self.open_forests.write().await;

            // Check if the forest is already in the cache
            if let Some(fs) = cache.get(key) {
                return Ok(fs.clone());
            }

            // Open the forest from disk
            let fs = self.open_forest_from_disk(key)?;

            // Add the forest to the LRU cache
            cache.put(key.clone(), fs.clone());

            return Ok(fs);
        }

        // Now we know this thread is the first and only one trying to create the forest.
        // Create a new forest on disk
        let fs = self.create_new_forest_on_disk(key)?;

        // Register the forest as known and add to the cache
        known_forest_write_lock.insert(key.clone());
        self.open_forests.write().await.put(key.clone(), fs.clone());

        Ok(fs)
    }

    async fn remove_forest_storage(&mut self, key: &Self::Key) {
        // Ensure the Arc in the LRU cache is the one we modify.
        // This guarantees all short-term new Arcs share the same instance.
        if let Some(fs_arc) = self.get(key).await {
            // Acquire write lock and set deleting flag.
            // From this point, any other Arc<RwLock<FS>> sharing this instance
            // will fail on all operations.
            let mut fs = fs_arc.write().await;
            fs.deleting = true;
            drop(fs);
        }

        // Delete directory from disk.
        if let Some(ref storage_path) = self.storage_path {
            let mut dir_path = std::path::PathBuf::new();
            dir_path.push(storage_path);
            dir_path.push(FOREST_STORAGE_PATH);
            dir_path.push(key.to_string());

            if dir_path.exists() {
                if let Err(e) = std::fs::remove_dir_all(&dir_path) {
                    error!(
                        target: LOG_TARGET,
                        "Failed to delete forest directory [{}]: {}. Continuing with removal from caches.",
                        dir_path.display(),
                        e
                    );
                }
            }
        }

        // Remove from open_forests (LRU cache).
        // If another thread passed the known_forests check and is waiting on this lock,
        // it will see the forest is gone from the LRU, try to open from disk, and fail.
        self.open_forests.write().await.pop(key);

        // Remove from known_forests.
        self.known_forests.write().await.remove(key);
    }

    async fn is_forest_storage_present(&self, key: &Self::Key) -> bool {
        if let Some(storage_path) = &self.storage_path {
            let mut db_path = PathBuf::new();
            db_path.push(storage_path);
            db_path.push(FOREST_STORAGE_PATH);
            db_path.push(key.to_string());
            db_path.is_dir()
        } else {
            false
        }
    }

    // TODO: This implementation is very expensive. It copies the entire forest from disk to disk,
    // TODO: and holds a read lock on the source forest during the copy.
    // TODO: Consider using RocksDB's native checkpoint feature.
    async fn snapshot(
        &self,
        src_key: &Self::Key,
        dest_key: &Self::Key,
    ) -> Option<Arc<RwLock<Self::FS>>> {
        // Return potentially existing instance since we have to wait for the lock.
        // This is for the case where many threads called `snapshot` at the same time with the same `dest_key`.
        if let Some(fs) = self.get(dest_key).await {
            return Some(fs);
        }

        // Get the source forest (loads it into cache if not present).
        // We keep the Arc to prevent the forest from being dropped even if evicted from cache.
        let src_fs_arc = self.get(src_key).await?;

        let storage_path = self
            .storage_path
            .clone()
            .expect("Storage path should be set");

        // Build source and destination paths
        let mut src_path = PathBuf::new();
        src_path.push(&storage_path);
        src_path.push(FOREST_STORAGE_PATH);
        src_path.push(src_key.to_string());

        let mut dest_path = PathBuf::new();
        dest_path.push(&storage_path);
        dest_path.push(FOREST_STORAGE_PATH);
        dest_path.push(dest_key.to_string());

        let src = src_path.to_string_lossy().to_string();
        let dest = dest_path.to_string_lossy().to_string();

        // Hold a read lock on the source forest during copy to ensure consistency.
        let src_fs = src_fs_arc.read().await;

        // Copy the full source forest files to the destination
        let underlying_db = match rocksdb::copy_db(src, dest) {
            Ok(db) => db,
            Err(e) => {
                error!(target: LOG_TARGET, "Failed to copy RocksDB: {}", e);
                return None;
            }
        };

        // Release the source forest lock
        drop(src_fs);

        // Create a new forest storage instance
        let forest_storage =
            RocksDBForestStorage::new(underlying_db).expect("Failed to create Forest Storage");
        let forest_storage = Arc::new(RwLock::new(forest_storage));

        // Register the destination forest as known and add to the cache
        self.known_forests.write().await.insert(dest_key.clone());
        self.open_forests
            .write()
            .await
            .put(dest_key.clone(), forest_storage.clone());

        Some(forest_storage)
    }
}
