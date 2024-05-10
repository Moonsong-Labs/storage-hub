#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Forest storage error: {0}")]
    ForestStorage(#[from] ForestStorageError),
    #[error(transparent)]
    Codec(#[from] codec::Error),
}

/// Error type for the in-memory forest storage.
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum ForestStorageError {
    /// Failed to create trie iterator.
    #[error("Failed to create trie iterator")]
    FailedToCreateTrieIterator,
    /// Failed to seek to the challenged file key.
    #[error("Failed to seek to the challenged file key")]
    FailedToSeek,
    /// Failed to read leaf.
    #[error("Failed to read leaf")]
    FailedToReadLeaf,
    /// Failed to insert file key.
    #[error("Failed to insert file key")]
    FailedToInsertFileKey,
    /// Expecting root to be in storage.
    #[error("Expecting root to be in storage")]
    ExpectingRootToBeInStorage,
    /// Failed to parse root.
    #[error("Failed to parse root")]
    FailedToParseRoot,
    /// Failed to read storage.
    #[error("Failed to read storage")]
    FailedToReadStorage,
    /// Failed to write to storage.
    #[error("Failed to write to storage")]
    FailedToWriteToStorage,
    /// Failed to deserialize value.
    #[error("Failed to deserialize value")]
    FailedToDecodeValue,
    /// Failed to serialize value.
    #[error("Failed to serialize value")]
    FailedToSerializeValue,
    /// Failed to generate compact proof.
    #[error("Failed to generate compact proof")]
    FailedToGenerateCompactProof,
    /// Failed to insert file key.
    #[error("Failed to insert file key")]
    FileKeyAlreadyExists,
    /// Failed to get leaf or leaves to prove.
    #[error("Failed to get leaf or leaves to prove")]
    FailedToGetLeafOrLeavesToProve,
    /// Failed to remove file key.
    #[error("Failed to remove file key")]
    FailedToRemoveFileKey,
    /// Invalid proving scenario.
    #[error("Invalid proving scenario")]
    InvalidProvingScenario,
    /// Failed to get file key.
    #[error("Failed to get file key")]
    FailedToGetFileKey,
    /// Failed to construct proven leaves.
    ///
    /// This will normally happen if both left and right leaves are `None`.
    #[error("Failed to construct proven leaves")]
    FailedToConstructProvenLeaves,
}
