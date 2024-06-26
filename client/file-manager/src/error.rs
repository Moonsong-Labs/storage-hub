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
