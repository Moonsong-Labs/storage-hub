use std::fmt::Debug;

use sp_core::serde::de::DeserializeOwned;
use storage_hub_infra::types::ForestProof;

use crate::types::ForestStorageErrors;

/// Forest storage interface to be implemented by the storage providers.
pub trait ForestStorage: 'static {
    /// Lookup key type used perform operations on the trie.
    type LookupKey: AsRef<[u8]>;
    /// Raw key type used to construct a [`Leaf`](storage_hub_infra::types::Leaf) read from the trie.
    type RawKey: AsRef<[u8]> + From<Vec<u8>> + Clone;
    /// Value type stored in the trie leaves.
    type Value: DeserializeOwned + Clone + Debug;

    /// Get value for a file.
    fn get_value(&self, key: &Self::LookupKey) -> Result<Option<Self::Value>, ForestStorageErrors>;

    /// Generate proof for file key(s).
    fn generate_proof(
        &self,
        challenged_key: &Self::LookupKey,
    ) -> Result<ForestProof<Self::RawKey>, ForestStorageErrors>;

    /// Insert a file key and generate a proof for it.
    fn insert_file_key(
        &mut self,
        file_key: &Self::LookupKey,
        value: &Self::Value,
    ) -> Result<ForestProof<Self::RawKey>, ForestStorageErrors>;

    /// Delete a file key and generate a proof for it.
    fn delete_file_key(
        &mut self,
        file_key: &Self::LookupKey,
    ) -> Result<ForestProof<Self::RawKey>, ForestStorageErrors>;
}
