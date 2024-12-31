use kvdb::KeyValueDB;
use shc_common::types::StorageProofsMerkleTrieLayout;
use shc_file_manager::{
    in_memory::InMemoryFileStorage, rocksdb::RocksDbFileStorage, traits::FileStorage,
};
use shc_forest_manager::{
    in_memory::InMemoryForestStorage, rocksdb::RocksDBForestStorage, traits::ForestStorageHandler,
};

use super::forest_storage::ForestStorageCaching;

/// File Storage trait used in StorageHub services.
///
/// This trait makes the [`FileStorage`] trait's generic type parameter concrete, and sets
/// it to the [`StorageProofsMerkleTrieLayout`] used in StorageHub.
pub trait FileStorageT: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync {}
impl FileStorageT for InMemoryFileStorage<StorageProofsMerkleTrieLayout> {}
impl<DB> FileStorageT for RocksDbFileStorage<StorageProofsMerkleTrieLayout, DB> where
    DB: KeyValueDB + 'static
{
}

/// The type of Forest Storage handler used by a BSP implements this trait.
pub trait BspForestStorageHandlerT:
    ForestStorageHandler<Key = Vec<u8>> + Clone + Send + Sync + 'static
{
}
impl BspForestStorageHandlerT
    for ForestStorageCaching<Vec<u8>, InMemoryForestStorage<StorageProofsMerkleTrieLayout>>
{
}
impl BspForestStorageHandlerT
    for ForestStorageCaching<
        Vec<u8>,
        RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>,
    >
{
}

/// The type of Forest Storage handler used by an MSP implements this trait.
pub trait MspForestStorageHandlerT:
    ForestStorageHandler<Key = Vec<u8>> + Clone + Send + Sync + 'static
{
}
impl MspForestStorageHandlerT
    for ForestStorageCaching<Vec<u8>, InMemoryForestStorage<StorageProofsMerkleTrieLayout>>
{
}
impl MspForestStorageHandlerT
    for ForestStorageCaching<
        Vec<u8>,
        RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>,
    >
{
}
