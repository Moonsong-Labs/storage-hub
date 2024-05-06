use trie_db::{Hasher, TrieLayout};

/// The hash type of trie node keys
pub type HashT<T> = <T as TrieLayout>::Hash;
pub type HasherOutT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;

pub struct RawKey<T> {
    pub key: Vec<u8>,
    pub _phantom: std::marker::PhantomData<T>,
}

impl<T> RawKey<T> {
    pub fn new(key: Vec<u8>) -> Self {
        Self {
            key,
            _phantom: Default::default(),
        }
    }
}

impl<T> Clone for RawKey<T> {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            _phantom: Default::default(),
        }
    }
}

impl<T> From<Vec<u8>> for RawKey<T> {
    fn from(key: Vec<u8>) -> Self {
        Self {
            key,
            _phantom: Default::default(),
        }
    }
}

impl<T> AsRef<[u8]> for RawKey<T> {
    fn as_ref(&self) -> &[u8] {
        &self.key
    }
}

/// Error type for the in-memory forest storage.
#[derive(Debug)]
pub enum ForestStorageErrors {
    /// Failed to create trie iterator.
    FailedToCreateTrieIterator,
    /// Failed to seek to the challenged file key.
    FailedToSeek,
    /// Failed to read leaf.
    FailedToReadLeaf,
    /// Failed to insert file key.
    FailedToInsertFileKey,
    /// Expecting root to be in storage.
    ExpectingRootToBeInStorage,
    /// Failed to parse root.
    FailedToParseRoot,
    /// Failed to read storage.
    FailedToReadStorage,
    /// Failed to deserialize value.
    FailedToDeserializeValue,
    /// Failed to serialize value.
    FailedToSerializeValue,
    /// Failed to generate compact proof.
    FailedToGenerateCompactProof,
    /// Failed to insert file key.
    FileKeyAlreadyExists,
    /// Failed to get leaf or leaves to prove.
    FailedToGetLeafOrLeavesToProve,
    /// Failed to remove file key.
    FailedToRemoveFileKey,
    /// Invalid proving scenario.
    InvalidProvingScenario,
    /// Failed to get file key.
    FailedToGetFileKey,
    /// Failed to construct proven leaves.
    ///
    /// This will normally happen if both left and right leaves are `None`.
    FailedToConstructProvenLeaves,
}
