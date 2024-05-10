use common::types::{ForestProof, HasherOutT, Metadata};
use trie_db::TrieLayout;

use crate::error::Error;

/// Forest storage interface to be implemented by the storage providers.
pub trait ForestStorage<T: TrieLayout> {
    /// Get `file_key` from storage and decode the value.
    fn get_metadata(&self, file_key: &HasherOutT<T>) -> Result<Option<Metadata>, Error>;

    /// Generate proof for file key(s).
    fn generate_proof(&self, challenged_key: Vec<HasherOutT<T>>) -> Result<ForestProof<T>, Error>;

    /// Insert metadata and get back the file key (hash of the metadata).
    fn insert_metadata(&mut self, metadata: &Metadata) -> Result<HasherOutT<T>, Error>;

    /// Delete a file key and generate a proof for it.
    fn delete_file_key(&mut self, file_key: &HasherOutT<T>) -> Result<(), Error>;
}
