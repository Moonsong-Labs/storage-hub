use std::{collections::HashMap, fmt::Debug, hash::Hash, sync::Arc};

use async_trait::async_trait;
use log::error;
use shc_common::types::StorageProofsMerkleTrieLayout;
use shc_forest_manager::{
    in_memory::InMemoryForestStorage,
    rocksdb::{self, RocksDBForestStorage},
    traits::{ForestStorage, ForestStorageHandler},
};
use tokio::sync::RwLock;

const LOG_TARGET: &str = "forest-storage-handler";

/// Forest storage handler that manages a single forest storage instance.
#[derive(Debug)]
pub struct ForestStorageSingle<FS>
where
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
{
    storage_path: Option<String>,
    fs_instance: Arc<RwLock<FS>>,
}

impl<FS> Clone for ForestStorageSingle<FS>
where
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
{
    fn clone(&self) -> Self {
        Self {
            storage_path: self.storage_path.clone(),
            fs_instance: self.fs_instance.clone(),
        }
    }
}

impl ForestStorageSingle<InMemoryForestStorage<StorageProofsMerkleTrieLayout>> {
    pub fn new() -> Self {
        Self {
            storage_path: None,
            fs_instance: Arc::new(RwLock::new(InMemoryForestStorage::new())),
        }
    }
}

impl
    ForestStorageSingle<RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>>
{
    pub fn new(storage_path: String) -> Self {
        let fs = rocksdb::create_db::<StorageProofsMerkleTrieLayout>(storage_path.clone())
            .expect("Failed to create RocksDB");

        let fs = RocksDBForestStorage::new(fs).expect("Failed to create Forest Storage");

        Self {
            storage_path: Some(storage_path),
            fs_instance: Arc::new(RwLock::new(fs)),
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
impl ForestStorageHandler
    for ForestStorageSingle<InMemoryForestStorage<StorageProofsMerkleTrieLayout>>
{
    type Key = NoKey;
    type FS = InMemoryForestStorage<StorageProofsMerkleTrieLayout>;

    async fn get(&self, _key: &Self::Key) -> Option<Arc<RwLock<Self::FS>>> {
        Some(self.fs_instance.clone())
    }

    async fn insert(&mut self, _key: &Self::Key) -> Arc<RwLock<Self::FS>> {
        let fs: InMemoryForestStorage<sp_trie::LayoutV1<polkadot_primitives::BlakeTwo256>> =
            InMemoryForestStorage::new();

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
impl ForestStorageHandler
    for ForestStorageSingle<
        RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>,
    >
{
    type Key = NoKey;
    type FS = RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>;

    async fn get(&self, _key: &Self::Key) -> Option<Arc<RwLock<Self::FS>>> {
        Some(self.fs_instance.clone())
    }

    async fn insert(&mut self, _key: &Self::Key) -> Arc<RwLock<Self::FS>> {
        let fs = rocksdb::create_db::<StorageProofsMerkleTrieLayout>(
            self.storage_path
                .clone()
                .expect("Storage path should be set for RocksDB implementation"),
        )
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
pub struct ForestStorageCaching<K, FS>
where
    K: Eq + Hash + Send + Sync,
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
{
    storage_path: Option<String>,
    fs_instances: Arc<RwLock<HashMap<K, Arc<RwLock<FS>>>>>,
}

impl<K, FS> Clone for ForestStorageCaching<K, FS>
where
    K: Eq + Hash + Send + Sync,
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
{
    fn clone(&self) -> Self {
        Self {
            storage_path: self.storage_path.clone(),
            fs_instances: self.fs_instances.clone(),
        }
    }
}

impl<K> ForestStorageCaching<K, InMemoryForestStorage<StorageProofsMerkleTrieLayout>>
where
    K: Eq + Hash + Send + Sync,
{
    pub fn new() -> Self {
        Self {
            storage_path: None,
            fs_instances: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl<K>
    ForestStorageCaching<
        K,
        RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>,
    >
where
    K: Eq + Hash + Send + Sync,
{
    pub fn new(storage_path: String) -> Self {
        Self {
            storage_path: Some(storage_path),
            fs_instances: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl<K> ForestStorageHandler
    for ForestStorageCaching<K, InMemoryForestStorage<StorageProofsMerkleTrieLayout>>
where
    K: Eq + Hash + From<Vec<u8>> + Clone + Debug + Send + Sync + 'static,
{
    type Key = K;
    type FS = InMemoryForestStorage<StorageProofsMerkleTrieLayout>;

    async fn get(&self, key: &Self::Key) -> Option<Arc<RwLock<Self::FS>>> {
        self.fs_instances.read().await.get(key).cloned()
    }

    async fn insert(&mut self, key: &Self::Key) -> Arc<RwLock<Self::FS>> {
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
impl<K> ForestStorageHandler
    for ForestStorageCaching<
        K,
        RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>,
    >
where
    K: Eq + Hash + From<Vec<u8>> + Clone + Debug + Send + Sync + 'static,
{
    type Key = K;
    type FS = RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>;

    async fn get(&self, key: &Self::Key) -> Option<Arc<RwLock<Self::FS>>> {
        self.fs_instances.read().await.get(key).cloned()
    }

    async fn insert(&mut self, key: &Self::Key) -> Arc<RwLock<Self::FS>> {
        let mut fs_instances = self.fs_instances.write().await;

        // Return potentially existing instance since we waited for the lock.
        // This is for the case where many threads called `insert` at the same time with the same `key`.
        if let Some(fs) = fs_instances.get(key) {
            return fs.clone();
        }

        let new_db_storage_path = format!(
            "{}_{:?}",
            self.storage_path
                .clone()
                .expect("Storage path should be set for RocksDB implementation"),
            key.clone()
        );

        let underlying_db =
            rocksdb::create_db::<StorageProofsMerkleTrieLayout>(new_db_storage_path)
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

        let underlying_db = match rocksdb::copy_db(src, dest) {
            Ok(db) => db,
            Err(e) => {
                error!(target: LOG_TARGET, "Failed to copy RocksDB: {}", e);
                return None;
            }
        };

        let forest_storage =
            RocksDBForestStorage::new(underlying_db).expect("Failed to create Forest Storage");

        let forest_storage = Arc::new(RwLock::new(forest_storage));

        fs_instances.insert(dest_key.clone(), forest_storage.clone());

        Some(forest_storage)
    }
}
