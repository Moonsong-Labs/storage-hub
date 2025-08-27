use std::io;

use shc_common::telemetry_error::{ErrorCategory, TelemetryErrorCategory};
use shc_common::types::HasherOutT;
use trie_db::CError;

use crate::traits::{FileStorageError, FileStorageWriteError};

pub(crate) type ErrorT<T> = Error<HasherOutT<T>, CError<T>>;

type BoxTrieError<H, CodecError> = Box<trie_db::TrieError<H, CodecError>>;

#[derive(thiserror::Error, Debug)]
pub enum Error<H, CodecError> {
    #[error("File storage error: {0:?}")]
    FileStorage(FileStorageError),
    #[error("File storage write error: {0:?}")]
    FileStorageWrite(FileStorageWriteError),
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

impl<H, CodecError> From<FileStorageError> for Error<H, CodecError> {
    fn from(x: FileStorageError) -> Self {
        Error::FileStorage(x)
    }
}

impl<H, CodecError> From<FileStorageWriteError> for Error<H, CodecError> {
    fn from(x: FileStorageWriteError) -> Self {
        Error::FileStorageWrite(x)
    }
}

pub fn other_io_error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

impl TelemetryErrorCategory for FileStorageError {
    fn telemetry_category(&self) -> ErrorCategory {
        match self {
            Self::FileAlreadyExists
            | Self::FileDoesNotExist
            | Self::IncompleteFile
            | Self::FileIsEmpty
            | Self::FingerprintAndStoredFileMismatch
            | Self::FailedToParseKey
            | Self::FailedToParseFileMetadata
            | Self::FailedToParseFingerprint
            | Self::FailedToParseChunkWithId
            | Self::FailedToConstructFileKeyProof => ErrorCategory::FileOperation,

            Self::FailedToReadStorage
            | Self::FailedToWriteToStorage
            | Self::FailedToInsertFileChunk
            | Self::FailedToGetFileChunk
            | Self::FailedToDeleteFileChunk
            | Self::FileChunkAlreadyExists
            | Self::FileChunkDoesNotExist
            | Self::FailedToConstructTrieIter
            | Self::FailedToParsePartialRoot
            | Self::FailedToHasherOutput
            | Self::FailedToAddEntityToExcludeList
            | Self::FailedToAddEntityFromExcludeList
            | Self::ErrorParsingExcludeType => ErrorCategory::Storage,

            Self::FailedToGenerateCompactProof => ErrorCategory::Proof,
        }
    }
}

impl TelemetryErrorCategory for FileStorageWriteError {
    fn telemetry_category(&self) -> ErrorCategory {
        match self {
            Self::FileDoesNotExist
            | Self::FileChunkAlreadyExists
            | Self::FingerprintAndStoredFileMismatch
            | Self::FailedToContructFileTrie
            | Self::FailedToParseFileMetadata
            | Self::FailedToParseFingerprint
            | Self::FailedToParsePartialRoot => ErrorCategory::FileOperation,

            Self::FailedToInsertFileChunk
            | Self::FailedToGetFileChunk
            | Self::FailedToPersistChanges
            | Self::FailedToDeleteRoot
            | Self::FailedToDeleteChunk
            | Self::FailedToConstructTrieIter
            | Self::FailedToReadStorage
            | Self::FailedToUpdatePartialRoot
            | Self::FailedToGetStoredChunksCount => ErrorCategory::Storage,

            Self::ChunkCountOverflow => ErrorCategory::Capacity,
        }
    }
}
