use trie_db::{Hasher, TrieLayout};

/// The hash type of trie node keys
pub(crate) type HashT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;

/// Error type for the in-memory forest storage.
#[derive(Debug)]
pub enum ForestStorageErrors {
    /// Failed to create trie iterator.
    FailedToCreateTrieIterator,
    /// Failed to seek to the challenged file key.
    FailedToSeek,
    /// Failed to read leaf.
    FailedToReadLeaf,
    /// Failed to deserialize value.
    FailedToDeserializeValue,
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
}
