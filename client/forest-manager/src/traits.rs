use std::{fmt::Debug, hash};

use hash_db::MaybeDebug;
use sp_core::serde::de::DeserializeOwned;
use storage_hub_infra::types::ForestProof;

use crate::types::ForestStorageErrors;

/// Forest storage interface to be implemented by the storage providers.
pub trait ForestStorage {
    /// Lookup key type used query trie nodes.
    type LookupKey: AsRef<[u8]>
        + AsMut<[u8]>
        + Default
        + MaybeDebug
        + core::cmp::Ord
        + PartialEq
        + Eq
        + hash::Hash
        + Send
        + Sync
        + Clone
        + Copy;
    /// Raw key type used to construct a [`Leaf`](storage_hub_infra::types::Leaf) read from the trie.
    type RawKey: AsRef<[u8]> + From<Vec<u8>> + Clone;
    /// Value type stored in the trie leaves.
    type Value: From<Vec<u8>> + DeserializeOwned + Clone + Debug + Send;

    /// Get file key metadata.
    fn get_file_key(
        &self,
        key: &Self::LookupKey,
    ) -> Result<Option<Self::Value>, ForestStorageErrors>;

    /// Generate proof for file key(s).
    fn generate_proof(
        &self,
        challenged_key: &Vec<Self::LookupKey>,
    ) -> Result<ForestProof<Self::RawKey>, ForestStorageErrors>;

    /// Insert a file key and generate a proof for it.
    fn insert_file_key(
        &mut self,
        file_key: &Self::RawKey,
        value: &Self::Value,
    ) -> Result<Self::LookupKey, ForestStorageErrors>;

    /// Delete a file key and generate a proof for it.
    fn delete_file_key(&mut self, file_key: &Self::LookupKey) -> Result<(), ForestStorageErrors>;
}
