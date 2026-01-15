use std::{fmt::Debug, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use shc_common::{
    traits::StorageEnableRuntime,
    types::{FileMetadata, ForestProof, HasherOutT, StorageProofsMerkleTrieLayout},
};
use tokio::sync::RwLock;
use trie_db::TrieLayout;

use crate::error::ErrorT;

/// Forest storage interface to be implemented by the storage providers.
pub trait ForestStorage<T: TrieLayout, Runtime: StorageEnableRuntime>: 'static {
    /// Get the root hash of the forest.
    fn root(&self) -> HasherOutT<T>;
    /// Check if the file key exists in the storage.
    fn contains_file_key(&self, file_key: &HasherOutT<T>) -> Result<bool, ErrorT<T>>;
    /// Get the file metadata for a file key.
    fn get_file_metadata(
        &self,
        file_key: &HasherOutT<T>,
    ) -> Result<Option<FileMetadata>, ErrorT<T>>;
    /// Get all files stored in this forest.
    ///
    /// Returns a vector of `(file_key, file_metadata)` pairs.
    fn get_all_files(&self) -> Result<Vec<(HasherOutT<T>, FileMetadata)>, ErrorT<T>>;
    /// Generate proof for file key(s).
    fn generate_proof(
        &self,
        challenged_key: Vec<HasherOutT<T>>,
    ) -> Result<ForestProof<T>, ErrorT<T>>;
    /// Insert files metadata and get back the file keys (hash of the metadata) that were inserted.
    ///
    /// If an empty vector is passed, the method will return an empty vector.
    fn insert_files_metadata(
        &mut self,
        files_metadata: &[FileMetadata],
    ) -> Result<Vec<HasherOutT<T>>, ErrorT<T>>;
    /// Delete a file key.
    fn delete_file_key(&mut self, file_key: &HasherOutT<T>) -> Result<(), ErrorT<T>>;
    /// Get all the files that belong to a particular user.
    fn get_files_by_user(
        &self,
        user: &Runtime::AccountId,
    ) -> Result<Vec<(HasherOutT<T>, FileMetadata)>, ErrorT<T>>;
}

/// Handler to manage file storage instances.
///
/// The key is optional in all methods, allowing for a single ForestStorage instance to be managed without a key.
#[async_trait]
pub trait ForestStorageHandler<Runtime: StorageEnableRuntime> {
    /// The key type used to identify forest storage instances.
    type Key: From<Vec<u8>> + AsRef<[u8]> + Debug + Send + Sync;
    /// Type representing the forest storage instance.
    type FS: ForestStorage<StorageProofsMerkleTrieLayout, Runtime> + Send + Sync;

    /// Get forest storage instance.
    async fn get(&self, key: &Self::Key) -> Option<Arc<RwLock<Self::FS>>>;
    /// Create a new forest storage instance.
    async fn create(&mut self, key: &Self::Key) -> Result<Arc<RwLock<Self::FS>>>;
    /// Remove forest storage instance.
    async fn remove_forest_storage(&mut self, key: &Self::Key);

    /// Create a copy (snapshot) of the forest storage instance.
    ///
    /// Returns `Some` with the copied forest storage instance for `key` if it exists,
    /// otherwise returns `None`.
    /// The instance returned is the one corresponding to `key`, not the one corresponding to `key_for_copy`.
    async fn snapshot(
        &self,
        src_key: &Self::Key,
        dest_key: &Self::Key,
    ) -> Option<Arc<RwLock<Self::FS>>>;

    /// Get or create forest storage instance.
    async fn get_or_create(&mut self, key: &Self::Key) -> Result<Arc<RwLock<Self::FS>>> {
        if let Some(forest_storage) = self.get(key).await {
            Ok(forest_storage)
        } else {
            self.create(key).await
        }
    }
}
