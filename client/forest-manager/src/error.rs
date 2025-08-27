use shc_common::telemetry_error::{ErrorCategory, TelemetryErrorCategory};
use shc_common::types::HasherOutT;
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

/// Error type for the forest storage implementations.
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
    #[error("Failed to parse key")]
    FailedToParseKey,
    #[error("Failed to read storage")]
    FailedToReadStorage,
    #[error("Failed to write to storage")]
    FailedToWriteToStorage,
    #[error("Failed to decode value")]
    FailedToDecodeValue,
    #[error("Failed to generate compact proof")]
    FailedToGenerateCompactProof,
    #[error("Failed to insert file key: ({0:x?})")]
    FileKeyAlreadyExists(H),
    #[error("Invalid proving scenario")]
    InvalidProvingScenario,
    #[error("Failed to construct proven leaves")]
    FailedToConstructProvenLeaves,
    #[error("Failed to copy RocksDB database to another directory")]
    FailedToCopyRocksDB,
}

impl<H> TelemetryErrorCategory for ForestStorageError<H> {
    fn telemetry_category(&self) -> ErrorCategory {
        match self {
            Self::FailedToCreateTrieIterator
            | Self::FailedToSeek(_)
            | Self::FailedToReadLeaf(_)
            | Self::FailedToInsertFileKey(_)
            | Self::FileKeyAlreadyExists(_)
            | Self::FailedToParseKey
            | Self::FailedToDecodeValue
            | Self::FailedToConstructProvenLeaves => ErrorCategory::ForestOperation,

            Self::ExpectingRootToBeInStorage
            | Self::FailedToReadStorage
            | Self::FailedToWriteToStorage
            | Self::FailedToCopyRocksDB => ErrorCategory::Storage,

            Self::FailedToGenerateCompactProof | Self::InvalidProvingScenario => {
                ErrorCategory::Proof
            }
        }
    }
}
