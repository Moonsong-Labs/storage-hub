use async_trait::async_trait;
use serde::de::DeserializeOwned;
use shc_common::types::StorageProofsMerkleTrieLayout;
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use std::{collections::HashMap, fmt::Debug, hash::Hash, sync::Arc};
use tokio::sync::RwLock;

pub struct ForestStorageSingle<FS>
where
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
{
    fs_instance: Arc<RwLock<FS>>,
}

impl<FS> Clone for ForestStorageSingle<FS>
where
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
{
    fn clone(&self) -> Self {
        Self {
            fs_instance: self.fs_instance.clone(),
        }
    }
}

impl<FS> ForestStorageSingle<FS>
where
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
{
    pub fn new(fs: FS) -> Self {
        Self {
            fs_instance: Arc::new(RwLock::new(fs)),
        }
    }
}

#[async_trait]
impl<FS> ForestStorageHandler for ForestStorageSingle<FS>
where
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
{
    type Key = ();
    type FS = FS;

    async fn get(&self, _key: &Self::Key) -> Option<Arc<RwLock<Self::FS>>> {
        Some(self.fs_instance.clone())
    }

    async fn insert(&mut self, _key: &Self::Key, fs: Self::FS) -> Arc<RwLock<Self::FS>> {
        let fs = Arc::new(RwLock::new(fs));
        self.fs_instance = fs.clone();
        fs
    }

    async fn remove_forest_storage(&mut self, _key: &Self::Key) {}
}

pub struct ForestStorageCaching<K, FS>
where
    K: Eq + Hash + Send + Sync,
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
{
    fs_instances: Arc<RwLock<HashMap<K, Arc<RwLock<FS>>>>>,
}

impl<K, FS> Clone for ForestStorageCaching<K, FS>
where
    K: Eq + Hash + Send + Sync,
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
{
    fn clone(&self) -> Self {
        Self {
            fs_instances: self.fs_instances.clone(),
        }
    }
}

impl<K, FS> ForestStorageCaching<K, FS>
where
    K: Eq + Hash + Send + Sync,
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
{
    pub fn new() -> Self {
        Self {
            fs_instances: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl<K, FS> ForestStorageHandler for ForestStorageCaching<K, FS>
where
    K: Eq + Hash + DeserializeOwned + Debug + Clone + Send + Sync + 'static,
    FS: ForestStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
{
    type Key = K;
    type FS = FS;

    async fn get(&self, key: &Self::Key) -> Option<Arc<RwLock<Self::FS>>> {
        self.fs_instances.read().await.get(key).cloned()
    }

    async fn insert(&mut self, key: &Self::Key, fs: Self::FS) -> Arc<RwLock<Self::FS>> {
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
