use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    hash::Hash,
    marker::PhantomData,
    path::PathBuf,
    sync::Arc,
};

use async_trait::async_trait;
use log::{debug, error, info, warn};
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

    async fn create(&mut self, _key: &Self::Key) -> Arc<RwLock<Self::FS>> {
        let fs: InMemoryForestStorage<StorageProofsMerkleTrieLayout> = InMemoryForestStorage::new();

        let fs = Arc::new(RwLock::new(fs));
        self.fs_instance = fs.clone();
        fs
    }

    async fn remove_forest_storage(&mut self, _key: &Self::Key) {}

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

    async fn create(&mut self, _key: &Self::Key) -> Arc<RwLock<Self::FS>> {
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
            .expect("Failed to create RocksDB");

        let fs = RocksDBForestStorage::new(fs).expect("Failed to create Forest Storage");

        let fs = Arc::new(RwLock::new(fs));
        self.fs_instance = fs.clone();
        fs
    }

    async fn remove_forest_storage(&mut self, _key: &Self::Key) {}

    async fn snapshot(
        &self,
        _key: &Self::Key,
        _key_for_copy: &Self::Key,
    ) -> Option<Arc<RwLock<Self::FS>>> {
        None
    }
}

/// Forest storage handler that manages multiple forest storage instances.
///
/// The name caching comes from the fact that it maintains a list of existing forest storage instances.
#[derive(Debug)]
pub struct ForestStorageCaching<K, FS, Runtime>
where
    K: Eq + Hash + Send + Sync,
    FS: ForestStorage<StorageProofsMerkleTrieLayout, Runtime> + Send + Sync,
    Runtime: StorageEnableRuntime,
{
    storage_path: Option<String>,
    fs_instances: Arc<RwLock<HashMap<K, Arc<RwLock<FS>>>>>,
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
            fs_instances: self.fs_instances.clone(),
            _runtime: PhantomData,
        }
    }
}

impl<K, Runtime>
    ForestStorageCaching<K, InMemoryForestStorage<StorageProofsMerkleTrieLayout>, Runtime>
where
    K: Eq + Hash + From<Vec<u8>> + Clone + Debug + Display + Send + Sync,
    Runtime: StorageEnableRuntime,
{
    pub fn new() -> Self {
        Self {
            storage_path: None,
            fs_instances: Arc::new(RwLock::new(HashMap::new())),
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
    pub fn new(storage_path: String) -> Self {
        // Build initial map by restoring any pre-existing bucket forests from disk
        let mut instances: HashMap<
            K,
            Arc<
                RwLock<RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>>,
            >,
        > = HashMap::new();

        // Compute the base folder where per-bucket RocksDB instances live
        let mut base = PathBuf::new();
        base.push(&storage_path);
        base.push(FOREST_STORAGE_PATH);

        // Best-effort scan of existing bucket directories
        match std::fs::read_dir(&base) {
            Ok(entries) => {
                let mut restored_count: usize = 0;
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

                    // Build the exact DB path expected by rocksdb::create_db (base/<display(key)>)
                    // We reconstruct the key from bytes using K: From<Vec<u8>>
                    let key: K = K::from(key_bytes);

                    let mut db_path = PathBuf::new();
                    db_path.push(&storage_path);
                    db_path.push(FOREST_STORAGE_PATH);
                    db_path.push(key.to_string());

                    let db_path_str = db_path.to_string_lossy().to_string();
                    match rocksdb::create_db::<StorageProofsMerkleTrieLayout>(db_path_str) {
                        Ok(storage_db) => match RocksDBForestStorage::new(storage_db) {
                            Ok(fs) => {
                                instances.insert(key, Arc::new(RwLock::new(fs)));
                                restored_count += 1;
                            }
                            Err(e) => {
                                warn!(target: LOG_TARGET, "Failed to initialise forest at '{}': {:?}", name, e);
                            }
                        },
                        Err(_e) => {
                            warn!(target: LOG_TARGET, "Failed to open RocksDB for forest dir '{}'; skipping", name);
                        }
                    }
                }
                if restored_count > 0 {
                    info!(target: LOG_TARGET, "🌳 Restored {} forest(s) from disk", restored_count);
                } else {
                    debug!(target: LOG_TARGET, "No existing forests found to restore at {}", base.to_string_lossy());
                }
            }
            Err(e) => {
                debug!(target: LOG_TARGET, "Forest base path not found or unreadable ({}): {}", base.to_string_lossy(), e);
            }
        }

        Self {
            storage_path: Some(storage_path),
            fs_instances: Arc::new(RwLock::new(instances)),
            _runtime: PhantomData,
        }
    }
}

#[async_trait]
impl<K, Runtime> ForestStorageHandler<Runtime>
    for ForestStorageCaching<K, InMemoryForestStorage<StorageProofsMerkleTrieLayout>, Runtime>
where
    K: Eq + Hash + From<Vec<u8>> + Clone + Debug + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    type Key = K;
    type FS = InMemoryForestStorage<StorageProofsMerkleTrieLayout>;

    async fn get(&self, key: &Self::Key) -> Option<Arc<RwLock<Self::FS>>> {
        self.fs_instances.read().await.get(key).cloned()
    }

    async fn create(&mut self, key: &Self::Key) -> Arc<RwLock<Self::FS>> {
        let mut fs_instances = self.fs_instances.write().await;

        // Return potentially existing instance since we waited for the lock.
        // This is for the case where many threads called `insert` at the same time with the same `key`.
        if let Some(fs) = fs_instances.get(key) {
            return fs.clone();
        }

        let forest_storage = InMemoryForestStorage::new();

        let forest_storage = Arc::new(RwLock::new(forest_storage));

        fs_instances.insert(key.clone(), forest_storage.clone());

        forest_storage
    }

    async fn remove_forest_storage(&mut self, key: &Self::Key) {
        self.fs_instances.write().await.remove(key);
    }

    async fn snapshot(
        &self,
        src_key: &Self::Key,
        dest_key: &Self::Key,
    ) -> Option<Arc<RwLock<Self::FS>>> {
        let mut fs_instances = self.fs_instances.write().await;

        // Return potentially existing instance since we waited for the lock
        // This is for the case where many threads called `snapshot` at the same time with the same `dest_key`.
        if let Some(fs) = fs_instances.get(dest_key) {
            return Some(fs.clone());
        }

        let forest_storage_src = fs_instances.get(src_key)?;

        // Create a copy of the Forest Storage
        let forest_storage_dest = forest_storage_src.read().await.clone();
        let forest_storage_dest = Arc::new(RwLock::new(forest_storage_dest));

        fs_instances.insert(dest_key.clone(), forest_storage_dest.clone());

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
    K: Eq + Hash + From<Vec<u8>> + Clone + Debug + Display + Send + Sync + 'static,
    Runtime: StorageEnableRuntime,
{
    type Key = K;
    type FS = RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>;

    async fn get(&self, key: &Self::Key) -> Option<Arc<RwLock<Self::FS>>> {
        self.fs_instances.read().await.get(key).cloned()
    }

    async fn create(&mut self, key: &Self::Key) -> Arc<RwLock<Self::FS>> {
        let mut fs_instances = self.fs_instances.write().await;

        // Return potentially existing instance since we waited for the lock.
        // This is for the case where many threads called `insert` at the same time with the same `key`.
        if let Some(fs) = fs_instances.get(key) {
            return fs.clone();
        }

        let storage_path = self
            .storage_path
            .clone()
            .expect("Storage path should be set for RocksDB implementation");

        let mut path = PathBuf::new();
        path.push(storage_path);
        path.push(FOREST_STORAGE_PATH);
        path.push(key.to_string());

        let path_str = path.to_string_lossy().to_string();
        debug!(target: LOG_TARGET, "Creating RocksDB at path: {}", path_str);

        let underlying_db = rocksdb::create_db::<StorageProofsMerkleTrieLayout>(path_str)
            .expect("Failed to create RocksDB");

        let forest_storage =
            RocksDBForestStorage::new(underlying_db).expect("Failed to create Forest Storage");

        let forest_storage = Arc::new(RwLock::new(forest_storage));

        fs_instances.insert(key.clone(), forest_storage.clone());

        forest_storage
    }

    async fn remove_forest_storage(&mut self, key: &Self::Key) {
        self.fs_instances.write().await.remove(key);
    }

    async fn snapshot(
        &self,
        src_key: &Self::Key,
        dest_key: &Self::Key,
    ) -> Option<Arc<RwLock<Self::FS>>> {
        let mut fs_instances = self.fs_instances.write().await;

        // Return potentially existing instance since we waited for the lock.
        // This is for the case where many threads called `snapshot` at the same time with the same `dest_key`.
        if let Some(fs) = fs_instances.get(dest_key) {
            return Some(fs.clone());
        }

        let storage_path = self
            .storage_path
            .clone()
            .expect("Storage path should be set");
        let src = format!("{}_{:?}", storage_path, src_key);
        let dest = format!("{}_{:?}", storage_path, dest_key);

        // Read-lock the source Forest Storage.
        let src_fs = fs_instances.get(src_key)?.read().await;

        // Copy the full source Forest Storage files to the destination Forest Storage.
        let underlying_db = match rocksdb::copy_db(src, dest) {
            Ok(db) => db,
            Err(e) => {
                error!(target: LOG_TARGET, "Failed to copy RocksDB: {}", e);
                return None;
            }
        };

        // Release the lock on the source Forest Storage.
        drop(src_fs);

        // Create and insert new Forest Storage instance for the destination Forest Storage.
        let forest_storage =
            RocksDBForestStorage::new(underlying_db).expect("Failed to create Forest Storage");
        let forest_storage = Arc::new(RwLock::new(forest_storage));
        fs_instances.insert(dest_key.clone(), forest_storage.clone());

        Some(forest_storage)
    }
}
