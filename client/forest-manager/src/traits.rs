use shc_common::types::{FileMetadata, ForestProof, HasherOutT};
use trie_db::TrieLayout;

use crate::error::ErrorT;

/// Forest storage interface to be implemented by the storage providers.
pub trait ForestStorage<T: TrieLayout> {
    /// Check if the file key exists in the storage.
    fn contains_file_key(&self, file_key: &HasherOutT<T>) -> Result<bool, ErrorT<T>>;
    /// Generate proof for file key(s).
    fn generate_proof(
        &self,
        challenged_key: Vec<HasherOutT<T>>,
    ) -> Result<ForestProof<T>, ErrorT<T>>;
    /// Insert metadata and get back the file key (hash of the metadata).
    fn insert_metadata(&mut self, metadata: &FileMetadata) -> Result<HasherOutT<T>, ErrorT<T>>;
    /// Delete a file key and generate a proof for it.
    fn delete_file_key(&mut self, file_key: &HasherOutT<T>) -> Result<(), ErrorT<T>>;
}
