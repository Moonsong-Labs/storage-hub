use async_trait::async_trait;
use shc_common::types::StorageProofsMerkleTrieLayout;
use shc_forest_manager::{
    in_memory::InMemoryForestStorage,
    rocksdb::RocksDBForestStorage,
    traits::{ForestStorage, ForestStorageHandler},
};
use std::{collections::HashMap, fmt::Debug, hash::Hash, sync::Arc};
use tokio::sync::RwLock;

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
        let fs = RocksDBForestStorage::<
            StorageProofsMerkleTrieLayout,
            kvdb_rocksdb::Database,
        >::rocksdb_storage(storage_path.clone())
        .expect("Failed to create RocksDB for BspProvider");

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
        let fs = RocksDBForestStorage::<
            StorageProofsMerkleTrieLayout,
            kvdb_rocksdb::Database,
        >::rocksdb_storage(self.storage_path.clone().expect("Storage path should be set for RocksDB implementation"))
        .expect("Failed to create RocksDB for BspProvider");

        let fs = RocksDBForestStorage::new(fs).expect("Failed to create Forest Storage");

        let fs = Arc::new(RwLock::new(fs));
        self.fs_instance = fs.clone();
        fs
    }

    async fn remove_forest_storage(&mut self, _key: &Self::Key) {}
}

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
        let fs = InMemoryForestStorage::new();

        let fs = Arc::new(RwLock::new(fs));

        self.fs_instances
            .write()
            .await
            .insert(key.clone(), fs.clone());
        fs
    }

    async fn remove_forest_storage(&mut self, key: &Self::Key) {
        self.fs_instances.write().await.remove(key);
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
        let fs = RocksDBForestStorage::<
            StorageProofsMerkleTrieLayout,
            kvdb_rocksdb::Database,
        >::rocksdb_storage(self.storage_path.clone().expect("Storage path should be set for RocksDB implementation"))
        .expect("Failed to create RocksDB for BspProvider");

        let fs = RocksDBForestStorage::new(fs).expect("Failed to create Forest Storage");

        let fs = Arc::new(RwLock::new(fs));

        self.fs_instances
            .write()
            .await
            .insert(key.clone(), fs.clone());
        fs
    }

    async fn remove_forest_storage(&mut self, key: &Self::Key) {
        self.fs_instances.write().await.remove(key);
    }
}
