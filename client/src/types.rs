use kvdb::KeyValueDB;
use shc_common::{traits::StorageEnableRuntime, types::StorageProofsMerkleTrieLayout};
use shc_file_manager::{
    in_memory::InMemoryFileStorage, rocksdb::RocksDbFileStorage, traits::FileStorage,
};
use shc_forest_manager::{
    in_memory::InMemoryForestStorage, rocksdb::RocksDBForestStorage, traits::ForestStorageHandler,
};

use super::forest_storage::{ForestStorageCaching, ForestStorageSingle, NoKey};

/// A StorageHub node must [`FileStorage`](shc_file_manager::traits::FileStorage) and a [`ForestStorageHandler`]
/// to store and retrieve Files and Forests, respectively.
///
/// A set of [`ShRole`] and [`ShStorageLayer`] can define a [`ShNodeType`], therefore this trait is implemented
/// for combinations of [`ShRole`] and [`ShStorageLayer`].
pub trait ShNodeType<Runtime: StorageEnableRuntime> {
    type FL: FileStorageT;
    type FSH: ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static;
}

impl<Runtime: StorageEnableRuntime> ShNodeType<Runtime> for (BspProvider, InMemoryStorageLayer) {
    type FL = InMemoryFileStorage<StorageProofsMerkleTrieLayout>;
    type FSH = ForestStorageCaching<
        Vec<u8>,
        InMemoryForestStorage<StorageProofsMerkleTrieLayout>,
        Runtime,
    >;
}

impl<Runtime: StorageEnableRuntime> ShNodeType<Runtime> for (BspProvider, RocksDbStorageLayer) {
    type FL = RocksDbFileStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>;
    type FSH = ForestStorageCaching<
        Vec<u8>,
        RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>,
        Runtime,
    >;
}

impl<Runtime: StorageEnableRuntime> ShNodeType<Runtime> for (MspProvider, InMemoryStorageLayer) {
    type FL = InMemoryFileStorage<StorageProofsMerkleTrieLayout>;
    type FSH = ForestStorageCaching<
        Vec<u8>,
        InMemoryForestStorage<StorageProofsMerkleTrieLayout>,
        Runtime,
    >;
}

impl<Runtime: StorageEnableRuntime> ShNodeType<Runtime> for (MspProvider, RocksDbStorageLayer) {
    type FL = RocksDbFileStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>;
    type FSH = ForestStorageCaching<
        Vec<u8>,
        RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>,
        Runtime,
    >;
}

// TODO: Implement default empty implementations for the forest storage handler since the user role only needs the file storage.
/// There is no default empty implementation for [`FileStorageT`] and [`ForestStorageHandler`] so
/// we use the in-memory storage layers which won't be used by the user role.
impl<Runtime: StorageEnableRuntime> ShNodeType<Runtime> for (UserRole, NoStorageLayer) {
    type FL = InMemoryFileStorage<StorageProofsMerkleTrieLayout>;
    type FSH = ForestStorageSingle<InMemoryForestStorage<StorageProofsMerkleTrieLayout>, Runtime>;
}

/// FishermanRole uses ForestStorageSingle for processing file deletions
impl<Runtime: StorageEnableRuntime> ShNodeType<Runtime> for (FishermanRole, NoStorageLayer) {
    type FL = InMemoryFileStorage<StorageProofsMerkleTrieLayout>;
    type FSH = ForestStorageSingle<InMemoryForestStorage<StorageProofsMerkleTrieLayout>, Runtime>;
}

/// Supported roles used in the StorageHub system implement this trait.
///
/// Currently supported roles are:
/// - [`BspProvider`]
/// - [`MspProvider`]
/// - [`UserRole`] (only for testing)
pub trait ShRole {}

/// Backup Storage Provider (BSP) role. Implements the [`ShRole`] trait.
pub struct BspProvider;
impl ShRole for BspProvider {}

/// Main Storage Provider (MSP) role. Implements the [`ShRole`] trait.
pub struct MspProvider;
impl ShRole for MspProvider {}

/// User role. Implements the [`ShRole`] trait.
/// Only used for testing.
pub struct UserRole;
impl ShRole for UserRole {}

/// Fisherman role. Implements the [`ShRole`] trait.
/// Used for monitoring and processing file deletion requests.
pub struct FishermanRole;
impl ShRole for FishermanRole {}

/// Storage layers supported by the StorageHub system.
///
/// Currently supported storage layers are:
/// - [`RocksDbStorageLayer`]
/// - [`InMemoryStorageLayer`]
/// - [`NoStorageLayer`]
pub trait ShStorageLayer {}

/// RocksDB storage layer. Implements the [`ShStorageLayer`] trait.
///
/// Stores data in a RocksDB key-value database. Efficient for Merkle Patricia Trie (MPT) data.
pub struct RocksDbStorageLayer;
impl ShStorageLayer for RocksDbStorageLayer {}

/// In-memory storage layer. Implements the [`ShStorageLayer`] trait.
/// Used for testing. Stored data is lost when the node is stopped.
pub struct InMemoryStorageLayer;
impl ShStorageLayer for InMemoryStorageLayer {}

/// No storage layer. Implements the [`ShStorageLayer`] trait.
/// Used for testing alongside the [`UserRole`].
pub struct NoStorageLayer;
impl ShStorageLayer for NoStorageLayer {}

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
pub trait BspForestStorageHandlerT<Runtime: StorageEnableRuntime>:
    ForestStorageHandler<Runtime, Key = Vec<u8>> + Clone + Send + Sync + 'static
{
}
impl<Runtime: StorageEnableRuntime> BspForestStorageHandlerT<Runtime>
    for ForestStorageCaching<Vec<u8>, InMemoryForestStorage<StorageProofsMerkleTrieLayout>, Runtime>
{
}
impl<Runtime: StorageEnableRuntime> BspForestStorageHandlerT<Runtime>
    for ForestStorageCaching<
        Vec<u8>,
        RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>,
        Runtime,
    >
{
}

/// The type of Forest Storage handler used by an MSP implements this trait.
pub trait MspForestStorageHandlerT<Runtime: StorageEnableRuntime>:
    ForestStorageHandler<Runtime, Key = Vec<u8>> + Clone + Send + Sync + 'static
{
}
impl<Runtime: StorageEnableRuntime> MspForestStorageHandlerT<Runtime>
    for ForestStorageCaching<Vec<u8>, InMemoryForestStorage<StorageProofsMerkleTrieLayout>, Runtime>
{
}
impl<Runtime: StorageEnableRuntime> MspForestStorageHandlerT<Runtime>
    for ForestStorageCaching<
        Vec<u8>,
        RocksDBForestStorage<StorageProofsMerkleTrieLayout, kvdb_rocksdb::Database>,
        Runtime,
    >
{
}

/// The type of Forest Storage handler used by a Fisherman implements this trait.
pub trait FishermanForestStorageHandlerT<Runtime: StorageEnableRuntime>:
    ForestStorageHandler<Runtime, Key = NoKey> + Clone + Send + Sync + 'static
{
}
impl<Runtime: StorageEnableRuntime> FishermanForestStorageHandlerT<Runtime>
    for ForestStorageSingle<InMemoryForestStorage<StorageProofsMerkleTrieLayout>, Runtime>
{
}
