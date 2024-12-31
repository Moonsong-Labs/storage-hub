// TODO: Remove this once we don't need the examples in this file
#![allow(dead_code)]

pub mod bsp_charge_fees;
pub mod bsp_delete_file;
pub mod bsp_download_file;
pub mod bsp_move_bucket;
pub mod bsp_submit_proof;
pub mod bsp_upload_file;
pub mod mock_bsp_volunteer;
pub mod mock_sp_react_to_event;
pub mod msp_charge_fees;
pub mod msp_delete_bucket;
pub mod msp_move_bucket;
pub mod msp_upload_file;
pub mod sp_slash_provider;
pub mod user_sends_file;

use kvdb::KeyValueDB;
use shc_common::types::StorageProofsMerkleTrieLayout;
use shc_file_manager::{
    in_memory::InMemoryFileStorage, rocksdb::RocksDbFileStorage, traits::FileStorage,
};
use shc_forest_manager::{
    in_memory::InMemoryForestStorage, rocksdb::RocksDBForestStorage, traits::ForestStorageHandler,
};

use crate::services::{forest_storage::ForestStorageCaching, handler::StorageHubHandler};

pub trait FileStorageT: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync {}
impl FileStorageT for InMemoryFileStorage<StorageProofsMerkleTrieLayout> {}
impl<DB> FileStorageT for RocksDbFileStorage<StorageProofsMerkleTrieLayout, DB> where
    DB: KeyValueDB + 'static
{
}

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
