use common::types::HasherOutT;
use trie_db::CError;

pub(crate) type ErrorT<T> = Error<HasherOutT<T>, CError<T>>;

type BoxTrieError<H, CodecError> = Box<trie_db::TrieError<H, CodecError>>;

#[derive(thiserror::Error, Debug)]
pub enum Error<H, CodecError> {
    #[error("Forest storage error: {0}")]
    ForestStorage(ForestStorageError<H>),
    #[error(transparent)]
    Codec(#[from] codec::Error),
    #[error(transparent)]
    TrieError(BoxTrieError<H, CodecError>),
    #[error(transparent)]
    CompactProofError(#[from] sp_trie::CompactProofError<H, sp_trie::Error<H>>),
}

impl<H, CodecError> From<BoxTrieError<H, CodecError>> for Error<H, CodecError> {
    fn from(x: BoxTrieError<H, CodecError>) -> Self {
        Error::TrieError(x)
    }
}

impl<H, CodecError> From<ForestStorageError<H>> for Error<H, CodecError> {
    fn from(x: ForestStorageError<H>) -> Self {
        Error::ForestStorage(x)
    }
}

/// Error type for the in-memory forest storage.
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum ForestStorageError<H> {
    #[error("Failed to create trie iterator")]
    FailedToCreateTrieIterator,
    #[error("Failed to seek to the challenged file key: ({0:x?})")]
    FailedToSeek(H),
    #[error("Failed to read leaf: ({0:x?})")]
    FailedToReadLeaf(H),
    #[error("Failed to insert file key: ({0:x?})")]
    FailedToInsertFileKey(H),
    #[error("Expecting root to be in storage")]
    ExpectingRootToBeInStorage,
    #[error("Failed to parse root")]
    FailedToParseRoot,
    #[error("Failed to read storage")]
    FailedToReadStorage,
    #[error("Failed to write to storage")]
    FailedToWriteToStorage,
    #[error("Failed to decode value")]
    FailedToDecodeValue,
    #[error("Failed to encode value")]
    FailedToEncodeValue,
    #[error("Failed to generate compact proof")]
    FailedToGenerateCompactProof,
    #[error("Failed to insert file key: ({0:x?})")]
    FileKeyAlreadyExists(H),
    #[error("Failed to get leaf or leaves to prove")]
    FailedToGetLeafOrLeavesToProve,
    #[error("Failed to remove file key: ({0:x?})")]
    FailedToRemoveFileKey(H),
    #[error("Invalid proving scenario")]
    InvalidProvingScenario,
    #[error("Failed to get file key: ({0:x?})")]
    FailedToGetFileKey(H),
    #[error("Failed to construct proven leaves")]
    FailedToConstructProvenLeaves,
}
